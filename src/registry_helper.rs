use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    REG_VALUE_TYPE,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[allow(dead_code)]
pub struct CopilotKeyStatus {
    pub choice_type: String,
    pub aumid: String,
    pub is_copilot_remap_active: bool,
}

pub fn get_copilot_key_status() -> CopilotKeyStatus {
    let subkey = to_wide(r"Software\Microsoft\Windows\Shell\BrandedKey");
    let mut hkey = HKEY::default();

    let mut choice_type = String::new();
    let mut aumid = String::new();

    unsafe {
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        ) == ERROR_SUCCESS
        {
            // Read BrandedKeyChoiceType
            let choice_name = to_wide("BrandedKeyChoiceType");
            let mut buf = [0u16; 256];
            let mut buf_size = (buf.len() * 2) as u32;
            let mut reg_type = REG_VALUE_TYPE(0);

            if RegQueryValueExW(
                hkey,
                PCWSTR(choice_name.as_ptr()),
                None,
                Some(&mut reg_type as *mut _),
                Some(buf.as_mut_ptr() as *mut u8),
                Some(&mut buf_size),
            ) == ERROR_SUCCESS
            {
                let len = (buf_size as usize / 2).saturating_sub(1);
                choice_type = String::from_utf16_lossy(&buf[..len]);
            }

            // Read AppAumid
            let aumid_name = to_wide("AppAumid");
            let mut aumid_buf = [0u16; 512];
            let mut aumid_buf_size = (aumid_buf.len() * 2) as u32;

            if RegQueryValueExW(
                hkey,
                PCWSTR(aumid_name.as_ptr()),
                None,
                Some(&mut reg_type as *mut _),
                Some(aumid_buf.as_mut_ptr() as *mut u8),
                Some(&mut aumid_buf_size),
            ) == ERROR_SUCCESS
            {
                let len = (aumid_buf_size as usize / 2).saturating_sub(1);
                aumid = String::from_utf16_lossy(&aumid_buf[..len]);
            }

            let _ = RegCloseKey(hkey);
        }
    }

    let is_copilot_remap_active =
        choice_type.eq_ignore_ascii_case("App") && aumid.to_lowercase().contains("copilotremap");

    CopilotKeyStatus {
        choice_type,
        aumid,
        is_copilot_remap_active,
    }
}

pub fn set_copilot_key_provider(aumid: &str) -> bool {
    let subkey = to_wide(r"Software\Microsoft\Windows\Shell\BrandedKey");
    let mut hkey = HKEY::default();

    unsafe {
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            Some(0),
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        ) == ERROR_SUCCESS
        {
            let choice_name = to_wide("BrandedKeyChoiceType");
            let choice_val = to_wide("App");
            let choice_bytes: &[u8] = std::slice::from_raw_parts(
                choice_val.as_ptr() as *const u8,
                choice_val.len() * 2,
            );

            let aumid_name = to_wide("AppAumid");
            let aumid_val = to_wide(aumid);
            let aumid_bytes: &[u8] = std::slice::from_raw_parts(
                aumid_val.as_ptr() as *const u8,
                aumid_val.len() * 2,
            );

            let res1 = RegSetValueExW(
                hkey,
                PCWSTR(choice_name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(choice_bytes),
            );

            let res2 = RegSetValueExW(
                hkey,
                PCWSTR(aumid_name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(aumid_bytes),
            );

            let _ = RegCloseKey(hkey);
            res1 == ERROR_SUCCESS && res2 == ERROR_SUCCESS
        } else {
            false
        }
    }
}

pub fn open_windows_copilot_settings() {
    let uri = to_wide("ms-settings:personalization-textinput-copilot-hardwarekey");
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(uri.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}
