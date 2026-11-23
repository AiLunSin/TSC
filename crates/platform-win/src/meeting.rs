//! 会议应用检测：进程名命中后，再按可见窗口标题过滤。
//!
//! 常驻类客户端（微信/钉钉）要求窗口标题含会议关键词，避免后台驻留被误判；
//! 腾讯会议/飞书的窗口即通话界面，只要存在可见窗口即可判定。
//! 只读取进程名与窗口标题，不读取会议内容。

const TARGETS: &[(&str, &str, bool)] = &[
    ("wemeetapp.exe", "腾讯会议", false),
    ("tencentmeeting.exe", "腾讯会议", false),
    ("wechat.exe", "微信", true),
    ("weixin.exe", "微信", true),
    ("feishu.exe", "飞书", false),
    ("lark.exe", "飞书", false),
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
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, INVALID_HANDLE_VALUE, LPARAM};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    use super::{display_name_for, requires_title_match, title_has_meeting_keyword};

    struct TargetProcess {
        pid: u32,
        display_name: &'static str,
        requires_title: bool,
    }

    struct ScanContext {
        targets: Vec<TargetProcess>,
        found: Option<(&'static str, String)>,
    }

    /// 当前是否有已知会议客户端处于会议窗口状态。
    pub fn detect_running() -> Option<(&'static str, String)> {
        let targets = collect_target_processes();
        if targets.is_empty() {
            return None;
        }
        let mut context = ScanContext {
            targets,
            found: None,
        };
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

    unsafe extern "system" fn scan_window(window: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut ScanContext);
        if IsWindowVisible(window) == 0 {
            return 1;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(window, &mut pid);
        let Some(target) = context.targets.iter().find(|item| item.pid == pid) else {
            return 1;
        };
        let length = GetWindowTextLengthW(window);
        if length <= 0 {
            return 1;
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let written = GetWindowTextW(window, buffer.as_mut_ptr(), buffer.len() as i32);
        if written <= 0 {
            return 1;
        }
        let title = String::from_utf16_lossy(&buffer[..written as usize]);
        if target.requires_title && !title_has_meeting_keyword(&title) {
            return 1;
        }
        context.found = Some((target.display_name, title));
        0
    }

    fn collect_target_processes() -> Vec<TargetProcess> {
        // SAFETY: 快照句柄在本函数内关闭；结构体按 API 要求置零并设置 dwSize。
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Vec::new();
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut targets = Vec::new();
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let name = wide_to_string(&entry.szExeFile);
                    if let Some(display_name) = display_name_for(&name) {
                        targets.push(TargetProcess {
                            pid: entry.th32ProcessID,
                            display_name,
                            requires_title: requires_title_match(&name),
                        });
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
            targets
        }
    }

    fn wide_to_string(buffer: &[u16]) -> String {
        let len = buffer
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..len])
    }
}

#[cfg(not(windows))]
mod imp {
    /// 非 Windows 目标恒为未检测。
    pub fn detect_running() -> Option<(&'static str, String)> {
        None
    }
}

pub use imp::detect_running;

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
        assert!(!requires_title_match("Feishu.exe"));
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
