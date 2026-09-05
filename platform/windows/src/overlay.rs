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
        SetForegroundWindow, SetLayeredWindowAttributes, ShowWindow, TranslateMessage, CS_DBLCLKS,
        HTCLIENT, IDC_ARROW, IDC_CROSS, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE,
        IDC_SIZEWE, LWA_ALPHA, MSG, PM_REMOVE, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN,
        WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONDOWN,
        WM_SETCURSOR, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    /// Brightness of the frozen frame outside the selection (55%).
    const DIM_FACTOR: u8 = 55;

    /// Hit-test zones around an existing selection: corners take precedence
    /// over edges, and Inside (move) beats Outside (draw a new one).
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Zone {
        Outside,
        Inside,
        L,
        R,
        T,
        B,
        Tl,
        Tr,
        Bl,
        Br,
    }

    const CORNER_HIT: i32 = 10;
    const EDGE_HIT: i32 = 6;
    /// Minimum selection size while resizing; sel.valid() needs > 2.
    const MIN_SEL: i32 = 4;

    fn zone_at(sel: SelRect, x: i32, y: i32) -> Zone {
        if !sel.valid() {
            return Zone::Outside;
        }
        let near_l = (x - sel.l).abs() <= CORNER_HIT;
        let near_r = (x - sel.r).abs() <= CORNER_HIT;
        let near_t = (y - sel.t).abs() <= CORNER_HIT;
        let near_b = (y - sel.b).abs() <= CORNER_HIT;
        if (near_l || near_r) && (near_t || near_b) {
            return match (near_l, near_t) {
                (true, true) => Zone::Tl,
                (false, true) => Zone::Tr,
                (true, false) => Zone::Bl,
                (false, false) => Zone::Br,
            };
        }
        let edge_l = (x - sel.l).abs() <= EDGE_HIT;
        let edge_r = (x - sel.r).abs() <= EDGE_HIT;
        let edge_t = (y - sel.t).abs() <= EDGE_HIT;
        let edge_b = (y - sel.b).abs() <= EDGE_HIT;
        let inside_x = x > sel.l && x < sel.r;
        let inside_y = y > sel.t && y < sel.b;
        if edge_l && inside_y {
            Zone::L
        } else if edge_r && inside_y {
            Zone::R
        } else if edge_t && inside_x {
            Zone::T
        } else if edge_b && inside_x {
            Zone::B
        } else if inside_x && inside_y {
            Zone::Inside
        } else {
            Zone::Outside
        }
    }

    fn zone_cursor(zone: Zone) -> PCWSTR {
        match zone {
            Zone::L | Zone::R => IDC_SIZEWE,
            Zone::T | Zone::B => IDC_SIZENS,
            Zone::Tl | Zone::Br => IDC_SIZENWSE,
            Zone::Tr | Zone::Bl => IDC_SIZENESW,
            Zone::Inside => IDC_SIZEALL,
            Zone::Outside => IDC_ARROW,
        }
    }

    /// In-progress adjustment after the initial draw (or a later drag).
    enum Adjust {
        Move { off_x: i32, off_y: i32 },
        Resize(Zone),
    }

    /// Native confirm/cancel mini toolbar shown next to the selection during
    /// the adjust phase, so confirming is discoverable (WeChat-screenshot
    /// style). Returns the panel and the two button rects, window-local.
    fn toolbar_rects(sel: SelRect, cx: i32, cy: i32) -> (RECT, RECT, RECT) {
        const BW: i32 = 34;
        const BH: i32 = 26;
        const GAP: i32 = 4;
        const PAD: i32 = 4;
        let total_w = PAD * 2 + BW * 2 + GAP;
        let total_h = BH + PAD * 2;
        let bx = (sel.r - total_w).clamp(0, (cx - total_w).max(0));
        let by = if sel.b + 8 + total_h <= cy {
            sel.b + 8
        } else {
            (sel.t - 8 - total_h).max(0)
        };
        (
            RECT {
                left: bx,
                top: by,
                right: bx + total_w,
                bottom: by + total_h,
            },
            RECT {
                left: bx + PAD,
                top: by + PAD,
                right: bx + PAD + BW,
                bottom: by + PAD + BH,
            },
            RECT {
                left: bx + PAD + BW + GAP,
                top: by + PAD,
                right: bx + PAD + BW + GAP + BW,
                bottom: by + PAD + BH,
            },
        )
    }

    fn pt_in_rect(x: i32, y: i32, r: &RECT) -> bool {
        x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
    }

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
        adjust: Option<Adjust>,
        /// Last known cursor position, for WM_SETCURSOR hit-testing.
        mouse_x: i32,
        mouse_y: i32,
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
                adjust: None,
                mouse_x: 0,
                mouse_y: 0,
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
                    style: CS_DBLCLKS, // double-click confirms the selection
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
                if (lparam.0 & 0xFFFF) as u16 == HTCLIENT as u16 {
                    set_zone_cursor();
                    LRESULT(1)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                let (cx, cy) = virtual_size();
                let action = STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    if state.sel.valid() {
                        let (panel, confirm, cancel) = toolbar_rects(state.sel, cx, cy);
                        if pt_in_rect(x, y, &confirm) {
                            return Some(true);
                        }
                        if pt_in_rect(x, y, &cancel) {
                            return Some(false);
                        }
                        if pt_in_rect(x, y, &panel) {
                            return None; // panel padding: swallow the click
                        }
                    }
                    let zone = zone_at(state.sel, x, y);
                    match zone {
                        Zone::Inside => {
                            state.dragging = false;
                            state.adjust = Some(Adjust::Move {
                                off_x: x - state.sel.l,
                                off_y: y - state.sel.t,
                            });
                        }
                        Zone::L
                        | Zone::R
                        | Zone::T
                        | Zone::B
                        | Zone::Tl
                        | Zone::Tr
                        | Zone::Bl
                        | Zone::Br => {
                            state.dragging = false;
                            state.adjust = Some(Adjust::Resize(zone));
                        }
                        Zone::Outside => {
                            // Start a fresh selection.
                            state.dragging = true;
                            state.adjust = None;
                            state.anchor_x = x;
                            state.anchor_y = y;
                            state.sel = SelRect {
                                l: x,
                                t: y,
                                r: x,
                                b: y,
                            };
                        }
                    }
                    None
                });
                match action {
                    Some(confirm) => {
                        if confirm {
                            STATE.with(|s| s.borrow_mut().confirmed = true);
                        }
                        RUNNING.store(false, Ordering::SeqCst);
                        PostQuitMessage(0);
                    }
                    None => invalidate(hwnd),
                }
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                let (cx, cy) = virtual_size();
                let changed = STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.mouse_x = x;
                    state.mouse_y = y;
                    if state.dragging {
                        state.sel = SelRect::from_points(state.anchor_x, state.anchor_y, x, y);
                        return true;
                    }
                    match state.adjust {
                        Some(Adjust::Move { off_x, off_y }) => {
                            let w = state.sel.width();
                            let h = state.sel.height();
                            state.sel.l = (x - off_x).clamp(0, (cx - w).max(0));
                            state.sel.t = (y - off_y).clamp(0, (cy - h).max(0));
                            state.sel.r = state.sel.l + w;
                            state.sel.b = state.sel.t + h;
                            true
                        }
                        Some(Adjust::Resize(zone)) => {
                            let sel = &mut state.sel;
                            match zone {
                                Zone::L => sel.l = x.clamp(0, sel.r - MIN_SEL),
                                Zone::R => sel.r = x.clamp(sel.l + MIN_SEL, cx),
                                Zone::T => sel.t = y.clamp(0, sel.b - MIN_SEL),
                                Zone::B => sel.b = y.clamp(sel.t + MIN_SEL, cy),
                                Zone::Tl => {
                                    sel.l = x.clamp(0, sel.r - MIN_SEL);
                                    sel.t = y.clamp(0, sel.b - MIN_SEL);
                                }
                                Zone::Tr => {
                                    sel.r = x.clamp(sel.l + MIN_SEL, cx);
                                    sel.t = y.clamp(0, sel.b - MIN_SEL);
                                }
                                Zone::Bl => {
                                    sel.l = x.clamp(0, sel.r - MIN_SEL);
                                    sel.b = y.clamp(sel.t + MIN_SEL, cy);
                                }
                                Zone::Br => {
                                    sel.r = x.clamp(sel.l + MIN_SEL, cx);
                                    sel.b = y.clamp(sel.t + MIN_SEL, cy);
                                }
                                _ => {}
                            }
                            true
                        }
                        None => false,
                    }
                });
                if changed {
                    invalidate(hwnd);
                }
                // The overlay holds mouse capture for its whole lifetime and
                // Windows does NOT send WM_SETCURSOR while input is captured,
                // so the cursor must be updated from the move handler.
                set_zone_cursor();
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                // Releasing the initial drag enters the adjust phase (edges,
                // corners, move); confirming is explicit: Enter, double-click
                // or the toolbar. Any in-flight adjustment ends with the
                // button — otherwise a released resize would keep tracking
                // the mouse forever.
                STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.dragging = false;
                    state.adjust = None;
                });
                invalidate(hwnd);
                LRESULT(0)
            }
            WM_LBUTTONDBLCLK => {
                let confirm = STATE.with(|s| s.borrow().sel.valid());
                if confirm {
                    STATE.with(|s| s.borrow_mut().confirmed = true);
                    RUNNING.store(false, Ordering::SeqCst);
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            WM_RBUTTONDOWN => {
                RUNNING.store(false, Ordering::SeqCst);
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_KEYDOWN => match wparam.0 as u32 {
                // Enter confirms the adjustable selection.
                0x0D => {
                    let confirm = STATE.with(|s| s.borrow().sel.valid());
                    if confirm {
                        STATE.with(|s| s.borrow_mut().confirmed = true);
                        RUNNING.store(false, Ordering::SeqCst);
                        PostQuitMessage(0);
                    }
                    LRESULT(0)
                }
                0x1B => {
                    RUNNING.store(false, Ordering::SeqCst);
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                _ => LRESULT(0),
            },
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
            } else {
                // Adjustable phase: corner handles show the selection can be
                // resized (edges too) and moved before confirming.
                const HS: i32 = 4;
                let handle_brush = CreateSolidBrush(COLORREF(0x00FFFFFF));
                let corners = [
                    (sel.l, sel.t),
                    (sel.r, sel.t),
                    (sel.l, sel.b),
                    (sel.r, sel.b),
                ];
                for (hx, hy) in corners {
                    let rect = RECT {
                        left: hx - HS,
                        top: hy - HS,
                        right: hx + HS,
                        bottom: hy + HS,
                    };
                    FillRect(hdc, &rect, handle_brush);
                }
                let _ = DeleteObject(handle_brush.into());

                // Confirm / cancel mini toolbar next to the selection.
                let (cx, cy) = virtual_size();
                let (panel, confirm, cancel) = toolbar_rects(sel, cx, cy);
                let panel_bg = CreateSolidBrush(COLORREF(0x002E1C15));
                FillRect(hdc, &panel, panel_bg);
                let _ = DeleteObject(panel_bg.into());
                let panel_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00452D23));
                let old_pen = SelectObject(hdc, panel_pen.into());
                let null_brush = GetStockObject(NULL_BRUSH);
                let old_brush = SelectObject(hdc, null_brush);
                let _ = Rectangle(hdc, panel.left, panel.top, panel.right, panel.bottom);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                let _ = DeleteObject(panel_pen.into());

                // Confirm button: accent background with a white check.
                let confirm_bg = CreateSolidBrush(COLORREF(0x00FFAE00));
                FillRect(hdc, &confirm, confirm_bg);
                let _ = DeleteObject(confirm_bg.into());
                let white_pen = CreatePen(PS_SOLID, 2, COLORREF(0x00FFFFFF));
                let old_pen2 = SelectObject(hdc, white_pen.into());
                let _ = MoveToEx(hdc, confirm.left + 9, confirm.top + 13, None);
                let _ = LineTo(hdc, confirm.left + 14, confirm.top + 18);
                let _ = LineTo(hdc, confirm.left + 25, confirm.top + 7);

                // Cancel button: dark gray with a white cross.
                let cancel_bg = CreateSolidBrush(COLORREF(0x00584238));
                FillRect(hdc, &cancel, cancel_bg);
                let _ = DeleteObject(cancel_bg.into());
                let _ = MoveToEx(hdc, cancel.left + 11, cancel.top + 7, None);
                let _ = LineTo(hdc, cancel.left + 23, cancel.top + 19);
                let _ = MoveToEx(hdc, cancel.left + 23, cancel.top + 7, None);
                let _ = LineTo(hdc, cancel.left + 11, cancel.top + 19);
                SelectObject(hdc, old_pen2);
                let _ = DeleteObject(white_pen.into());
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

    /// Pick and set the cursor for the current selection phase. Called from
    /// WM_MOUSEMOVE — WM_SETCURSOR is not sent while the overlay holds mouse
    /// capture. Drawing keeps the crosshair; the adjust phase switches per
    /// zone (edge/corner sizes, move, plain arrow over toolbar/outside).
    unsafe fn set_zone_cursor() {
        let cursor = STATE.with(|s| {
            let st = s.borrow();
            if st.dragging || !st.sel.valid() {
                return IDC_CROSS;
            }
            let (cx, cy) = virtual_size();
            let (panel, _, _) = toolbar_rects(st.sel, cx, cy);
            if pt_in_rect(st.mouse_x, st.mouse_y, &panel) {
                return IDC_ARROW;
            }
            zone_cursor(zone_at(st.sel, st.mouse_x, st.mouse_y))
        });
        if let Ok(cursor) = LoadCursorW(None, cursor) {
            let _ = SetCursor(Some(cursor));
        }
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
