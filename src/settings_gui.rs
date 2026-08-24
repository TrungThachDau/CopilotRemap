#![allow(unsafe_op_in_unsafe_fn)]

use crate::app_scanner::{self, InstalledApp};
use crate::config::{ActionType, AppConfig, CustomCommandConfig, LaunchAppConfig, OpenUrlConfig, SendKeysConfig};
use crate::executor;
use crate::registry_helper;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreateSolidBrush, DeleteObject, GetStockObject, GetSysColor, InvalidateRect,
    SetBkColor, SetBkMode, SetTextColor, CLIP_DEFAULT_PRECIS, COLOR_BTNFACE, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_DONTCARE, FONT_QUALITY, FW_BOLD, FW_NORMAL, HBRUSH, HDC, HFONT,
    OUT_DEFAULT_PRECIS, TRANSPARENT, WHITE_BRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, MessageBoxW, PostQuitMessage,
    RegisterClassExW, SendMessageW, SetWindowLongPtrW, SetWindowTextW, ShowWindow,
    TranslateMessage, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, GWLP_USERDATA, HCURSOR, HICON, HMENU, MB_ICONINFORMATION, MB_OK, MSG,
    SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN,
    WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_DESTROY, WM_SETFONT, WNDCLASSEXW,
    WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_EX_CLIENTEDGE, WS_MINIMIZEBOX, WS_OVERLAPPED,
    WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_UNCHECKED: usize = 0x0000;
const BST_CHECKED: usize = 0x0001;

const BS_GROUPBOX: u32 = 0x00000007;
const BS_AUTOCHECKBOX: u32 = 0x00000003;
const ES_AUTOHSCROLL: u32 = 0x0080;

const ID_MAIN_COMBO: usize = 101;
const ID_BTN_WIN_SETTINGS: usize = 102;

// App controls
const ID_COMBO_INSTALLED_APPS: usize = 110;
const ID_EDIT_APP_PATH: usize = 111;
const ID_BTN_BROWSE: usize = 112;
const ID_EDIT_APP_ARGS: usize = 113;
const ID_EDIT_APP_DIR: usize = 114;

// URL controls
const ID_EDIT_URL: usize = 124;

// Keys controls
const ID_BTN_HK_POWERTOYS: usize = 130;
const ID_BTN_HK_SNIP: usize = 131;
const ID_BTN_HK_TASKMGR: usize = 132;
const ID_BTN_HK_TASKVIEW: usize = 133;
const ID_EDIT_KEYS: usize = 134;

// Command controls
const ID_EDIT_CMD: usize = 140;
const ID_EDIT_CMD_ARGS: usize = 141;
const ID_CHK_CMD_HIDDEN: usize = 142;

// Action buttons
const ID_BTN_TEST: usize = 150;
const ID_BTN_SAVE: usize = 151;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

unsafe fn send_msg(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> LRESULT {
    SendMessageW(hwnd, msg, Some(WPARAM(wparam)), Some(LPARAM(lparam)))
}

fn get_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        GetWindowTextW(hwnd, &mut buf);
        let actual_len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..actual_len])
    }
}

fn set_text(hwnd: HWND, text: &str) {
    let wide = to_wide(text);
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(wide.as_ptr()));
    }
}

fn is_checked(hwnd: HWND) -> bool {
    unsafe {
        let res = send_msg(hwnd, BM_GETCHECK, 0, 0);
        res.0 == BST_CHECKED as isize
    }
}

fn set_checked(hwnd: HWND, checked: bool) {
    let val = if checked { BST_CHECKED } else { BST_UNCHECKED };
    unsafe {
        let _ = send_msg(hwnd, BM_SETCHECK, val, 0);
    }
}

struct WindowControls {
    h_font_normal: HFONT,
    h_font_bold: HFONT,
    h_bg_brush: HBRUSH,

    cb_main_mode: HWND,
    current_mode: std::cell::Cell<usize>, // 0: App, 1: URL, 2: Keys, 3: Cmd

    // Group 0: App controls
    grp_app: Vec<HWND>,
    cb_installed_apps: HWND,
    e_app_path: HWND,
    e_app_args: HWND,
    e_app_dir: HWND,

    // Group 1: URL controls
    grp_url: Vec<HWND>,
    e_url: HWND,

    // Group 2: Keys controls
    grp_keys: Vec<HWND>,
    e_keys: HWND,

    // Group 3: Command controls
    grp_cmd: Vec<HWND>,
    e_cmd: HWND,
    e_cmd_args: HWND,
    chk_cmd_hidden: HWND,

    installed_apps: Vec<InstalledApp>,
}

