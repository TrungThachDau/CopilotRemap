#![windows_subsystem = "windows"]

use copilot_remap::{config, executor, key_handler, registry_helper, settings_gui};
use windows::Win32::System::Environment::GetCommandLineW;

fn main() {
    let full_cmd_line = unsafe {
        let ptr = GetCommandLineW();
        let mut len = 0;
        while *ptr.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr.0, len))
    };

    let args: Vec<String> = std::env::args().collect();

    // Log launch command for diagnostics (%LOCALAPPDATA%\CopilotRemap\last_launch.log)
    let log_path = config::AppConfig::get_appdata_dir().join("last_launch.log");
    let _ = std::fs::write(
        &log_path,
        format!("Timestamp: {:?}\nCmdLine: {}\nArgs: {:?}\n", std::time::SystemTime::now(), full_cmd_line, args),
    );

    // Check if test mode
    if args.iter().any(|a| a == "--test" || a == "-t") {
        let config = config::AppConfig::load();
        executor::execute_action(&config);
        return;
    }

    // Check if list apps command
    if args.iter().any(|a| a == "--list-apps" || a == "-l") {
        let apps = copilot_remap::app_scanner::get_installed_apps();
        let log_file = config::AppConfig::get_appdata_dir().join("installed_apps.txt");
        let mut text = format!("Discovered {} installed applications:\n", apps.len());
        for app in apps {
            text.push_str(&format!("- {} [{}] -> {}\n", app.name, if app.is_store_app { "Store" } else { "Desktop" }, app.path));
        }
        let _ = std::fs::write(&log_file, text);
        return;
    }

    // Check if registry setup command
    if let Some(pos) = args.iter().position(|a| a == "--set-aumid") {
        if let Some(aumid) = args.get(pos + 1) {
            registry_helper::set_copilot_key_provider(aumid);
            return;
        }
    }

    // Check if user explicitly launched Settings via command line
    let is_explicit_settings = args.iter().any(|a| {
        let lower = a.to_lowercase();
        lower == "--settings"
            || lower == "-s"
            || lower == "/settings"
            || lower == "-settings"
            || lower == "--config"
            || lower == "-c"
    });

    if is_explicit_settings {
        settings_gui::run_settings_window();
        return;
    }

    // If config file doesn't exist yet, open Settings on first run
    let config_path = config::AppConfig::get_config_path();
    if !config_path.exists() {
        settings_gui::run_settings_window();
        return;
    }

    // Main action: Execute configured app/URL/shortcut immediately
    key_handler::handle_key_activation(&args, &full_cmd_line);
}
