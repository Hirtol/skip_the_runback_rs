//! Lies of P specific module for demonstration's sake

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use frida_gum::interceptor::{InvocationContext, ProbeListener};

use crate::plugins::{PluginIdentifiers, SkipPlugin};

/// Signature of the instruction which exclusively reads from the player coordinates struct.
pub static READ_FROM_COORDS_SIG: &str = "41 0F 10 89 C0 01 00 00 48 8D 44 24 28";

type CoordinatePtr = Arc<Mutex<Option<usize>>>;

pub struct LOPPlugin {
    position_ptr: CoordinatePtr,
    listener: Option<Pin<Box<LopCoordinatesIntercept>>>,
}

impl LOPPlugin {
    pub fn new() -> Self {
        Self {
            position_ptr: Default::default(),
            listener: None,
        }
    }
}

impl SkipPlugin for LOPPlugin {
    fn identifiers(&self) -> PluginIdentifiers {
        PluginIdentifiers {
            plugin_name: "Lies of P Skip Runback".to_string(),
            expected_module: Some("LOP-Win64-Shipping.exe".to_string()),
            expected_exe_name: Some("LOP.exe".to_string()),
        }
    }

    fn start(&mut self) -> eyre::Result<()> {
        let listener = LopCoordinatesIntercept(self.position_ptr.clone());

        self.listener = Some(super::attach_listener_to_signature(READ_FROM_COORDS_SIG, listener)?);

        Ok(())
    }

    fn get_current_coordinates(&mut self) -> eyre::Result<Option<crate::plugins::PlayerCoordinates>> {
        let coords = self.position_ptr.lock().unwrap();
        let out = unsafe {
            coords
                .map(|ptr| *(ptr as *mut PlayerCoordinates))
                .map(|coords| super::PlayerCoordinates {
                    x: coords.x,
                    y: coords.y,
                    z: coords.z,
                })
        };

        Ok(out)
    }

    fn set_current_coordinates(&mut self, target: crate::plugins::PlayerCoordinates) -> eyre::Result<()> {
        if let Some(coords) = self.position_ptr.lock().unwrap().clone() {
            let coords = coords as *mut PlayerCoordinates;
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

pub struct LopCoordinatesIntercept(CoordinatePtr);

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct PlayerCoordinates {
    x: f32,
    z: f32,
    y: f32,
}

impl ProbeListener for LopCoordinatesIntercept {
    fn on_hit(&mut self, context: InvocationContext) {
        let base_ptr = context.cpu_context().r9();
        let position_ptr = base_ptr + 0x1C0;
        let coords = position_ptr as usize;

        let mut lock = self.0.lock().unwrap();

        if lock.as_ref().map(|v| *v != coords).unwrap_or(true) {
            let old = lock.map(|ptr| ptr).unwrap_or_default();
            *lock = Some(coords);
            log::trace!("Updated LoP player position pointer from `{old:#X}` to {:#X}", coords);
        }
    }
}
