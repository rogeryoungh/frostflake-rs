use serde::Serialize;
use windows::{
    core::{Error, Result, HSTRING},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, S_OK},
        System::Console::{
            GetConsoleMode, GetConsoleWindow, GetStdHandle, SetConsoleMode, CONSOLE_MODE,
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
        },
        UI::WindowsAndMessaging::{EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextW, SetForegroundWindow},
    },
    UI::Notifications::{ToastNotification, ToastNotificationManager, ToastTemplateType},
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
        SetForegroundWindow(hwnd).ok()
    }
}

pub fn enable_virtual_terminal_sequences() -> Result<()> {
    unsafe {
        // 获取标准输出句柄
        let handle = GetStdHandle(STD_OUTPUT_HANDLE)?;
        let mut mode: CONSOLE_MODE = CONSOLE_MODE(0);
        GetConsoleMode(handle, &mut mode as *mut _)?;
        // 启用虚拟终端处理，支持彩色
        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        SetConsoleMode(handle, mode)
    }
}

pub fn active_console_window() -> Result<()> {
    unsafe {
        let hwnd = GetConsoleWindow();
        let result = SetForegroundWindow(hwnd).ok();
        if let Err(err) = result {
            if err.code() != S_OK {
                Err(err)
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

pub fn notify_message(title: &str, message: &str) -> Result<()> {
    let template = ToastTemplateType::ToastText01;
    let toast_xml = ToastNotificationManager::GetTemplateContent(template)?;
    let text_elements = toast_xml.GetElementsByTagName(&HSTRING::from("text"))?;
    text_elements
        .Item(0)?
        .AppendChild(&toast_xml.CreateTextNode(&HSTRING::from(message))?)?;
    let toast = ToastNotification::CreateToastNotification(&toast_xml)?;
    let app_id = HSTRING::from(title);
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&app_id)?;
    notifier.Show(&toast)
}

// 回调函数：被 `EnumWindows` 调用
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    let get_f_into_string = |f: unsafe fn(HWND, &mut [u16]) -> i32, hwnd: HWND| {
        let mut buffer = vec![0u16; 1024_usize];
        if f(hwnd, &mut buffer) > 0 {
            let s = String::from_utf16(&buffer)?;
            Ok(String::from(s.trim_end_matches('\0')))
        } else {
            Err(Error::from_win32())
        }
    };

    let get_window_size = |hwnd: HWND| {
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Some(rect)
        } else {
            None
        }
    };

    let title = get_f_into_string(GetWindowTextW, hwnd).unwrap_or("??????".to_string());

    let class_name = get_f_into_string(GetClassNameW, hwnd).unwrap_or("??????".to_string());

    let rect = get_window_size(hwnd).unwrap_or_default();
    let (x, y, width, height) = (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top);

    if !title.is_empty() && width > 10 && height > 10 {
        windows.push(WindowInfo {
            class_name,
            title: title.to_string(),
            hwnd: hwnd.0 as usize,
            height,
            width,
            x,
            y,
        });
    }

    BOOL::from(true) // 返回 true 继续枚举
}
