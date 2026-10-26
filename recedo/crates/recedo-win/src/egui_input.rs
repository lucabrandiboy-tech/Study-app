//! Win32 messages to egui input.
//!
//! The panel is driven by mouse messages. Windows synthesises those from the pen in both
//! tablet modes (from `WM_POINTER*` passed to `DefWindowProc` in Ink mode, from the driver's
//! cursor in Wintab mode), so the pen can press buttons and drag sliders like a mouse. The
//! canvas does not use these; it reads the tablet directly.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, pos2, vec2};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// Not in windows-sys's WindowsAndMessaging module.
const WM_MOUSELEAVE: u32 = 0x02A3;

pub struct EguiInput {
    raw: RawInput,
    start: std::time::Instant,
    tracking_leave: bool,
    buttons_down: u32,
    high_surrogate: Option<u16>,
    /// The keys egui was last told are held.
    mods: Modifiers,
}

fn lparam_pos(lparam: isize) -> (i32, i32) {
    ((lparam & 0xFFFF) as i16 as i32, ((lparam >> 16) & 0xFFFF) as i16 as i32)
}

pub fn modifiers() -> Modifiers {
    // SAFETY: plain key state queries.
    let down = |vk: u16| unsafe { GetKeyState(vk as i32) } as u16 & 0x8000 != 0;
    let ctrl = down(VK_CONTROL);
    Modifiers { alt: down(VK_MENU), ctrl, shift: down(VK_SHIFT), mac_cmd: false, command: ctrl }
}

fn key_from_vk(vk: u16) -> Option<Key> {
    Some(match vk {
        VK_LEFT => Key::ArrowLeft,
        VK_RIGHT => Key::ArrowRight,
        VK_UP => Key::ArrowUp,
        VK_DOWN => Key::ArrowDown,
        VK_ESCAPE => Key::Escape,
        VK_TAB => Key::Tab,
        VK_BACK => Key::Backspace,
        VK_RETURN => Key::Enter,
        VK_SPACE => Key::Space,
        VK_INSERT => Key::Insert,
        VK_DELETE => Key::Delete,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_OEM_MINUS | VK_SUBTRACT => Key::Minus,
        VK_OEM_PLUS => Key::Equals,
        VK_ADD => Key::Plus,
        VK_OEM_COMMA => Key::Comma,
        VK_OEM_PERIOD => Key::Period,
        // [ and ] where a US keyboard has them (brush size).
        VK_OEM_4 => Key::OpenBracket,
        VK_OEM_6 => Key::CloseBracket,
        0x30..=0x39 => [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9][(vk - 0x30) as usize],
        0x41..=0x5A => {
            const LETTERS: [Key; 26] = [
                Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M,
                Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z,
            ];
            LETTERS[(vk - 0x41) as usize]
        }
        0x70..=0x7B => {
            const F: [Key; 12] = [Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12];
            F[(vk - 0x70) as usize]
        }
        _ => return None,
    })
}

impl Default for EguiInput {
    fn default() -> EguiInput {
        EguiInput::new()
    }
}

impl EguiInput {
    pub fn new() -> EguiInput {
        EguiInput { raw: RawInput::default(), start: std::time::Instant::now(), tracking_leave: false, buttons_down: 0, high_surrogate: None, mods: Modifiers::NONE }
    }

    fn pos(lparam: isize, ppp: f32) -> Pos2 {
        let (x, y) = lparam_pos(lparam);
        pos2(x as f32 / ppp, y as f32 / ppp)
    }

    fn button(&mut self, hwnd: HWND, lparam: isize, ppp: f32, button: PointerButton, pressed: bool) {
        let bit = 1 << (button as u32);
        if pressed {
            if self.buttons_down == 0 {
                // SAFETY: valid window.
                unsafe { SetCapture(hwnd) };
            }
            self.buttons_down |= bit;
        } else {
            self.buttons_down &= !bit;
            if self.buttons_down == 0 {
                // SAFETY: plain call.
                unsafe { ReleaseCapture() };
            }
        }
        self.raw.events.push(Event::PointerButton { pos: Self::pos(lparam, ppp), button, pressed, modifiers: modifiers() });
    }

