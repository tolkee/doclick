use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::state::{
    default_broadcast_keys, CharacterProfile, InnerState, Orientation, OverlayScale, OverlaySizes,
    ShortcutBindings,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedConfig {
    #[serde(default)]
    pub profiles: Vec<CharacterProfile>,
    #[serde(default)]
    pub broadcast_keys: Vec<u32>,
    #[serde(default = "default_panic_hotkey")]
    pub panic_hotkey: String,
    #[serde(default)]
    pub pvp_warning_acknowledged: bool,
    #[serde(default)]
    pub overlay_position: Option<(i32, i32)>,
    #[serde(default)]
    pub overlay_sizes: OverlaySizes,
    #[serde(default)]
    pub settings_size: Option<(u32, u32)>,
    #[serde(default)]
    pub settings_position: Option<(i32, i32)>,
    #[serde(default)]
    pub main_character_id: Option<String>,
    #[serde(default)]
    pub profile_order: Vec<String>,
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub overlay_scale: OverlayScale,
    #[serde(default)]
    pub shortcuts: ShortcutBindings,
}

fn default_panic_hotkey() -> String {
    "Ctrl+Shift+F12".into()
}

impl Default for PersistedConfig {
    fn default() -> Self {
        let mut shortcuts = ShortcutBindings::default();
        shortcuts.ensure_focus_char_slots();
        Self {
            profiles: Vec::new(),
            broadcast_keys: default_broadcast_keys(),
            panic_hotkey: default_panic_hotkey(),
            pvp_warning_acknowledged: false,
            overlay_position: None,
            overlay_sizes: OverlaySizes::default(),
            settings_size: None,
            settings_position: None,
            main_character_id: None,
            profile_order: Vec::new(),
            orientation: Orientation::default(),
            overlay_scale: OverlayScale::default(),
            shortcuts,
        }
    }
}

impl PersistedConfig {
    pub fn from_inner(inner: &InnerState) -> Self {
        Self {
            profiles: inner.profiles.clone(),
            broadcast_keys: inner.broadcast_keys.clone(),
            panic_hotkey: inner.panic_hotkey.clone(),
            pvp_warning_acknowledged: inner.pvp_warning_acknowledged,
            overlay_position: inner.overlay_position,
            overlay_sizes: inner.overlay_sizes,
            settings_size: inner.settings_size,
            settings_position: inner.settings_position,
            main_character_id: inner.main_character_id.clone(),
            profile_order: inner.profile_order.clone(),
            orientation: inner.orientation,
            overlay_scale: inner.overlay_scale,
            shortcuts: inner.shortcuts.clone(),
        }
    }
}

const FILE_NAME: &str = "profiles.json";

pub fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

pub fn load(app_data_dir: &Path) -> PersistedConfig {
    let path = config_path(app_data_dir);
    let Ok(bytes) = std::fs::read(&path) else {
        return PersistedConfig::default();
    };
    match serde_json::from_slice::<PersistedConfig>(&bytes) {
        Ok(mut cfg) => {
            if cfg.broadcast_keys.is_empty() {
                cfg.broadcast_keys = default_broadcast_keys();
            }
            cfg.shortcuts.ensure_focus_char_slots();
            cfg
        }
        Err(err) => {
            tracing::warn!(
                ?err,
                "config: failed to parse profiles.json, using defaults"
            );
            PersistedConfig::default()
        }
    }
}

pub fn save(app_data_dir: &Path, cfg: &PersistedConfig) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = config_path(app_data_dir);
    let bytes = serde_json::to_vec_pretty(cfg)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
