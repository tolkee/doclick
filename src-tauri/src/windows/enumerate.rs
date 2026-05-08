use std::path::PathBuf;

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, MAX_PATH};
use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible,
};

use crate::state::LiveWindow;

/// Process basenames considered Dofus 3 windows.
///
/// `Dofus.exe` is the most likely target for the Unity client. We list a few
/// fallbacks observed in launcher setups; the live process name MUST be
/// verified against an actual Dofus 3 install with Process Explorer.
const DOFUS_PROCESS_NAMES: &[&str] = &["Dofus.exe", "DofusInvoker.exe"];

/// Enumerate every visible top-level window owned by a Dofus process.
pub fn enumerate_dofus_windows() -> Vec<LiveWindow> {
    let mut found: Vec<LiveWindow> = Vec::new();
    let user_param = LPARAM(&mut found as *mut _ as isize);
    unsafe {
        let _ = EnumWindows(Some(enum_proc), user_param);
    }
    found
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let acc = unsafe { &mut *(lparam.0 as *mut Vec<LiveWindow>) };

    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return true.into();
    }

    let title = read_window_title(hwnd);
    if title.is_empty() {
        return true.into();
    }

    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return true.into();
    }

    let exe = match process_basename(pid) {
        Some(name) => name,
        None => return true.into(),
    };
    if !DOFUS_PROCESS_NAMES.iter().any(|n| n.eq_ignore_ascii_case(&exe)) {
        return true.into();
    }

    let class_name = read_class_name(hwnd);
    let dofus_class = parse_dofus_class(&title);
    let character_name = parse_character_name(&title);

    acc.push(LiveWindow {
        hwnd: hwnd.0 as isize,
        pid,
        title,
        class_name,
        dofus_class,
        character_name,
    });

    true.into()
}

/// Parse the Dofus class slug from a window title.
///
/// Title format observed in Dofus 3 is `<name> - <class> - <version> - Release`.
/// We split on " - " and return the second segment lowercased and ASCII-folded
/// (so "Crâ" → "cra", matching `public/avatars/cra.jpg`).
pub fn parse_dofus_class(title: &str) -> Option<String> {
    let parts: Vec<&str> = title.split(" - ").collect();
    if parts.len() < 4 {
        return None;
    }
    let raw = parts[1].trim();
    if raw.is_empty() {
        return None;
    }
    Some(fold_ascii_lower(raw))
}

/// Extract the character name (first " - "-delimited segment) from a Dofus title.
pub fn parse_character_name(title: &str) -> Option<String> {
    let parts: Vec<&str> = title.split(" - ").collect();
    if parts.len() < 4 {
        return None;
    }
    let name = parts[0].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn fold_ascii_lower(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'À' | 'Á' | 'Â' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c => c,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn read_window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len as usize) + 1];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

fn read_class_name(hwnd: HWND) -> String {
    unsafe {
        let mut buf = [0u16; 256];
        let copied = GetClassNameW(hwnd, &mut buf);
        if copied <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_class_from_release_title() {
        assert_eq!(
            parse_dofus_class("Tolkee - Iop - 2.79.0 - Release"),
            Some("iop".into())
        );
        assert_eq!(
            parse_dofus_class("Cra - Cra - 2.79.0 - Release"),
            Some("cra".into())
        );
    }

    #[test]
    fn folds_accents_for_class() {
        assert_eq!(
            parse_dofus_class("Bob - Crâ - 2 - Release"),
            Some("cra".into())
        );
        assert_eq!(
            parse_dofus_class("Bob - Ëniripsa - 2 - Release"),
            Some("eniripsa".into())
        );
    }

    #[test]
    fn rejects_titles_with_too_few_segments() {
        assert_eq!(parse_dofus_class("Just one"), None);
        assert_eq!(parse_dofus_class("One - Two"), None);
        assert_eq!(parse_dofus_class("One - Two - Three"), None);
    }

    #[test]
    fn parses_character_name() {
        assert_eq!(
            super::parse_character_name("Tolkee - Iop - 2.79.0 - Release"),
            Some("Tolkee".into())
        );
        assert_eq!(
            super::parse_character_name("  Spaced  - Cra - 2 - Release"),
            Some("Spaced".into())
        );
        assert_eq!(super::parse_character_name("No segments"), None);
    }
}

fn process_basename(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        )
        .or_else(|_| OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid))
        .ok()?;
        let mut buf = vec![0u16; MAX_PATH as usize];
        let copied = GetModuleBaseNameW(handle, None, &mut buf);
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        if copied == 0 {
            return None;
        }
        let s = String::from_utf16_lossy(&buf[..copied as usize]);
        Some(PathBuf::from(s).file_name()?.to_string_lossy().to_string())
    }
}
