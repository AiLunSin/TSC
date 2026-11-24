//! Windows OCR 回退通道：截取会议窗口底部区域并交给 `Windows.Media.Ocr`。
//!
//! 仅在 UI Automation 读不到文本时使用；只处理内存位图，不落盘原图。

const MAX_LINES: usize = 200;

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF)
}

fn compact_cjk(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (index, ch) in chars.iter().enumerate() {
        if *ch == ' ' {
            let previous = index.checked_sub(1).and_then(|slot| chars.get(slot));
            let next = chars.get(index + 1);
            if previous.is_some_and(|item| is_cjk(*item)) && next.is_some_and(|item| is_cjk(*item))
            {
                continue;
            }
        }
        out.push(*ch);
    }
    out
}

fn normalize_lines(lines: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        let text = compact_cjk(line.trim());
        if text.is_empty() {
            continue;
        }
        out.push(text);
        if out.len() >= MAX_LINES {
            break;
        }
    }
    out
}

/// 识别指定窗口底部区域内的文字行。失败返回可读原因，不 panic。
pub fn recognize_window(handle: isize) -> Result<Vec<String>, String> {
    let lines = imp::recognize_window(handle)?;
    Ok(normalize_lines(lines))
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Security::Cryptography::CryptographicBuffer;
    use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowInfo, WINDOWINFO,
    };

    const RPC_E_CHANGED_MODE: i32 = 0x8001_0106u32 as i32;
    const MAX_OCR_DIMENSION: i32 = 4096;

    struct RoGuard {
        owned: bool,
    }

    impl RoGuard {
        fn init() -> Result<Self, String> {
            // SAFETY: 在当前线程初始化 WinRT；只有真正初始化成功时才在 Drop 配对释放。
            let result = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
            match result {
                Ok(()) => Ok(Self { owned: true }),
                Err(error) if error.code().0 == RPC_E_CHANGED_MODE => Ok(Self { owned: false }),
                Err(error) => Err(format!("WinRT 初始化失败：{error}")),
            }
        }
    }

    impl Drop for RoGuard {
        fn drop(&mut self) {
            if self.owned {
                // SAFETY: 与 RoGuard::init 的 RoInitialize 成对。
                unsafe { RoUninitialize() };
            }
        }
    }

    struct Captured {
        width: i32,
        height: i32,
        pixels: Vec<u8>,
    }

    pub(super) fn bottom_band(height: i32) -> (i32, i32) {
        let top = height * 3 / 5;
        (top, (height - top).max(1))
    }

    fn capture_bottom_band(handle: isize) -> Result<Captured, String> {
        // 屏幕 DC 会抓到最上层内容；目标窗口不在前台时直接跳过，避免读到其它窗口的隐私内容。
        if unsafe { GetForegroundWindow() } != handle {
            return Err("会议窗口不在前台，跳过 OCR".to_owned());
        }
        let mut win: WINDOWINFO = unsafe { std::mem::zeroed() };
        win.cbSize = std::mem::size_of::<WINDOWINFO>() as u32;
        // SAFETY: handle 来自 EnumWindows；WINDOWINFO 为可写输出，cbSize 已置。
        if unsafe { GetWindowInfo(handle, &mut win) } == 0 {
            return Err("读取窗口信息失败".to_owned());
        }
        let width = win.rcClient.right - win.rcClient.left;
        let height = win.rcClient.bottom - win.rcClient.top;
        if width <= 0 || height <= 0 {
            return Err("窗口尺寸无效".to_owned());
        }
        if width > MAX_OCR_DIMENSION || height > MAX_OCR_DIMENSION {
            return Err("窗口尺寸超出 OCR 上限".to_owned());
        }
        let (top, band_height) = bottom_band(height);
        let origin_x = win.rcClient.left;
        let origin_y = win.rcClient.top + top;
        // SAFETY: 屏幕 DC 才能取到 DWM 合成内容（窗口 DC 对 GPU 渲染窗口返回全黑）；
        // DC 与位图在本函数内成对释放，每个失败路径都逐项回收。
        unsafe {
            let screen = GetDC(0);
            if screen == 0 {
                return Err("获取屏幕设备上下文失败".to_owned());
            }
            let memory = CreateCompatibleDC(screen);
            if memory == 0 {
                ReleaseDC(0, screen);
                return Err("创建内存设备上下文失败".to_owned());
            }
            let bitmap = CreateCompatibleBitmap(screen, width, band_height);
            if bitmap == 0 {
                DeleteDC(memory);
                ReleaseDC(0, screen);
                return Err("创建位图失败".to_owned());
            }
            let previous = SelectObject(memory, bitmap);
            let copied = BitBlt(
                memory,
                0,
                0,
                width,
                band_height,
                screen,
                origin_x,
                origin_y,
                SRCCOPY,
            );
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = width;
            info.bmiHeader.biHeight = -band_height;
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let mut pixels = vec![0u8; (width as usize) * (band_height as usize) * 4];
            let rows = if copied != 0 {
                GetDIBits(
                    memory,
                    bitmap,
                    0,
                    band_height as u32,
                    pixels.as_mut_ptr() as *mut c_void,
                    &mut info,
                    DIB_RGB_COLORS,
                )
            } else {
                0
            };
            SelectObject(memory, previous);
            DeleteObject(bitmap);
            DeleteDC(memory);
            ReleaseDC(0, screen);
            if copied == 0 {
                return Err("截图失败".to_owned());
            }
            if rows == 0 {
                return Err("读取像素失败".to_owned());
            }
            Ok(Captured {
                width,
                height: band_height,
                pixels,
            })
        }
    }

    pub(super) fn recognize_window(handle: isize) -> Result<Vec<String>, String> {
        let _guard = RoGuard::init()?;
        let captured = capture_bottom_band(handle)?;
        let buffer = CryptographicBuffer::CreateFromByteArray(&captured.pixels)
            .map_err(|error| format!("创建像素缓冲失败：{error}"))?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            captured.width,
            captured.height,
        )
        .map_err(|error| format!("创建软件位图失败：{error}"))?;
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .map_err(|error| format!("未找到可用的 OCR 语言包：{error}"))?;
        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(|error| format!("启动 OCR 失败：{error}"))?
            .get()
            .map_err(|error| format!("等待 OCR 结果失败：{error}"))?;
        let lines = result
            .Lines()
            .map_err(|error| format!("读取 OCR 行失败：{error}"))?;
        let count = lines
            .Size()
            .map_err(|error| format!("读取 OCR 行数失败：{error}"))?;
        let mut out = Vec::new();
        for index in 0..count {
            let Ok(line) = lines.GetAt(index) else {
                continue;
            };
            let Ok(text) = line.Text() else {
                continue;
            };
            out.push(text.to_string());
        }
        Ok(out)
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn recognize_window(_handle: isize) -> Result<Vec<String>, String> {
        Err("当前平台不支持 Windows OCR".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lines_trims_and_drops_empty() {
        let lines = normalize_lines(vec![" 甲 ".to_owned(), "  ".to_owned(), "乙".to_owned()]);
        assert_eq!(lines, vec!["甲".to_owned(), "乙".to_owned()]);
    }

    #[test]
    fn normalize_lines_caps_count() {
        let lines = normalize_lines((0..MAX_LINES + 10).map(|index| format!("行{index}")));
        assert_eq!(lines.len(), MAX_LINES);
    }

    #[test]
    fn compact_cjk_removes_spaces_between_han_characters() {
        assert_eq!(compact_cjk("加 入 会 议"), "加入会议");
        assert_eq!(compact_cjk("SLA 99.9% 与 灰度"), "SLA 99.9% 与灰度");
    }

    #[test]
    fn invalid_handle_returns_error_without_panicking() {
        assert!(recognize_window(-1).is_err());
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::imp;
    use super::recognize_window;

    #[test]
    fn empty_window_band_is_rejected() {
        assert!(recognize_window(0).is_err());
    }

    #[test]
    fn bottom_band_covers_lower_forty_percent() {
        assert_eq!(imp::bottom_band(1000), (600, 400));
        assert_eq!(imp::bottom_band(10), (6, 4));
    }
}
