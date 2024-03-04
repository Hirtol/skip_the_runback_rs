//! Sekiro specific module for demonstration's sake

use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use frida_gum::interceptor::{InvocationContext, ProbeListener};
use frida_gum::NativePointer;
use rust_hooking_utils::patching::process::GameProcess;

use crate::plugins::{PluginIdentifiers, SkipPlugin};

/// Signature of the instruction which exclusively reads from the player coordinates struct.
pub static READ_FROM_COORDS_SIG: &str = "0F 28 81 80 00 00 00 4D";

type CoordinatePtr = Arc<Mutex<Option<usize>>>;

pub struct SekiroPlugin {
    coords: CoordinatePtr,
}

impl SekiroPlugin {
    pub fn new() -> Self {
        Self {
            coords: Default::default(),
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
        let camera_fn_call_ptr = GameProcess::current_process()
            .get_base_module()?
            .to_local()?
            .scan_for_pattern(READ_FROM_COORDS_SIG)
            .map_err(|e| eyre::eyre!(Box::new(e)))? as usize;

        log::info!("Found Sekiro position call ptr: {:#X}", camera_fn_call_ptr);
        let mut intercept = SekiroCoordinatesIntercept(self.coords.clone());

        // Awful, but need to lend out `intercept` as `mut` permanently 🙃
        // `Box::leak` would work, but `Interceptor` has a non 'static lifetime making it a pain to work with as it depends on `gum`.
        std::thread::spawn(move || loop {
            let gum = unsafe { frida_gum::Gum::obtain() };
            let mut intercp = frida_gum::interceptor::Interceptor::obtain(&gum);
            intercp.attach_instruction(NativePointer(camera_fn_call_ptr as *mut c_void), &mut intercept);

            std::thread::sleep(Duration::MAX)
        });

        Ok(())
    }

    fn get_current_coordinates(&mut self) -> eyre::Result<Option<crate::plugins::PlayerCoordinates>> {
        let coords = self.coords.lock().unwrap();
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
        if let Some(coords) = self.coords.lock().unwrap().clone() {
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

pub struct SekiroCoordinatesIntercept(CoordinatePtr);

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct PlayerCoordinates {
    x: f32,
    y: f32,
    z: f32,
}

impl ProbeListener for SekiroCoordinatesIntercept {
    fn on_hit(&mut self, context: InvocationContext) {
        // Non-player character filter
        // if context.cpu_context().rcx() != 0x0 {
        //     return;
        // }
        let base_ptr = context.cpu_context().rcx();
        let position_ptr = base_ptr + 0x80;
        let coords = position_ptr as usize;

        let mut lock = self.0.lock().unwrap();

        if lock.as_ref().map(|v| *v != coords).unwrap_or(true) {
            let old = lock.map(|ptr| ptr).unwrap_or_default();
            *lock = Some(coords);
            log::trace!("Updated Sekiro player pointer from `{old:#X}` to {:#X}", coords);
        }
    }
}
