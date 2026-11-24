//! 会议应用检测：遍历可见顶层窗口，按窗口进程映像名命中后再按标题过滤。
//!
//! 常驻类客户端（微信/钉钉/飞书）要求窗口标题含会议关键词，避免把主界面或后台驻留当成会议；
//! 腾讯会议的窗口标题固定含“会议”，只要求存在可见窗口。
//! 只读取进程映像名与窗口标题，不读取会议内容。

const TARGETS: &[(&str, &str, bool)] = &[
    ("wemeetapp.exe", "腾讯会议", false),
    ("tencentmeeting.exe", "腾讯会议", false),
    ("wechat.exe", "微信", true),
    ("weixin.exe", "微信", true),
    ("feishu.exe", "飞书", true),
    ("lark.exe", "飞书", true),
    ("dingtalk.exe", "钉钉", true),
];

const TITLE_KEYWORDS: &[&str] = &[
    "会议", "通话", "视频", "语音", "直播", "共享", "Meet", "Call", "Meeting", "Video", "Voice",
];

/// 由可执行文件名映射到会议应用显示名。
pub fn display_name_for(exe_name: &str) -> Option<&'static str> {
    TARGETS
        .iter()
        .find(|(exe, _, _)| exe.eq_ignore_ascii_case(exe_name))
        .map(|(_, name, _)| *name)
}

/// 该进程是否为常驻客户端（需要窗口标题过滤）。
pub fn requires_title_match(exe_name: &str) -> bool {
    TARGETS
        .iter()
        .find(|(exe, _, _)| exe.eq_ignore_ascii_case(exe_name))
        .is_some_and(|(_, _, needs)| *needs)
}

/// 窗口标题是否含会议关键词（大小写不敏感）。
pub fn title_has_meeting_keyword(title: &str) -> bool {
    TITLE_KEYWORDS
        .iter()
        .any(|keyword| title.to_lowercase().contains(&keyword.to_lowercase()))
}

#[cfg(windows)]
mod imp {
    use std::collections::HashMap;

    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    use super::{display_name_for, requires_title_match, title_has_meeting_keyword};

    struct ScanContext {
        pid_cache: HashMap<u32, Option<(&'static str, bool)>>,
        found: Option<(&'static str, String)>,
        best_priority: usize,
        windows: Vec<(&'static str, isize)>,
    }

    impl ScanContext {
        fn new() -> Self {
            Self {
                pid_cache: HashMap::new(),
                found: None,
                best_priority: usize::MAX,
                windows: Vec::new(),
            }
        }
    }

    fn priority_for(display_name: &str) -> usize {
        match display_name {
            "腾讯会议" => 0,
            "飞书" => 1,
            "微信" => 2,
            _ => 3,
        }
    }

    /// 当前是否有已知会议客户端处于会议窗口状态。
    pub fn detect_running() -> Option<(&'static str, String)> {
        let mut context = ScanContext::new();
        // SAFETY: 回调在 EnumWindows 同步执行期间访问栈上的 context；
        // 回调只读取窗口标题与进程号，不做任何写操作。
        unsafe {
            EnumWindows(
                Some(scan_window),
                &mut context as *mut ScanContext as LPARAM,
            );
        }
        context.found
    }

    /// 收集目标会议进程的全部可见顶层窗口句柄，供 UI Automation 遍历。
    pub fn find_meeting_windows() -> Option<Vec<(&'static str, isize)>> {
        let mut context = ScanContext::new();
        // SAFETY: 回调在 EnumWindows 同步执行期间访问栈上的 context；
        // 只把匹配窗口的句柄追加到 Vec，不做其它写操作。
        unsafe {
            EnumWindows(
                Some(collect_window),
                &mut context as *mut ScanContext as LPARAM,
            );
        }
        if context.windows.is_empty() {
            return None;
        }
        context.windows.sort_by_key(|(name, _)| priority_for(name));
        Some(context.windows)
    }

    unsafe fn matched_window(
        context: &mut ScanContext,
        window: HWND,
    ) -> Option<(&'static str, String)> {
        if IsWindowVisible(window) == 0 {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(window, &mut pid);
        let (display_name, requires_title) = target_for_pid(context, pid)?;
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(window, &mut rect) == 0 {
            return None;
        }
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return None;
        }
        let title = window_title(window);
        if requires_title && !title_has_meeting_keyword(&title) {
            return None;
        }
        Some((display_name, title))
    }

    fn target_for_pid(context: &mut ScanContext, pid: u32) -> Option<(&'static str, bool)> {
        if let Some(cached) = context.pid_cache.get(&pid) {
            return *cached;
        }
        let found = exe_name_for_pid(pid)
            .and_then(|exe| display_name_for(&exe).map(|name| (name, requires_title_match(&exe))));
        context.pid_cache.insert(pid, found);
        found
    }

    fn exe_name_for_pid(pid: u32) -> Option<String> {
        // SAFETY: 句柄在本函数内关闭；缓冲区长度与 API 约定一致。
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return None;
            }
            let mut buffer = [0u16; 520];
            let mut size = buffer.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size);
            CloseHandle(handle);
            if ok == 0 {
                return None;
            }
            let path = String::from_utf16_lossy(&buffer[..size as usize]);
            path.rsplit(['\\', '/']).next().map(str::to_owned)
        }
    }

