//! Windows shell: window creation, the message loop and the window procedure.
//!
//! The [`App`] lives in a thread-local. The window procedure borrows it for each message;
//! if a message arrives while it is already borrowed (Windows *sends* some messages from
//! inside API calls), it gets default handling instead. Nothing important is lost that
//! way: sizes are re-read every frame and pen/tablet input is always *posted*, never sent.

pub mod app;
pub mod dialogs;
pub mod selftest;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use recedo_win::sys;
use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL, MONITORINFO, MonitorFromPoint, MonitorFromRect};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::settings::Settings;
use app::{App, Deferred};

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

const CLASS: &str = "Recedo";

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
        // Sent while the app was handling another message: ignored (the user can close
        // again); closing without the app's unsaved-changes check could lose work.
        WM_CLOSE => 0,
        // SAFETY: forwarding the message unchanged.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Per-monitor DPI awareness, so all coordinates are physical pixels and the interface is
/// rendered at the monitor's real resolution (crisp at 125% / 150%).
pub fn set_dpi_awareness() {
    // SAFETY: called before any window exists.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// The work area (screen minus taskbar) of the monitor under the mouse.
pub fn work_area() -> Option<RECT> {
    // SAFETY: out-parameter calls.
    unsafe {
        let mut p = POINT { x: 0, y: 0 };
        GetCursorPos(&mut p);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        (GetMonitorInfoW(MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST), &mut mi) != 0).then_some(mi.rcWork)
    }
}

/// Creates the main window. `placement`: saved (left, top, right, bottom, maximised).
pub fn create_window(title: &str, placement: Option<(i32, i32, i32, i32, bool)>, topmost: bool) -> Result<HWND, String> {
    let class = sys::wide(CLASS);
    let title = sys::wide(title);
    // SAFETY: standard window class registration and creation.
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        // The icon compiled into the .exe (resource 1), or Windows' default one.
        let mut icon = LoadIconW(hinst, std::ptr::without_provenance(1));
        if icon.is_null() {
            icon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        }
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hIcon: icon,
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
            // First run: the work area of the monitor under the mouse (the window is then
            // shown maximised there).
            None => match work_area() {
                Some(r) => (r.left, r.top, r.right - r.left, r.bottom - r.top),
                None => (CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT),
            },
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

/// Shows the file dialogs the interface asked for. Runs outside the app borrow: the dialogs
/// run their own message loop, and this window keeps getting (default-handled) messages.
fn run_deferred() {
    let ops = with_app(|a| a.take_deferred()).unwrap_or_default();
    for op in ops {
        let Some((hwnd, folder, name)) = with_app(|a| (a.hwnd, a.session.ui.settings.last_folder.clone(), a.session.ed.title())) else { return };
        let stem = std::path::Path::new(&name).file_stem().map_or_else(|| name.clone(), |s| s.to_string_lossy().into_owned());
        match op {
            Deferred::Open => {
                if let Some(p) = dialogs::show(hwnd, dialogs::Kind::Open, folder.as_deref(), "") {
                    with_app(|a| a.open_path(p));
                }
            }
            Deferred::SaveAs => {
                let picked = dialogs::show(hwnd, dialogs::Kind::Kra, folder.as_deref(), &format!("{stem}.kra"));
                with_app(|a| match picked {
                    Some(p) => a.save_as(p),
                    None => a.save_cancelled(),
                });
            }
            Deferred::Export(format) => {
                let (kind, ext) = match format {
                    crate::editor::ExportFormat::Png => (dialogs::Kind::Png, "png"),
                    crate::editor::ExportFormat::Jpeg(_) => (dialogs::Kind::Jpeg, "jpg"),
                };
                if let Some(p) = dialogs::show(hwnd, kind, folder.as_deref(), &format!("{stem}.{ext}")) {
                    with_app(|a| a.export(p, format));
                }
            }
        }
        with_app(|a| a.request_frame());
    }
}

/// One turn of the main loop: messages, then a frame if one is due.
///
/// A frame first waits for a free swap-chain image, then reads the input that arrived
/// meanwhile, then draws, so what is shown is as fresh as possible.
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
    } else {
        if block {
            let timeout = with_app(|a| a.idle_timeout()).unwrap_or(Duration::from_millis(100));
            wait_for_input(timeout);
        }
        with_app(|a| a.idle_tick());
    }
    true
}

pub fn install(app: App) {
    APP.with(|cell| *cell.borrow_mut() = Some(app));
}

