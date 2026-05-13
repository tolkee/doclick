use tauri::AppHandle;

use crate::state::AppState;

use super::config::ActionKind;
use super::process::{is_path_error, is_process_running, spawn_detached};
use super::{clear_path_error, mark_running, probe, record_path_error, StepOutcome};

const GANYMEDE_BASENAME: &str = "ganymede.exe";

pub async fn run_step(app: &AppHandle, state: &AppState) -> Result<StepOutcome, String> {
    mark_running(app, state, ActionKind::Ganymede);

    let cfg = state.read().startup_flow.ganymede.clone();

    if let Some(msg) = cfg.last_path_error.clone() {
        return Err(msg);
    }

    if is_process_running(GANYMEDE_BASENAME) {
        return Ok(StepOutcome::Skipped("Ganymede déjà ouvert".into()));
    }

    let exe = match cfg.exe_path.clone().or_else(probe::default_ganymede_path) {
        Some(p) if p.exists() => p,
        Some(p) => {
            let msg = format!("Fichier introuvable : {}", p.display());
            record_path_error(app, state, ActionKind::Ganymede, msg.clone());
            return Err(msg);
        }
        None => {
            let msg =
                "Aucun exécutable détecté. Sélectionnez le fichier dans Parcourir…".to_string();
            record_path_error(app, state, ActionKind::Ganymede, msg.clone());
            return Err(msg);
        }
    };

    match spawn_detached(&exe) {
        Ok(_) => {
            clear_path_error(app, state, ActionKind::Ganymede);
            Ok(StepOutcome::Done("Ganymede lancé".into()))
        }
        Err(e) if is_path_error(&e) => {
            let msg = format!("Lancement impossible : {e}");
            record_path_error(app, state, ActionKind::Ganymede, msg.clone());
            Err(msg)
        }
        Err(e) => Err(format!("Erreur transitoire : {e}")),
    }
}
