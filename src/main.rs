// 오른쪽 아래 팝업 차단기
// - 카카오톡: "광고" 크로미움 뷰를 가진 창을 숨김
// - 그 외 앱: allow.txt 에 없는 프로그램이 화면 오른쪽 아래에 띄우는 팝업을 숨김
// 폴링 없이 WinEvent 훅 + GetMessage 대기만 사용하므로 평소 CPU 사용량은 0 이다.
#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Accessibility::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const KAKAO_EXE: &str = "kakaotalk.exe";
const MAX_W: i32 = 800;
const MAX_H: i32 = 600;
const LOG_LIMIT: u64 = 256 * 1024;
const DEFAULT_ALLOW: &str = "\
# 오른쪽 아래 팝업을 허용할 프로그램의 실행 파일 이름 (한 줄에 하나, 대소문자 무시)
# C:\\Windows 아래의 시스템 프로그램과 Windows 기본 알림은 이 목록과 무관하게 항상 허용됩니다.
# 수정한 뒤에는 프로그램을 다시 실행해야 반영됩니다.
KakaoTalk.exe
";

struct Proc {
    name: String, // 소문자 exe 이름
    system: bool,
    allowed: bool,
}

struct State {
    allow: Vec<String>,
    windir: String,
    cache: HashMap<u32, Proc>,
    log: PathBuf,
    recent: Vec<usize>, // 최근에 숨긴 창 (로그 중복 방지)
    kakao_pid: u32,
    loc_hook: HWINEVENTHOOK, // 카카오톡 프로세스 한정 위치·크기 변경 훅
}

