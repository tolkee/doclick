use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Persisted configuration for the startup-flow feature. Every field is
/// `#[serde(default)]` so a `profiles.json` written by an older build that
/// doesn't know about startup_flow loads cleanly with everything off.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StartupFlowConfig {
    /// Master switch. When `false`: auto-on-start is ignored and the
    /// `TriggerStartupFlow` shortcut is unregistered.
    pub enabled: bool,
    /// When both this and `enabled` are `true`, the flow runs once
    /// during `lib.rs::setup()`.
    pub run_on_app_start: bool,
    pub accounts: LaunchAccountsAction,
    pub ganymede: LaunchGanymedeAction,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchAccountsAction {
    pub enabled: bool,
    pub exe_path: Option<PathBuf>,
    /// Sticky path-failure marker. When `Some`, the runner short-circuits
    /// this step with `Failed` and never attempts a spawn. Cleared the
    /// moment the user edits `exe_path` via `set_startup_flow_config`.
    /// Format: French-localized message for direct display in the UI.
    pub last_path_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchGanymedeAction {
    pub enabled: bool,
    pub exe_path: Option<PathBuf>,
    pub last_path_error: Option<String>,
}

/// Which action a `record_path_error` / `clear_path_error` call targets.
#[derive(Debug, Clone, Copy)]
pub enum ActionKind {
    Accounts,
    Ganymede,
}
