use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::time::Duration;
use tauri::{Monitor, WebviewWindow};

const MEASUREMENT_ERROR: &str = "最大化ウィンドウの描画領域を取得できませんでした。";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MaximizedDisplay {
    pub client_physical: PixelSize,
    pub scale_factor: f64,
    pub basis: String,
}

pub fn measure_maximized(window: &WebviewWindow) -> Result<MaximizedDisplay, String> {
    let owned_window = window.clone();
    let (send, receive) = mpsc::sync_channel(1);
    window
        .run_on_main_thread(move || {
            let result = owned_window
                .current_monitor()
                .map_err(|_| MEASUREMENT_ERROR.to_string())
                .and_then(|monitor| monitor.ok_or_else(|| MEASUREMENT_ERROR.to_string()))
                .and_then(|monitor| platform_measure(&owned_window, &monitor));
            let _ = send.send(result);
        })
        .map_err(|_| MEASUREMENT_ERROR.to_string())?;
    receive
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| MEASUREMENT_ERROR.to_string())?
}

fn checked_display(
    work: PixelSize,
    client: PixelSize,
    scale_factor: f64,
) -> Result<MaximizedDisplay, String> {
    if !scale_factor.is_finite()
        || !(0.5..=8.0).contains(&scale_factor)
        || work.width == 0
        || work.height == 0
        || work.width > 32768
        || work.height > 32768
        || client.width == 0
        || client.height == 0
        || client.width > work.width
        || client.height >= work.height
    {
        return Err(MEASUREMENT_ERROR.into());
    }
    Ok(MaximizedDisplay {
        client_physical: client,
        scale_factor,
        basis: "native-standard-window".into(),
    })
}

fn work_size(monitor: &Monitor) -> PixelSize {
    PixelSize {
        width: monitor.work_area().size.width,
        height: monitor.work_area().size.height,
    }
}

#[cfg(target_os = "macos")]
fn platform_measure(window: &WebviewWindow, monitor: &Monitor) -> Result<MaximizedDisplay, String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSWindow, NSWindowStyleMask};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let marker = MainThreadMarker::new().ok_or_else(|| MEASUREMENT_ERROR.to_string())?;
    let pointer = window
        .ns_window()
        .map_err(|_| MEASUREMENT_ERROR.to_string())?;
    let native_window = unsafe { pointer.cast::<NSWindow>().as_ref() }
        .ok_or_else(|| MEASUREMENT_ERROR.to_string())?;
    let screen = native_window
        .screen()
        .ok_or_else(|| MEASUREMENT_ERROR.to_string())?;
    let scale_factor = screen.backingScaleFactor();
    let work = work_size(monitor);
    let visible = screen.convertRectToBacking(screen.visibleFrame());
    if !matching_backing_size(visible.size.width, visible.size.height, work) {
        return Err(MEASUREMENT_ERROR.into());
    }
    let frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(
            f64::from(work.width) / scale_factor,
            f64::from(work.height) / scale_factor,
        ),
    );
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;
    let content = NSWindow::contentRectForFrameRect_styleMask(frame, style, marker);
    let backing = screen.convertRectToBacking(content);
    let client = rounded_size(backing.size.width, backing.size.height)?;
    checked_display(work, client, scale_factor)
}

#[cfg(any(target_os = "macos", test))]
fn rounded_size(width: f64, height: f64) -> Result<PixelSize, String> {
    if !width.is_finite()
        || !height.is_finite()
        || !(1.0..=32768.0).contains(&width)
        || !(1.0..=32768.0).contains(&height)
    {
        return Err(MEASUREMENT_ERROR.into());
    }
    Ok(PixelSize {
        width: width.round() as u32,
        height: height.round() as u32,
    })
}

#[cfg(any(target_os = "macos", test))]
fn matching_backing_size(width: f64, height: f64, work: PixelSize) -> bool {
    width.is_finite()
        && height.is_finite()
        && (width - f64::from(work.width)).abs() <= 1.0
        && (height - f64::from(work.height)).abs() <= 1.0
}

#[cfg(target_os = "windows")]
fn platform_measure(
    _window: &WebviewWindow,
    monitor: &Monitor,
) -> Result<MaximizedDisplay, String> {
    windows_measure::measure(monitor)
}