thread_local! {
    static ST: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe fn class_of(h: HWND) -> String {
    let mut b = [0u16; 128];
    let n = unsafe { GetClassNameW(h, b.as_mut_ptr(), b.len() as i32) };
    String::from_utf16_lossy(&b[..n.max(0) as usize])
}

unsafe fn title_of(h: HWND) -> String {
    let mut b = [0u16; 256];
    let n = unsafe { GetWindowTextW(h, b.as_mut_ptr(), b.len() as i32) };
    String::from_utf16_lossy(&b[..n.max(0) as usize])
}

unsafe fn exe_path(pid: u32) -> Option<String> {
    unsafe {
        let hp = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if hp.is_null() {
            return None;
        }
        let mut b = [0u16; 1024];
        let mut n = b.len() as u32;
        let ok = QueryFullProcessImageNameW(hp, 0, b.as_mut_ptr(), &mut n);
        CloseHandle(hp);
        (ok != 0).then(|| String::from_utf16_lossy(&b[..n as usize]).to_lowercase())
    }
}

/// (exe 이름, 시스템 여부, 허용 여부). 프로세스 정보는 pid 별로 캐시한다.
fn classify(pid: u32) -> Option<(String, bool, bool)> {
    ST.with(|s| {
        let mut s = s.try_borrow_mut().ok()?;
        let st = s.as_mut()?;
        if !st.cache.contains_key(&pid) {
            if st.cache.len() > 512 {
                st.cache.clear();
            }
            let p = match unsafe { exe_path(pid) } {
                Some(path) => {
                    let name = path.rsplit('\\').next().unwrap_or("").to_string();
                    Proc {
                        system: path.starts_with(&st.windir),
                        allowed: st.allow.iter().any(|a| *a == name),
                        name,
                    }
                }
                // 정보를 읽을 수 없는 프로세스(권한 상승 등)는 건드리지 않는다.
                None => Proc { name: String::new(), system: true, allowed: true },
            };
            st.cache.insert(pid, p);
        }
        let p = &st.cache[&pid];
        Some((p.name.clone(), p.system, p.allowed))
    })
}

unsafe extern "system" fn find_ad(h: HWND, lp: LPARAM) -> BOOL {
    unsafe {
        if class_of(h) == "Chrome_WidgetWin_1" && title_of(h).contains("광고") {
            *(lp as *mut bool) = true;
            return 0;
        }
    }
    1
}

/// 카카오톡 광고 창: 자식 중 제목에 "광고"가 들어간 크로미움 뷰가 있다 (예: "MOMENT 광고").
unsafe fn has_kakao_ad(root: HWND) -> bool {
    let mut found = false;
    unsafe { EnumChildWindows(root, Some(find_ad), &mut found as *mut bool as LPARAM) };
    found
}

/// 제목 표시줄이 없고 작은 최상위 창이 모니터 작업 영역의 오른쪽 아래 구역에 있는지 확인한다.
/// 슬라이드 인 중이라 일부가 화면 밖에 있어도 걸리도록 오른쪽/아래 경계는 넘어도 허용한다.
unsafe fn is_corner_popup(h: HWND) -> bool {
    unsafe {
        if IsWindowVisible(h) == 0 {
            return false;
        }
        let cls = class_of(h);
        if cls == "#32768" || cls == "tooltips_class32" {
            return false; // 메뉴, 툴팁
        }
        let style = GetWindowLongW(h, GWL_STYLE) as u32;
        if style & WS_CAPTION == WS_CAPTION {
            return false; // 일반 창
        }
        let mut r: RECT = zeroed();
        GetWindowRect(h, &mut r);
        let (w, hh) = (r.right - r.left, r.bottom - r.top);
        if w <= 0 || hh <= 0 || w > MAX_W || hh > MAX_H {
            return false;
        }
        let mon = MonitorFromRect(&r, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = zeroed();
        mi.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(mon, &mut mi) == 0 {
            return false;
        }
        let wa = mi.rcWork;
        r.left >= (wa.left + wa.right) / 2
            && r.top >= (wa.top + wa.bottom) / 2
            && r.right >= wa.right - 150
    }
}

unsafe fn hide(h: HWND, exe: &str, why: &str) {
    unsafe { ShowWindow(h, SW_HIDE) };
    let line = unsafe {
        let mut r: RECT = zeroed();
        GetWindowRect(h, &mut r);
        let mut t = zeroed();
        GetLocalTime(&mut t);
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}  {}  [{}]  cls={} title={} {}x{}\r\n",
            t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond,
            exe, why, class_of(h), title_of(h), r.right - r.left, r.bottom - r.top
        )
    };
    ST.with(|s| {
        let Ok(mut s) = s.try_borrow_mut() else { return };
        let Some(st) = s.as_mut() else { return };
        let key = h as usize;
        if st.recent.contains(&key) {
            return;
        }
        if st.recent.len() >= 32 {
            st.recent.remove(0);
        }
        st.recent.push(key);
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&st.log) {
            let _ = f.write_all(line.as_bytes());
        }
    });
}

unsafe fn is_kakao_main(h: HWND) -> bool {
    unsafe { class_of(h) == "EVA_Window_Dblclk" && title_of(h) == "카카오톡" }
}

