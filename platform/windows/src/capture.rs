use anyhow::{Context, Result};
use capture_core::{CaptureBackend, CaptureFrame, CaptureOptions, MonitorInfo, Rect, WindowInfo};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
mod win {
    use super::*;
    use image::RgbaImage;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
        EnumDisplayMonitors, GetDC, GetDIBits, GetMonitorInfoW, ReleaseDC, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HMONITOR, MONITORINFO,
        MONITORINFOEXW, MONITOR_DEFAULTTOPRIMARY, SRCCOPY,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetSystemMetrics, GetWindowRect, GetWindowTextW, IsIconic,
        IsWindowVisible, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    pub struct WindowsCaptureBackend;

    impl WindowsCaptureBackend {
        pub fn new() -> Result<Self> {
            Ok(Self)
        }

        fn capture_rect(&self, rect: &Rect) -> Result<CaptureFrame> {
            unsafe {
                let hdc_screen = GetDC(None);
                if hdc_screen.is_invalid() {
                    anyhow::bail!("Failed to get screen DC");
                }

                let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
                if hdc_mem.is_invalid() {
                    ReleaseDC(None, hdc_screen);
                    anyhow::bail!("Failed to create compatible DC");
                }

                let width = rect.width as i32;
                let height = rect.height as i32;

                let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
                if hbitmap.is_invalid() {
                    let _ = DeleteDC(hdc_mem);
                    ReleaseDC(None, hdc_screen);
                    anyhow::bail!("Failed to create compatible bitmap");
                }

                let old_obj = SelectObject(hdc_mem, hbitmap.into());

                let success = BitBlt(
                    hdc_mem,
                    0,
                    0,
                    width,
                    height,
                    Some(hdc_screen),
                    rect.x,
                    rect.y,
                    SRCCOPY,
                );

                if success.is_err() {
                    SelectObject(hdc_mem, old_obj);
                    let _ = DeleteObject(hbitmap.into());
                    let _ = DeleteDC(hdc_mem);
                    ReleaseDC(None, hdc_screen);
                    anyhow::bail!("BitBlt failed");
                }

                let mut bmi = BITMAPINFO::default();
                bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bmi.bmiHeader.biWidth = width;
                bmi.bmiHeader.biHeight = -height; // top-down
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB.0 as u32;

                let buf_size = (width * height * 4) as usize;
                let mut buffer: Vec<u8> = vec![0; buf_size];

                let lines = GetDIBits(
                    hdc_mem,
                    hbitmap,
                    0,
                    height as u32,
                    Some(buffer.as_mut_ptr() as *mut _),
                    &mut bmi,
                    DIB_RGB_COLORS,
                );

                SelectObject(hdc_mem, old_obj);
                let _ = DeleteObject(hbitmap.into());
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(None, hdc_screen);

                if lines == 0 {
                    anyhow::bail!("GetDIBits failed");
                }

                // Convert BGRA to RGBA
                let mut rgba_data = vec![0u8; buf_size];
                for i in (0..buf_size).step_by(4) {
                    rgba_data[i] = buffer[i + 2]; // R <- B
                    rgba_data[i + 1] = buffer[i + 1]; // G <- G
                    rgba_data[i + 2] = buffer[i]; // B <- R
                    rgba_data[i + 3] = 255; // A
                }

                let image = RgbaImage::from_raw(width as u32, height as u32, rgba_data)
                    .context("Failed to create RgbaImage from captured data")?;

                let monitor = self.find_monitor_for_rect(rect)?;
                let timestamp_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                Ok(CaptureFrame {
                    image,
                    monitor,
                    timestamp_ms,
                })
            }
        }

        fn find_monitor_for_rect(&self, rect: &Rect) -> Result<MonitorInfo> {
            let monitors = self.enumerate_monitors()?;
            let cx = rect.x + rect.width as i32 / 2;
            let cy = rect.y + rect.height as i32 / 2;

            for m in &monitors {
                if cx >= m.bounds.x
                    && cx < m.bounds.x + m.bounds.width as i32
                    && cy >= m.bounds.y
                    && cy < m.bounds.y + m.bounds.height as i32
                {
                    return Ok(m.clone());
                }
            }

            monitors
                .into_iter()
                .find(|m| m.is_primary)
                .or_else(|| self.enumerate_monitors().ok().and_then(|m| m.into_iter().next()))
                .context("No monitor found")
        }

        fn get_virtual_screen_rect() -> Rect {
            unsafe {
                Rect {
                    x: GetSystemMetrics(SM_XVIRTUALSCREEN),
                    y: GetSystemMetrics(SM_YVIRTUALSCREEN),
                    width: GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32,
                    height: GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32,
                }
            }
        }
    }

    impl CaptureBackend for WindowsCaptureBackend {
        fn capture_region(&self, options: &CaptureOptions) -> Result<CaptureFrame> {
            let rect = options
                .region
                .clone()
                .context("capture_region requires a region in options")?;
            self.capture_rect(&rect)
        }

        fn capture_fullscreen(&self) -> Result<CaptureFrame> {
            let rect = Self::get_virtual_screen_rect();
            self.capture_rect(&rect)
        }