pub fn run_settings_window() {
    unsafe {
        let hinstance = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();
        let class_name = to_wide("CopilotRemapWinFormsClass");

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(hinstance.0),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: CreateSolidBrush(COLORREF(GetSysColor(COLOR_BTNFACE))),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: HICON::default(),
        };

        let _ = RegisterClassExW(&wnd_class);

        let title = to_wide("Copilot Key Remapper - Settings");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            550,
            425,
            None,
            None,
            Some(HINSTANCE(hinstance.0)),
            None,
        );

        if let Ok(hwnd) = hwnd {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            init_window_controls(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let controls = &*(ptr as *const WindowControls);
                let hdc = HDC(wparam.0 as *mut _);
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(0x00000000));
                return LRESULT(controls.h_bg_brush.0 as isize);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let hdc = HDC(wparam.0 as *mut _);
            SetBkColor(hdc, COLORREF(0x00FFFFFF));
            SetTextColor(hdc, COLORREF(0x00000000));
            LRESULT(GetStockObject(WHITE_BRUSH).0 as isize)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as usize;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u16;

            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let controls = &*(ptr as *const WindowControls);
                handle_command(hwnd, controls, id, code);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let controls = Box::from_raw(ptr as *mut WindowControls);
                let _ = DeleteObject(controls.h_font_normal.into());
                let _ = DeleteObject(controls.h_font_bold.into());
                let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(controls.h_bg_brush.0));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn init_window_controls(hwnd: HWND) {
    let font_name = to_wide("Segoe UI");
    let h_font_normal = CreateFontW(
        16, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, FONT_QUALITY(0),
        (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
        PCWSTR(font_name.as_ptr()),
    );
    let h_font_bold = CreateFontW(
        16, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, FONT_QUALITY(0),
        (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
        PCWSTR(font_name.as_ptr()),
    );

    let h_bg_brush = CreateSolidBrush(COLORREF(GetSysColor(COLOR_BTNFACE)));
    let hinstance = HINSTANCE(GetModuleHandleW(PCWSTR::null()).unwrap_or_default().0);

    let create_control = |class: &str, text: &str, ex_style: u32, style: u32, x, y, w, h, id: usize| -> HWND {
        let wide_class = to_wide(class);
        let wide_text = to_wide(text);
        let ctrl = CreateWindowExW(
            WINDOW_EX_STYLE(ex_style),
            PCWSTR(wide_class.as_ptr()),
            PCWSTR(wide_text.as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(style),
            x, y, w, h,
            Some(hwnd),
            Some(HMENU(id as *mut _)),
            Some(hinstance),
            None,
        ).unwrap_or_default();

        let _ = send_msg(ctrl, WM_SETFONT, h_font_normal.0 as usize, 1);
        ctrl
    };

    // 1. Action Selector Section
    let lbl_main = create_control("STATIC", "Action to perform when Copilot key is pressed:", 0, 0, 18, 14, 400, 18, 0);
    let _ = send_msg(lbl_main, WM_SETFONT, h_font_bold.0 as usize, 1);

    let combo_style = 0x0003 | 0x0200 | WS_VSCROLL.0 | WS_TABSTOP.0;
    let cb_main_mode = create_control("COMBOBOX", "", 0, combo_style, 18, 34, 498, 250, ID_MAIN_COMBO);

    let main_options = [
        "Launch Application",
        "Open Website (URL)",
        "Send Hotkey Shortcut",
        "Run Shell Command",
    ];

    for opt in main_options {
        let wide = to_wide(opt);
        let _ = send_msg(cb_main_mode, CB_ADDSTRING, 0, wide.as_ptr() as isize);
    }

    // 2. WinForms GroupBox Container
    let grp_box = create_control("BUTTON", " Configuration ", 0, BS_GROUPBOX, 18, 70, 498, 236, 0);
    let _ = send_msg(grp_box, WM_SETFONT, h_font_bold.0 as usize, 1);

    // ==========================================
    // GROUP 0: Launch App Controls (Inside GroupBox)
    // ==========================================
    let mut grp_app = Vec::new();

    let lbl_app_choose = create_control("STATIC", "Select installed application:", 0, 0, 34, 96, 250, 18, 0);
    grp_app.push(lbl_app_choose);

    let cb_installed_apps = create_control("COMBOBOX", "", 0, combo_style, 34, 116, 466, 350, ID_COMBO_INSTALLED_APPS);
    grp_app.push(cb_installed_apps);

    let installed_apps = app_scanner::get_installed_apps();
    let prompt_wide = to_wide("-- Choose from installed Windows applications --");
    let _ = send_msg(cb_installed_apps, CB_ADDSTRING, 0, prompt_wide.as_ptr() as isize);

    for app in &installed_apps {
        let wide = to_wide(&app.name);
        let _ = send_msg(cb_installed_apps, CB_ADDSTRING, 0, wide.as_ptr() as isize);
    }

    let lbl_app_path = create_control("STATIC", "Application path / executable:", 0, 0, 34, 150, 300, 18, 0);
    grp_app.push(lbl_app_path);

    let e_app_path = create_control("EDIT", "", WS_EX_CLIENTEDGE.0, WS_TABSTOP.0 | ES_AUTOHSCROLL, 34, 170, 376, 23, ID_EDIT_APP_PATH);
    grp_app.push(e_app_path);

    let btn_browse = create_control("BUTTON", "Browse...", 0, WS_TABSTOP.0, 418, 169, 82, 25, ID_BTN_BROWSE);
    grp_app.push(btn_browse);

    let lbl_app_args = create_control("STATIC", "Arguments (optional):", 0, 0, 34, 204, 150, 18, 0);
    grp_app.push(lbl_app_args);

    let e_app_args = create_control("EDIT", "", WS_EX_CLIENTEDGE.0, WS_TABSTOP.0 | ES_AUTOHSCROLL, 34, 224, 218, 23, ID_EDIT_APP_ARGS);
    grp_app.push(e_app_args);

    let lbl_app_dir = create_control("STATIC", "Working directory (optional):", 0, 0, 266, 204, 180, 18, 0);
    grp_app.push(lbl_app_dir);

    let e_app_dir = create_control("EDIT", "", WS_EX_CLIENTEDGE.0, WS_TABSTOP.0 | ES_AUTOHSCROLL, 266, 224, 234, 23, ID_EDIT_APP_DIR);
    grp_app.push(e_app_dir);

    // ==========================================
    // GROUP 1: URL Controls (Inside GroupBox)
    // ==========================================
    let mut grp_url = Vec::new();

    let lbl_url = create_control("STATIC", "Target Website URL:", 0, 0, 34, 96, 200, 18, 0);
    grp_url.push(lbl_url);

    let e_url = create_control("EDIT", "", WS_EX_CLIENTEDGE.0, WS_TABSTOP.0 | ES_AUTOHSCROLL, 34, 116, 466, 23, ID_EDIT_URL);
    grp_url.push(e_url);

    // ==========================================
    // GROUP 2: Keys Controls (Inside GroupBox)
    // ==========================================
    let mut grp_keys = Vec::new();

    let lbl_hk_preset = create_control("STATIC", "Quick Presets:", 0, 0, 34, 96, 200, 18, 0);
    grp_keys.push(lbl_hk_preset);

    let btn_hk_pt = create_control("BUTTON", "PowerToys Run (Alt+Space)", 0, WS_TABSTOP.0, 34, 118, 226, 26, ID_BTN_HK_POWERTOYS);
    grp_keys.push(btn_hk_pt);

    let btn_hk_snip = create_control("BUTTON", "Snipping Tool (Win+Shift+S)", 0, WS_TABSTOP.0, 274, 118, 226, 26, ID_BTN_HK_SNIP);
    grp_keys.push(btn_hk_snip);

    let btn_hk_taskmgr = create_control("BUTTON", "Task Manager (Ctrl+Shift+Esc)", 0, WS_TABSTOP.0, 34, 152, 226, 26, ID_BTN_HK_TASKMGR);
    grp_keys.push(btn_hk_taskmgr);

    let btn_hk_taskview = create_control("BUTTON", "Task View (Win+Tab)", 0, WS_TABSTOP.0, 274, 152, 226, 26, ID_BTN_HK_TASKVIEW);
    grp_keys.push(btn_hk_taskview);

    let lbl_keys = create_control("STATIC", "Shortcut keys (comma-separated, e.g. Alt, Space):", 0, 0, 34, 192, 350, 18, 0);
    grp_keys.push(lbl_keys);

    let e_keys = create_control("EDIT", "", WS_EX_CLIENTEDGE.0, WS_TABSTOP.0 | ES_AUTOHSCROLL, 34, 214, 466, 23, ID_EDIT_KEYS);
    grp_keys.push(e_keys);

    // ==========================================
    // GROUP 3: Command Controls (Inside GroupBox)
    // ==========================================
    let mut grp_cmd = Vec::new();

    let lbl_cmd = create_control("STATIC", "Command or Executable:", 0, 0, 34, 96, 200, 18, 0);
    grp_cmd.push(lbl_cmd);

    let e_cmd = create_control("EDIT", "", WS_EX_CLIENTEDGE.0, WS_TABSTOP.0 | ES_AUTOHSCROLL, 34, 116, 466, 23, ID_EDIT_CMD);
    grp_cmd.push(e_cmd);

    let lbl_cmd_args = create_control("STATIC", "Arguments (optional):", 0, 0, 34, 150, 200, 18, 0);
    grp_cmd.push(lbl_cmd_args);

    let e_cmd_args = create_control("EDIT", "", WS_EX_CLIENTEDGE.0, WS_TABSTOP.0 | ES_AUTOHSCROLL, 34, 170, 466, 23, ID_EDIT_CMD_ARGS);
    grp_cmd.push(e_cmd_args);

    let chk_cmd_hidden = create_control("BUTTON", "Run in background silently (hide console window)", 0, BS_AUTOCHECKBOX | WS_TABSTOP.0, 34, 208, 400, 22, ID_CHK_CMD_HIDDEN);
    grp_cmd.push(chk_cmd_hidden);

    // 3. Bottom WinForms Button Bar (y = 325..355)
    let btn_win_settings = create_control("BUTTON", "Windows Settings...", 0, WS_TABSTOP.0, 18, 325, 140, 28, ID_BTN_WIN_SETTINGS);
    let _ = send_msg(btn_win_settings, WM_SETFONT, h_font_normal.0 as usize, 1);

    let btn_test = create_control("BUTTON", "Test Action", 0, WS_TABSTOP.0, 288, 325, 110, 28, ID_BTN_TEST);
    let _ = send_msg(btn_test, WM_SETFONT, h_font_normal.0 as usize, 1);

    let btn_save = create_control("BUTTON", "Save", 0, WS_TABSTOP.0, 406, 325, 110, 28, ID_BTN_SAVE);
    let _ = send_msg(btn_save, WM_SETFONT, h_font_bold.0 as usize, 1);

    let controls = Box::new(WindowControls {
        h_font_normal,
        h_font_bold,
        h_bg_brush,

        cb_main_mode,
        current_mode: std::cell::Cell::new(0),

        grp_app,
        cb_installed_apps,
        e_app_path,
        e_app_args,
        e_app_dir,

        grp_url,
        e_url,

        grp_keys,
        e_keys,

        grp_cmd,
        e_cmd,
        e_cmd_args,
        chk_cmd_hidden,

        installed_apps,
    });

    let config = AppConfig::load();
    populate_ui(hwnd, &controls, &config);
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(controls) as isize);
}

unsafe fn switch_mode(hwnd: HWND, controls: &WindowControls, mode: usize) {
    controls.current_mode.set(mode);

    for &ctrl in &controls.grp_app {
        let _ = ShowWindow(ctrl, if mode == 0 { SW_SHOW } else { SW_HIDE });
    }
    for &ctrl in &controls.grp_url {
        let _ = ShowWindow(ctrl, if mode == 1 { SW_SHOW } else { SW_HIDE });
    }
    for &ctrl in &controls.grp_keys {
        let _ = ShowWindow(ctrl, if mode == 2 { SW_SHOW } else { SW_HIDE });
    }
    for &ctrl in &controls.grp_cmd {
        let _ = ShowWindow(ctrl, if mode == 3 { SW_SHOW } else { SW_HIDE });
    }

    let _ = InvalidateRect(Some(hwnd), None, true);
}

unsafe fn populate_ui(hwnd: HWND, controls: &WindowControls, config: &AppConfig) {
    set_text(controls.e_url, &config.open_url.url);
    set_text(controls.e_app_path, &config.launch_app.path);
    set_text(controls.e_app_args, &config.launch_app.arguments);
    set_text(controls.e_app_dir, &config.launch_app.working_dir);
    set_text(controls.e_keys, &config.send_keys.keys.join(", "));
    set_text(controls.e_cmd, &config.custom_command.command);
    set_text(controls.e_cmd_args, &config.custom_command.arguments);
    set_checked(controls.chk_cmd_hidden, config.custom_command.run_hidden);

    // Try to match installed app
    let mut matched_app_idx = 0;
    if !config.launch_app.path.is_empty() {
        for (i, app) in controls.installed_apps.iter().enumerate() {
            if app.path.eq_ignore_ascii_case(&config.launch_app.path)
                || app.name.eq_ignore_ascii_case(&config.launch_app.path)
            {
                matched_app_idx = i + 1;
                break;
            }
        }
    }
    let _ = send_msg(controls.cb_installed_apps, CB_SETCURSEL, matched_app_idx, 0);

    let mode = match config.action_type {
        ActionType::LaunchApp => 0,
        ActionType::OpenUrl => 1,
        ActionType::SendKeys => 2,
        ActionType::CustomCommand => 3,
    };

    let _ = send_msg(controls.cb_main_mode, CB_SETCURSEL, mode, 0);
    switch_mode(hwnd, controls, mode);
}

fn get_config_from_ui(controls: &WindowControls) -> AppConfig {
    let mode = controls.current_mode.get();

    let action_type = match mode {
        0 => ActionType::LaunchApp,
        1 => ActionType::OpenUrl,
        2 => ActionType::SendKeys,
        _ => ActionType::CustomCommand,
    };

    let keys_str = get_text(controls.e_keys);
    let keys = keys_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    AppConfig {
        version: 1,
        action_type,
        launch_app: LaunchAppConfig {
            path: get_text(controls.e_app_path),
            arguments: get_text(controls.e_app_args),
            working_dir: get_text(controls.e_app_dir),
        },
        open_url: OpenUrlConfig {
            url: get_text(controls.e_url),
            browser: "Default".to_string(),
        },
        send_keys: SendKeysConfig { keys },
        custom_command: CustomCommandConfig {
            command: get_text(controls.e_cmd),
            arguments: get_text(controls.e_cmd_args),
            run_hidden: is_checked(controls.chk_cmd_hidden),
        },
    }
}

unsafe fn handle_command(hwnd: HWND, controls: &WindowControls, id: usize, code: u16) {
    match id {
        ID_MAIN_COMBO if code == 1 => {
            let sel = send_msg(controls.cb_main_mode, CB_GETCURSEL, 0, 0).0;
            if sel >= 0 && sel <= 3 {
                switch_mode(hwnd, controls, sel as usize);
            }
        }
        ID_COMBO_INSTALLED_APPS if code == 1 => {
            let sel = send_msg(controls.cb_installed_apps, CB_GETCURSEL, 0, 0).0;
            if sel > 0 && (sel - 1) < controls.installed_apps.len() as isize {
                let app = &controls.installed_apps[(sel - 1) as usize];
                set_text(controls.e_app_path, &app.path);
            }
        }
        ID_BTN_HK_POWERTOYS => {
            set_text(controls.e_keys, "Alt, Space");
        }
        ID_BTN_HK_SNIP => {
            set_text(controls.e_keys, "Win, Shift, S");
        }
        ID_BTN_HK_TASKMGR => {
            set_text(controls.e_keys, "Ctrl, Shift, Esc");
        }
        ID_BTN_HK_TASKVIEW => {
            set_text(controls.e_keys, "Win, Tab");
        }
        ID_BTN_BROWSE => {
            let mut file_buf = [0u16; 1024];
            let filter = to_wide("Executable & Shortcut Files (*.exe;*.lnk;*.bat;*.cmd)\0*.exe;*.lnk;*.bat;*.cmd\0All Files (*.*)\0*.*\0\0");
            let title = to_wide("Select Application to Launch");

            let mut ofn = OPENFILENAMEW {
                lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
                hwndOwner: hwnd,
                lpstrFilter: PCWSTR(filter.as_ptr()),
                lpstrFile: PWSTR(file_buf.as_mut_ptr()),
                nMaxFile: file_buf.len() as u32,
                lpstrTitle: PCWSTR(title.as_ptr()),
                Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_EXPLORER,
                ..Default::default()
            };

            if GetOpenFileNameW(&mut ofn).as_bool() {
                let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
                let path = String::from_utf16_lossy(&file_buf[..len]);
                set_text(controls.e_app_path, &path);
            }
        }
        ID_BTN_WIN_SETTINGS => {
            registry_helper::open_windows_copilot_settings();
        }
        ID_BTN_TEST => {
            let config = get_config_from_ui(controls);
            executor::execute_action(&config);
        }
        ID_BTN_SAVE => {
            let config = get_config_from_ui(controls);
            if let Err(e) = config.save() {
                let err_msg = to_wide(&format!("Failed to save settings: {}", e));
                let title = to_wide("Error");
                let _ = MessageBoxW(Some(hwnd), PCWSTR(err_msg.as_ptr()), PCWSTR(title.as_ptr()), MB_OK);
            } else {
                let msg = to_wide("Settings saved successfully!");
                let title = to_wide("Saved");
                let _ = MessageBoxW(Some(hwnd), PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_ICONINFORMATION | MB_OK);
                let _ = DestroyWindow(hwnd);
            }
        }
        _ => {}
    }
}
