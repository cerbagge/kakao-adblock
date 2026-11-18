# kakao-adblock AI 작업 가이드

이 문서는 Claude Code, Codex, Cursor, Gemini CLI 같은 **AI 코딩 도구가 읽고 그대로 따라 할 수 있도록** 작성한 작업 지침입니다. 사용자는 명령어를 몰라도, AI 도구에게 이 문서를 넘기기만 하면 설치, 설정, 문제 해결을 맡길 수 있습니다.

## 사용자용: AI에게 이렇게 요청하세요

AI 도구의 대화창에 아래 문장 중 하나를 붙여 넣으면 됩니다.

```
https://raw.githubusercontent.com/cerbagge/kakao-adblock/main/AI_GUIDE.md 문서를 읽고, kakao-adblock을 설치하고 시작 프로그램에 등록해줘.
```

```
https://raw.githubusercontent.com/cerbagge/kakao-adblock/main/AI_GUIDE.md 문서를 읽고, blocked.log를 확인해서 허용 목록에 추가할 프로그램을 알려줘.
```

```
https://raw.githubusercontent.com/cerbagge/kakao-adblock/main/AI_GUIDE.md 문서를 읽고, 카카오톡 광고가 다시 보이는 원인을 진단해줘.
```

AI 도구가 인터넷 주소를 읽지 못하면, 이 파일을 내려받아 대화창에 첨부하시면 됩니다.

---

## AI용: 반드시 지킬 규칙

이 아래부터는 AI 도구에게 주는 지침입니다.

1. **카카오톡 파일을 수정하지 마십시오.** `C:\Program Files\Kakao\` 아래의 파일을 패치하거나 교체하는 작업은 이용약관 위반이며 계정 제재 위험이 있습니다. 이 프로그램은 실행 중인 창을 숨기는 방식만 사용합니다.
2. **실행 파일은 공식 저장소의 Releases에서만 받으십시오.** 출처: `https://github.com/cerbagge/kakao-adblock/releases`. 다른 사이트에 올라온 재배포본은 사용하지 마십시오.
3. **시스템을 바꾸는 작업 전에는 사용자에게 확인하십시오.** 시작 프로그램 등록과 해제, 프로그램 제거, 다른 프로그램 종료가 여기에 해당합니다.
4. **셸은 Windows PowerShell 5.1을 기준으로 합니다.** 이 문서의 명령은 모두 PowerShell 명령입니다.
5. **`allow.txt`는 BOM 없는 UTF-8로 쓰십시오.** PowerShell 5.1의 `Add-Content`/`Set-Content -Encoding UTF8`은 BOM을 붙이므로 사용하지 말고, 아래 예시처럼 `[IO.File]` 메서드를 사용하십시오.
6. 작업을 마치면 무엇을 바꿨는지(파일 경로, 등록 여부)를 사용자에게 보고하십시오.

## 기본 정보

| 항목 | 값 |
|---|---|
| 지원 환경 | Windows 10/11 (x64), 카카오톡 PC |
| 권장 설치 위치 | `%LOCALAPPDATA%\Programs\kakao-adblock` |
| 실행 파일 | `kakao-adblock.exe` (콘솔 창, 트레이 아이콘 없음) |
| 허용 목록 | 실행 파일과 같은 폴더의 `allow.txt` |
| 차단 기록 | 실행 파일과 같은 폴더의 `blocked.log` (256KB 초과 시 다음 실행 때 초기화) |
| 시작 프로그램 바로 가기 | `shell:startup` 폴더의 `kakao-adblock.lnk` |
| 중복 실행 | 불가 (두 번째 실행은 즉시 종료됨) |
| 설정 반영 | `allow.txt` 수정 후 프로그램을 다시 실행해야 반영됨 |

아래 명령들은 공통으로 이 변수를 먼저 정의한다고 가정합니다.

```powershell
$dir = Join-Path $env:LOCALAPPDATA "Programs\kakao-adblock"
```

---

## 작업 1. 설치 (또는 최신 버전으로 업데이트)

```powershell
$dir = Join-Path $env:LOCALAPPDATA "Programs\kakao-adblock"
$rel = Invoke-RestMethod "https://api.github.com/repos/cerbagge/kakao-adblock/releases/latest"
$asset = $rel.assets | Where-Object { $_.name -like "kakao-adblock-*.zip" } | Select-Object -First 1
$zip = Join-Path $env:TEMP $asset.name
Invoke-WebRequest $asset.browser_download_url -OutFile $zip
Stop-Process -Name kakao-adblock -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
New-Item -ItemType Directory -Force $dir | Out-Null
Expand-Archive $zip -DestinationPath $dir -Force
Remove-Item $zip
Start-Process "$dir\kakao-adblock.exe"
"설치 버전: $($rel.tag_name)"
```