#[cfg(target_os = "windows")]
mod windows_measure {
    use super::*;
    use std::ptr::null;
    use std::ptr::null_mut;
    use std::sync::atomic::{AtomicU64, Ordering};
    use windows_sys::Win32::Foundation::{HINSTANCE, HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::HiDpi::{
        GetDpiForWindow, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, IsWindow, IsWindowVisible,
        IsZoomed, RegisterClassW, UnregisterClassW, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, WNDCLASSW,
        WS_CAPTION, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_APPWINDOW, WS_MAXIMIZE, WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    };

    static NEXT_PROBE: AtomicU64 = AtomicU64::new(0);

    struct DpiScope(DPI_AWARENESS_CONTEXT);

    impl Drop for DpiScope {
        fn drop(&mut self) {
            unsafe { SetThreadDpiAwarenessContext(self.0) };
        }
    }

    struct Probe {
        window: HWND,
        instance: HINSTANCE,
        class_name: Vec<u16>,
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            unsafe {
                if !self.window.is_null() {
                    DestroyWindow(self.window);
                }
                UnregisterClassW(self.class_name.as_ptr(), self.instance);
            }
        }
    }

    pub(super) fn measure(monitor: &Monitor) -> Result<MaximizedDisplay, String> {
        let area = monitor.work_area();
        measure_area(area.position.x, area.position.y, work_size(monitor))
    }