        fn capture_window(&self, window_id: u64) -> Result<CaptureFrame> {
            unsafe {
                let hwnd = HWND(window_id as *mut _);
                let mut rect = RECT::default();
                GetWindowRect(hwnd, &mut rect).context("Failed to get window rect")?;

                let capture_rect = Rect {
                    x: rect.left,
                    y: rect.top,
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                };

                self.capture_rect(&capture_rect)
            }
        }

        fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>> {
            unsafe {
                extern "system" fn monitor_enum_proc(
                    hmonitor: HMONITOR,
                    _hdc: HDC,
                    _rect: *mut RECT,
                    data: LPARAM,
                ) -> windows::core::BOOL {
                    unsafe {
                        let monitors = &mut *(data.0 as *mut Vec<MonitorInfo>);

                        let mut info = MONITORINFOEXW::default();
                        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

                        let info_ptr =
                            &mut info as *mut MONITORINFOEXW as *mut MONITORINFO;

                        if !GetMonitorInfoW(hmonitor, info_ptr).as_bool() {
                            return windows::core::BOOL(1);
                        }

                        let rc = info.monitorInfo.rcMonitor;
                        let wa = info.monitorInfo.rcWork;
                        let is_primary =
                            info.monitorInfo.dwFlags & MONITOR_DEFAULTTOPRIMARY.0 != 0;

                        let name_end = info
                            .szDevice
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(info.szDevice.len());
                        let name = String::from_utf16_lossy(&info.szDevice[..name_end]);

                        let mut dpi_x: u32 = 96;
                        let mut dpi_y: u32 = 96;
                        let _ = GetDpiForMonitor(
                            hmonitor,
                            MDT_EFFECTIVE_DPI,
                            &mut dpi_x,
                            &mut dpi_y,
                        );

                        let scale_factor = dpi_x as f64 / 96.0;

                        monitors.push(MonitorInfo {
                            index: monitors.len(),
                            name,
                            bounds: Rect {
                                x: rc.left,
                                y: rc.top,
                                width: (rc.right - rc.left) as u32,
                                height: (rc.bottom - rc.top) as u32,
                            },
                            work_area: Rect {
                                x: wa.left,
                                y: wa.top,
                                width: (wa.right - wa.left) as u32,
                                height: (wa.bottom - wa.top) as u32,
                            },
                            scale_factor,
                            is_primary,
                        });

                        windows::core::BOOL(1)
                    }
                }

                let mut monitors: Vec<MonitorInfo> = Vec::new();
                let _ = EnumDisplayMonitors(
                    None,
                    None,
                    Some(monitor_enum_proc),
                    LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize),
                );

                Ok(monitors)
            }
        }

        fn enumerate_windows(&self) -> Result<Vec<WindowInfo>> {
            unsafe {
                extern "system" fn window_enum_proc(
                    hwnd: HWND,
                    data: LPARAM,
                ) -> windows::core::BOOL {
                    unsafe {
                        let windows_list = &mut *(data.0 as *mut Vec<WindowInfo>);

                        if IsWindowVisible(hwnd).as_bool() {
                            let mut title_buf = [0u16; 256];
                            let title_len = GetWindowTextW(hwnd, &mut title_buf);
                            let title =
                                String::from_utf16_lossy(&title_buf[..title_len as usize]);

                            if title.is_empty() {
                                return windows::core::BOOL(1);
                            }

                            let mut class_buf = [0u16; 256];
                            let class_len = GetClassNameW(hwnd, &mut class_buf);
                            let class_name =
                                String::from_utf16_lossy(&class_buf[..class_len as usize]);

                            let mut rect = RECT::default();
                            let _ = GetWindowRect(hwnd, &mut rect);

                            let is_minimized = IsIconic(hwnd).as_bool();

                            windows_list.push(WindowInfo {
                                id: hwnd.0 as u64,
                                title,
                                class_name,
                                bounds: Rect {
                                    x: rect.left,
                                    y: rect.top,
                                    width: (rect.right - rect.left) as u32,
                                    height: (rect.bottom - rect.top) as u32,
                                },
                                is_visible: true,
                                is_minimized,
                            });
                        }

                        windows::core::BOOL(1)
                    }
                }

                let mut windows_list: Vec<WindowInfo> = Vec::new();
                let _ = EnumWindows(
                    Some(window_enum_proc),
                    LPARAM(&mut windows_list as *mut Vec<WindowInfo> as isize),
                );

                Ok(windows_list)
            }
        }
    }
}

#[cfg(windows)]
pub use win::WindowsCaptureBackend;

#[cfg(not(windows))]
pub struct WindowsCaptureBackend;

#[cfg(not(windows))]
impl WindowsCaptureBackend {
    pub fn new() -> Result<Self> {
        anyhow::bail!("WindowsCaptureBackend is only available on Windows")
    }
}

