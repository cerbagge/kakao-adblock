# kakao-adblock

[한국어](README.md) | **English**

A tiny Windows background utility that hides ads in KakaoTalk for PC and blocks bottom-right popups from apps that are not on your allowlist.

- It **does not modify** any KakaoTalk files. It only hides ad windows while KakaoTalk is running.
- It never polls. It wakes up only on Windows window events (WinEvent hooks), so it uses 0% CPU while idle and about 1 MB of memory.
- No console window, no tray icon. A single executable (about 150 KB).

## How it works

1. **KakaoTalk ads**: Hides any KakaoTalk window that hosts a Chromium view whose title contains "광고" ("ad", e.g. `MOMENT 광고`). The KakaoTalk main window is never hidden.
   - It also hides the empty ad area at the bottom of the main window and stretches the chat/friend list down to the bottom edge. When you resize the window, it re-applies the layout using a location-change hook scoped to the KakaoTalk process only.
2. **Bottom-right popups**: Hides a window from any program not listed in `allow.txt` when all of the following are true:
   - It is a top-level window without a title bar
   - It is 800×600 pixels or smaller
   - It sits in the bottom-right area of the monitor's work area (popups sliding in from off-screen are included)
3. System programs under `C:\Windows` and native Windows notifications (Action Center toasts) are always allowed. Manage native notifications per app in `Settings → System → Notifications`.

## Install with an AI tool

If you are not comfortable with the command line, you can let an AI coding tool such as Claude Code or Codex do it for you. Paste the sentence below into the AI tool. See [AI_GUIDE.md](AI_GUIDE.md) for details (the guide is written in Korean, but AI tools can follow it regardless of your language).

```
Read https://raw.githubusercontent.com/cerbagge/kakao-adblock/main/AI_GUIDE.md and install kakao-adblock, then register it as a startup program.
```

## Install

1. Download the latest `kakao-adblock-*.zip` from [Releases](../../releases) and extract it to any folder.
2. Run `kakao-adblock.exe`. On first launch, it creates `allow.txt` in the same folder.
3. To start it automatically at sign-in, press `Win + R`, enter `shell:startup`, and put a shortcut to `kakao-adblock.exe` in the folder that opens.

To stop it, end `kakao-adblock.exe` in Task Manager or run:

```
taskkill /IM kakao-adblock.exe
```

## Configuration (`allow.txt`)

List the executable names of programs that are allowed to show bottom-right popups, one per line. Names are case-insensitive. Restart the program after editing the file.

```
KakaoTalk.exe
Discord.exe
```

If a popup you need has disappeared, check `blocked.log` in the same folder. It records which program's window was hidden, so you can add that name to `allow.txt`. The log is cleared on the next launch once it exceeds 256 KB.

If an ad popup was *not* hidden, look for `[참고: ...]` ("note") lines in `blocked.log`. They record windows that appeared in the bottom-right area but were left alone because they have a title bar or are too large, along with their position and size.

## Build

```
cargo build --release
```

`examples/popup_test.rs` is a test program that opens a bottom-right popup and a control popup, then prints whether each one is still visible.

```
cargo run --release --example popup_test
```

## Notes

- Detection is based on window structure, so ads may reappear if a KakaoTalk update changes how its ad windows are built.
- This is an unofficial project and is not affiliated with Kakao Corp.

## License

[MIT](LICENSE)
