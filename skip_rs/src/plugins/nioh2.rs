//! Nioh 2 specific module
super::macros::simple_plugin!(
    Nioh2Plugin,
    GenericConfig {
        identifiers: PluginIdentifiers {
            plugin_name: "Nioh 2".into(),
            expected_module: None,
            expected_exe_name: Some("nioh2.exe".into()),
        },
        position: GenericPositionConfig::InterceptPtr(InterceptConfig {
            intercept_signature: "0F 28 80 F0 00 00 00 66 0F 7F 45 A0".into(),
            register: Register::Rax,
            filter: None,
        }),
        pointer_offsets: OffsetsConfig {
            x: 0xF0,
            y: 0xF4,
            z: 0xF8,
        },
    }
);
