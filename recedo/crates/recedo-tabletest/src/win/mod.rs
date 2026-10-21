//! Windows shell: window creation, the message loop and the window procedure.
//!
//! The [`App`] lives in a thread-local. The window procedure borrows it for each message;
//! if a message arrives while it is already borrowed (Windows *sends* some messages from
//! inside API calls), it gets default handling instead. Nothing important is lost that
//! way: sizes are re-read every frame and pen/tablet input is always *posted*, never sent.

pub mod app;
pub mod egui_input;
pub mod selftest;
pub mod sys;

use std::cell::RefCell;
use std::time::Duration;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL, MONITORINFO, MonitorFromPoint, MonitorFromRect};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::model::Settings;
use app::{App, Deferred};

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

const CLASS: &str = "RecedoTabletTest";

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handled = APP.with(|cell| match cell.try_borrow_mut() {
        Ok(mut guard) => guard.as_mut().and_then(|app| app.handle(hwnd, msg, wparam, lparam)),
        Err(_) => None,
    });
    if let Some(r) = handled {
        return r;
    }
    match msg {
        WM_DESTROY => {
            // SAFETY: plain call.
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_ERASEBKGND => 1,
        // SAFETY: forwarding the message unchanged.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Per-monitor DPI awareness, so all coordinates are physical pixels and the panel text is
/// rendered at the monitor's real resolution (crisp at 125% / 150%).
pub fn set_dpi_awareness() {
    // SAFETY: called before any window exists.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// Creates the main window. `placement`: saved (left, top, right, bottom, maximised).
pub fn create_window(title: &str, placement: Option<(i32, i32, i32, i32, bool)>, topmost: bool) -> Result<HWND, String> {
    let class = sys::wide(CLASS);
    let title = sys::wide(title);
    // SAFETY: standard window class registration and creation.
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        // Use the saved placement only if it is still on a connected monitor.
        let saved = placement.filter(|&(l, t, r, b, _)| {
            let rect = RECT { left: l, top: t, right: r, bottom: b };
            r - l >= 320 && b - t >= 240 && !MonitorFromRect(&rect, MONITOR_DEFAULTTONULL).is_null()
        });
        let (x, y, w, h) = match saved {
            Some((l, t, r, b, _)) => (l, t, r - l, b - t),
            None => {
                // First run: the work area of the monitor under the mouse (the window is
                // then shown maximised there).
                let mut p = POINT { x: 0, y: 0 };
                GetCursorPos(&mut p);
                let mut mi: MONITORINFO = std::mem::zeroed();
                mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST), &mut mi) != 0 {
                    let r = mi.rcWork;
                    (r.left, r.top, r.right - r.left, r.bottom - r.top)
                } else {
                    (CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT)
                }
            }
        };
        let ex = if topmost { WS_EX_TOPMOST } else { 0 };
        let hwnd = CreateWindowExW(ex, class.as_ptr(), title.as_ptr(), WS_OVERLAPPEDWINDOW, x, y, w, h, std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null());
        if hwnd.is_null() {
            return Err("could not create the window".into());
        }
        Ok(hwnd)
    }
}

/// Processes every queued message. Returns false once the app should quit.
pub fn pump() -> bool {
    // SAFETY: standard message loop.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if msg.message == WM_QUIT {
                return false;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    true
}

fn wait_for_input(timeout: Duration) {
    if timeout.is_zero() {
        return;
    }
    let ms = timeout.as_millis().min(u32::MAX as u128 - 1) as u32;
    // SAFETY: waits for any input or the timeout.
    unsafe { MsgWaitForMultipleObjectsEx(0, std::ptr::null(), ms, QS_ALLINPUT, MWMO_INPUTAVAILABLE) };
}

pub fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| cell.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

fn run_deferred() {
    let ops = with_app(|a| a.take_deferred()).unwrap_or_default();
    for op in ops {
        let hwnd = with_app(|a| a.hwnd);
        if let (Some(hwnd), Deferred::MoveTo(r)) = (hwnd, op) {
            // SAFETY: valid window. Runs outside the app borrow because SetWindowPos sends
            // WM_SIZE / WM_DPICHANGED back synchronously.
            unsafe {
                ShowWindow(hwnd, SW_RESTORE);
                SetWindowPos(hwnd, std::ptr::null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            }
        }
    }
}

/// One turn of the main loop: messages, then a frame if one is due.
///
/// A frame first waits for a free swap-chain image, then reads the input that arrived
/// meanwhile, then draws — so what is shown is as fresh as possible.
pub fn step(block: bool) -> bool {
    if !pump() {
        return false;
    }
    run_deferred();
    if with_app(|a| a.wants_frame()).unwrap_or(false) {
        with_app(|a| a.acquire());
        if !pump() {
            return false;
        }
        with_app(|a| a.frame());
        run_deferred();
    } else if block {
        let timeout = with_app(|a| a.idle_timeout()).unwrap_or(Duration::from_millis(100));
        wait_for_input(timeout);
    }
    true
}

pub fn install(app: App) {
    APP.with(|cell| *cell.borrow_mut() = Some(app));
}

pub fn uninstall() -> Option<App> {
    APP.with(|cell| cell.borrow_mut().take())
}

/// Runs the tablet test app until its window closes.
pub fn run() -> i32 {
    set_dpi_awareness();
    let settings = Settings::load(&sys::settings_path());
    let placement = settings.window;
    let hwnd = match create_window("Recedo tablet test", placement, false) {
        Ok(h) => h,
        Err(e) => return fatal(&e),
    };
    let app = match App::new(hwnd, settings, true) {
        Ok(a) => a,
        Err(e) => return fatal(&format!("Recedo could not start the GPU: {e}")),
    };
    install(app);
    let maximized = placement.is_none_or(|p| p.4);
    // SAFETY: valid window.
    unsafe { ShowWindow(hwnd, if maximized { SW_SHOWMAXIMIZED } else { SW_SHOWNORMAL }) };
    while step(true) {}
    drop(uninstall());
    0
}

pub fn fatal(msg: &str) -> i32 {
    let text = sys::wide(msg);
    let title = sys::wide("Recedo tablet test");
    // SAFETY: valid strings.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
    1
}
