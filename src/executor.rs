use crate::config::{
    ActionType, AppConfig, CustomCommandConfig, LaunchAppConfig, OpenUrlConfig, SendKeysConfig,
};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{w, PCWSTR};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F10,
    VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16, VK_F17, VK_F18, VK_F19, VK_F2, VK_F20,
    VK_F21, VK_F22, VK_F23, VK_F24, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME,
    VK_INSERT, VK_LEFT, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK,
    VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_SNAPSHOT, VK_SPACE,
    VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn execute_action(config: &AppConfig) {
    match config.action_type {
        ActionType::LaunchApp => launch_app(&config.launch_app),
        ActionType::OpenUrl => open_url(&config.open_url),
        ActionType::SendKeys => send_keys(&config.send_keys),
        ActionType::CustomCommand => run_custom_command(&config.custom_command),
    }
}

pub fn launch_app(config: &LaunchAppConfig) {
    let path = config.path.trim();
    if path.is_empty() {
        return;
    }

    let lower_path = path.to_lowercase();

    // Check if this is a Windows Store / UWP / AUMID app or shell:AppsFolder URI
    let is_shell_uri = lower_path.starts_with("shell:")
        || path.contains('!')
        || (path.starts_with('{') && path.contains('}'))
        || path.starts_with("Microsoft.")
        || path.starts_with("electron.app.")
        || path.starts_with("com.");

    if is_shell_uri {
        let target = if lower_path.starts_with("shell:appsfolder\\") || lower_path.starts_with("shell:") {
            path.to_string()
        } else {
            format!(r"shell:AppsFolder\{}", path)
        };

        let wide_explorer = to_wide("explorer.exe");
        let wide_target = to_wide(&target);
        unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(wide_explorer.as_ptr()),
                PCWSTR(wide_target.as_ptr()),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
        return;
    }

    let wide_path = to_wide(path);
    let wide_args = if config.arguments.trim().is_empty() {
        None
    } else {
        Some(to_wide(config.arguments.trim()))
    };
    let wide_dir = if config.working_dir.trim().is_empty() {
        None
    } else {
        Some(to_wide(config.working_dir.trim()))
    };

    unsafe {
        let args_ptr = wide_args.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));
        let dir_ptr = wide_dir.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));

        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide_path.as_ptr()),
            args_ptr,
            dir_ptr,
            SW_SHOWNORMAL,
        );
    }
}

pub fn open_url(config: &OpenUrlConfig) {
    let url = config.url.trim();
    if url.is_empty() {
        return;
    }

    let url = if !url.starts_with("http://")
        && !url.starts_with("https://")
        && !url.contains("://")
    {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    let wide_url = to_wide(&url);

    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide_url.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

fn parse_virtual_key(key_str: &str) -> Option<VIRTUAL_KEY> {
    let s = key_str.trim().to_uppercase();
    match s.as_str() {
        "CTRL" | "CONTROL" | "LCTRL" | "RCTRL" => Some(VK_CONTROL),
        "ALT" | "LALT" | "RALT" | "MENU" => Some(VK_MENU),
        "SHIFT" | "LSHIFT" | "RSHIFT" => Some(VK_SHIFT),
        "WIN" | "WINDOWS" | "LWIN" | "SUPER" => Some(VK_LWIN),
        "RWIN" => Some(VK_RWIN),
        "SPACE" => Some(VK_SPACE),
        "ENTER" | "RETURN" => Some(VK_RETURN),
        "TAB" => Some(VK_TAB),
        "ESC" | "ESCAPE" => Some(VK_ESCAPE),
        "BACKSPACE" | "BKSP" => Some(VK_BACK),
        "DELETE" | "DEL" => Some(VK_DELETE),
        "INSERT" | "INS" => Some(VK_INSERT),
        "HOME" => Some(VK_HOME),
        "END" => Some(VK_END),
        "PAGEUP" | "PGUP" => Some(VK_PRIOR),
        "PAGEDOWN" | "PGDN" => Some(VK_NEXT),
        "UP" => Some(VK_UP),
        "DOWN" => Some(VK_DOWN),
        "LEFT" => Some(VK_LEFT),
        "RIGHT" => Some(VK_RIGHT),
        "PRINTSCREEN" | "PRTSC" => Some(VK_SNAPSHOT),
        "VOLUMEUP" | "VOLUP" => Some(VK_VOLUME_UP),
        "VOLUMEDOWN" | "VOLDOWN" => Some(VK_VOLUME_DOWN),
        "MUTE" => Some(VK_VOLUME_MUTE),
        "PLAYPAUSE" => Some(VK_MEDIA_PLAY_PAUSE),
        "NEXTTRACK" => Some(VK_MEDIA_NEXT_TRACK),
        "PREVTRACK" => Some(VK_MEDIA_PREV_TRACK),
        "F1" => Some(VK_F1),
        "F2" => Some(VK_F2),
        "F3" => Some(VK_F3),
        "F4" => Some(VK_F4),
        "F5" => Some(VK_F5),
        "F6" => Some(VK_F6),
        "F7" => Some(VK_F7),
        "F8" => Some(VK_F8),
        "F9" => Some(VK_F9),
        "F10" => Some(VK_F10),
        "F11" => Some(VK_F11),
        "F12" => Some(VK_F12),
        "F13" => Some(VK_F13),
        "F14" => Some(VK_F14),
        "F15" => Some(VK_F15),
        "F16" => Some(VK_F16),
        "F17" => Some(VK_F17),
        "F18" => Some(VK_F18),
        "F19" => Some(VK_F19),
        "F20" => Some(VK_F20),
        "F21" => Some(VK_F21),
        "F22" => Some(VK_F22),
        "F23" => Some(VK_F23),
        "F24" => Some(VK_F24),
        single_char if single_char.len() == 1 => {
            let ch = single_char.chars().next().unwrap();
            if ch.is_ascii_alphanumeric() {
                Some(VIRTUAL_KEY(ch as u16))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn send_keys(config: &SendKeysConfig) {
    if config.keys.is_empty() {
        return;
    }

    let mut vk_list = Vec::new();
    for k in &config.keys {
        if let Some(vk) = parse_virtual_key(k) {
            vk_list.push(vk);
        }
    }

    if vk_list.is_empty() {
        return;
    }

    let mut inputs = Vec::new();

    // Key downs
    for &vk in &vk_list {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    // Key ups in reverse order
    for &vk in vk_list.iter().rev() {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn run_custom_command(config: &CustomCommandConfig) {
    let cmd = config.command.trim();
    if cmd.is_empty() {
        return;
    }

    let mut command = std::process::Command::new("cmd.exe");
    command.arg("/C");
    if !config.arguments.trim().is_empty() {
        command.arg(format!("{} {}", cmd, config.arguments.trim()));
    } else {
        command.arg(cmd);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        if config.run_hidden {
            command.creation_flags(CREATE_NO_WINDOW);
        }
    }

    let _ = command.spawn();
}
