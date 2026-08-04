use anyhow::Result;
use capture_core::Rect;

#[cfg(windows)]
mod win {
    use super::*;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
        GetStockObject, InvalidateRect, Rectangle, SelectObject, SetBkMode, SetTextColor,
        TextOutW, NULL_BRUSH, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
        GetSystemMetrics, LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassExW,
        SetForegroundWindow, SetLayeredWindowAttributes, ShowWindow, TranslateMessage, IDC_CROSS,
        LWA_ALPHA, MSG, PM_REMOVE, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SW_SHOW, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WNDCLASSEXW, WS_EX_LAYERED,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    struct OverlayState {
        start_x: i32,
        start_y: i32,
        current_x: i32,
        current_y: i32,
        is_selecting: bool,
        confirmed: bool,
    }

    impl OverlayState {
        fn new() -> Self {
            Self {
                start_x: 0,
                start_y: 0,
                current_x: 0,
                current_y: 0,
                is_selecting: false,
                confirmed: false,
            }
        }

        fn selection_rect(&self) -> Rect {
            let x = self.start_x.min(self.current_x);
            let y = self.start_y.min(self.current_y);
            let width = (self.current_x - self.start_x).unsigned_abs();
            let height = (self.current_y - self.start_y).unsigned_abs();
            Rect::new(x, y, width, height)
        }
    }

    thread_local! {
        static STATE: RefCell<OverlayState> = RefCell::new(OverlayState::new());
    }

    static RUNNING: AtomicBool = AtomicBool::new(false);

    pub struct WindowsOverlay;

    pub enum OverlayResult {
        Selected(Rect),
        Cancelled,
    }

    impl WindowsOverlay {
        pub fn new() -> Self {
            Self
        }

        pub fn show_and_select(&mut self) -> Result<OverlayResult> {
            unsafe {
                let hinstance = GetModuleHandleW(None)?;
                let class_name = wide_null("FalconShotOverlay");

                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: Default::default(),
                    lpfnWndProc: Some(wnd_proc),
                    hInstance: hinstance.into(),
                    hCursor: LoadCursorW(None, IDC_CROSS)?,
                    hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                        GetStockObject(windows::Win32::Graphics::Gdi::BLACK_BRUSH).0,
                    ),
                    lpszClassName: PCWSTR(class_name.as_ptr()),
                    ..Default::default()
                };

                RegisterClassExW(&wc);

                let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let cx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let cy = GetSystemMetrics(SM_CYVIRTUALSCREEN);

                let hwnd = CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
                    PCWSTR(class_name.as_ptr()),
                    PCWSTR::null(),
                    WS_POPUP,
                    x,
                    y,
                    cx,
                    cy,
                    None,
                    None,
                    Some(hinstance.into()),
                    None,
                )?;

