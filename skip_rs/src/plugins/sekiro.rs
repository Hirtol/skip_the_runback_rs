//! Sekiro specific module for demonstration's sake

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use frida_gum::interceptor::{InvocationContext, ProbeListener};

use crate::plugins::{CPlayerCoordinates, PluginIdentifiers, SkipPlugin};

/// Signature of the instruction which exclusively reads from the player coordinates struct.
pub static READ_FROM_COORDS_SIG: &str = "0F 28 81 80 00 00 00 4D";

type CoordinatePtr = Arc<Mutex<Option<usize>>>;

pub struct SekiroPlugin {
    coords: CoordinatePtr,
    listener: Option<Pin<Box<SekiroCoordinatesIntercept>>>,
}

impl SekiroPlugin {
    pub fn new() -> Self {
        Self {
            coords: Default::default(),
            listener: None,
        }
    }
}

impl SkipPlugin for SekiroPlugin {
    fn identifiers(&self) -> PluginIdentifiers {
        PluginIdentifiers {
            plugin_name: "Sekiro Skip Runback".to_string(),
            expected_module: Some("sekiro.exe".to_string()),
            expected_exe_name: Some("sekiro.exe".to_string()),
        }
    }

    fn start(&mut self) -> eyre::Result<()> {
        let listener = SekiroCoordinatesIntercept(self.coords.clone());
        self.listener = Some(super::attach_listener_to_signature(READ_FROM_COORDS_SIG, listener)?);

        Ok(())
    }

    fn get_current_coordinates(&mut self) -> eyre::Result<Option<crate::plugins::PlayerCoordinates>> {
        let coords = self.coords.lock().unwrap();
        let out = unsafe {
            coords
                .map(|ptr| *(ptr as *mut CPlayerCoordinates))
                .map(|coords| super::PlayerCoordinates {
                    x: coords.x,
                    y: coords.y,
                    z: coords.z,
                })
        };

        Ok(out)
    }

    fn set_current_coordinates(&mut self, target: crate::plugins::PlayerCoordinates) -> eyre::Result<()> {
        if let Some(coords) = *self.coords.lock().unwrap() {
            let coords = coords as *mut CPlayerCoordinates;
            unsafe {
                (*coords).x = target.x;
                (*coords).y = target.y;
                (*coords).z = target.z;
            }

            Ok(())
        } else {
            eyre::bail!("Pointer not initialised")
        }
    }
}

pub struct SekiroCoordinatesIntercept(CoordinatePtr);

impl ProbeListener for SekiroCoordinatesIntercept {
    fn on_hit(&mut self, context: InvocationContext) {
        let base_ptr = context.cpu_context().rcx();
        let position_ptr = base_ptr + 0x80;
        let coords = position_ptr as usize;

        let mut lock = self.0.lock().unwrap();

        if lock.map(|ptr| ptr != coords).unwrap_or(true) {
            let old = lock.map(|ptr| ptr).unwrap_or_default();
            *lock = Some(coords);
            log::trace!("Updated Sekiro player pointer from `{old:#X}` to {:#X}", coords);
        }
    }
}
