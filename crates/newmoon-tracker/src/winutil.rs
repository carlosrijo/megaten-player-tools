//! Small Win32 helpers for the window: title bar colours, icon, bounds, light/dark setting, message box, explorer.

use std::ffi::c_void;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, PCWSTR, w};

pub fn hwnd_of(raw: isize) -> HWND {
    HWND(raw as *mut c_void)
}

/// The window's background colours: dark #141413, light #f6f5f2 (C# `ApplyChrome`).
pub fn chrome_rgb(dark: bool) -> (u8, u8, u8) {
    if dark { (0x14, 0x14, 0x13) } else { (0xf6, 0xf5, 0xf2) }
}

/// C# `SystemUsesDark()`: HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize AppsUseLightTheme == 0.
pub fn system_uses_dark() -> bool {
    unsafe {
        let mut data = 1u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        let r = RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut c_void),
            Some(&mut size),
        );
        r.is_ok() && data == 0
    }
}

/// The title bar (DWMWA_USE_IMMERSIVE_DARK_MODE = 20, DWMWA_CAPTION_COLOR = 35 on Windows 11) and the brush the
/// window class paints with, so there is no white flash before the page is drawn.
pub fn apply_chrome(hwnd: HWND, dark: bool) {
    let (r, g, b) = chrome_rgb(dark);
    unsafe {
        let on: i32 = if dark { 1 } else { 0 };
        let _ = DwmSetWindowAttribute(hwnd, windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(20), &on as *const i32 as *const c_void, 4);
        let caption: u32 = r as u32 | ((g as u32) << 8) | ((b as u32) << 16); // COLORREF
        let _ = DwmSetWindowAttribute(hwnd, windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(35), &caption as *const u32 as *const c_void, 4);
        let brush = CreateSolidBrush(COLORREF(caption));
        let old = SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, brush.0 as isize);
        if old != 0 {
            let _ = DeleteObject(HGDIOBJ(old as *mut c_void));
        }
        let _ = InvalidateRect(Some(hwnd), None, true);
    }
}

/// The window's and the taskbar's icon: the exe's own icon resource (app.ico), at the sizes Windows wants at this
/// screen scale (32 px at 100%, 48 px at 150%, ...), like the C# `new Icon(ico, SystemInformation.IconSize)`.
pub fn set_icons(hwnd: HWND) {
    unsafe {
        let dpi = GetDpiForWindow(hwnd).max(96);
        let Ok(module) = GetModuleHandleW(None) else { return };
        let hinst = windows::Win32::Foundation::HINSTANCE(module.0);
        for (which, cx, cy) in [
            (ICON_BIG, GetSystemMetricsForDpi(SM_CXICON, dpi), GetSystemMetricsForDpi(SM_CYICON, dpi)),
            (ICON_SMALL, GetSystemMetricsForDpi(SM_CXSMICON, dpi), GetSystemMetricsForDpi(SM_CYSMICON, dpi)),
        ] {
            // resource id 1: the icon winresource embeds
            if let Ok(icon) = LoadImageW(Some(hinst), PCWSTR(1usize as *const u16), IMAGE_ICON, cx, cy, LR_DEFAULTCOLOR) {
                SendMessageW(hwnd, WM_SETICON, Some(WPARAM(which as usize)), Some(LPARAM(icon.0 as isize)));
            }
        }
    }
}

/// Work areas of every screen (`Screen.AllScreens[].WorkingArea`).
pub fn work_areas() -> Vec<RECT> {
    unsafe extern "system" fn each(h: HMONITOR, _dc: HDC, _r: *mut RECT, lparam: LPARAM) -> BOOL {
        unsafe {
            let v = &mut *(lparam.0 as *mut Vec<RECT>);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if GetMonitorInfoW(h, &mut mi).as_bool() {
                v.push(mi.rcWork);
            }
            BOOL(1)
        }
    }
    let mut v: Vec<RECT> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&mut v as *mut Vec<RECT> as isize));
    }
    v
}

/// The work area of the primary screen.
pub fn primary_work_area() -> RECT {
    let mut r = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut r as *mut RECT as *mut c_void), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    if r.right <= r.left { RECT { left: 0, top: 0, right: 1920, bottom: 1080 } } else { r }
}

pub fn intersects(a: &RECT, b: &RECT) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/// Outer bounds of the window, and whether it is maximized: the normal (restored) bounds when it is not in its normal
/// state (C# `WindowState == Normal ? Bounds : RestoreBounds`).
pub fn saved_bounds(hwnd: HWND) -> Option<([i32; 4], bool)> {
    unsafe {
        let mut wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
        GetWindowPlacement(hwnd, &mut wp).ok()?;
        let maximized = wp.showCmd == SW_SHOWMAXIMIZED.0 as u32;
        let r = if wp.showCmd == SW_SHOWNORMAL.0 as u32 {
            let mut r = RECT::default();
            GetWindowRect(hwnd, &mut r).ok()?;
            r
        } else {
            wp.rcNormalPosition
        };
        Some(([r.left, r.top, r.right - r.left, r.bottom - r.top], maximized))
    }
}

/// Put the window's outer rectangle (pixels) where it was.
pub fn set_outer_bounds(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn outer_size(hwnd: HWND) -> (i32, i32) {
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut r);
    }
    (r.right - r.left, r.bottom - r.top)
}

pub fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut r = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut r);
    }
    (r.right, r.bottom)
}

pub fn show_maximized(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWMAXIMIZED);
    }
}

pub fn show_normal(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
}

/// An error box (C# `MessageBox.Show(this, text, title, OK, Error)`).
pub fn error_box(hwnd: Option<HWND>, text: &str, title: &str) {
    let t: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let c: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(hwnd, PCWSTR(t.as_ptr()), PCWSTR(c.as_ptr()), MB_OK | MB_ICONERROR);
    }
}

/// `Process.Start("explorer.exe", args)`: the arguments are passed as they are.
pub fn explorer(args: &str) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer.exe").raw_arg(args).spawn();
}

/// The virtual screen's right edge (`SystemInformation.VirtualScreen.Right`): past the last monitor.
pub fn virtual_screen_right() -> i32 {
    unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN) }
}

/// A live tour's "resize" step (C# `MainForm.ResizeAboutCentre`): the window is sized so its client area is w x h
/// (`SizeFromClientSize`), keeping the window's centre where it was.
pub fn resize_about_centre(hwnd: HWND, w: i32, h: i32) {
    unsafe {
        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r).is_err() {
            return;
        }
        let (cx, cy) = (r.left + (r.right - r.left) / 2, r.top + (r.bottom - r.top) / 2);
        let mut want = RECT { left: 0, top: 0, right: w, bottom: h };
        let style = WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32);
        let ex = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
        let dpi = GetDpiForWindow(hwnd).max(96);
        let _ = AdjustWindowRectExForDpi(&mut want, style, false, ex, dpi);
        let (ow, oh) = (want.right - want.left, want.bottom - want.top);
        let _ = SetWindowPos(hwnd, None, cx - ow / 2, cy - oh / 2, ow, oh, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

/// `Process.GetProcessById(pid).ProcessName`: the running program's file name without ".exe"; None when no such
/// process is running (or it cannot be looked at).
pub fn process_name(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let file = path.rsplit(['\\', '/']).next().unwrap_or(&path);
        Some(file.strip_suffix(".exe").or_else(|| file.strip_suffix(".EXE")).unwrap_or(file).to_string())
    }
}
