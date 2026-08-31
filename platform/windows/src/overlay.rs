use anyhow::Result;
use capture_core::Rect;

#[cfg(windows)]
mod win {
    use super::*;
    use capture_core::CaptureBackend;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen,
        CreateSolidBrush, DeleteDC, DeleteObject, EndPaint, FillRect, GetDC, GetStockObject,
        InvalidateRect, LineTo, MoveToEx, Rectangle, ReleaseDC, SelectObject, SetBkMode,
        SetDIBitsToDevice, SetTextColor, TextOutW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, NULL_BRUSH, PAINTSTRUCT, PS_SOLID, SRCCOPY,
        TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetSystemMetrics,
        LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassExW, SetCursor,
        SetForegroundWindow, SetLayeredWindowAttributes, ShowWindow, TranslateMessage, IDC_CROSS,
        LWA_ALPHA, MSG, PM_REMOVE, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SW_SHOW, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONDOWN, WM_SETCURSOR, WNDCLASSEXW,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    /// Brightness of the frozen frame outside the selection (55%).
    const DIM_FACTOR: u8 = 55;

    /// Selection in window-local coordinates (virtual screen origin is subtracted).
    #[derive(Clone, Copy, PartialEq, Eq)]
    struct SelRect {
        l: i32,
        t: i32,
        r: i32,
        b: i32,
    }

    impl SelRect {
        fn width(&self) -> i32 {
            self.r - self.l
        }

        fn height(&self) -> i32 {
            self.b - self.t
        }

        fn valid(&self) -> bool {
            self.width() > 2 && self.height() > 2
        }

        fn from_points(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
            Self {
                l: x1.min(x2),
                t: y1.min(y2),
                r: x1.max(x2),
                b: y1.max(y2),
            }
        }
    }

