use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use eyre::{ContextCompat, Result};
use log::LevelFilter;
use rust_hooking_utils::patching::process::GameProcess;
use rust_hooking_utils::raw_input::key_manager::KeyboardManager;
use rust_hooking_utils::raw_input::virtual_keys::VirtualKey;
use windows::core::HSTRING;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MB_OK, MessageBoxExW};

use crate::config::SkipConfig;

mod app;
mod config;
mod waypoints;

mod utils;

mod plugins;

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

pub fn dll_attach(hinst_dll: windows::Win32::Foundation::HMODULE) -> Result<()> {
    let dll_path = rust_hooking_utils::get_current_dll_path(hinst_dll).map_err(|e| eyre::eyre!(Box::new(e)))?;
    let save_config_directory = dll_path.parent().context("DLL is in root")?;
    let cfg = simplelog::ConfigBuilder::new().build();

    // Ignore result in case we have double initialisation of the DLL.
    simplelog::SimpleLogger::init(LevelFilter::Trace, cfg)?;

    config::create_initial_config(save_config_directory)?;

    let Ok(mut conf) = load_validated_config(save_config_directory, None) else {
        std::process::exit(1)
    };

    if conf.console {
        unsafe {
            windows::Win32::System::Console::AllocConsole()?;
        }
    }

    log::info!("Loaded config: {:#?}", conf);

    let main_window = GameProcess::current_process()
        .get_main_window_blocking(None)
        .context("Failed to get window")?;

    log::info!("Found main window: {:?} ({:?})", main_window.title(), main_window.0);

    let mut key_manager = KeyboardManager::new();
    let mut update_duration = Duration::from_secs_f64(1.0 / 60.);

    let plugins = plugins::get_all_plugins(save_config_directory);

    let plugin_to_use = plugins
        .into_iter()
        .find(|p| p.should_apply())
        .context("No applicable plugin could be found, disabling SkipTheRunback")?;

    log::info!(
        "Found `{}` as the plugin to use for skipping runback",
        plugin_to_use.identifiers().plugin_name
    );

    let mut app = app::SkipApp::new(save_config_directory, plugin_to_use)?;

    while !SHUTDOWN_FLAG.load(Ordering::Acquire) {
        if let Some(reload) = &conf.reload_config_keys {
            if key_manager.all_pressed(reload.iter().copied().map(VirtualKey::to_virtual_key)) {
                conf = reload_config(save_config_directory, &mut conf, main_window.0)?;
            }
        }

        unsafe {
            // Only run if we're in the foreground. A bit hacky, but eh...
            if main_window.is_foreground_window() {
                app.run(&conf, &mut key_manager)?;
            }
        }

        std::thread::sleep(update_duration);
        key_manager.end_frame();
    }

    Ok(())
}

pub fn dll_detach(_hinst_dll: windows::Win32::Foundation::HMODULE) -> Result<()> {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    log::info!("Detached! {:?}", std::thread::current().id());

    Ok(())
}

fn reload_config(config_dir: impl AsRef<Path>, old: &mut SkipConfig, parent_window: HWND) -> eyre::Result<SkipConfig> {
    log::debug!("Reloading config");
    let conf = load_validated_config(config_dir.as_ref(), Some(parent_window))?;

    // Open/close console
    if old.console && !conf.console {
        unsafe {
            windows::Win32::System::Console::FreeConsole()?;
        }
    } else if !old.console && conf.console {
        unsafe {
            windows::Win32::System::Console::AllocConsole()?;
        }
    }

    log::debug!("New config loaded: {:#?}", conf);

    Ok(conf)
}

fn load_validated_config(config_dir: &Path, parent_window: Option<HWND>) -> eyre::Result<SkipConfig> {
    match config::load_config(config_dir) {
        Ok(conf) => Ok(conf),
        Err(e) => unsafe {
            let message = format!("Error: {}\nSkipRun will now exit", e);
            let _ = MessageBoxExW(
                parent_window.unwrap_or_default(),
                &HSTRING::from(message),
                windows::core::w!("Failed to validate SkipRun config"),
                MB_OK,
                0,
            );
            Err(e)
        },
    }
}
