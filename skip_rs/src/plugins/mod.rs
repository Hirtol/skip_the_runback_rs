use std::ffi::c_void;
use std::path::Path;
use std::pin::Pin;
use std::sync::Mutex;

use frida_gum::interceptor::ProbeListener;
use frida_gum::NativePointer;
use once_cell::sync::Lazy;
use rust_hooking_utils::patching::process::GameProcess;

use crate::utils::NullLock;

mod generic;
mod lop;
mod macros;
mod sekiro;
mod wolong;

pub static GUM: Lazy<frida_gum::Gum> = Lazy::new(|| unsafe { frida_gum::Gum::obtain() });
pub static PROBE_INTERCEPTOR: Lazy<Mutex<NullLock<frida_gum::interceptor::Interceptor>>> =
    Lazy::new(|| Mutex::new(NullLock(frida_gum::interceptor::Interceptor::obtain(&GUM))));

/// Get all plugins which could apply.
pub fn get_all_plugins(search_path: &Path) -> Vec<Box<dyn SkipPlugin>> {
    let mut generic_skips = generic::ConfigBasedPlugin::find_all(search_path).unwrap_or_default();

    generic_skips.push(Box::new(lop::LOPPlugin::new()));
    generic_skips.push(Box::new(sekiro::SekiroPlugin::new()));
    generic_skips.push(Box::new(wolong::WoLongPlugin::new()));

    generic_skips
}

pub trait SkipPlugin {
    /// Retrieve the identifiers which will be used to check whether the current exe matches, unless the match function has been replaced.
    fn identifiers(&self) -> PluginIdentifiers;

    /// Indicate to this plugin that it should run, apply its hooks, etc.
    fn start(&mut self) -> eyre::Result<()>;

    /// Check whether this plugin should be applied
    fn should_apply(&self) -> bool {
        let idents = self.identifiers();
        let process = rust_hooking_utils::patching::process::GameProcess::current_process();
        let base_module = process.get_base_module().expect("Failed to get base module");
        let modules_match = idents
            .expected_module
            .and_then(|expected| process.get_module(&expected).ok())
            .is_some();

        let exe_path = rust_hooking_utils::get_current_dll_path(base_module.module_handle());
        let exe_matches = idents
            .expected_exe_name
            .and_then(|exe| {
                exe_path
                    .ok()
                    .and_then(|path| path.file_name().map(|f| f.to_string_lossy() == exe))
            })
            .unwrap_or_default();

        exe_matches || modules_match
    }

    /// Return the current player coordinates.
    ///
    /// Can return [None] if the player pointer hasn't been identifier yet.
    fn get_current_coordinates(&mut self) -> eyre::Result<Option<PlayerCoordinates>>;

    /// Set the given coordinates as the new player coordinates.
    ///
    /// # Errors
    ///
    /// If the location could not be set
    fn set_current_coordinates(&mut self, coordinates: PlayerCoordinates) -> eyre::Result<()>;

    /// Force the current plugins to reload their configs, and restart their interceptors if needed.
    fn reload_config(&mut self) -> eyre::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialOrd, PartialEq)]
pub struct PlayerCoordinates {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CPlayerCoordinates {
    x: f32,
    z: f32,
    y: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub struct PluginIdentifiers {
    pub plugin_name: String,
    pub expected_module: Option<String>,
    pub expected_exe_name: Option<String>,
}

/// Find the given signature in the current process' base module, and subsequently attach the [PROBE_INTERCEPTOR]
/// with the given `listener`.
///
/// The pinned `listener` is returned, and must be saved for the remaining lifetime of the program.
///
/// The [Unpin] bound is unfortunately necessary due to the fact that we can't pass the [Pin] directly...
pub fn attach_listener_to_signature<T: ProbeListener + Unpin>(
    signature: &str,
    listener: T,
) -> eyre::Result<Pin<Box<T>>> {
    let position_fn_ptr = GameProcess::current_process()
        .get_base_module()?
        .to_local()?
        .scan_for_pattern(signature)
        .map_err(|e| eyre::eyre!(Box::new(e)))? as usize;

    log::info!("Found position modification ptr: {:#X}", position_fn_ptr);

    // Undocumented in `attach_instruction`, but this *needs* to be pinned as they save the pointer we pass
    let mut listener = Box::pin(listener);
    PROBE_INTERCEPTOR
        .lock()
        .unwrap()
        .attach_instruction(NativePointer(position_fn_ptr as *mut c_void), &mut *listener);

    log::info!("Initiated interceptor at {position_fn_ptr:#X}");

    Ok(listener)
}
