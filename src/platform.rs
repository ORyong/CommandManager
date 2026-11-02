//! 桌面服务封装：托盘、全局热键、单实例与开机自启。
use anyhow::{Context as _, Result, ensure};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
use tray_icon::{
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hotkey: String,
    pub autostart: bool,
    pub theme: crate::style::ThemeChoice,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt+X".into(),
            autostart: false,
            theme: crate::style::ThemeChoice::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Show,
    Toggle,
    Settings,
    Quit,
}

pub struct Desktop {
    _tray: TrayIcon,
    manager: GlobalHotKeyManager,
    hotkey: Option<HotKey>,
    rx: mpsc::Receiver<Event>,
    pub settings: Settings,
    pub initial_error: Option<String>,
    path: PathBuf,
}

impl Desktop {
    pub fn new(directory: &Path) -> Result<Self> {
        let path = directory.join("settings.json");
        let mut initial_error = None;
        let settings: Settings = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(settings) => settings,
                Err(error) => {
                    initial_error =
                        Some(format!("设置文件无法读取，使用默认值，原文件保留：{error}"));
                    Settings::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(error) => {
                initial_error = Some(format!("读取设置失败：{error}"));
                Settings::default()
            }
        };
        let manager = GlobalHotKeyManager::new()?;
        let hotkey = match settings.hotkey.parse::<HotKey>() {
            Ok(key) => match manager.register(key) {
                Ok(()) => Some(key),
                Err(error) => {
                    initial_error = Some(format!(
                        "快捷键 {} 被占用或无法注册，请在设置中修改：{error}",
                        settings.hotkey
                    ));
                    None
                }
            },
            Err(error) => {
                initial_error = Some(format!("快捷键设置不合法：{error}"));
                None
            }
        };
        let (tx, rx) = mpsc::channel();
        let menu = Menu::new();
        let show = MenuItem::new("打开命令管理器", true, None);
        let settings_item = MenuItem::new("设置", true, None);
        let quit = MenuItem::new("退出", true, None);
        menu.append_items(&[
            &show,
            &settings_item,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;
        let show_id = show.id().clone();
        let settings_id = settings_item.id().clone();
        let quit_id = quit.id().clone();
        let menu_tx = tx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let event = if event.id == show_id {
                Event::Show
            } else if event.id == settings_id {
                Event::Settings
            } else if event.id == quit_id {
                Event::Quit
            } else {
                return;
            };
            let _ = menu_tx.send(event);
        }));
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = tx.send(Event::Show);
            }
        }));
        let pixels = include_bytes!("../assets/app-icon-32.rgba").to_vec();
        let tray = TrayIconBuilder::new()
            .with_tooltip(format!(
                "命令管理器 {} · {}",
                env!("CARGO_PKG_VERSION"),
                settings.hotkey
            ))
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_icon(tray_icon::Icon::from_rgba(pixels, 32, 32)?)
            .build()?;
        Ok(Self {
            _tray: tray,
            manager,
            hotkey,
            rx,
            settings,
            initial_error,
            path,
        })
    }

    pub fn poll(&self) -> Vec<Event> {
        let mut events: Vec<_> = self.rx.try_iter().collect();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Pressed
                && self.hotkey.is_some_and(|key| key.id() == event.id)
            {
                events.push(Event::Toggle);
            }
        }
        events
    }

    pub fn prepare_settings(&mut self, next: &Settings) -> Result<()> {
        let key: HotKey = next
            .hotkey
            .parse()
            .context("快捷键格式错误，例如 Ctrl+Alt+X")?;
        ensure!(
            key.mods.bits() != 0,
            "全局快捷键必须包含 Ctrl、Alt、Shift 或 Win 修饰键"
        );
        let changed = self.hotkey != Some(key);
        if changed {
            self.manager
                .register(key)
                .context("快捷键被占用，原快捷键继续有效")?;
        }
        Ok(())
    }

    pub fn finish_settings(&mut self, next: Settings, success: bool) {
        let key = next
            .hotkey
            .parse::<HotKey>()
            .expect("previously validated hotkey");
        if success {
            if self.hotkey != Some(key) {
                if let Some(old) = self.hotkey {
                    let _ = self.manager.unregister(old);
                }
            }
            self.hotkey = Some(key);
            self.settings = next;
        } else if self.hotkey != Some(key) {
            let _ = self.manager.unregister(key);
        }
    }

    pub fn settings_path(&self) -> PathBuf {
        self.path.clone()
    }
}