    unsafe fn window_title(window: HWND) -> String {
        let length = GetWindowTextLengthW(window);
        if length <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let written = GetWindowTextW(window, buffer.as_mut_ptr(), buffer.len() as i32);
        if written <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..written as usize])
    }

    unsafe extern "system" fn scan_window(window: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut ScanContext);
        let Some((display_name, title)) = matched_window(context, window) else {
            return 1;
        };
        let priority = priority_for(display_name);
        if priority < context.best_priority {
            context.best_priority = priority;
            context.found = Some((display_name, title));
        }
        1
    }

    unsafe extern "system" fn collect_window(window: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut ScanContext);
        if let Some((display_name, _title)) = matched_window(context, window) {
            context.windows.push((display_name, window));
        }
        1
    }
}

#[cfg(not(windows))]
mod imp {
    /// 非 Windows 目标恒为未检测。
    pub fn detect_running() -> Option<(&'static str, String)> {
        None
    }

    /// 非 Windows 目标没有可遍历的窗口句柄。
    pub fn find_meeting_windows() -> Option<Vec<(&'static str, isize)>> {
        None
    }
}

pub use imp::{detect_running, find_meeting_windows};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_executables_map_to_display_names() {
        assert_eq!(display_name_for("wemeetapp.exe"), Some("腾讯会议"));
        assert_eq!(display_name_for("WeChat.exe"), Some("微信"));
        assert_eq!(display_name_for("Weixin.exe"), Some("微信"));
        assert_eq!(display_name_for("Feishu.exe"), Some("飞书"));
        assert_eq!(display_name_for("dingtalk.exe"), Some("钉钉"));
        assert_eq!(display_name_for("explorer.exe"), None);
    }

    #[test]
    fn resident_clients_require_title_match() {
        assert!(requires_title_match("WeChat.exe"));
        assert!(requires_title_match("DingTalk.exe"));
        assert!(!requires_title_match("wemeetapp.exe"));
        assert!(requires_title_match("Feishu.exe"));
        assert!(requires_title_match("Lark.exe"));
    }

    #[test]
    fn meeting_titles_are_recognized() {
        assert!(title_has_meeting_keyword("张三的视频通话"));
        assert!(title_has_meeting_keyword("项目评审会议"));
        assert!(title_has_meeting_keyword("Group video meeting"));
        assert!(!title_has_meeting_keyword("文件传输助手"));
        assert!(!title_has_meeting_keyword(""));
    }
}
