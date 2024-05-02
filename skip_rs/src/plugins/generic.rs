use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use frida_gum::interceptor::{InvocationContext, ProbeListener};

pub use crate::plugins::{PlayerCoordinates, PluginIdentifiers, SkipPlugin};
pub use crate::plugins::generic::config::*;

pub static SKIP_PLUGIN_FILENAME: &str = "skip_runback_plugin.json";

type CoordinatePtr = Arc<Mutex<Option<usize>>>;

pub struct ConfigBasedPlugin {
    position_ptr: CoordinatePtr,
    listener: Option<Pin<Box<GenericCoordinateIntercept>>>,
    config_path: Option<PathBuf>,
    config: GenericConfig,
}

impl ConfigBasedPlugin {
    pub fn find_all(base_path: &Path) -> eyre::Result<Vec<Box<dyn SkipPlugin>>> {
        let file = base_path.join(SKIP_PLUGIN_FILENAME);
        let mut output: Vec<Box<dyn SkipPlugin>> = Vec::new();

        if file.exists() {
            output.push(Box::new(Self::new(file)?));
        }

        Ok(output)
    }

    pub fn new(config_path: impl Into<PathBuf>) -> eyre::Result<Self> {
        let file = config_path.into();
        let conf = Self::load_config(&file)?;
        let ptr = CoordinatePtr::default();

        Ok(Self {
            config: conf,
            position_ptr: ptr,
            listener: None,
            config_path: Some(file),
        })
    }

    pub fn from_config(config: GenericConfig) -> ConfigBasedPlugin {
        Self {
            position_ptr: CoordinatePtr::default(),
            listener: None,
            config_path: None,
            config,
        }
    }

    fn load_config(cfg_path: &Path) -> eyre::Result<GenericConfig> {
        let data = std::fs::read(cfg_path)?;
        let conf = serde_json::from_slice(&data)?;
        Ok(conf)
    }

    fn start_intercept(&mut self, intercept: config::InterceptConfig) -> eyre::Result<()> {
        let listener = GenericCoordinateIntercept {
            position_ptr: self.position_ptr.clone(),
            config: intercept.clone(),
        };

        self.listener = Some(super::attach_listener_to_signature(
            &intercept.intercept_signature,
            listener,
        )?);

        Ok(())
    }

    fn start_given_ptr(&mut self, intercept: config::PointerTypeConfig) -> eyre::Result<()> {
        *self.position_ptr.lock().unwrap() = Some(intercept.get_non_null_ptr()?.as_ptr() as usize);

        log::info!("Using given pointer pointing to `{intercept:#?}`");

        Ok(())
    }
}

impl super::SkipPlugin for ConfigBasedPlugin {
    fn identifiers(&self) -> PluginIdentifiers {
        self.config.identifiers.clone()
    }

    fn start(&mut self) -> eyre::Result<()> {
        match self.config.position.clone() {
            GenericPositionConfig::InterceptPtr(cfg) => self.start_intercept(cfg)?,
            GenericPositionConfig::AbsolutePtr(cfg) => self.start_given_ptr(cfg)?,
        }

        Ok(())
    }

    fn get_current_coordinates(&mut self) -> eyre::Result<Option<PlayerCoordinates>> {
        if let Some(opt) = self.position_ptr.lock().unwrap().as_ref() {
            let ptr = *opt as *mut f32;
            unsafe {
                let out = PlayerCoordinates {
                    x: ptr.byte_offset(self.config.pointer_offsets.x).read(),
                    y: ptr.byte_offset(self.config.pointer_offsets.y).read(),
                    z: ptr.byte_offset(self.config.pointer_offsets.z).read(),
                };

                Ok(Some(out))
            }
        } else {
            Ok(None)
        }
    }

    fn set_current_coordinates(&mut self, coordinates: PlayerCoordinates) -> eyre::Result<()> {
        if let Some(opt) = self.position_ptr.lock().unwrap().as_ref() {
            let ptr = *opt as *mut f32;

            unsafe {
                ptr.byte_offset(self.config.pointer_offsets.x).write(coordinates.x);
                ptr.byte_offset(self.config.pointer_offsets.y).write(coordinates.y);
                ptr.byte_offset(self.config.pointer_offsets.z).write(coordinates.z);
            }

            Ok(())
        } else {
            eyre::bail!("Pointer not initialised")
        }
    }

    fn reload_config(&mut self) -> eyre::Result<()> {
        let Some(path) = self.config_path.as_ref() else {
            return Ok(());
        };
        match Self::load_config(path) {
            Ok(cfg) => {
                if self.config == cfg {
                    return Ok(());
                }
                // Lock the position pointer to prevent any race conditions while we're updating
                let mut lock = self.position_ptr.lock().unwrap();
                *lock = None;

                match &cfg.position {
                    GenericPositionConfig::InterceptPtr(intr) => {
                        // Can safely update this directly as the changes will take effect the next iteration
                        if let Some(listener) = &mut self.listener {
                            listener.config = intr.clone();
                        }
                    }
                    GenericPositionConfig::AbsolutePtr(ptr) => {
                        *lock = Some(ptr.get_non_null_ptr()?.as_ptr() as usize);
                    }
                }

                self.config = cfg;
            }
            Err(e) => {
                log::warn!("Failed to reload config of generic skip due to `{e:?}`");
            }
        }

        Ok(())
    }
}

pub struct GenericCoordinateIntercept {
    position_ptr: CoordinatePtr,
    config: InterceptConfig,
}

