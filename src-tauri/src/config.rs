use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::state::{
    default_broadcast_keys, CharacterProfile, Orientation, OverlaySizes, ShortcutBindings,
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
    pub main_character_id: Option<String>,
    #[serde(default)]
    pub profile_order: Vec<String>,
    #[serde(default)]
    pub orientation: Orientation,
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
            main_character_id: None,
            profile_order: Vec::new(),
            orientation: Orientation::default(),
            shortcuts,
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
            tracing::warn!(?err, "config: failed to parse profiles.json, using defaults");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CharacterProfile, MatchStrategy, Role};
    use tempfile::TempDir;

    fn sample_profile() -> CharacterProfile {
        CharacterProfile {
            id: "abc".into(),
            display_name: "Tolkee".into(),
            role: Role::Follower,
            match_strategy: MatchStrategy::WindowTitleContains("Tolkee".into()),
            dofus_class: Some("iop".into()),
        }
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let cfg = load(dir.path());
        assert!(cfg.profiles.is_empty());
        assert_eq!(cfg.panic_hotkey, "Ctrl+Shift+F12");
        assert_eq!(cfg.shortcuts.focus_char.len(), 8);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = TempDir::new().unwrap();
        let mut cfg = PersistedConfig::default();
        cfg.profiles.push(sample_profile());
        cfg.overlay_position = Some((100, 200));
        cfg.overlay_sizes.horizontal = Some((1200, 104));
        cfg.main_character_id = Some("abc".into());
        cfg.profile_order.push("abc".into());

        save(dir.path(), &cfg).unwrap();
        let loaded = load(dir.path());

        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.profiles[0].id, "abc");
        assert_eq!(loaded.overlay_position, Some((100, 200)));
        assert_eq!(loaded.overlay_sizes.horizontal, Some((1200, 104)));
        assert_eq!(loaded.main_character_id.as_deref(), Some("abc"));
        assert_eq!(loaded.profile_order, vec!["abc".to_string()]);
    }

    #[test]
    fn legacy_role_main_deserializes_as_follower() {
        let dir = TempDir::new().unwrap();
        let json = r#"{"profiles":[{"id":"x","display_name":"X","role":"main","match_strategy":{"kind":"WindowTitleContains","value":"X"}}]}"#;
        std::fs::write(dir.path().join("profiles.json"), json).unwrap();
        let cfg = load(dir.path());
        assert_eq!(cfg.profiles.len(), 1);
        assert_eq!(cfg.profiles[0].role, Role::Follower);
    }

    #[test]
    fn malformed_json_falls_back_to_defaults() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("profiles.json"), "{ not valid json").unwrap();
        let cfg = load(dir.path());
        assert!(cfg.profiles.is_empty());
        assert!(!cfg.broadcast_keys.is_empty());
    }
}
