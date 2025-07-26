//! AI Limit specific module
super::macros::simple_plugin!(
    AILimitPlugin,
    GenericConfig {
        identifiers: PluginIdentifiers {
            plugin_name: "AI Limit Skip Runback".into(),
            expected_module: Some("GameAssembly.dll".into()),
            expected_exe_name: Some("AI-LIMIT.exe".into()),
        },
        position: GenericPositionConfig::InterceptPtr(InterceptConfig {
            intercept_signature: "F2 0F 11 43 28 89 4B 30 40".into(),
            register: Register::Rbx,
            filter: Some(Filter {
                compare: Register::R10,
                comparison: Comparison::NEqual,
                compare_to: 0xA,
            }),
        }),
        pointer_offsets: OffsetsConfig {
            x: 0x28,
            y: 0x2C,
            z: 0x30,
        },
    }
);
