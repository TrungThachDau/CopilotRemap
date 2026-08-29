use crate::config::AppConfig;
use crate::executor;

pub fn handle_key_activation(args: &[String], full_cmd_line: &str) {
    let mut state = "Tap".to_string();

    let combined = format!("{} {}", full_cmd_line, args.join(" ")).to_lowercase();

    if let Some(pos) = combined.find("state=") {
        let s = &combined[pos + 6..];
        let s = s.split(['&', ' ', '"', '\'']).next().unwrap_or(s);
        state = s.trim().to_string();
    }

    let config = AppConfig::load();

    match state.to_lowercase().as_str() {
        "tap" | "single" => {
            executor::execute_action(&config);
        }
        "down" => {
            executor::execute_action(&config);
        }
        "up" => {
            // Key release
        }
        _ => {
            executor::execute_action(&config);
        }
    }
}