#[cfg(not(windows))]
impl CaptureBackend for WindowsCaptureBackend {
    fn capture_region(&self, _options: &CaptureOptions) -> Result<CaptureFrame> {
        anyhow::bail!("Not supported on this platform")
    }
    fn capture_fullscreen(&self) -> Result<CaptureFrame> {
        anyhow::bail!("Not supported on this platform")
    }
    fn capture_window(&self, _window_id: u64) -> Result<CaptureFrame> {
        anyhow::bail!("Not supported on this platform")
    }
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>> {
        anyhow::bail!("Not supported on this platform")
    }
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>> {
        anyhow::bail!("Not supported on this platform")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    mod windows_tests {
        use super::*;

        #[test]
        fn backend_creation_succeeds() {
            let backend = WindowsCaptureBackend::new();
            assert!(backend.is_ok());
        }

        #[test]
        fn enumerate_monitors_returns_at_least_one() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let monitors = backend.enumerate_monitors().unwrap();
            assert!(!monitors.is_empty(), "Should have at least one monitor");

            let primary = monitors.iter().find(|m| m.is_primary);
            assert!(primary.is_some(), "Should have exactly one primary monitor");
        }

        #[test]
        fn monitor_bounds_are_valid() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let monitors = backend.enumerate_monitors().unwrap();

            for m in &monitors {
                assert!(m.bounds.width > 0, "Monitor width should be > 0");
                assert!(m.bounds.height > 0, "Monitor height should be > 0");
                assert!(m.scale_factor > 0.0, "Scale factor should be > 0");
                assert!(
                    m.scale_factor >= 1.0 && m.scale_factor <= 5.0,
                    "Scale factor {} seems unreasonable",
                    m.scale_factor
                );
                assert!(!m.name.is_empty(), "Monitor name should not be empty");
            }
        }

        #[test]
        fn monitor_work_area_within_bounds() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let monitors = backend.enumerate_monitors().unwrap();

            for m in &monitors {
                assert!(m.work_area.x >= m.bounds.x);
                assert!(m.work_area.y >= m.bounds.y);
                assert!(m.work_area.width <= m.bounds.width);
                assert!(m.work_area.height <= m.bounds.height);
            }
        }

        #[test]
        fn enumerate_windows_returns_visible_windows() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let windows = backend.enumerate_windows().unwrap();
            assert!(!windows.is_empty(), "Should have at least one visible window");

            for w in &windows {
                assert!(w.is_visible);
                assert!(!w.title.is_empty(), "Window title should not be empty");
                assert!(w.id != 0, "Window ID should not be zero");
            }
        }

        #[test]
        fn capture_fullscreen_produces_image() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let frame = backend.capture_fullscreen().unwrap();

            assert!(frame.image.width() > 0);
            assert!(frame.image.height() > 0);
            assert!(frame.timestamp_ms > 0);
            assert!(frame.monitor.bounds.width > 0);
        }

        #[test]
        fn capture_region_produces_correct_size() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let options = CaptureOptions {
                region: Some(Rect::new(0, 0, 200, 150)),
                ..Default::default()
            };
            let frame = backend.capture_region(&options).unwrap();

            assert_eq!(frame.image.width(), 200);
            assert_eq!(frame.image.height(), 150);
        }

        #[test]
        fn capture_region_without_region_fails() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let options = CaptureOptions::default();
            let result = backend.capture_region(&options);
            assert!(result.is_err());
        }

        #[test]
        fn capture_region_negative_coords() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let monitors = backend.enumerate_monitors().unwrap();
            let primary = monitors.iter().find(|m| m.is_primary).unwrap();

            let options = CaptureOptions {
                region: Some(Rect::new(primary.bounds.x, primary.bounds.y, 100, 100)),
                ..Default::default()
            };
            let frame = backend.capture_region(&options).unwrap();
            assert_eq!(frame.image.width(), 100);
            assert_eq!(frame.image.height(), 100);
        }

        #[test]
        fn captured_pixels_are_not_all_zero() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let options = CaptureOptions {
                region: Some(Rect::new(0, 0, 100, 100)),
                ..Default::default()
            };
            let frame = backend.capture_region(&options).unwrap();

            let has_nonzero = frame.image.pixels().any(|p| p[0] != 0 || p[1] != 0 || p[2] != 0);
            assert!(has_nonzero, "Captured image should not be entirely black");
        }

        #[test]
        fn captured_alpha_is_opaque() {
            let backend = WindowsCaptureBackend::new().unwrap();
            let options = CaptureOptions {
                region: Some(Rect::new(0, 0, 50, 50)),
                ..Default::default()
            };
            let frame = backend.capture_region(&options).unwrap();

            for pixel in frame.image.pixels() {
                assert_eq!(pixel[3], 255, "Alpha channel should be fully opaque");
            }
        }
    }

    #[cfg(not(windows))]
    mod non_windows_tests {
        use super::*;

        #[test]
        fn backend_creation_fails_on_non_windows() {
            let result = WindowsCaptureBackend::new();
            assert!(result.is_err());
        }

        #[test]
        fn all_operations_fail_on_non_windows() {
            // Can't create backend on non-windows, so this tests the stub exists
            let result = WindowsCaptureBackend::new();
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("only available on Windows"));
        }
    }
}