- 업데이트할 때도 같은 명령을 사용합니다. 압축 파일에는 `allow.txt`가 들어 있지 않으므로, 사용자가 만든 허용 목록은 그대로 유지됩니다.
- 설치 확인: `Get-Process kakao-adblock`의 결과가 나오면 실행 중입니다.

## 작업 2. 시작 프로그램 등록 / 해제

**등록** (사용자 확인 후 실행):

```powershell
$dir = Join-Path $env:LOCALAPPDATA "Programs\kakao-adblock"
$lnk = Join-Path ([Environment]::GetFolderPath("Startup")) "kakao-adblock.lnk"
$s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
$s.TargetPath = "$dir\kakao-adblock.exe"
$s.WorkingDirectory = $dir
$s.Save()
```

**해제**:

```powershell
Remove-Item (Join-Path ([Environment]::GetFolderPath("Startup")) "kakao-adblock.lnk") -ErrorAction SilentlyContinue
```

## 작업 3. 허용 목록 관리

이 프로그램은 `allow.txt`에 없는 프로그램이 화면 오른쪽 아래에 띄우는 팝업을 숨깁니다. 사용자가 "어떤 알림이 안 떠요"라고 하면 이 작업을 수행하십시오.

**1) 무엇이 차단됐는지 확인**:

```powershell
$dir = Join-Path $env:LOCALAPPDATA "Programs\kakao-adblock"
Get-Content "$dir\blocked.log" -Encoding UTF8 -Tail 30
```

로그 한 줄의 형식은 `날짜 시각  exe이름  [사유]  cls=창클래스 title=창제목 너비x높이`입니다. 사유는 `카카오 광고` 또는 `오른쪽 아래 팝업` 중 하나입니다.

**2) 사용자에게 확인한 뒤 허용 목록에 추가** (예: `Discord.exe`):

```powershell
$dir = Join-Path $env:LOCALAPPDATA "Programs\kakao-adblock"
[IO.File]::AppendAllText("$dir\allow.txt", "`r`nDiscord.exe`r`n", (New-Object Text.UTF8Encoding $false))
Stop-Process -Name kakao-adblock -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Start-Process "$dir\kakao-adblock.exe"
```

**3) 현재 허용 목록 보기**:

```powershell
Get-Content (Join-Path $env:LOCALAPPDATA "Programs\kakao-adblock\allow.txt") -Encoding UTF8
```

참고 사항:
- 줄마다 실행 파일 이름 하나를 적고, `#`으로 시작하는 줄은 주석입니다. 대소문자는 구분하지 않습니다.
- `KakaoTalk.exe`를 지우면 카카오톡 메시지 알림도 차단되므로 지우지 않도록 안내하십시오.
- `C:\Windows` 아래의 시스템 프로그램과 Windows 기본 알림(알림 센터 토스트)은 목록과 무관하게 항상 허용됩니다. Windows 기본 알림을 끄고 싶다는 요청이면 `설정 → 시스템 → 알림`을 안내하십시오.

## 작업 4. 종료 / 제거

**종료**:

```powershell
Stop-Process -Name kakao-adblock
```

**완전 제거** (사용자 확인 후 실행. 허용 목록과 로그도 함께 삭제됩니다):

```powershell
Stop-Process -Name kakao-adblock -ErrorAction SilentlyContinue
Remove-Item (Join-Path ([Environment]::GetFolderPath("Startup")) "kakao-adblock.lnk") -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Remove-Item (Join-Path $env:LOCALAPPDATA "Programs\kakao-adblock") -Recurse -Force
```

## 작업 5. 카카오톡 광고가 다시 보일 때 진단

카카오톡 업데이트로 창 구조가 바뀌면 광고가 다시 보일 수 있습니다. 아래 순서대로 확인하십시오.

**1) 프로그램이 실행 중인지 확인**: `Get-Process kakao-adblock`. 실행 중이 아니면 작업 1의 마지막 줄로 실행합니다.

**2) 최신 버전인지 확인**: 작업 1을 실행해 최신 릴리스로 업데이트합니다.

**3) 광고가 화면에 보이는 상태에서 카카오톡 창 구조를 덤프**:

```powershell
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public static class KW {
  public delegate bool EP(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EP f, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EP f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct R { public int l,t,r,b; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  static string D(IntPtr h) {
    var c = new StringBuilder(256); var t = new StringBuilder(256); R r;
    GetClassName(h, c, 256); GetWindowText(h, t, 256); GetWindowRect(h, out r);
    return String.Format("0x{0:X} cls=[{1}] title=[{2}] vis={3} {4}x{5} at({6},{7}) parent=0x{8:X}",
      h.ToInt64(), c, t, IsWindowVisible(h), r.r - r.l, r.b - r.t, r.l, r.t, GetParent(h).ToInt64());
  }
  public static void Dump(uint pid) {
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) { Console.WriteLine("TOP " + D(h));
        EnumChildWindows(h, (c, l2) => { Console.WriteLine("    " + D(c)); return true; }, IntPtr.Zero); }
      return true; }, IntPtr.Zero);
  }
}
"@
[KW]::Dump([uint32](Get-Process KakaoTalk).Id)
```