pub fn uninstall() -> Option<App> {
    APP.with(|cell| cell.borrow_mut().take())
}

/// `Documents\Recedo` (crash reports, self-test output).
pub fn output_dir() -> PathBuf {
    sys::documents_dir().join("Recedo")
}

/// On a crash (panic), saves what happened to `Documents\Recedo\crash-<time>.txt` and, in
/// the normal app, says so in a message box. Without this the window would just vanish.
/// The self-test passes `interactive = false`: no message box, so CI can never hang on one.
pub fn install_crash_report(interactive: bool) {
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_else(|| "unknown place".into());
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".into());
        let text = format!(
            "Recedo crashed.\r\nversion {} ({})\r\nos: {}\r\nwhere: {location}\r\nwhat: {message}\r\n",
            crate::ui::VERSION,
            crate::GIT_HASH,
            sys::os_version()
        );
        let dir = output_dir();
        let path = dir.join(format!("crash-{}.txt", sys::timestamp()));
        let saved = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &text)).is_ok();
        eprintln!("{text}");
        println!("CRASH  {location}: {message}");
        if interactive {
            let file = if saved {
                format!("This was saved to:\r\n{}\r\n\r\nPlease send that file in the chat.", path.display())
            } else {
                "(Could not save it to a file; please take a screenshot of this message.)".to_string()
            };
            let note = format!(
                "{text}\r\nIf the painting had unsaved changes, Recedo offers its last autosaved copy the next time it starts.\r\n\r\n{file}"
            );
            let msg = sys::wide(&note);
            let title = sys::wide("Recedo: crash");
            // SAFETY: valid strings; no owner window (it may be in a broken state).
            unsafe { MessageBoxW(std::ptr::null_mut(), msg.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
        }
    }));
}

/// Is another Recedo already running? (Its autosaved copies are then in use, not left over
/// from a crash, so they are not offered for recovery.) The mutex lives until the process ends.
fn another_instance_running() -> bool {
    let name = sys::wide("Local\\Recedo.Running");
    // SAFETY: plain calls; the handle is deliberately kept open for the process's lifetime.
    unsafe {
        let h = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        !h.is_null() && GetLastError() == ERROR_ALREADY_EXISTS
    }
}

/// Runs Recedo until its window closes. `open`: a file named on the command line (also how
/// "Open with" and dropping a file on the .exe arrive).
pub fn run(open: Option<PathBuf>) -> i32 {
    install_crash_report(true);
    set_dpi_awareness();
    let others = another_instance_running();
    let settings = Settings::load(&app::settings_path());
    let placement = settings.window;
    let hwnd = match create_window("Recedo", placement, false) {
        Ok(h) => h,
        Err(e) => return fatal(&e),
    };
    let recovery = app::recovery_dir();
    let found = if others { Vec::new() } else { recedo_core::autosave::find_recoverable(&recovery) };
    let app = match App::new(hwnd, settings, Some(recovery), true) {
        Ok(a) => a,
        Err(e) => return fatal(&format!("Recedo could not start the GPU: {e}")),
    };
    install(app);
    let maximized = placement.is_none_or(|p| p.4);
    // SAFETY: valid window.
    unsafe { ShowWindow(hwnd, if maximized { SW_SHOWMAXIMIZED } else { SW_SHOWNORMAL }) };
    with_app(|a| {
        a.session.ui.offer_recovery(found);
        if let Some(p) = open {
            a.open_path(p);
        }
    });
    while step(true) {
        if with_app(|a| a.close_requested).unwrap_or(false) {
            break;
        }
    }
    with_app(|a| a.closing());
    shutdown(hwnd);
    0
}

/// Drops the app (GPU surface first), then destroys the window and drains its last messages.
pub fn shutdown(hwnd: HWND) {
    drop(uninstall());
    // SAFETY: valid window; the app no longer refers to it.
    unsafe { DestroyWindow(hwnd) };
    pump();
}

pub fn fatal(msg: &str) -> i32 {
    let text = sys::wide(msg);
    let title = sys::wide("Recedo");
    // SAFETY: valid strings.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
    1
}

/// Command line: `recedo.exe [file]` or `recedo.exe --selftest [output folder]`.
pub fn main() -> i32 {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    match args.first().and_then(|a| a.to_str()) {
        Some("--selftest") => selftest::run(args.get(1).map(PathBuf::from)),
        _ => run(args.first().map(PathBuf::from)),
    }
}