    fn measure_area(left: i32, top: i32, work: PixelSize) -> Result<MaximizedDisplay, String> {
        let x = i32::try_from(i64::from(left) + i64::from(work.width) / 2)
            .map_err(|_| MEASUREMENT_ERROR.to_string())?;
        let y = i32::try_from(i64::from(top) + i64::from(work.height) / 2)
            .map_err(|_| MEASUREMENT_ERROR.to_string())?;
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if previous.is_null() {
            return Err(MEASUREMENT_ERROR.into());
        }
        let _dpi_scope = DpiScope(previous);
        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            return Err(MEASUREMENT_ERROR.into());
        }
        let class_name = format!(
            "PrismRelayDisplayProbe-{}",
            NEXT_PROBE.fetch_add(1, Ordering::Relaxed)
        )
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            lpfnWndProc: Some(DefWindowProcW),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(MEASUREMENT_ERROR.into());
        }
        let mut probe = Probe {
            window: null_mut(),
            instance,
            class_name,
        };
        let style = WS_CLIPSIBLINGS
            | WS_CLIPCHILDREN
            | WS_SYSMENU
            | WS_MINIMIZEBOX
            | WS_CAPTION
            | WS_MAXIMIZEBOX
            | WS_THICKFRAME
            | WS_MAXIMIZE;
        probe.window = unsafe {
            CreateWindowExW(
                WS_EX_APPWINDOW,
                probe.class_name.as_ptr(),
                null(),
                style,
                x,
                y,
                32,
                32,
                null_mut(),
                null_mut(),
                instance,
                null(),
            )
        };
        if probe.window.is_null()
            || unsafe { IsWindowVisible(probe.window) } != 0
            || unsafe { IsZoomed(probe.window) } == 0
        {
            return Err(MEASUREMENT_ERROR.into());
        }
        let mut actual_monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let handle = unsafe { MonitorFromWindow(probe.window, MONITOR_DEFAULTTONEAREST) };
        if unsafe { GetMonitorInfoW(handle, &mut actual_monitor) } == 0
            || actual_monitor.rcWork.left != left
            || actual_monitor.rcWork.top != top
            || i64::from(actual_monitor.rcWork.right) - i64::from(actual_monitor.rcWork.left)
                != i64::from(work.width)
            || i64::from(actual_monitor.rcWork.bottom) - i64::from(actual_monitor.rcWork.top)
                != i64::from(work.height)
        {
            return Err(MEASUREMENT_ERROR.into());
        }
        let mut client = RECT::default();
        if unsafe { GetClientRect(probe.window, &mut client) } == 0 {
            return Err(MEASUREMENT_ERROR.into());
        }
        let client = PixelSize {
            width: u32::try_from(i64::from(client.right) - i64::from(client.left))
                .map_err(|_| MEASUREMENT_ERROR.to_string())?,
            height: u32::try_from(i64::from(client.bottom) - i64::from(client.top))
                .map_err(|_| MEASUREMENT_ERROR.to_string())?,
        };
        let scale_factor = f64::from(unsafe { GetDpiForWindow(probe.window) }) / 96.0;
        let measured = checked_display(work, client, scale_factor)?;
        let handle = probe.window;
        drop(probe);
        if unsafe { IsWindow(handle) } != 0 {
            return Err(MEASUREMENT_ERROR.into());
        }
        Ok(measured)
    }

    #[test]
    fn native_maximized_probe_measures_client_and_is_destroyed_between_calls() {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY};

        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        assert!(!previous.is_null());
        let _dpi_scope = DpiScope(previous);
        let handle = unsafe { MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        assert_ne!(unsafe { GetMonitorInfoW(handle, &mut info) }, 0);
        let work = PixelSize {
            width: (info.rcWork.right - info.rcWork.left) as u32,
            height: (info.rcWork.bottom - info.rcWork.top) as u32,
        };
        let first = measure_area(info.rcWork.left, info.rcWork.top, work).unwrap();
        let second = measure_area(info.rcWork.left, info.rcWork.top, work).unwrap();
        assert_eq!(first, second);
        assert!(first.client_physical.width <= work.width);
        assert!(first.client_physical.height < work.height);
        assert_eq!(first.basis, "native-standard-window");
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_measure(
    _window: &WebviewWindow,
    _monitor: &Monitor,
) -> Result<MaximizedDisplay, String> {
    Err(MEASUREMENT_ERROR.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: u32, height: u32) -> PixelSize {
        PixelSize { width, height }
    }

    #[test]
    fn decorated_maximized_client_excludes_caption_and_retains_backing_pixels() {
        let measured = checked_display(size(3024, 1820), size(3024, 1776), 2.0).unwrap();
        assert_eq!(measured.client_physical, size(3024, 1776));
        assert_eq!(measured.scale_factor, 2.0);
        assert_eq!(measured.basis, "native-standard-window");
        let encoded = serde_json::to_value(measured).unwrap();
        assert!(encoded.get("clientPhysical").is_some());
        assert!(encoded.get("scaleFactor").is_some());
    }

    #[test]
    fn windows_dpi_does_not_convert_client_pixels_into_logical_pixels() {
        let measured = checked_display(size(2560, 1380), size(2560, 1345), 1.5).unwrap();
        assert_eq!(measured.client_physical, size(2560, 1345));
        assert_eq!(measured.scale_factor, 1.5);
    }

    #[test]
    fn invalid_geometry_never_falls_back_to_fullscreen_or_whole_work_area() {
        for (work, client, scale) in [
            (size(1920, 1040), size(1920, 1040), 1.0),
            (size(1920, 1040), size(1920, 1080), 1.0),
            (size(1920, 1040), size(1921, 1000), 1.0),
            (size(1920, 1040), size(0, 1000), 1.0),
            (size(0, 1040), size(100, 1000), 1.0),
            (size(40000, 1040), size(100, 1000), 1.0),
            (size(1920, 1040), size(1920, 1000), f64::NAN),
            (size(1920, 1040), size(1920, 1000), 0.0),
        ] {
            assert!(checked_display(work, client, scale).is_err());
        }
    }

    #[test]
    fn backing_conversion_rejects_nonfinite_and_changed_monitor_geometry() {
        assert!(matching_backing_size(3024.0, 1820.0, size(3024, 1820)));
        assert!(matching_backing_size(3024.4, 1819.6, size(3024, 1820)));
        assert!(!matching_backing_size(1512.0, 910.0, size(3024, 1820)));
        assert!(!matching_backing_size(f64::NAN, 1820.0, size(3024, 1820)));
        assert_eq!(rounded_size(3024.0, 1775.6).unwrap(), size(3024, 1776));
        assert!(rounded_size(f64::INFINITY, 100.0).is_err());
        assert!(rounded_size(0.0, 100.0).is_err());
    }
}
