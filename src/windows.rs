use serde::Serialize;
use windows::{
    core::Result,
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        UI::WindowsAndMessaging::{SetForegroundWindow, EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextW, IsWindowVisible, SetWindowPos},
    },
};

#[derive(Debug, Serialize)]
pub struct WindowInfo {
    pub class_name: String,
    pub title: String,
    pub hwnd: usize,
    pub height: i32,
    pub width: i32,
    pub x: i32,
    pub y: i32,
}

// 列出所有窗口的安全接口
pub fn list_windows() -> Result<Vec<WindowInfo>> {
    let mut windows = Vec::new();

    // 调用 `EnumWindows`，内部通过回调函数枚举窗口
    unsafe {
        EnumWindows(Some(enum_windows_callback), LPARAM(&mut windows as *mut _ as _))?;
    }

    Ok(windows)
}

pub fn active_window(hwnd: usize) -> Result<()> {
    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        SetForegroundWindow(hwnd);
    }
    Ok(())
}

// 回调函数：被 `EnumWindows` 调用
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    let get_window_text = |hwnd: HWND| -> Option<String> {
        let mut buffer = vec![0u16; 1024 as usize];
        if GetWindowTextW(hwnd, &mut buffer) > 0 {
            return String::from_utf16(&buffer).ok();
        } else {
            return None;
        }
    };
    let get_class_name = |hwnd: HWND| -> Option<String> {
        let mut buffer = vec![0u16; 1024 as usize];
        if GetClassNameW(hwnd, &mut buffer) > 0 {
            return String::from_utf16(&buffer).ok();
        } else {
            return None;
        }
    };

    let get_window_size = |hwnd: HWND| -> Option<(i32, i32, i32, i32)> {
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            return Some((rect.right - rect.left, rect.bottom - rect.top, rect.left, rect.top));
        } else {
            return None;
        }
    };

    let title = get_window_text(hwnd).unwrap_or("??????".to_string());
    let class_name = get_class_name(hwnd).unwrap_or("??????".to_string());
    let (x, y, width, height) = get_window_size(hwnd).unwrap_or((0, 0, 0, 0));
    // 检查窗口是否可见
    if IsWindowVisible(hwnd).as_bool() && !title.is_empty() && width > 10 && height > 10 {
        windows.push(WindowInfo {
            class_name,
            title,
            hwnd: hwnd.0 as usize,
            height,
            width,
            x,
            y,
        });
    }

    BOOL::from(true) // 返回 true 继续枚举
}
