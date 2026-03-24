/// Platform-specific window embedding via Win32 APIs.
#[cfg(windows)]
pub mod platform {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    struct PidSearch {
        pid: u32,
        title_contains: String,
        result: Option<HWND>,
    }

    /// Find Void's own window handle by matching the title prefix.
    pub fn find_void_hwnd() -> Option<HWND> {
        let result: Mutex<Option<HWND>> = Mutex::new(None);

        unsafe {
            let _ = EnumWindows(
                Some(enum_void_callback),
                LPARAM(&result as *const _ as isize),
            );
        }

        result.into_inner().ok().flatten()
    }

    unsafe extern "system" fn enum_void_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let result = &*(lparam.0 as *const Mutex<Option<HWND>>);
        let mut title_buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        if len > 0 {
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);
            if title.starts_with("Void |") {
                if let Ok(mut r) = result.lock() {
                    *r = Some(hwnd);
                }
                return FALSE;
            }
        }
        TRUE
    }

    /// Launch an app and wait for its window to appear.
    #[allow(dead_code)]
    pub fn launch_and_find(
        exe_path: &str,
        title_contains: &str,
        timeout: Duration,
    ) -> anyhow::Result<(std::process::Child, HWND)> {
        let child = std::process::Command::new(exe_path).spawn()?;
        let pid = child.id();
        let start = Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Timeout waiting for window from PID {pid}"));
            }
            std::thread::sleep(Duration::from_millis(200));

            if let Some(hwnd) = find_window_by_pid(pid, title_contains) {
                return Ok((child, hwnd));
            }
        }
    }

    /// Reparent a window into Void as a child window.
    pub fn embed_window(child: HWND, parent: HWND) -> anyhow::Result<()> {
        unsafe {
            let style = GetWindowLongPtrW(child, GWL_STYLE) as u32;
            let new_style = (style & !(WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0))
                | WS_CHILD.0
                | WS_VISIBLE.0;
            SetWindowLongPtrW(child, GWL_STYLE, new_style as isize);

            let ex_style = GetWindowLongPtrW(child, GWL_EXSTYLE) as u32;
            let new_ex = ex_style & !(WS_EX_APPWINDOW.0 | WS_EX_TOOLWINDOW.0);
            SetWindowLongPtrW(child, GWL_EXSTYLE, new_ex as isize);

            let _ = SetParent(child, parent);

            let _ = SetWindowPos(
                child,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED,
            );
        }
        Ok(())
    }

    /// Reposition the embedded window to match a panel rect.
    pub fn reposition(child: HWND, x: i32, y: i32, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        unsafe {
            let _ = MoveWindow(child, x, y, w, h, TRUE);
        }
    }

    /// Detach a window — restore to normal top-level window.
    pub fn detach_window(child: HWND) {
        unsafe {
            let style = GetWindowLongPtrW(child, GWL_STYLE) as u32;
            let new_style = (style & !WS_CHILD.0) | WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0;
            SetWindowLongPtrW(child, GWL_STYLE, new_style as isize);

            let _ = SetParent(child, HWND::default());

            let _ = SetWindowPos(
                child,
                HWND_TOP,
                100,
                100,
                800,
                600,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
        }
    }

    /// Show or hide an embedded window.
    pub fn set_visible(hwnd: HWND, visible: bool) {
        unsafe {
            let _ = ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    /// Find a window belonging to a specific process ID with a matching title.
    pub fn find_window_by_pid(pid: u32, title_contains: &str) -> Option<HWND> {
        let params = Mutex::new(PidSearch {
            pid,
            title_contains: title_contains.to_string(),
            result: None,
        });

        unsafe {
            let _ = EnumWindows(
                Some(enum_pid_callback),
                LPARAM(&params as *const _ as isize),
            );
        }

        params.into_inner().ok().and_then(|p| p.result)
    }

    unsafe extern "system" fn enum_pid_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let params = &*(lparam.0 as *const Mutex<PidSearch>);

        let mut window_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

        if let Ok(mut p) = params.lock() {
            if window_pid == p.pid {
                let mut title_buf = [0u16; 256];
                let len = GetWindowTextW(hwnd, &mut title_buf);
                if len > 0 {
                    let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                    if title.contains(&p.title_contains) && IsWindowVisible(hwnd).as_bool() {
                        p.result = Some(hwnd);
                        return FALSE;
                    }
                }
            }
        }
        TRUE
    }
}

#[cfg(not(windows))]
pub mod platform {
    /// Stub for non-Windows platforms — app embedding not yet supported.
    pub fn find_void_hwnd() -> Option<()> {
        None
    }
}
