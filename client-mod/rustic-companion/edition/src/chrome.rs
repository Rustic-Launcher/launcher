//! The game window's title bar in Rustic's colours (ADR 0148), on Windows only: the native bar
//! is kept (snap, resize, double-click, several screens all work as before), DWM paints it dark
//! (`DWMWA_USE_IMMERSIVE_DARK_MODE`; Windows 11 also takes the caption, text and border colours
//! of the launcher's dark theme) and its text says "Rustic · Horizon 26.3". The window and task
//! bar icon is Rustic's, given to the game before its window opens (`main`).
//!
//! The game makes its windows itself (winit, class `Window Class`) and gives mods no handle to
//! them, so a thread of its own looks for this process's visible top-level game windows a
//! couple of times a second and styles each new one; the text goes back whenever something
//! sets another (Horizon names its windows "Horizon"). Nothing here touches the game's state.
//! Everything that calls Windows is in [`win32`], the only module where `unsafe` is allowed.
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The launcher's dark theme (launcher/packages/ui/src/tokens/tokens.css) as `0xRRGGBB`.
pub mod colors {
    /// `--rl-surface` (#0d0d0c): the caption.
    pub const CAPTION: u32 = 0x0D0D0C;
    /// `--rl-fg` (#f3f1ee): the title text.
    pub const TEXT: u32 = 0xF3F1EE;
    /// `--rl-border-default` (#262421): the window's 1-pixel border.
    pub const BORDER: u32 = 0x262421;
}

/// Windows' `COLORREF` (`0x00BBGGRR`) of `0xRRGGBB`.
pub fn colorref(rgb: u32) -> u32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    u32::from_le_bytes([r, g, b, 0])
}

/// A top-level window of this process, as the watcher sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub id: isize,
    pub class: String,
    pub title: String,
    pub visible: bool,
    pub owned: bool,
}

/// The class winit gives the game's windows.
const GAME_CLASS: &str = "Window Class";
/// DWM's colours stay with a window; they are set again this often anyway (a theme change makes
/// winit set dark mode itself).
const RESTYLE: Duration = Duration::from_secs(10);

/// What to do with `w`: `(style, retitle)`.
pub fn decide(
    w: &Seen,
    title: &str,
    styled: Option<Instant>,
    now: Instant,
) -> Option<(bool, bool)> {
    if !w.visible || w.owned || w.class != GAME_CLASS {
        return None;
    }
    let retitle = w.title != title;
    let style = retitle || styled.is_none_or(|t| now - t >= RESTYLE);
    (style || retitle).then_some((style, retitle))
}

/// Starts the watcher (Windows; nothing elsewhere).
pub fn start(title: String) {
    if !cfg!(windows) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("rustic-window-chrome".into())
        .spawn(move || run(&title));
    if let Err(e) = spawned {
        eprintln!("Rustic: the title bar keeps the system's look ({e})");
    }
}

