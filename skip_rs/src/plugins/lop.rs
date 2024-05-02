//! Lies of P specific module
super::macros::simple_plugin!(
    LOPPlugin,
    GenericConfig {
        identifiers: PluginIdentifiers {
            plugin_name: "Lies of P Skip Runback".into(),
            expected_module: Some("LOP-Win64-Shipping.exe".into()),
            expected_exe_name: Some("LOP.exe".into()),
        },
        position: GenericPositionConfig::InterceptPtr(InterceptConfig {
            intercept_signature: "41 0F 10 89 C0 01 00 00 48 8D 44 24 28".into(),
            register: Register::R9,
            filter: None,
        }),
        pointer_offsets: OffsetsConfig {
            x: 0x1C0,
            y: 0x1C4,
            z: 0x1C8,
        },
    }
);
