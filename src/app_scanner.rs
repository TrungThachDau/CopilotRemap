#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::{w, PCWSTR};
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName,
    SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
};

#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
    pub is_store_app: bool,
}

pub fn get_installed_apps() -> Vec<InstalledApp> {
    let mut apps: BTreeMap<String, (String, bool)> = BTreeMap::new();

    // 1. Built-in standard Windows apps
    let system_apps = [
        ("Windows Terminal", "wt.exe"),
        ("Task Manager", r"C:\Windows\System32\Taskmgr.exe"),
        ("Notepad", "notepad.exe"),
        ("File Explorer", "explorer.exe"),
        ("Calculator", "calc.exe"),
        ("Command Prompt", "cmd.exe"),
        ("PowerShell", "powershell.exe"),
        ("Paint", "mspaint.exe"),
        ("Snipping Tool", "snippingtool.exe"),
    ];
    for (name, path) in system_apps {
        apps.insert(name.to_string(), (path.to_string(), false));
    }

    // 2. Scan Windows Store & Modern Apps via shell:AppsFolder
    scan_shell_apps_folder(&mut apps);

    // 3. Scan Start Menu folders (System + User)
    if let Ok(program_data) = std::env::var("ProgramData") {
        let path = PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs");
        scan_directory_for_links(&path, &mut apps);
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        let path = PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs");
        scan_directory_for_links(&path, &mut apps);
    }

    // Convert map to sorted Vec
    let mut result: Vec<InstalledApp> = apps
        .into_iter()
        .map(|(name, (path, is_store_app))| InstalledApp {
            name,
            path,
            is_store_app,
        })
        .collect();

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

fn scan_shell_apps_folder(apps: &mut BTreeMap<String, (String, bool)>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        if let Ok(apps_folder) =
            SHCreateItemFromParsingName::<PCWSTR, _, IShellItem>(w!("shell:AppsFolder"), None)
        {
            if let Ok(enum_items) =
                apps_folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems)
            {
                let mut items = [None];
                let mut fetched = 0;

                while enum_items.Next(&mut items, Some(&mut fetched)).is_ok() && fetched > 0 {
                    if let Some(item) = &items[0] {
                        let name = get_item_string(item, SIGDN_NORMALDISPLAY);
                        let parsing_name = get_item_string(item, SIGDN_DESKTOPABSOLUTEPARSING);

                        if let (Some(name), Some(parsing_name)) = (name, parsing_name) {
                            let lower = name.to_lowercase();
                            if !name.trim().is_empty()
                                && !lower.contains("uninstall")
                                && !lower.contains("gỡ cài đặt")
                                && !lower.contains("setup")
                                && !lower.starts_with("ms-resource:")
                            {
                                let is_store = parsing_name.contains('!')
                                    || parsing_name.to_lowercase().contains("windowsapps")
                                    || parsing_name.to_lowercase().contains("shell:appsfolder");

                                let clean_path = if !parsing_name.to_lowercase().starts_with("shell:appsfolder\\")
                                    && parsing_name.contains('!')
                                {
                                    format!(r"shell:AppsFolder\{}", parsing_name)
                                } else {
                                    parsing_name
                                };

                                apps.entry(name.trim().to_string())
                                    .or_insert((clean_path, is_store));
                            }
                        }
                    }
                }
            }
        }

        CoUninitialize();
    }
}

unsafe fn get_item_string(
    item: &IShellItem,
    sigdn: windows::Win32::UI::Shell::SIGDN,
) -> Option<String> {
    if let Ok(raw_str) = item.GetDisplayName(sigdn) {
        if !raw_str.is_null() {
            let mut len = 0;
            while *raw_str.0.add(len) != 0 {
                len += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(raw_str.0, len));
            CoTaskMemFree(Some(raw_str.0 as *mut _));
            return Some(s);
        }
    }
    None
}

fn scan_directory_for_links(dir: &Path, apps: &mut BTreeMap<String, (String, bool)>) {
    if !dir.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_directory_for_links(&path, apps);
            } else if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("lnk") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let lower = stem.to_lowercase();
                        // Filter out non-app shortcuts
                        if lower.contains("uninstall")
                            || lower.contains("gỡ cài đặt")
                            || lower.contains("help")
                            || lower.contains("trợ giúp")
                            || lower.contains("readme")
                            || lower.contains("documentation")
                            || lower.contains("tài liệu")
                            || lower.contains("website")
                            || lower.starts_with("visit ")
                            || lower.starts_with("remove ")
                        {
                            continue;
                        }

                        let clean_name = stem.trim().to_string();
                        if !clean_name.is_empty() {
                            let full_path = path.to_string_lossy().to_string();
                            apps.entry(clean_name).or_insert((full_path, false));
                        }
                    }
                }
            }
        }
    }
}