**4) 덤프 결과를 아래 "현재 판별 규칙"과 비교**해서 어느 규칙이 더 이상 맞지 않는지 찾습니다. 광고로 보이는 창(크기, 위치, `광고`가 들어간 제목, `Chrome_WidgetWin_1` 클래스)이 어떤 구조인지 사용자에게 보고하십시오.

**5) 규칙을 고쳐야 하면** 아래 "소스 수정과 빌드"로 진행하거나, 덤프 결과를 첨부해 저장소에 이슈를 올리도록 안내하십시오: `https://github.com/cerbagge/kakao-adblock/issues`

## 현재 판별 규칙 (v1.1.0 기준)

| 대상 | 판별 기준 | 처리 | 소스 위치 |
|---|---|---|---|
| 카카오톡 광고 팝업 | `KakaoTalk.exe`의 최상위 창 중, 자식에 제목이 `광고`를 포함하는 `Chrome_WidgetWin_1`이 있는 창 (예: `MOMENT 광고`) | 숨김 | `has_kakao_ad` |
| 카카오톡 메인 창 | 클래스 `EVA_Window_Dblclk`, 제목 `카카오톡` | 절대 숨기지 않음 | `is_kakao_main` |
| 메인 창 하단 광고 자리 | 메인 창의 직속 자식, 클래스 `EVA_ChildWindow`, 제목 없음, 창 아래 끝에 붙어 있음, 높이 150px 미만 | 숨김 | `fix_layout` |
| 메인 창 목록 영역 | 제목이 `OnlineMainView` 또는 `LockModeView`로 시작하는 직속 자식 | 높이를 창 아래 끝까지 늘림 | `fix_layout` |
| 오른쪽 아래 팝업 | 허용 목록에 없고 시스템 프로그램이 아닌 프로세스의 최상위 창 중, 제목 표시줄 없음, 800×600 이하, 왼쪽 위 꼭짓점이 작업 영역 중심보다 오른쪽 아래, 오른쪽 끝이 작업 영역 오른쪽 끝에서 150px 이내 | 숨김 | `is_corner_popup` |
| 제외 대상 | 메뉴(`#32768`), 툴팁(`tooltips_class32`), `C:\Windows` 아래 프로그램, 정보를 읽을 수 없는 프로세스 | 무시 | `is_corner_popup`, `classify` |

동작 방식: 폴링 없이 `EVENT_OBJECT_SHOW`, `EVENT_OBJECT_NAMECHANGE` 전역 WinEvent 훅과, 카카오톡 프로세스에 한정한 `EVENT_OBJECT_LOCATIONCHANGE` 훅으로만 동작합니다. 새 규칙을 추가할 때도 **타이머나 주기적 검사를 넣지 마십시오.** 대기 중 CPU 0%를 유지하는 것이 이 프로젝트의 설계 목표입니다.

## 소스 수정과 빌드

필요한 도구: Rust(stable, `x86_64-pc-windows-msvc`), Visual Studio Build Tools(C++ 데스크톱 개발).

```powershell
git clone https://github.com/cerbagge/kakao-adblock
cd kakao-adblock
cargo build --release
```

- 소스는 `src/main.rs` 한 파일입니다.
- 결과물: `target\release\kakao-adblock.exe`. 설치본을 교체하려면 실행 중인 프로그램을 종료한 뒤 `$dir`에 덮어쓰고 다시 실행합니다.
- 경로가 너무 길면 링크 단계에서 `LNK1104` 오류가 납니다. 이 경우 `$env:CARGO_TARGET_DIR = "C:\kab-target"`처럼 짧은 경로를 지정하고 다시 빌드하십시오.
- 오른쪽 아래 팝업 규칙은 검증용 예제로 확인할 수 있습니다. 차단기를 실행한 상태에서 아래 명령을 실행하면 `corner visible = false`, `normal visible = true`가 나와야 합니다.

```powershell
cargo run --release --example popup_test
```

- 버전 규칙: 주요 기능 변경은 `#.0.0`, 기능 추가나 중요한 버그 수정은 `0.#.0`, 작은 버그 수정은 `0.0.#`를 올립니다. `v*` 태그를 푸시하면 GitHub Actions가 빌드하고 릴리스를 자동으로 만듭니다.