#[cfg(windows)]
pub fn autostart(enabled: bool) -> Result<()> {
    use windows_sys::Win32::System::Registry::*;
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut key = std::ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run").as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    ensure!(status == 0, "无法设置登录启动：Windows 错误 {status}");
    let name = wide("StandaloneCommandManager");
    let status = if enabled {
        let command = wide(&format!(
            "\"{}\" --background",
            std::env::current_exe()?.display()
        ));
        unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, name.as_ptr()) }
    };
    unsafe {
        RegCloseKey(key);
    }
    ensure!(
        status == 0 || (!enabled && status == 2),
        "无法修改登录启动：Windows 错误 {status}"
    );
    Ok(())
}

#[cfg(not(windows))]
pub fn autostart(_: bool) -> Result<()> {
    anyhow::bail!("当前安装包仅支持 Windows")
}

pub fn hwnd(window: &gpui::Window) -> isize {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window)
        .ok()
        .map(|h| h.as_raw())
    {
        Some(RawWindowHandle::Win32(handle)) => handle.hwnd.get(),
        _ => 0,
    }
}

/// Strip the caption bits Windows still adds and switch off DWM's non-client
/// rendering: otherwise the window keeps an invisible border plus a drop shadow
/// that reads as a grey halo around the panel.
#[cfg(windows)]
pub fn make_frameless(window: &gpui::Window) {
    use windows_sys::Win32::Graphics::Dwm::{
        DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetWindowLongW, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SetWindowLongW, SetWindowPos, WS_BORDER, WS_CAPTION, WS_DLGFRAME, WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    };
    let handle = hwnd(window) as *mut core::ffi::c_void;
    if handle.is_null() {
        return;
    }
    unsafe {
        let style = GetWindowLongW(handle, GWL_STYLE);
        let bare = style
            & !((WS_CAPTION | WS_BORDER | WS_DLGFRAME | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX
                | WS_MAXIMIZEBOX) as i32);
        if bare != style {
            SetWindowLongW(handle, GWL_STYLE, bare);
        }
        let policy = DWMNCRP_DISABLED;
        DwmSetWindowAttribute(
            handle,
            DWMWA_NCRENDERING_POLICY as u32,
            &policy as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&policy) as u32,
        );
        SetWindowPos(
            handle,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        );
    }
}

#[cfg(not(windows))]
pub fn make_frameless(_window: &gpui::Window) {}

pub fn hide(window: &gpui::Window) {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
            hwnd(window) as _,
            windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
        );
    }
    release_working_set();
}

/// 收进托盘后把常驻页还给系统。
///
/// 窗口隐藏后字库、字形图集和 GPU 映射还留在工作集里；这里让系统把它们
/// 换出，隐藏态从几十 MB 降到个位数。再次唤出会重新缺页载入，代价是一次
/// 可见的重绘延迟，换来托盘常驻时不占内存。
#[cfg(windows)]
pub(crate) fn release_working_set() {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
    unsafe {
        SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

#[cfg(not(windows))]
pub(crate) fn release_working_set() {}

pub fn set_window_icon(window: &gpui::Window) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*,
        };
        let module = GetModuleHandleW(std::ptr::null());
        for (kind, side) in [(ICON_SMALL, 16), (ICON_BIG, 32)] {
            let icon = LoadImageW(
                module,
                1usize as *const u16,
                IMAGE_ICON,
                side,
                side,
                LR_SHARED,
            );
            if !icon.is_null() {
                SendMessageW(hwnd(window) as _, WM_SETICON, kind as usize, icon as isize);
            }
        }
    }
}

pub fn is_visible(window: &gpui::Window) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};
        let handle = hwnd(window) as _;
        IsWindowVisible(handle) != 0 && IsIconic(handle) == 0
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        true
    }
}

pub fn show(window: &gpui::Window) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        ShowWindow(hwnd(window) as _, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd(window) as _);
    }
    window.activate_window();
}

