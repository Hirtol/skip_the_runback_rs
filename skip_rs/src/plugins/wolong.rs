super::macros::simple_plugin!(
    WoLongPlugin,
    GenericConfig {
        identifiers: PluginIdentifiers {
            plugin_name: "Wo Long Fallen Dynasty Runback".into(),
            expected_module: Some("WoLong.exe".into()),
            expected_exe_name: Some("WoLong.exe".into()),
        },
        position: GenericPositionConfig::InterceptPtr(InterceptConfig {
            intercept_signature: "0F 28 80 10 02 00 00 0F 29 44".into(),
            register: Register::Rax,
            filter: None,
        }),
        pointer_offsets: OffsetsConfig {
            x: 0x210,
            y: 0x214,
            z: 0x218,
        },
    }
);
