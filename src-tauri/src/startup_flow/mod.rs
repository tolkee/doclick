pub mod accounts;
pub mod config;
pub mod ganymede;
pub mod probe;
pub mod process;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::events::EVT_STARTUP_FLOW_STATE;
use crate::state::AppState;

pub use self::config::{ActionKind, StartupFlowConfig};

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
    #[default]
    Idle,
    Running,
    Skipped,
    Done,
    Failed,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct StepState {
    pub status: StepStatus,
    pub message: Option<String>,
    pub last_run_ms: Option<i64>,
}

/// Runtime-only mirror of the flow's progress. Lives on `InnerState`,
/// is not persisted, and emits via `EVT_STARTUP_FLOW_STATE` on every
/// transition.
#[derive(Debug, Clone, Default, Serialize)]
pub struct StartupRuntimeState {
    pub running: bool,
    pub accounts: StepState,
    pub ganymede: StepState,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupFlowStatePayload {
    pub running: bool,
    pub accounts: StepState,
    pub ganymede: StepState,
}

pub fn snapshot_payload(state: &AppState) -> StartupFlowStatePayload {
    let r = &state.read().startup_flow_runtime;
    StartupFlowStatePayload {
        running: r.running,
        accounts: r.accounts.clone(),
        ganymede: r.ganymede.clone(),
    }
}

fn emit_state(app: &AppHandle, state: &AppState) {
    let _ = app.emit(EVT_STARTUP_FLOW_STATE, snapshot_payload(state));
}

pub enum StepOutcome {
    Done(String),
    Skipped(String),
}

/// The orchestrator. Spawn this from `tauri::async_runtime::spawn` —
/// callers should not `await` it inline because the Ctrl+N keystroke
/// loop blocks for several seconds while the launcher window paints.
///
/// Concurrent runs are gated by `startup_flow_runtime.running`: if a run
/// is already in flight the call returns immediately. The `running`
/// flag is cleared in a single point (the end of this function) so a
/// panic inside a step still releases the lock via the on-drop guard
/// below — see `RunGuard`.
pub async fn run(app: AppHandle, state: AppState) {
    {
        let mut inner = state.write();
        if inner.startup_flow_runtime.running {
            return;
        }
        inner.startup_flow_runtime.running = true;
        inner.startup_flow_runtime.accounts = StepState::default();
        inner.startup_flow_runtime.ganymede = StepState::default();
    }
    let _guard = RunGuard {
        app: app.clone(),
        state: state.clone(),
    };
    emit_state(&app, &state);

    // Per the plan's confirmed run order: launcher + accounts first, then
    // Ganymede.
    let accounts_enabled = state.read().startup_flow.accounts.enabled;
    if accounts_enabled {
        run_one(
            &app,
            &state,
            ActionKind::Accounts,
            accounts::run_step(&app, &state).await,
        );
    } else {
        skip_disabled(&app, &state, ActionKind::Accounts);
    }

    let ganymede_enabled = state.read().startup_flow.ganymede.enabled;
    if ganymede_enabled {
        run_one(
            &app,
            &state,
            ActionKind::Ganymede,
            ganymede::run_step(&app, &state).await,
        );
    } else {
        skip_disabled(&app, &state, ActionKind::Ganymede);
    }
}

fn run_one(
    app: &AppHandle,
    state: &AppState,
    kind: ActionKind,
    outcome: Result<StepOutcome, String>,
) {
    let now = now_ms();
    let step = match outcome {
        Ok(StepOutcome::Done(msg)) => StepState {
            status: StepStatus::Done,
            message: Some(msg),
            last_run_ms: Some(now),
        },
        Ok(StepOutcome::Skipped(msg)) => StepState {
            status: StepStatus::Skipped,
            message: Some(msg),
            last_run_ms: Some(now),
        },
        Err(msg) => StepState {
            status: StepStatus::Failed,
            message: Some(msg),
            last_run_ms: Some(now),
        },
    };
    write_step(state, kind, step);
    emit_state(app, state);
}

fn skip_disabled(app: &AppHandle, state: &AppState, kind: ActionKind) {
    write_step(
        state,
        kind,
        StepState {
            status: StepStatus::Idle,
            message: Some("Action désactivée".into()),
            last_run_ms: Some(now_ms()),
        },
    );
    emit_state(app, state);
}

/// Called by step implementations right before they start work, so the UI
/// renders a running badge instead of the previous run's terminal state.
pub fn mark_running(app: &AppHandle, state: &AppState, kind: ActionKind) {
    write_step(
        state,
        kind,
        StepState {
            status: StepStatus::Running,
            message: None,
            last_run_ms: None,
        },
    );
    emit_state(app, state);
}

fn write_step(state: &AppState, kind: ActionKind, step: StepState) {
    let mut inner = state.write();
    match kind {
        ActionKind::Accounts => inner.startup_flow_runtime.accounts = step,
        ActionKind::Ganymede => inner.startup_flow_runtime.ganymede = step,
    }
}

/// Persist a path-failure marker on the given action and notify the UI
/// via `EVT_PREFS_CHANGED` so the input renders the error inline.
pub fn record_path_error(app: &AppHandle, state: &AppState, kind: ActionKind, message: String) {
    {
        let mut inner = state.write();
        match kind {
            ActionKind::Accounts => inner.startup_flow.accounts.last_path_error = Some(message),
            ActionKind::Ganymede => inner.startup_flow.ganymede.last_path_error = Some(message),
        }
    }
    let _ = crate::commands::persist(app, state);
    let _ = app.emit(crate::events::EVT_PREFS_CHANGED, ());
}

/// Clear a previously-recorded path error after a successful spawn.
pub fn clear_path_error(app: &AppHandle, state: &AppState, kind: ActionKind) {
    let cleared = {
        let mut inner = state.write();
        let slot = match kind {
            ActionKind::Accounts => &mut inner.startup_flow.accounts.last_path_error,
            ActionKind::Ganymede => &mut inner.startup_flow.ganymede.last_path_error,
        };
        let had_value = slot.is_some();
        *slot = None;
        had_value
    };
    if cleared {
        let _ = crate::commands::persist(app, state);
        let _ = app.emit(crate::events::EVT_PREFS_CHANGED, ());
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Drop guard so a panic in a step still flips `running` back to false
/// (otherwise the next trigger silently no-ops). The orchestrator itself
/// never panics in practice; the guard is defense in depth.
struct RunGuard {
    app: AppHandle,
    state: AppState,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.state.write().startup_flow_runtime.running = false;
        emit_state(&self.app, &self.state);
    }
}
