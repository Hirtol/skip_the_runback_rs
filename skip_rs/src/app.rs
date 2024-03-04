use std::path::{Path, PathBuf};

use rust_hooking_utils::raw_input::key_manager::KeyboardManager;

use crate::config::SkipConfig;
use crate::waypoints::WaypointSave;

pub static WAYPOINTS_FILE_NAME: &str = "skip_waypoints.json";

pub struct SkipApp {
    current_plugin: Box<dyn crate::plugins::SkipPlugin>,
    save_directory: PathBuf,
    waypoints: WaypointSave,
}

impl SkipApp {
    pub fn new(save_path: impl Into<PathBuf>, mut plugin: Box<dyn crate::plugins::SkipPlugin>) -> eyre::Result<Self> {
        let save_path = save_path.into();
        let waypoints = get_waypoints(&save_path)?;

        plugin.start()?;

        Ok(Self {
            current_plugin: plugin,
            save_directory: save_path,
            waypoints,
        })
    }

    pub fn run(&mut self, config: &SkipConfig, keyboard: &mut KeyboardManager) -> eyre::Result<()> {
        if keyboard.all_pressed(config.keybinds.save_waypoint.iter().map(|k| k.to_virtual_key())) {
            self.save_waypoint()?;
        }
        if keyboard.all_pressed(config.keybinds.teleport_to_waypoint.iter().map(|k| k.to_virtual_key())) {
            // TODO: Add warning (maybe when the player pointer changes? as that seems to indicate a change in area in most games
            // as it gets re-allocated) for when a player tries to teleport to a waypoint made in a different area.
            self.teleport_to_waypoint()?;
        }

        Ok(())
    }

    fn save_waypoint(&mut self) -> eyre::Result<()> {
        let coords = self.current_plugin.get_current_coordinates()?;

        if let Some(coords) = coords {
            self.waypoints.most_recent = Some(coords);
            let save_file = self.save_directory.join(WAYPOINTS_FILE_NAME);
            let out = serde_json::to_string(&self.waypoints)?;
            std::fs::write(save_file, out)?;
            log::info!("Saved new waypoint at: {coords:#?}");
        } else {
            log::info!("No player pointer was found, couldn't save coordinates!")
        }

        Ok(())
    }

    fn teleport_to_waypoint(&mut self) -> eyre::Result<()> {
        if let Some(coords) = self.waypoints.most_recent.as_ref() {
            if let Err(e) = self.current_plugin.set_current_coordinates(*coords) {
                log::info!("Failed to teleport, maybe the player pointer wasn't initialized yet? {e:?}");
            } else {
                log::info!("Teleported player to: {coords:#?}");
            }
        } else {
            log::info!("No waypoint exists as of yet, not teleporting")
        }

        Ok(())
    }
}

fn get_waypoints(save_path: &Path) -> eyre::Result<WaypointSave> {
    let save_file = save_path.join(WAYPOINTS_FILE_NAME);

    if let Ok(file) = std::fs::read(save_file) {
        Ok(serde_json::from_slice(&file)?)
    } else {
        Ok(Default::default())
    }
}