/// Screen rectangle in physical pixels.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScreenRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// Hands the window to the system drag loop. GPUI's default implementation is
/// a no-op on Windows, so the caption hit-test is posted explicitly. It must be
/// posted, not sent: a synchronous call would re-enter GPUI's own message pump
/// while the event handler still holds its context borrow.
#[cfg(windows)]
pub fn begin_drag(window: &gpui::Window) {
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::ReleaseCapture,
        WindowsAndMessaging::{HTCAPTION, PostMessageW, WM_NCLBUTTONDOWN},
    };
    let handle = hwnd(window);
    if handle == 0 {
        return;
    }
    unsafe {
        ReleaseCapture();
        PostMessageW(handle as _, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
    }
}

#[cfg(not(windows))]
pub fn begin_drag(_window: &gpui::Window) {}

#[cfg(windows)]
pub fn left_button_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    const VK_LBUTTON: i32 = 0x01;
    unsafe { (GetAsyncKeyState(VK_LBUTTON) as u16 & 0x8000) != 0 }
}

#[cfg(not(windows))]
pub fn left_button_down() -> bool {
    false
}

#[cfg(windows)]
pub fn window_rect(window: &gpui::Window) -> ScreenRect {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::GetWindowRect,
    };
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        GetWindowRect(hwnd(window) as _, &mut rect);
    }
    ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

#[cfg(not(windows))]
pub fn window_rect(_window: &gpui::Window) -> ScreenRect {
    ScreenRect::default()
}

/// Work area of the monitor the window sits on, excluding the taskbar.
#[cfg(windows)]
pub fn work_area(window: &gpui::Window) -> ScreenRect {
    use windows_sys::Win32::{
        Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
        },
        UI::WindowsAndMessaging::GetCursorPos,
    };
    let empty = windows_sys::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: empty,
        rcWork: empty,
        dwFlags: 0,
    };
    unsafe {
        let monitor = MonitorFromWindow(hwnd(window) as _, MONITOR_DEFAULTTONEAREST);
        if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
            let r = info.rcWork;
            return ScreenRect {
                left: r.left,
                top: r.top,
                right: r.right,
                bottom: r.bottom,
            };
        }
    }
    let _ = GetCursorPos;
    ScreenRect::default()
}

#[cfg(not(windows))]
pub fn work_area(_window: &gpui::Window) -> ScreenRect {
    ScreenRect::default()
}

#[cfg(windows)]
pub fn move_window(window: &gpui::Window, left: i32, top: i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };
    unsafe {
        SetWindowPos(
            hwnd(window) as _,
            std::ptr::null_mut(),
            left,
            top,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(windows))]
pub fn move_window(_window: &gpui::Window, _left: i32, _top: i32) {}

#[cfg(windows)]
pub fn cursor_pos() -> (i32, i32) {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point);
    }
    (point.x, point.y)
}

#[cfg(not(windows))]
pub fn cursor_pos() -> (i32, i32) {
    (0, 0)
}

#[cfg(windows)]
pub struct Singleton {
    mutex: windows_sys::Win32::Foundation::HANDLE,
    event: windows_sys::Win32::Foundation::HANDLE,
}
#[cfg(windows)]
impl Singleton {
    pub fn acquire() -> Result<Option<Self>> {
        use windows_sys::Win32::{Foundation::*, System::Threading::*};
        let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        // A suffix lets a development build run beside the installed copy.
        let suffix = std::env::var("COMMAND_MANAGER_INSTANCE").unwrap_or_default();
        let mutex = unsafe {
            CreateMutexW(
                std::ptr::null(),
                0,
                wide(&format!("Local\\StandaloneCommandManagerMutex-v1{suffix}")).as_ptr(),
            )
        };
        ensure!(!mutex.is_null(), "无法创建单实例锁");
        let existing = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let event = unsafe {
            CreateEventW(
                std::ptr::null(),
                0,
                0,
                wide(&format!("Local\\StandaloneCommandManagerShow-v1{suffix}")).as_ptr(),
            )
        };
        if event.is_null() {
            unsafe {
                CloseHandle(mutex);
            }
            anyhow::bail!("无法创建实例唤醒事件");
        }
        let instance = Self { mutex, event };
        if existing {
            unsafe {
                SetEvent(event);
            }
            return Ok(None);
        }
        Ok(Some(instance))
    }
    pub fn take_show(&self) -> bool {
        unsafe { windows_sys::Win32::System::Threading::WaitForSingleObject(self.event, 0) == 0 }
    }
}
#[cfg(windows)]
impl Drop for Singleton {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.event);
            windows_sys::Win32::Foundation::CloseHandle(self.mutex);
        }
    }
}
