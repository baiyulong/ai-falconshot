use anyhow::Result;
use floating_core::{FloatingState, FloatingWindow, TransformState};
use std::path::Path;

#[cfg(windows)]
mod win {
    use super::*;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, POINT, SIZE};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, RegisterClassExW,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, UpdateLayeredWindow, GWL_EXSTYLE, HTCAPTION,
        HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, ULW_ALPHA,
        WM_DESTROY, WM_NCHITTEST, WM_NCLBUTTONDBLCLK, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_TOPMOST,
        WS_EX_TRANSPARENT, WS_POPUP,
    };

    pub struct WindowsFloatingWindow {
        state: FloatingState,
        hwnd: isize,
    }

    unsafe impl Send for WindowsFloatingWindow {}

    impl Default for WindowsFloatingWindow {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Margin (device px) added around a pinned image for its glowing border.
    const PIN_GLOW_MARGIN: u32 = 10;
    const PIN_ACCENT: [u8; 3] = [0, 174, 255]; // #00AEFF

    /// Expand the capture with a 1px accent border and an alpha glow that
    /// fades outward. Channels are premultiplied for UpdateLayeredWindow.
    fn decorate_pin(image: image::RgbaImage) -> image::RgbaImage {
        let (w, h) = image.dimensions();
        let m = PIN_GLOW_MARGIN;
        let (ew, eh) = (w + 2 * m, h + 2 * m);
        let mut out = image::RgbaImage::new(ew, eh);

        for ey in 0..eh {
            for ex in 0..ew {
                // Chebyshev distance outside the image rect (0 = on image).
                let dx = if ex < m {
                    (m - ex) as i32
                } else if ex >= m + w {
                    (ex - m - w + 1) as i32
                } else {
                    0
                };
                let dy = if ey < m {
                    (m - ey) as i32
                } else if ey >= m + h {
                    (ey - m - h + 1) as i32
                } else {
                    0
                };
                let d = dx.max(dy);

                let pixel = if d == 0 {
                    // On the image: tint the outermost pixel ring with the
                    // accent color so the border reads as crisp.
                    let ix = ex - m;
                    let iy = ey - m;
                    let mut px = *image.get_pixel(ix, iy);
                    let on_edge = ix == 0 || iy == 0 || ix == w - 1 || iy == h - 1;
                    if on_edge {
                        for c in 0..3 {
                            px[c] = ((px[c] as u32 * 6 + PIN_ACCENT[c] as u32 * 4) / 10) as u8;
                        }
                    }
                    px
                } else if d == 1 {
                    // Solid 1px accent ring just outside the image.
                    image::Rgba([PIN_ACCENT[0], PIN_ACCENT[1], PIN_ACCENT[2], 255])
                } else {
                    // Halo fading outward, premultiplied alpha.
                    let t = (d - 1) as f32 / m as f32;
                    let a = (0.5 * (1.0 - t) * 255.0) as u8;
                    let k = a as u32;
                    image::Rgba([
                        ((PIN_ACCENT[0] as u32 * k) / 255) as u8,
                        ((PIN_ACCENT[1] as u32 * k) / 255) as u8,
                        ((PIN_ACCENT[2] as u32 * k) / 255) as u8,
                        a,
                    ])
                };
                out.put_pixel(ex, ey, pixel);
            }
        }
        out
    }

    impl WindowsFloatingWindow {
        pub fn new() -> Self {
            Self {
                state: FloatingState {
                    id: String::new(),
                    image_path: String::new(),
                    x: 100,
                    y: 100,
                    width: 0,
                    height: 0,
                    transform: TransformState::default(),
                    opacity: 1.0,
                    always_on_top: true,
                    mouse_passthrough: false,
                    locked_position: false,
                    locked_size: false,
                    group_id: None,
                },
                hwnd: 0,
            }
        }

        fn get_hwnd(&self) -> HWND {
            HWND(self.hwnd as *mut _)
        }

        unsafe fn render_image(&self, image: &image::RgbaImage) -> Result<()> {
            let hwnd = self.get_hwnd();
            let (width, height) = image.dimensions();

            let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));

            let mut bmi = BITMAPINFO::default();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width as i32;
            bmi.bmiHeader.biHeight = -(height as i32);
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbitmap = windows::Win32::Graphics::Gdi::CreateDIBSection(
                Some(hdc_mem),
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            )
            .map_err(|e| anyhow::anyhow!("CreateDIBSection failed: {}", e))?;

            let dst =
                std::slice::from_raw_parts_mut(bits as *mut u8, (width * height * 4) as usize);
            for (i, pixel) in image.pixels().enumerate() {
                let offset = i * 4;
                dst[offset] = pixel[2];
                dst[offset + 1] = pixel[1];
                dst[offset + 2] = pixel[0];
                dst[offset + 3] = pixel[3];
            }

            let old_obj = SelectObject(hdc_mem, hbitmap.into());

            let pt_src = POINT { x: 0, y: 0 };
            let size = SIZE {
                cx: width as i32,
                cy: height as i32,
            };
            let pt_dst = POINT {
                x: self.state.x,
                y: self.state.y,
            };

            let blend = BLENDFUNCTION {
                BlendOp: 0,
                BlendFlags: 0,
                SourceConstantAlpha: (self.state.opacity * 255.0) as u8,
                AlphaFormat: 1,
            };

            UpdateLayeredWindow(
                hwnd,
                Some(hdc_screen),
                Some(&pt_dst),
                Some(&size),
                Some(hdc_mem),
                Some(&pt_src),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )?;

            SelectObject(hdc_mem, old_obj);
            let _ = DeleteObject(hbitmap.into());
            let _ = DeleteDC(hdc_mem);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);

            Ok(())
        }
    }

    impl FloatingWindow for WindowsFloatingWindow {
        fn create(&mut self, image_path: &Path, state: &FloatingState) -> Result<()> {
            unsafe {
                self.state = state.clone();
                self.state.image_path = image_path.to_string_lossy().to_string();

                let image = image::open(image_path)
                    .map_err(|e| anyhow::anyhow!("Failed to open image: {}", e))?
                    .to_rgba8();

                // Expand the image with a glowing 1px border + halo so the
                // pinned capture stands out from the desktop behind it.
                let image = decorate_pin(image);
                let (width, height) = image.dimensions();
                // The glow margin grows the window around the requested rect.
                self.state.x -= PIN_GLOW_MARGIN as i32;
                self.state.y -= PIN_GLOW_MARGIN as i32;
                self.state.width = width;
                self.state.height = height;

                let hinstance = GetModuleHandleW(None)?;
                let class_name: Vec<u16> = "FalconShotFloating\0".encode_utf16().collect();

                extern "system" fn floating_wnd_proc(
                    hwnd: HWND,
                    msg: u32,
                    wparam: WPARAM,
                    lparam: LPARAM,
                ) -> LRESULT {
                    unsafe {
                        match msg {
                            // The whole pinned image drags like a title bar.
                            WM_NCHITTEST => LRESULT(HTCAPTION as isize),
                            // Double-click closes the pinned image. Never post
                            // WM_QUIT here: pin windows share the app's main
                            // thread event loop.
                            WM_NCLBUTTONDBLCLK => {
                                let _ = DestroyWindow(hwnd);
                                LRESULT(0)
                            }
                            WM_DESTROY => DefWindowProcW(hwnd, msg, wparam, lparam),
                            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
                        }
                    }
                }

                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    lpfnWndProc: Some(floating_wnd_proc),
                    hInstance: hinstance.into(),
                    lpszClassName: PCWSTR(class_name.as_ptr()),
                    ..Default::default()
                };
                RegisterClassExW(&wc);

                let mut ex_style = WS_EX_LAYERED | WS_EX_TOPMOST;
                if self.state.mouse_passthrough {
                    ex_style |= WS_EX_TRANSPARENT;
                }

                let hwnd = CreateWindowExW(
                    ex_style,
                    PCWSTR(class_name.as_ptr()),
                    PCWSTR::null(),
                    WS_POPUP,
                    self.state.x,
                    self.state.y,
                    width as i32,
                    height as i32,
                    None,
                    None,
                    Some(hinstance.into()),
                    None,
                )?;

                self.hwnd = hwnd.0 as isize;
                self.render_image(&image)?;
                let _ = ShowWindow(hwnd, SW_SHOW);

                Ok(())
            }
        }

        fn close(&mut self) -> Result<()> {
            if self.hwnd != 0 {
                unsafe {
                    let _ = DestroyWindow(self.get_hwnd());
                }
                self.hwnd = 0;
            }
            Ok(())
        }

        fn show(&mut self) -> Result<()> {
            if self.hwnd != 0 {
                unsafe {
                    let _ = ShowWindow(self.get_hwnd(), SW_SHOW);
                }
            }
            Ok(())
        }

        fn hide(&mut self) -> Result<()> {
            if self.hwnd != 0 {
                unsafe {
                    let _ = ShowWindow(self.get_hwnd(), SW_HIDE);
                }
            }
            Ok(())
        }

        fn set_transform(&mut self, transform: &TransformState) -> Result<()> {
            self.state.transform = transform.clone();
            Ok(())
        }

        fn set_opacity(&mut self, opacity: f32) -> Result<()> {
            self.state.opacity = opacity.clamp(0.0, 1.0);
            if self.hwnd != 0 {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
                        self.get_hwnd(),
                        windows::Win32::Foundation::COLORREF(0),
                        (self.state.opacity * 255.0) as u8,
                        windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA,
                    );
                }
            }
            Ok(())
        }

        fn set_mouse_passthrough(&mut self, enabled: bool) -> Result<()> {
            self.state.mouse_passthrough = enabled;
            if self.hwnd != 0 {
                unsafe {
                    let hwnd = self.get_hwnd();
                    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                    let new_style = if enabled {
                        style | WS_EX_TRANSPARENT.0
                    } else {
                        style & !WS_EX_TRANSPARENT.0
                    };
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style as isize);
                }
            }
            Ok(())
        }

        fn set_always_on_top(&mut self, enabled: bool) -> Result<()> {
            self.state.always_on_top = enabled;
            if self.hwnd != 0 {
                unsafe {
                    let insert_after = if enabled { Some(HWND_TOPMOST) } else { None };
                    let _ = SetWindowPos(
                        self.get_hwnd(),
                        insert_after,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                    );
                }
            }
            Ok(())
        }

        fn get_state(&self) -> &FloatingState {
            &self.state
        }
    }
}

