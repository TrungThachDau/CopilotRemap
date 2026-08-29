use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActionType {
    LaunchApp,
    #[default]
    OpenUrl,
    SendKeys,
    CustomCommand,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaunchAppConfig {
    pub path: String,
    pub arguments: String,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenUrlConfig {
    pub url: String,
    pub browser: String,
}

impl Default for OpenUrlConfig {
    fn default() -> Self {
        Self {
            url: "https://chatgpt.com".to_string(),
            browser: "Default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendKeysConfig {
    pub keys: Vec<String>,
}

impl Default for SendKeysConfig {
    fn default() -> Self {
        Self {
            keys: vec!["Alt".to_string(), "Space".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommandConfig {
    pub command: String,
    pub arguments: String,
    pub run_hidden: bool,
}

impl Default for CustomCommandConfig {
    fn default() -> Self {
        Self {
            command: "wt.exe".to_string(),
            arguments: String::new(),
            run_hidden: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub action_type: ActionType,
    pub launch_app: LaunchAppConfig,
    pub open_url: OpenUrlConfig,
    pub send_keys: SendKeysConfig,
    pub custom_command: CustomCommandConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            action_type: ActionType::OpenUrl,
            launch_app: LaunchAppConfig::default(),
            open_url: OpenUrlConfig::default(),
            send_keys: SendKeysConfig::default(),
            custom_command: CustomCommandConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn get_appdata_dir() -> PathBuf {
        let local_appdata = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| "C:\\ProgramData".to_string());
        let dir = PathBuf::from(local_appdata).join("CopilotRemap");
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        dir
    }

    pub fn get_config_path() -> PathBuf {
        Self::get_appdata_dir().join("config.json")
    }

    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        if config_path.exists()
            && let Ok(content) = fs::read_to_string(&config_path)
            && let Ok(config) = serde_json::from_str::<AppConfig>(&content)
        {
            return config;
        }
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::get_config_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        fs::write(config_path, content)
    }
}