impl ProbeListener for GenericCoordinateIntercept {
    fn on_hit(&mut self, context: InvocationContext) {
        let ctx = context.cpu_context();

        if let Some(filter) = &self.config.filter {
            let value = filter.compare.to_value(&ctx) as usize;
            let should_proceed = match filter.comparison {
                Comparison::Equal => value == filter.compare_to,
                Comparison::NEqual => value != filter.compare_to,
                Comparison::Gt => value > filter.compare_to,
                Comparison::Lt => value < filter.compare_to,
            };

            if !should_proceed {
                return;
            }
        }

        let base_ptr = self.config.register.to_value(&ctx) as usize;

        let mut lock = self.position_ptr.lock().unwrap();

        if lock.map(|ptr| ptr != base_ptr).unwrap_or(true) {
            let old = lock.map(|ptr| ptr).unwrap_or_default();
            *lock = Some(base_ptr);
            log::trace!("Updated player pointer from `{old:#X}` to {:#X}", base_ptr);
        }
    }
}

mod config {
    use eyre::ContextCompat;
    use rust_hooking_utils::patching::process::GameProcess;
    use rust_hooking_utils::pointer::NonNullPtr;

    use crate::plugins::PluginIdentifiers;

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub struct GenericConfig {
        pub identifiers: PluginIdentifiers,
        pub position: GenericPositionConfig,
        /// The offset from the acquired pointer above for the x/y/z coordinates.
        pub pointer_offsets: OffsetsConfig,
    }

    impl Default for GenericConfig {
        fn default() -> Self {
            Self {
                identifiers: PluginIdentifiers {
                    plugin_name: "Generic Skip Sekiro Example".to_string(),
                    expected_module: Some("sekiro.exe".to_string()),
                    expected_exe_name: Some("sekiro.exe".to_string()),
                },
                position: GenericPositionConfig::InterceptPtr(InterceptConfig {
                    intercept_signature: crate::plugins::sekiro::READ_FROM_COORDS_SIG.to_string(),
                    register: Register::Rcx,
                    filter: None,
                }),
                pointer_offsets: OffsetsConfig {
                    x: 0x80,
                    y: 0x80 + 4,
                    z: 0x80 + 8,
                },
            }
        }
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub enum GenericPositionConfig {
        InterceptPtr(InterceptConfig),
        AbsolutePtr(PointerTypeConfig),
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub struct InterceptConfig {
        /// The signature of the code which will have the pointer to the player position passed during the execution
        pub intercept_signature: String,
        /// The register in which we'll find the player position pointer during execution of the code identifier by the above
        /// signature.
        pub register: Register,
        /// Optional filter for which calls to the `intercept_signature` identified code should be ignored
        /// (say other entities' positions are also altered by this code).
        pub filter: Option<Filter>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub struct OffsetsConfig {
        pub x: isize,
        pub y: isize,
        pub z: isize,
    }

    impl Default for OffsetsConfig {
        fn default() -> Self {
            Self { x: 0, y: 4, z: 8 }
        }
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub enum PointerTypeConfig {
        Absolute(NonNullPtr),
        Relative(RelativePointer),
    }

    impl PointerTypeConfig {
        pub fn get_non_null_ptr(&self) -> eyre::Result<NonNullPtr> {
            let ptr = match self {
                PointerTypeConfig::Absolute(abs) => *abs,
                PointerTypeConfig::Relative(rel) => {
                    let module_name = rel.module_name()?;
                    let module = GameProcess::current_process().get_module(module_name)?;
                    let base_ptr = module.base();
                    unsafe {
                        NonNullPtr::new(base_ptr.add(rel.offset()?) as usize).context("Invalid relative pointer")?
                    }
                }
            };

            Ok(ptr)
        }
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub struct RelativePointer(String);

    impl RelativePointer {
        pub fn module_name(&self) -> eyre::Result<&str> {
            let (module_name, _) = self.0.split_once('+').context("Invalid relative pointer syntax")?;
            Ok(module_name)
        }

        pub fn offset(&self) -> eyre::Result<usize> {
            let (_, offset) = self.0.split_once('+').context("Invalid relative pointer syntax")?;
            Ok(usize::from_str_radix(offset, 16)?)
        }
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, PartialOrd)]
    pub enum Register {
        Rax,
        Rbx,
        Rcx,
        Rdx,
        Rsi,
        Rdi,
        Rbp,
        Rsp,
        R8,
        R9,
        R10,
        R11,
        R12,
        R13,
        R14,
        R15,
        Rip,
    }

    impl Register {
        pub fn to_value(&self, ctx: &frida_gum::CpuContext) -> u64 {
            match self {
                Register::Rax => ctx.rax(),
                Register::Rbx => ctx.rbx(),
                Register::Rcx => ctx.rcx(),
                Register::Rdx => ctx.rdx(),
                Register::Rsi => ctx.rsi(),
                Register::Rdi => ctx.rdi(),
                Register::Rbp => ctx.rbp(),
                Register::Rsp => ctx.rsp(),
                Register::R8 => ctx.r8(),
                Register::R9 => ctx.r9(),
                Register::R10 => ctx.r10(),
                Register::R11 => ctx.r11(),
                Register::R12 => ctx.r12(),
                Register::R13 => ctx.r13(),
                Register::R14 => ctx.r14(),
                Register::R15 => ctx.r15(),
                Register::Rip => ctx.rip(),
            }
        }
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub struct Filter {
        pub compare: Register,
        pub comparison: Comparison,
        pub compare_to: usize,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, PartialOrd)]
    pub enum Comparison {
        Equal,
        NEqual,
        Gt,
        Lt,
    }
}
