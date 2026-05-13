use std::time::Duration;

use tauri::AppHandle;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_N};

use crate::state::AppState;

use super::config::ActionKind;
use super::process::{is_path_error, is_process_running, poll_for_window, spawn_detached};
use super::{mark_running, probe, record_path_error, StepOutcome};

const LAUNCHER_BASENAME_DEFAULT: &str = "Ankama Launcher.exe";
/// Time we wait for the launcher window to paint after spawning. Ankama
/// Launcher cold-starts in 5–15s on a typical SSD; 30s leaves headroom
/// for first-run / antivirus scans without blocking the user forever.
const LAUNCHER_WINDOW_WAIT: Duration = Duration::from_secs(30);
/// Settle window after the launcher has focus, before the first Ctrl+N.
const FOCUS_SETTLE: Duration = Duration::from_millis(150);
/// Dwell between Ctrl+N keystrokes — the launcher debounces rapid presses.
const CTRL_N_DWELL: Duration = Duration::from_millis(180);
/// Focus timeout passed to `focus_window`. The launcher process is large
/// enough that 400ms gives the z-order step time to settle.
const FOCUS_TIMEOUT: Duration = Duration::from_millis(400);

pub async fn run_step(app: &AppHandle, state: &AppState) -> Result<StepOutcome, String> {
    mark_running(app, state, ActionKind::Accounts);

    let cfg = state.read().startup_flow.accounts.clone();

    // Sticky path-error gate. Once set, the user must edit the path to
    // clear it — see startup_flow::record_path_error / clear_path_error.
    if let Some(msg) = cfg.last_path_error.clone() {
        return Err(msg);
    }

    let profiles_target = state.read().profile_order.len();
    if profiles_target == 0 {
        return Ok(StepOutcome::Skipped(
            "Aucun personnage importé. Ajoutez vos personnages dans l'onglet Personnages.".into(),
        ));
    }

    let exe = match cfg.exe_path.clone().or_else(probe::default_launcher_path) {
        Some(p) if p.exists() => p,
        Some(p) => {
            let msg = format!("Fichier introuvable : {}", p.display());
            record_path_error(app, state, ActionKind::Accounts, msg.clone());
            return Err(msg);
        }
        None => {
            let msg =
                "Aucun exécutable détecté. Sélectionnez le fichier dans Parcourir…".to_string();
            record_path_error(app, state, ActionKind::Accounts, msg.clone());
            return Err(msg);
        }
    };

    let basename = exe
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| LAUNCHER_BASENAME_DEFAULT.to_string());

    let launcher_pid = if is_process_running(&basename) {
        None
    } else {
        match spawn_detached(&exe) {
            Ok(pid) => Some(pid),
            Err(e) if is_path_error(&e) => {
                let msg = format!("Lancement impossible : {e}");
                record_path_error(app, state, ActionKind::Accounts, msg.clone());
                return Err(msg);
            }
            Err(e) => return Err(format!("Erreur transitoire : {e}")),
        }
    };

    super::clear_path_error(app, state, ActionKind::Accounts);

    let hwnd = match poll_for_window(launcher_pid, &basename, LAUNCHER_WINDOW_WAIT) {
        Some(h) => h,
        None => {
            return Err(format!(
                "Fenêtre du launcher non détectée après {}s",
                LAUNCHER_WINDOW_WAIT.as_secs()
            ));
        }
    };

    let current = crate::windows::enumerate::enumerate_dofus_windows().len();
    let delta = profiles_target.saturating_sub(current);
    if delta == 0 {
        return Ok(StepOutcome::Skipped(format!(
            "{current} fenêtre(s) Dofus déjà ouverte(s)"
        )));
    }

    let _ = crate::windows::focus::focus_window(hwnd, FOCUS_TIMEOUT);
    tokio::time::sleep(FOCUS_SETTLE).await;

    let mut sent = 0usize;
    for _ in 0..delta {
        let ok = crate::broadcast::dispatcher::send_key_combo(VK_CONTROL.0 as u32, VK_N.0 as u32);
        if ok {
            sent += 1;
        }
        tokio::time::sleep(CTRL_N_DWELL).await;
    }

    if sent == 0 {
        return Err("Aucune frappe Ctrl+N n'a abouti".into());
    }
    Ok(StepOutcome::Done(format!("{sent} compte(s) lancé(s)")))
}