/// 메인 창 하단의 광고 자리(제목 없는 EVA_ChildWindow)를 숨기고,
/// 목록 영역(OnlineMainView/LockModeView)을 창 아래 끝까지 늘린다.
unsafe fn fix_layout(main: HWND) {
    unsafe {
        let mut client: RECT = zeroed();
        GetClientRect(main, &mut client);
        let mut child = GetWindow(main, GW_CHILD);
        while !child.is_null() {
            let mut r: RECT = zeroed();
            GetWindowRect(child, &mut r);
            MapWindowPoints(null_mut(), main, &mut r as *mut RECT as *mut POINT, 2);
            let cls = class_of(child);
            let title = title_of(child);
            if title.starts_with("OnlineMainView") || title.starts_with("LockModeView") {
                if r.bottom != client.bottom {
                    SetWindowPos(
                        child, null_mut(), 0, 0, r.right - r.left, client.bottom - r.top,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            } else if cls == "EVA_ChildWindow"
                && title.is_empty()
                && IsWindowVisible(child) != 0
                && r.bottom >= client.bottom - 2
                && r.bottom - r.top < 150
            {
                ShowWindow(child, SW_HIDE);
            }
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }
}

unsafe extern "system" fn on_location(
    _: HWINEVENTHOOK,
    _: u32,
    hwnd: HWND,
    id_object: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if hwnd.is_null() || id_object != OBJID_WINDOW {
        return;
    }
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        if !root.is_null() && is_kakao_main(root) {
            fix_layout(root);
        }
    }
}

/// 카카오톡 pid 가 바뀌면(재시작 등) 위치·크기 변경 훅을 그 프로세스에 다시 건다.
unsafe fn ensure_location_hook(pid: u32) {
    let old = ST.with(|s| {
        let mut s = s.try_borrow_mut().ok()?;
        let st = s.as_mut()?;
        if st.kakao_pid == pid {
            return None;
        }
        st.kakao_pid = pid;
        Some(std::mem::replace(&mut st.loc_hook, null_mut()))
    });
    let Some(old) = old else { return };
    unsafe {
        if !old.is_null() {
            UnhookWinEvent(old);
        }
        let hook = SetWinEventHook(
            EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, null_mut(),
            Some(on_location), pid, 0, WINEVENT_OUTOFCONTEXT,
        );
        ST.with(|s| {
            if let Ok(mut s) = s.try_borrow_mut() {
                if let Some(st) = s.as_mut() {
                    st.loc_hook = hook;
                }
            }
        });
    }
}

unsafe fn check(root: HWND, corner: bool) {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(root, &mut pid);
        let Some((exe, system, allowed)) = classify(pid) else { return };
        if system {
            return;
        }
        if exe == KAKAO_EXE {
            if is_kakao_main(root) {
                // 메인 창은 절대 숨기지 않고, 하단 광고 자리만 정리한다
                ensure_location_hook(pid);
                fix_layout(root);
                return;
            }
            if has_kakao_ad(root) {
                hide(root, &exe, "카카오 광고");
                return;
            }
        }
        if !allowed && corner && is_corner_popup(root) {
            hide(root, &exe, "오른쪽 아래 팝업");
        }
    }
}

unsafe extern "system" fn on_event(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _: u32,
    _: u32,
) {
    if hwnd.is_null() || id_object != OBJID_WINDOW || id_child != CHILDID_SELF as i32 {
        return;
    }
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        if root.is_null() {
            return;
        }
        // 오른쪽 아래 판정은 최상위 창이 나타날 때만, 카카오 광고 판정은 자식 변화에도 수행한다.
        check(root, event == EVENT_OBJECT_SHOW && root == hwnd);
    }
}

unsafe extern "system" fn scan(h: HWND, _: LPARAM) -> BOOL {
    unsafe { check(h, true) };
    1
}

fn load_state() -> State {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();
    let allow_path = dir.join("allow.txt");
    if !allow_path.exists() {
        let _ = fs::write(&allow_path, DEFAULT_ALLOW);
    }
    let allow = fs::read_to_string(&allow_path)
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let log = dir.join("blocked.log");
    if fs::metadata(&log).map(|m| m.len() > LOG_LIMIT).unwrap_or(false) {
        let _ = fs::remove_file(&log);
    }
    let windir = std::env::var("SystemRoot")
        .unwrap_or_else(|_| "C:\\Windows".into())
        .to_lowercase()
        + "\\";
    State {
        allow,
        windir,
        cache: HashMap::new(),
        log,
        recent: Vec::new(),
        kakao_pid: 0,
        loc_hook: null_mut(),
    }
}

fn main() {
    unsafe {
        // 중복 실행 방지
        let name = wide("Local\\kakao-adblock-single");
        CreateMutexW(null_mut(), 1, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }

        ST.with(|s| *s.borrow_mut() = Some(load_state()));

        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let module: *mut c_void = null_mut();
        SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_SHOW, module, Some(on_event), 0, 0, flags);
        SetWinEventHook(EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE, module, Some(on_event), 0, 0, flags);

        // 이미 떠 있는 창 정리
        EnumWindows(Some(scan), 0);

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            DispatchMessageW(&msg);
        }
    }
}