    fn virtual_origin() -> (i32, i32) {
        unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
            )
        }
    }

    fn virtual_size() -> (i32, i32) {
        unsafe {
            (
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        }
    }

    struct OverlayState {
        sel: SelRect,
        dragging: bool,
        anchor_x: i32,
        anchor_y: i32,
        confirmed: bool,
        /// Full-screen frame captured before the overlay appeared.
        frozen: Option<image::RgbaImage>,
    }

    impl OverlayState {
        fn new() -> Self {
            Self {
                sel: SelRect {
                    l: 0,
                    t: 0,
                    r: 0,
                    b: 0,
                },
                dragging: false,
                anchor_x: 0,
                anchor_y: 0,
                confirmed: false,
                frozen: None,
            }
        }
    }

    /// Memory DCs holding the frozen frame: dimmed (background) and original
    /// (blitted at full brightness inside the current selection).
    struct FrameDc {
        hdc_dim: HDC,
        hbmp_dim: HBITMAP,
        old_dim: HGDIOBJ,
        hdc_orig: HDC,
        hbmp_orig: HBITMAP,
        old_orig: HGDIOBJ,
    }

    thread_local! {
        static STATE: RefCell<OverlayState> = RefCell::new(OverlayState::new());
        static FRAME: RefCell<Option<FrameDc>> = const { RefCell::new(None) };
    }

    static RUNNING: AtomicBool = AtomicBool::new(false);

    pub struct WindowsOverlay;

    impl Default for WindowsOverlay {
        fn default() -> Self {
            Self::new()
        }
    }

    pub enum OverlayResult {
        Selected(Rect, image::RgbaImage),
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

                // Freeze the screen BEFORE the overlay appears, so anything that
                // happens afterwards (window activation, animations, popups)
                // cannot change what gets captured.
                let frozen = {
                    let backend = crate::capture::WindowsCaptureBackend::new()?;
                    backend.capture_fullscreen()?.image
                };

                let (x, y) = virtual_origin();
                let (cx, cy) = virtual_size();

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

                // Fully opaque window; the dimmed frozen frame provides the
                // darkened look instead of window-level transparency.
                let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);

                let hdc_screen = GetDC(None);
                if hdc_screen.is_invalid() {
                    anyhow::bail!("Failed to get screen DC");
                }
                let dimmed = dim_image(&frozen);
                let frame = create_frame_dcs(hdc_screen, &frozen, &dimmed);
                let _ = ReleaseDC(None, hdc_screen);

                RUNNING.store(true, Ordering::SeqCst);
                STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    *state = OverlayState::new();
                    state.frozen = Some(frozen);
                });
                FRAME.with(|f| {
                    *f.borrow_mut() = Some(frame);
                });

                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);
                SetCapture(hwnd);

                // Force crosshair cursor (overrides system "busy" cursor)
                if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
                    let _ = SetCursor(Some(cursor));
                }

                let mut msg = MSG::default();
                while RUNNING.load(Ordering::SeqCst) {
                    if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                let _ = ReleaseCapture();

                destroy_frame_dcs();
                let _ = DestroyWindow(hwnd);

                let (vx, vy) = virtual_origin();
                let result = STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    if state.confirmed && state.sel.valid() {
                        if let Some(frozen) = state.frozen.take() {
                            let crop_x = state.sel.l.max(0) as u32;
                            let crop_y = state.sel.t.max(0) as u32;
                            let crop_w = (state.sel.width() as u32)
                                .min(frozen.width().saturating_sub(crop_x));
                            let crop_h = (state.sel.height() as u32)
                                .min(frozen.height().saturating_sub(crop_y));
                            if crop_w > 0 && crop_h > 0 {
                                let cropped = image::imageops::crop_imm(
                                    &frozen, crop_x, crop_y, crop_w, crop_h,
                                )
                                .to_image();
                                return OverlayResult::Selected(
                                    Rect::new(state.sel.l + vx, state.sel.t + vy, crop_w, crop_h),
                                    cropped,
                                );
                            }
                        }
                    }
                    OverlayResult::Cancelled
                });

                Ok(result)
            }
        }
    }

    fn dim_image(img: &image::RgbaImage) -> image::RgbaImage {
        image::ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
            let p = img.get_pixel(x, y);
            let dim = |c: u8| ((c as u32 * DIM_FACTOR as u32) / 100) as u8;
            image::Rgba([dim(p[0]), dim(p[1]), dim(p[2]), 255])
        })
    }

    unsafe fn create_frame_dcs(
        hdc_screen: HDC,
        original: &image::RgbaImage,
        dimmed: &image::RgbaImage,
    ) -> FrameDc {
        let (hdc_dim, hbmp_dim, old_dim) = upload_bitmap(hdc_screen, dimmed);
        let (hdc_orig, hbmp_orig, old_orig) = upload_bitmap(hdc_screen, original);
        FrameDc {
            hdc_dim,
            hbmp_dim,
            old_dim,
            hdc_orig,
            hbmp_orig,
            old_orig,
        }
    }

    unsafe fn upload_bitmap(hdc_screen: HDC, img: &image::RgbaImage) -> (HDC, HBITMAP, HGDIOBJ) {
        let (w, h) = (img.width() as i32, img.height() as i32);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbmp = CreateCompatibleBitmap(hdc_screen, w, h);
        let old = SelectObject(hdc_mem, hbmp.into());

        // GDI expects BGRA byte order; the source image is RGBA. Without the
        // swap the preview shows red/blue-shifted colors.
        let mut bgra = img.as_raw().clone();
        let (pixels, _) = bgra.as_chunks_mut::<4>();
        for px in pixels {
            px.swap(0, 2);
        }

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        SetDIBitsToDevice(
            hdc_mem,
            0,
            0,
            img.width(),
            img.height(),
            0,
            0,
            0,
            img.height(),
            bgra.as_ptr().cast(),
            &bmi,
            DIB_RGB_COLORS,
        );

        (hdc_mem, hbmp, old)
    }

    unsafe fn destroy_frame_dcs() {
        FRAME.with(|f| {
            if let Some(frame) = f.borrow_mut().take() {
                let _ = SelectObject(frame.hdc_dim, frame.old_dim);
                let _ = SelectObject(frame.hdc_orig, frame.old_orig);
                let _ = DeleteDC(frame.hdc_dim);
                let _ = DeleteDC(frame.hdc_orig);
                let _ = DeleteObject(frame.hbmp_dim.into());
                let _ = DeleteObject(frame.hbmp_orig.into());
            }
        });
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
                    let _ = SetCursor(Some(cursor));
                }
                LRESULT(1)
            }
            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.dragging = true;
                    state.anchor_x = x;
                    state.anchor_y = y;
                    state.sel = SelRect {
                        l: x,
                        t: y,
                        r: x,
                        b: y,
                    };
                });
                invalidate(hwnd);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                let changed = STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    if state.dragging {
                        state.sel = SelRect::from_points(state.anchor_x, state.anchor_y, x, y);
                        true
                    } else {
                        false
                    }
                });
                if changed {
                    invalidate(hwnd);
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                // Release confirms immediately: no separate confirm step.
                let should_confirm = STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    if state.dragging {
                        state.dragging = false;
                        state.sel.valid()
                    } else {
                        false
                    }
                });
                if should_confirm {
                    STATE.with(|s| s.borrow_mut().confirmed = true);
                    RUNNING.store(false, Ordering::SeqCst);
                    PostQuitMessage(0);
                } else {
                    invalidate(hwnd);
                }
                LRESULT(0)
            }
            WM_RBUTTONDOWN => {
                RUNNING.store(false, Ordering::SeqCst);
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_KEYDOWN => {
                if wparam.0 as u32 == 0x1B {
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

        let (cx, cy) = virtual_size();
        let has_frame = FRAME.with(|f| f.borrow().is_some());

        if has_frame {
            // Dimmed frozen frame as background
            if let Some(hdc_dim) = FRAME.with(|f| f.borrow().as_ref().map(|fr| fr.hdc_dim)) {
                let _ = BitBlt(hdc, 0, 0, cx, cy, Some(hdc_dim), 0, 0, SRCCOPY);
            }
        } else {
            // Fallback: dark background
            let dark_brush = CreateSolidBrush(COLORREF(0x00303030));
            let full_rect = RECT {
                left: 0,
                top: 0,
                right: cx,
                bottom: cy,
            };
            FillRect(hdc, &full_rect, dark_brush);
            let _ = DeleteObject(dark_brush.into());
        }

        let sel = STATE.with(|s| s.borrow().sel);

        if sel.width() > 0 && sel.height() > 0 {
            // Original brightness inside the selection
            if has_frame {
                if let Some(hdc_orig) = FRAME.with(|f| f.borrow().as_ref().map(|fr| fr.hdc_orig)) {
                    let _ = BitBlt(
                        hdc,
                        sel.l,
                        sel.t,
                        sel.width(),
                        sel.height(),
                        Some(hdc_orig),
                        sel.l,
                        sel.t,
                        SRCCOPY,
                    );
                }
            }

            // Selection border (cyan/aqua)
            let border_pen = CreatePen(PS_SOLID, 2, COLORREF(0x00FFAE00));
            let old_pen = SelectObject(hdc, border_pen.into());
            let null_brush = GetStockObject(NULL_BRUSH);
            let old_brush = SelectObject(hdc, null_brush);
            let _ = Rectangle(hdc, sel.l, sel.t, sel.r, sel.b);
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            let _ = DeleteObject(border_pen.into());

            // Crosshair lines while the drag is in progress
            let dragging = STATE.with(|s| s.borrow().dragging);
            if dragging {
                let cross_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00FFAE00));
                let old_pen2 = SelectObject(hdc, cross_pen.into());
                let mid_x = (sel.l + sel.r) / 2;
                let mid_y = (sel.t + sel.b) / 2;
                let _ = MoveToEx(hdc, mid_x, sel.t, None);
                let _ = LineTo(hdc, mid_x, sel.b);
                let _ = MoveToEx(hdc, sel.l, mid_y, None);
                let _ = LineTo(hdc, sel.r, mid_y);
                SelectObject(hdc, old_pen2);
                let _ = DeleteObject(cross_pen.into());
            }

            let dim_y = if sel.t >= 22 { sel.t - 22 } else { sel.b + 4 };
            draw_text(
                hdc,
                &format!("{} x {}", sel.width(), sel.height()),
                sel.l,
                dim_y,
                COLORREF(0x00FFFFFF),
            );
        }

        let _ = EndPaint(hwnd, &ps);
    }

    unsafe fn draw_text(hdc: HDC, text: &str, x: i32, y: i32, color: COLORREF) {
        let wide: Vec<u16> = text.encode_utf16().collect();
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, color);
        let _ = TextOutW(hdc, x, y, &wide);
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
    Selected(Rect, image::RgbaImage),
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
