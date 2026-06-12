use std::collections::{HashMap, HashSet};
use std::path::Path;

use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, MAX_PATH};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
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
pub(crate) const DOFUS_PROCESS_NAMES: &[&str] = &["Dofus.exe", "DofusInvoker.exe"];

/// Cache of pid → process basename (`None` = readable but not resolvable, so
/// we don't retry every pass). Owned by the window watcher and reused across
/// enumeration passes — without it every pass pays an `OpenProcess` +
/// image-name query for every visible top-level window on the system.
/// Entries whose pid wasn't seen in the latest pass are evicted, which keeps
/// the map bounded and defuses pid-reuse aliasing.
#[derive(Debug, Default)]
pub struct PidNameCache(HashMap<u32, Option<String>>);

impl PidNameCache {
    fn basename(&mut self, pid: u32) -> Option<&str> {
        self.0
            .entry(pid)
            .or_insert_with(|| process_basename(pid))
            .as_deref()
    }
}

struct EnumCtx<'a> {
    found: Vec<LiveWindow>,
    seen_pids: HashSet<u32>,
    cache: &'a mut PidNameCache,
}

/// Enumerate every visible top-level window owned by a Dofus process.
pub fn enumerate_dofus_windows(cache: &mut PidNameCache) -> Vec<LiveWindow> {
    let mut ctx = EnumCtx {
        found: Vec::new(),
        seen_pids: HashSet::new(),
        cache,
    };
    let user_param = LPARAM(&mut ctx as *mut EnumCtx as isize);
    unsafe {
        let _ = EnumWindows(Some(enum_proc), user_param);
    }
    let seen = ctx.seen_pids;
    ctx.cache.0.retain(|pid, _| seen.contains(pid));
    ctx.found
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumCtx);

    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return true.into();
    }
    ctx.seen_pids.insert(pid);

    let is_dofus = match ctx.cache.basename(pid) {
        Some(exe) => DOFUS_PROCESS_NAMES
            .iter()
            .any(|n| n.eq_ignore_ascii_case(exe)),
        None => false,
    };
    if !is_dofus {
        return true.into();
    }

    let title = read_window_title(hwnd);
    if title.is_empty() {
        return true.into();
    }

    let class_name = read_class_name(hwnd);
    let (character_name, dofus_class) = parse_title(&title);

    ctx.found.push(LiveWindow {
        hwnd: hwnd.0 as isize,
        pid,
        title,
        class_name,
        dofus_class,
        character_name,
    });

    true.into()
}

/// Parse `(character_name, class_slug)` from a Dofus window title.
///
/// Title format observed in Dofus 3 is `<name> - <class> - <version> - Release`.
/// Both pieces are `None` unless the title has at least 4 " - " segments, so
/// the launcher and other auxiliary Dofus-process windows parse to nothing.
/// The class slug is lowercased and ASCII-folded ("Crâ" → "cra") to match the
/// avatar asset names under `public/avatars/`.
fn parse_title(title: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<&str> = title.split(" - ").collect();
    if parts.len() < 4 {
        return (None, None);
    }
    let name = Some(parts[0].trim()).filter(|s| !s.is_empty());
    let class = Some(parts[1].trim()).filter(|s| !s.is_empty());
    (name.map(str::to_string), class.map(fold_ascii_lower))
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

/// Resolve a process's executable basename via
/// `QueryFullProcessImageNameW`, which only needs
/// `PROCESS_QUERY_LIMITED_INFORMATION` — unlike `GetModuleBaseNameW`
/// (`PROCESS_VM_READ`), it also works when Dofus runs elevated and
/// doclick doesn't.
pub(crate) fn process_basename(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; MAX_PATH as usize];
        let mut len = buf.len() as u32;
        let res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        res.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        Some(Path::new(&path).file_name()?.to_string_lossy().into_owned())
    }
}
