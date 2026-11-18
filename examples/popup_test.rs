// 검증용: 오른쪽 아래 팝업(차단 대상)과 왼쪽 위 팝업(대조군)을 띄우고 2초 뒤 표시 여부를 출력한다.
use std::mem::zeroed;
use std::ptr::null_mut;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn main() {
    unsafe {
        let mut wa: RECT = zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut wa as *mut _ as _, 0);
        let cls = wide("STATIC");
        let mk = |x, y, t: &str| {
            let t = wide(t);
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                cls.as_ptr(), t.as_ptr(), WS_POPUP | WS_VISIBLE,
                x, y, 320, 100, null_mut(), null_mut(), null_mut(), null_mut(),
            )
        };
        let corner = mk(wa.right - 330, wa.bottom - 110, "corner");
        let normal = mk(wa.left + 50, wa.top + 50, "normal");
        // 제목 표시줄이 있는 오른쪽 아래 창: 숨기지 않고 [참고]로만 기록되어야 한다
        let t = wide("captioned");
        let captioned = CreateWindowExW(
            WS_EX_TOOLWINDOW, cls.as_ptr(), t.as_ptr(), WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            wa.right - 330, wa.bottom - 250, 320, 120, null_mut(), null_mut(), null_mut(), null_mut(),
        );

        let start = Instant::now();
        let mut msg: MSG = zeroed();
        while start.elapsed() < Duration::from_secs(2) {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                DispatchMessageW(&msg);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        println!("corner visible = {}", IsWindowVisible(corner) != 0);
        println!("normal visible = {}", IsWindowVisible(normal) != 0);
        println!("captioned visible = {}", IsWindowVisible(captioned) != 0);
    }
}