#[cfg(windows)]
pub use win::WindowsFloatingWindow;

#[cfg(not(windows))]
pub struct WindowsFloatingWindow {
    state: FloatingState,
}

#[cfg(not(windows))]
impl WindowsFloatingWindow {
    pub fn new() -> Self {
        Self {
            state: FloatingState {
                id: String::new(),
                image_path: String::new(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                transform: TransformState::default(),
                opacity: 1.0,
                always_on_top: true,
                mouse_passthrough: false,
                locked_position: false,
                locked_size: false,
                group_id: None,
            },
        }
    }
}

#[cfg(not(windows))]
impl FloatingWindow for WindowsFloatingWindow {
    fn create(&mut self, _image_path: &Path, _state: &FloatingState) -> Result<()> {
        anyhow::bail!("Not supported on this platform")
    }
    fn close(&mut self) -> Result<()> {
        Ok(())
    }
    fn show(&mut self) -> Result<()> {
        Ok(())
    }
    fn hide(&mut self) -> Result<()> {
        Ok(())
    }
    fn set_transform(&mut self, _transform: &TransformState) -> Result<()> {
        Ok(())
    }
    fn set_opacity(&mut self, _opacity: f32) -> Result<()> {
        Ok(())
    }
    fn set_mouse_passthrough(&mut self, _enabled: bool) -> Result<()> {
        Ok(())
    }
    fn set_always_on_top(&mut self, _enabled: bool) -> Result<()> {
        Ok(())
    }
    fn get_state(&self) -> &FloatingState {
        &self.state
    }
}