                // Set uniform 40% opacity for the entire window
                let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 100, LWA_ALPHA);

                RUNNING.store(true, Ordering::SeqCst);
                STATE.with(|s| {
                    *s.borrow_mut() = OverlayState::new();
                });

                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);
                SetCapture(hwnd);

                // Force crosshair cursor (overrides system "busy" cursor)
                if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetCursor(Some(cursor));
                }

                let mut msg = MSG::default();
                while RUNNING.load(Ordering::SeqCst) {
                    if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                let _ = ReleaseCapture();
                let _ = DestroyWindow(hwnd);

                let result = STATE.with(|s| {
                    let state = s.borrow();
                    if state.confirmed {
                        let rect = state.selection_rect();
                        if rect.width > 2 && rect.height > 2 {
                            OverlayResult::Selected(rect)
                        } else {
                            OverlayResult::Cancelled
                        }
                    } else {
                        OverlayResult::Cancelled
                    }
                });

                Ok(result)
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_SETCURSOR => {
                if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetCursor(Some(cursor));
                }
                LRESULT(1)
            }
            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.start_x = x;
                    state.start_y = y;
                    state.current_x = x;
                    state.current_y = y;
                    state.is_selecting = true;
                });
                invalidate(hwnd);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let selecting = STATE.with(|s| s.borrow().is_selecting);
                if selecting {
                    let mut pos = POINT::default();
                    let _ = GetCursorPos(&mut pos);
                    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
                    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
                    STATE.with(|s| {
                        let mut state = s.borrow_mut();
                        state.current_x = pos.x - vx;
                        state.current_y = pos.y - vy;
                    });
                    invalidate(hwnd);
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.is_selecting = false;
                    let rect = state.selection_rect();
                    if rect.width > 2 && rect.height > 2 {
                        state.confirmed = true;
                        RUNNING.store(false, Ordering::SeqCst);
                        PostQuitMessage(0);
                    }
                });
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let vk = wparam.0 as u32;
                if vk == 0x1B {
                    RUNNING.store(false, Ordering::SeqCst);
                    PostQuitMessage(0);
                } else if vk == 0x0D {
                    STATE.with(|s| {
                        s.borrow_mut().confirmed = true;
                    });
                    RUNNING.store(false, Ordering::SeqCst);
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                paint(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                RUNNING.store(false, Ordering::SeqCst);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn paint(hwnd: HWND) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);

        let cx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let cy = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        // Dark background (will appear semi-transparent due to LWA_ALPHA)
        let dark_brush = CreateSolidBrush(COLORREF(0x00303030));
        let full_rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: cx,
            bottom: cy,
        };
        FillRect(hdc, &full_rect, dark_brush);
        let _ = DeleteObject(dark_brush.into());

        let sel = STATE.with(|s| s.borrow().selection_rect());
        if sel.width > 0 && sel.height > 0 {
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);

            let sel_rect = windows::Win32::Foundation::RECT {
                left: sel.x - vx,
                top: sel.y - vy,
                right: sel.x - vx + sel.width as i32,
                bottom: sel.y - vy + sel.height as i32,
            };

            // Selection border (cyan/aqua)
            let border_pen = CreatePen(PS_SOLID, 2, COLORREF(0x00FFAE00));
            let old_pen = SelectObject(hdc, border_pen.into());
            let null_brush = GetStockObject(NULL_BRUSH);
            let old_brush = SelectObject(hdc, null_brush);
            let _ = Rectangle(hdc, sel_rect.left, sel_rect.top, sel_rect.right, sel_rect.bottom);
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            let _ = DeleteObject(border_pen.into());

            // Crosshair lines through selection center
            let cross_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00FFAE00));
            let old_pen2 = SelectObject(hdc, cross_pen.into());
            let mid_x = (sel_rect.left + sel_rect.right) / 2;
            let mid_y = (sel_rect.top + sel_rect.bottom) / 2;
            let _ = windows::Win32::Graphics::Gdi::MoveToEx(hdc, mid_x, sel_rect.top, None);
            let _ = windows::Win32::Graphics::Gdi::LineTo(hdc, mid_x, sel_rect.bottom);
            let _ = windows::Win32::Graphics::Gdi::MoveToEx(hdc, sel_rect.left, mid_y, None);
            let _ = windows::Win32::Graphics::Gdi::LineTo(hdc, sel_rect.right, mid_y);
            SelectObject(hdc, old_pen2);
            let _ = DeleteObject(cross_pen.into());

            // Dimension text
            let text = format!("{} x {}", sel.width, sel.height);
            let text_wide: Vec<u16> = text.encode_utf16().collect();
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, COLORREF(0x00FFFFFF));
            let text_y = if sel_rect.top >= 22 { sel_rect.top - 22 } else { sel_rect.bottom + 4 };
            let _ = TextOutW(hdc, sel_rect.left, text_y, &text_wide);
        }

        let _ = EndPaint(hwnd, &ps);
    }

    unsafe fn invalidate(hwnd: HWND) {
        let _ = InvalidateRect(Some(hwnd), None, true);
    }

    fn wide_null(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
pub use win::{OverlayResult, WindowsOverlay};

#[cfg(not(windows))]
pub struct WindowsOverlay;

#[cfg(not(windows))]
pub enum OverlayResult {
    Selected(Rect),
    Cancelled,
}

#[cfg(not(windows))]
impl WindowsOverlay {
    pub fn new() -> Self {
        Self
    }

    pub fn show_and_select(&mut self) -> Result<OverlayResult> {
        anyhow::bail!("Overlay is only available on Windows")
    }
}