    /// Handles a message if it is input for egui. Returns true if it was consumed.
    pub fn on_message(&mut self, hwnd: HWND, msg: u32, wparam: usize, lparam: isize, ppp: f32) -> bool {
        // egui knows which of Ctrl, Shift and Alt are held only from these events (Ctrl+Enter
        // in the text box needs it).
        if (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&msg) || (WM_KEYFIRST..=WM_KEYLAST).contains(&msg) {
            let m = modifiers();
            if m != self.mods {
                self.mods = m;
                self.raw.events.push(Event::ModifiersChanged(m));
            }
        }
        match msg {
            WM_MOUSEMOVE => {
                if !self.tracking_leave {
                    let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                    // SAFETY: valid struct.
                    unsafe { TrackMouseEvent(&mut tme) };
                    self.tracking_leave = true;
                }
                self.raw.events.push(Event::PointerMoved(Self::pos(lparam, ppp)));
                true
            }
            WM_MOUSELEAVE => {
                self.tracking_leave = false;
                self.raw.events.push(Event::PointerGone);
                true
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                self.button(hwnd, lparam, ppp, PointerButton::Primary, true);
                true
            }
            WM_LBUTTONUP => {
                self.button(hwnd, lparam, ppp, PointerButton::Primary, false);
                true
            }
            WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => {
                self.button(hwnd, lparam, ppp, PointerButton::Secondary, true);
                true
            }
            WM_RBUTTONUP => {
                self.button(hwnd, lparam, ppp, PointerButton::Secondary, false);
                true
            }
            WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => {
                self.button(hwnd, lparam, ppp, PointerButton::Middle, true);
                true
            }
            WM_MBUTTONUP => {
                self.button(hwnd, lparam, ppp, PointerButton::Middle, false);
                true
            }
            WM_CAPTURECHANGED => {
                // Capture was taken away mid-drag: no button-up will come, so tell egui the
                // pointer is gone rather than leave a slider stuck to the pen.
                if self.buttons_down != 0 {
                    self.buttons_down = 0;
                    self.raw.events.push(Event::PointerGone);
                }
                false
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                let delta = ((wparam >> 16) & 0xFFFF) as i16 as f32 / WHEEL_DELTA as f32;
                let delta = if msg == WM_MOUSEWHEEL { vec2(0.0, delta) } else { vec2(-delta, 0.0) };
                self.raw.events.push(Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: modifiers(),
                });
                true
            }
            WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
                let pressed = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
                let vk = wparam as u16;
                let mods = modifiers();
                if pressed && mods.ctrl {
                    match vk {
                        0x43 => self.raw.events.push(Event::Copy),
                        0x58 => self.raw.events.push(Event::Cut),
                        0x56 => {
                            if let Some(t) = crate::sys::get_clipboard_text(hwnd) {
                                self.raw.events.push(Event::Paste(t));
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(key) = key_from_vk(vk) {
                    let repeat = pressed && (lparam >> 30) & 1 != 0;
                    self.raw.events.push(Event::Key { key, physical_key: Some(key), pressed, repeat, modifiers: mods });
                }
                // Let Alt+F4 and friends through.
                msg == WM_KEYDOWN || msg == WM_KEYUP
            }
            WM_CHAR => {
                let unit = wparam as u16;
                let text = if (0xD800..0xDC00).contains(&unit) {
                    self.high_surrogate = Some(unit);
                    None
                } else if (0xDC00..0xE000).contains(&unit) {
                    self.high_surrogate.take().map(|hi| String::from_utf16_lossy(&[hi, unit]))
                } else {
                    char::from_u32(unit as u32).map(String::from)
                };
                if let Some(t) = text
                    && !t.chars().any(char::is_control)
                    && !modifiers().ctrl
                {
                    self.raw.events.push(Event::Text(t));
                }
                true
            }
            WM_SETFOCUS => {
                self.raw.events.push(Event::WindowFocused(true));
                false
            }
            WM_KILLFOCUS => {
                // egui lets go of every key here.
                self.raw.events.push(Event::WindowFocused(false));
                self.mods = Modifiers::NONE;
                self.buttons_down = 0;
                false
            }
            _ => false,
        }
    }

    /// The input for the next frame.
    pub fn take(&mut self, size_px: (u32, u32), native_ppp: f32, ppp: f32) -> RawInput {
        let mut raw = std::mem::take(&mut self.raw);
        raw.time = Some(self.start.elapsed().as_secs_f64());
        raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(size_px.0 as f32 / ppp, size_px.1 as f32 / ppp)));
        raw.focused = true;
        if let Some(vp) = raw.viewports.get_mut(&raw.viewport_id) {
            vp.native_pixels_per_point = Some(native_ppp);
        } else {
            let info = egui::ViewportInfo { native_pixels_per_point: Some(native_ppp), focused: Some(true), ..Default::default() };
            raw.viewports.insert(raw.viewport_id, info);
        }
        raw
    }
}

/// egui cursor icon to a system cursor.
pub fn system_cursor(icon: egui::CursorIcon) -> Option<windows_sys::core::PCWSTR> {
    use egui::CursorIcon as C;
    Some(match icon {
        C::None => return None,
        C::PointingHand => IDC_HAND,
        C::Text | C::VerticalText => IDC_IBEAM,
        C::ResizeHorizontal | C::ResizeEast | C::ResizeWest | C::ResizeColumn => IDC_SIZEWE,
        C::ResizeVertical | C::ResizeNorth | C::ResizeSouth | C::ResizeRow => IDC_SIZENS,
        C::ResizeNeSw | C::ResizeNorthEast | C::ResizeSouthWest => IDC_SIZENESW,
        C::ResizeNwSe | C::ResizeNorthWest | C::ResizeSouthEast => IDC_SIZENWSE,
        C::Move | C::AllScroll | C::Grab | C::Grabbing => IDC_SIZEALL,
        C::NotAllowed | C::NoDrop => IDC_NO,
        C::Wait => IDC_WAIT,
        C::Progress => IDC_APPSTARTING,
        C::Crosshair => IDC_CROSS,
        C::Help => IDC_HELP,
        _ => IDC_ARROW,
    })
}
