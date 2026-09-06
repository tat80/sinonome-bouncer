use std::{io, sync::Arc};
use windows_sys::Win32::Foundation::{BOOL, LPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use crate::config::MonitorSelection;
use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

pub struct AppWindow {
    pub _event_window: Arc<Window>,
    pub hwnd: *mut core::ffi::c_void,
}

pub fn set_process_dpi_awareness() {
    unsafe {
        use windows_sys::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };

        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn virtual_work_area(selection: MonitorSelection) -> io::Result<(i32, i32, u32, u32)> {
    if selection == MonitorSelection::All {
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
                SM_YVIRTUALSCREEN,
            };
            return Ok((
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32,
                GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32,
            ));
        }
    }
    let monitor_number = match selection {
        MonitorSelection::Number(number) => number,
        MonitorSelection::All => unreachable!(),
    };
    let mut monitors = Vec::new();
    unsafe extern "system" fn collect_monitor(
        monitor: HMONITOR,
        _device_context: HDC,
        _monitor_rect: *mut windows_sys::Win32::Foundation::RECT,
        data: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(data as *mut Vec<(i32, i32, u32, u32)>);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..std::mem::zeroed()
        };
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            monitors.push((
                info.rcMonitor.left,
                info.rcMonitor.top,
                (info.rcMonitor.right - info.rcMonitor.left) as u32,
                (info.rcMonitor.bottom - info.rcMonitor.top) as u32,
            ));
        }
        1
    }
    unsafe {
        EnumDisplayMonitors(
            HDC::default(),
            core::ptr::null(),
            Some(collect_monitor),
            &mut monitors as *mut _ as LPARAM,
        );
    }
    monitors
        .get(monitor_number as usize - 1)
        .copied()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "指定されたモニタ番号がありません"))
}

pub fn make_window(event_loop: &ActiveEventLoop, area: (i32, i32, u32, u32)) -> AppWindow {
    let attributes = WindowAttributes::default()
        .with_decorations(false)
        .with_visible(false)
        .with_resizable(false)
        .with_inner_size(PhysicalSize::new(1, 1));
    let event_window = Arc::new(
        event_loop
            .create_window(attributes)
            .expect("イベント用ウインドウを作成できません"),
    );
    {
        AppWindow {
            _event_window: event_window,
            hwnd: create_native_window(area),
        }
    }
}

pub fn move_native_window(window: &AppWindow, x: i32, y: i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE};
    unsafe {
        SetWindowPos(
            window.hwnd as _,
            HWND_TOPMOST,
            x,
            y,
            0,
            0,
            SWP_NOACTIVATE | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE,
        );
    }
}

static NATIVE_CLOSE_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn native_close_requested() -> bool {
    NATIVE_CLOSE_REQUESTED.swap(false, std::sync::atomic::Ordering::AcqRel)
}

unsafe extern "system" fn native_window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, DestroyWindow, PostQuitMessage, WM_CLOSE,
    };
    if message == WM_CLOSE {
        NATIVE_CLOSE_REQUESTED.store(true, std::sync::atomic::Ordering::Release);
        DestroyWindow(hwnd);
        PostQuitMessage(0);
        return 0;
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

fn create_native_window(area: (i32, i32, u32, u32)) -> *mut core::ffi::c_void {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, RegisterClassW, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE,
        SWP_NOSENDCHANGING, SWP_NOSIZE, SW_SHOW, WNDCLASSW, WS_EX_NOREDIRECTIONBITMAP,
        WS_EX_TOOLWINDOW, WS_POPUP,
    };
    static CLASS_NAME: [u16; 15] = [
        b'S' as u16,
        b'i' as u16,
        b'n' as u16,
        b'o' as u16,
        b'n' as u16,
        b'o' as u16,
        b'm' as u16,
        b'e' as u16,
        b'G' as u16,
        b'p' as u16,
        b'u' as u16,
        b'W' as u16,
        b'n' as u16,
        b'd' as u16,
        0,
    ];
    static WINDOW_NAME: [u16; 8] = [
        b'B' as u16,
        b'o' as u16,
        b'u' as u16,
        b'n' as u16,
        b'c' as u16,
        b'e' as u16,
        b'r' as u16,
        0,
    ];
    unsafe {
        let instance = GetModuleHandleW(core::ptr::null());
        let class = WNDCLASSW {
            lpfnWndProc: Some(native_window_proc),
            hInstance: instance,
            lpszClassName: CLASS_NAME.as_ptr(),
            ..core::mem::zeroed()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW,
            CLASS_NAME.as_ptr(),
            WINDOW_NAME.as_ptr(),
            WS_POPUP,
            area.0,
            area.1,
            area.2 as i32,
            area.3 as i32,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            instance,
            core::ptr::null(),
        );
        ShowWindow(hwnd, SW_SHOW);
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            area.0,
            area.1,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOSENDCHANGING,
        );
        hwnd
    }
}
