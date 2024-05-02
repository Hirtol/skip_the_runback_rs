/// Define a simple plugin which uses the [ConfigBasedPlugin] as a base.
///
/// # Example
/// ```rust
/// # use crate::skip_rs::simple_plugin;
/// simple_plugin!(
///     WoLongPlugin,
///     GenericConfig {
///         identifiers: PluginIdentifiers {
///             plugin_name: "Wo Long Fallen Dynasty Runback".into(),
///             expected_module: Some("WoLong.exe".into()),
///             expected_exe_name: Some("WoLong.exe".into()),
///         },
///         position: GenericPositionConfig::InterceptPtr(InterceptConfig {
///             intercept_signature: "0F 28 80 10 02 00 00 0F 29 44".into(),
///             register: Register::Rax,
///             filter: None,
///         }),
///         pointer_offsets: OffsetsConfig {
///             x: 0x210,
///             y: 0x214,
///             z: 0x218,
///         },
///     }
/// );
/// ```
#[macro_export]
macro_rules! simple_plugin {
    ($name:ident, $config:expr) => {
        use $crate::plugins::generic::*;
        use $crate::plugins::{PluginIdentifiers, SkipPlugin};
        pub struct $name(ConfigBasedPlugin);

        impl $name {
            pub fn new() -> Self {
                Self(ConfigBasedPlugin::from_config($config))
            }
        }

        impl SkipPlugin for $name {
            fn identifiers(&self) -> PluginIdentifiers {
                self.0.identifiers()
            }

            fn start(&mut self) -> eyre::Result<()> {
                self.0.start()
            }

            fn get_current_coordinates(&mut self) -> eyre::Result<Option<$crate::plugins::PlayerCoordinates>> {
                self.0.get_current_coordinates()
            }

            fn set_current_coordinates(&mut self, target: $crate::plugins::PlayerCoordinates) -> eyre::Result<()> {
                self.0.set_current_coordinates(target)
            }
        }
    };
}

pub(crate) use simple_plugin;