fn run(title: &str) {
    let mut styled: HashMap<isize, Instant> = HashMap::new();
    let started = Instant::now();
    loop {
        let now = Instant::now();
        let windows = win32::own_windows();
        styled.retain(|id, _| windows.iter().any(|w| w.id == *id));
        for w in &windows {
            let Some((style, retitle)) = decide(w, title, styled.get(&w.id).copied(), now) else {
                continue;
            };
            if style {
                win32::style(w.id, colors::CAPTION, colors::TEXT, colors::BORDER);
                styled.insert(w.id, now);
            }
            if retitle {
                win32::set_title(w.id, title);
            }
        }
        // Often while the game starts (the loading window, then the game's), then calmly.
        let pause = if now - started < Duration::from_secs(30) {
            250
        } else {
            1000
        };
        std::thread::sleep(Duration::from_millis(pause));
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod win32 {
    use super::{Seen, colorref};
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
        DwmSetWindowAttribute,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GetClassNameW, GetWindow, GetWindowThreadProcessId,
        InternalGetWindowText, IsWindowVisible, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTEXT,
    };
    use windows_sys::core::BOOL;

    struct Found {
        pid: u32,
        windows: Vec<Seen>,
    }

    fn text(hwnd: HWND, get: unsafe extern "system" fn(HWND, *mut u16, i32) -> i32) -> String {
        let mut buf = [0u16; 256];
        // SAFETY: `buf` is a live, writable buffer of 256 UTF-16 units and its length is
        // passed, so the call writes at most that many (with the terminating zero).
        let n = unsafe { get(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.clamp(0, buf.len() as i32) as usize])
    }

    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `&mut Found` `own_windows` passed to `EnumWindows`, alive
        // for the whole enumeration, which calls this function on the same thread.
        let found = unsafe { &mut *(lparam as *mut Found) };
        let mut pid = 0u32;
        // SAFETY: `hwnd` comes from `EnumWindows`; `pid` is a valid place to write the id.
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == found.pid {
            // SAFETY: plain queries on a window handle `EnumWindows` gave (a window that has
            // gone away since just answers false/null).
            let (visible, owned) = unsafe {
                (
                    IsWindowVisible(hwnd) != 0,
                    !GetWindow(hwnd, GW_OWNER).is_null(),
                )
            };
            found.windows.push(Seen {
                id: hwnd as isize,
                class: text(hwnd, GetClassNameW),
                title: text(hwnd, InternalGetWindowText),
                visible,
                owned,
            });
        }
        1
    }

    /// This process's top-level windows.
    pub fn own_windows() -> Vec<Seen> {
        let mut found = Found {
            // SAFETY: no arguments, no preconditions.
            pid: unsafe { GetCurrentProcessId() },
            windows: Vec::new(),
        };
        // SAFETY: `each` matches `WNDENUMPROC` and only reads `lparam` as the `Found` passed
        // here, which outlives the call (`EnumWindows` returns once it has called back).
        unsafe { EnumWindows(Some(each), &mut found as *mut Found as LPARAM) };
        found.windows
    }

    /// Dark mode and the caption, text and border colours (`0xRRGGBB`). Windows 10 knows only
    /// the first and ignores the others.
    pub fn style(id: isize, caption: u32, text: u32, border: u32) {
        let hwnd = id as HWND;
        let dark: BOOL = 1;
        // SAFETY: each value is a live local of the size passed; DWM only reads it, and a
        // window that has gone away just makes the call fail.
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                (&raw const dark).cast(),
                size_of::<BOOL>() as u32,
            );
            for (attribute, rgb) in [
                (DWMWA_CAPTION_COLOR, caption),
                (DWMWA_TEXT_COLOR, text),
                (DWMWA_BORDER_COLOR, border),
            ] {
                let value = colorref(rgb);
                DwmSetWindowAttribute(
                    hwnd,
                    attribute as u32,
                    (&raw const value).cast(),
                    size_of::<u32>() as u32,
                );
            }
        }
    }

    /// The window's text, sent as `WM_SETTEXT` to the game's thread, which owns the window; at
    /// most a second's wait, so a busy game never holds this thread.
    pub fn set_title(id: isize, title: &str) {
        let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut result = 0usize;
        // SAFETY: `wide` is a zero-terminated UTF-16 string alive until the call returns
        // (`SendMessageTimeoutW` waits for the window procedure or the timeout, and the window
        // copies the text); `result` is a valid place to write.
        unsafe {
            SendMessageTimeoutW(
                id as HWND,
                WM_SETTEXT,
                0,
                wide.as_ptr() as LPARAM,
                SMTO_ABORTIFHUNG,
                1000,
                &mut result,
            )
        };
    }
}

#[cfg(not(windows))]
mod win32 {
    use super::Seen;
    pub fn own_windows() -> Vec<Seen> {
        Vec::new()
    }
    pub fn style(_: isize, _: u32, _: u32, _: u32) {}
    pub fn set_title(_: isize, _: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(title: &str) -> Seen {
        Seen {
            id: 1,
            class: GAME_CLASS.into(),
            title: title.into(),
            visible: true,
            owned: false,
        }
    }

    #[test]
    fn colours_are_windows_colorrefs() {
        assert_eq!(colorref(0x112233), 0x0033_2211);
        assert_eq!(colorref(colors::CAPTION), 0x000C_0D0D);
    }

    #[test]
    fn only_the_visible_game_windows_are_styled_and_renamed() {
        let now = Instant::now();
        let t = "Rustic · Horizon 26.3";
        assert_eq!(decide(&game("Horizon"), t, None, now), Some((true, true)));
        assert_eq!(decide(&game(t), t, None, now), Some((true, false)));
        assert_eq!(decide(&game(t), t, Some(now), now), None);
        // Something set its text again: renamed, and styled again with it.
        assert_eq!(
            decide(&game("Horizon"), t, Some(now), now),
            Some((true, true))
        );
        assert_eq!(
            decide(&game(t), t, Some(now), now + RESTYLE),
            Some((true, false))
        );
        for w in [
            Seen {
                visible: false,
                ..game("Horizon")
            },
            Seen {
                owned: true,
                ..game("Horizon")
            },
            Seen {
                class: "ConsoleWindowClass".into(),
                ..game("Horizon")
            },
        ] {
            assert_eq!(decide(&w, t, None, now), None);
        }
    }
}
