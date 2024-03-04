use std::fmt::Debug;
use std::path::Path;

use eyre::Context;
use rust_hooking_utils::raw_input::virtual_keys::VirtualKey;

pub const CONFIG_FILE_NAME: &str = "skip_rs_config.json";

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SkipConfig {
    /// Whether to open a console for logging
    pub console: bool,
    /// If set, will allow the config to be reloaded during gameplay by providing the given key codes.
    pub reload_config_keys: Option<Vec<VirtualKey>>,
    pub keybinds: KeybindsConfig,
}

impl Default for SkipConfig {
    fn default() -> Self {
        Self {
            console: false,
            reload_config_keys: Some(vec![VirtualKey::VK_CONTROL, VirtualKey::VK_SHIFT, VirtualKey::VK_R]),
            keybinds: Default::default(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct KeybindsConfig {
    pub save_waypoint: Vec<VirtualKey>,
    pub teleport_to_waypoint: Vec<VirtualKey>,
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        Self {
            save_waypoint: vec![VirtualKey::VK_F9],
            teleport_to_waypoint: vec![VirtualKey::VK_F10],
        }
    }
}

pub fn load_config(directory: impl AsRef<Path>) -> eyre::Result<SkipConfig> {
    let path = directory.as_ref().join(CONFIG_FILE_NAME);
    let file = std::fs::read(&path)?;

    if let Ok(conf) = serde_json::from_slice(&file) {
        validate_config(&conf)?;
        Ok(conf)
    } else {
        std::fs::remove_file(&path)?;
        create_initial_config(directory.as_ref())?;
        let file = std::fs::read(&path)?;
        serde_json::from_slice(&file).context("Couldn't load config.")
    }
}

pub fn create_initial_config(directory: impl AsRef<Path>) -> eyre::Result<()> {
    let default_conf = SkipConfig::default();
    let path = directory.as_ref().join(CONFIG_FILE_NAME);

    if !path.exists() {
        let mut file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(&mut file, &default_conf)?;
    }

    Ok(())
}

pub fn validate_config(_conf: &SkipConfig) -> eyre::Result<()> {
    Ok(())
}
