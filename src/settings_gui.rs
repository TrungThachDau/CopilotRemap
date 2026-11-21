use crate::app_scanner::{self, InstalledApp};
use crate::config::{
    ActionType, AppConfig, CustomCommandConfig, LaunchAppConfig, OpenUrlConfig, SendKeysConfig,
};
use crate::executor;
use crate::registry_helper;
use std::path::PathBuf;
use std::time::Duration;
use windows::Win32::Foundation::ERROR_CANCELLED;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance,
    CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_NODEREFERENCELINKS, FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::{HSTRING, w};
use windows_reactor::*;

const WINDOW_TITLE: &str = "Copilot Key Remapper - Settings";
const PROVIDER_POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_APP_SUGGESTIONS: usize = 50;

const ACTION_TYPES: [(ActionType, &str); 4] = [
    (ActionType::LaunchApp, "Launch an application"),
    (ActionType::OpenUrl, "Open a website"),
    (ActionType::SendKeys, "Send a keyboard shortcut"),
    (ActionType::CustomCommand, "Run a shell command"),
];

const URL_PRESETS: [(&str, &str); 4] = [
    ("ChatGPT", "https://chatgpt.com"),
    ("Claude AI", "https://claude.ai"),
    ("Google Gemini", "https://gemini.google.com"),
    ("Perplexity", "https://www.perplexity.ai"),
];

const KEY_PRESETS: [(&str, &str); 4] = [
    ("PowerToys Run (Alt+Space)", "Alt, Space"),
    ("Snipping Tool (Win+Shift+S)", "Win, Shift, S"),
    ("Task Manager (Ctrl+Shift+Esc)", "Ctrl, Shift, Esc"),
    ("Task View (Win+Tab)", "Win, Tab"),
];

pub fn run_settings_window() {
    if let Err(error) = App::run_component::<SettingsApp>(()) {
        let text = HSTRING::from(format!(
            "The settings window could not be started.\n\n{}",
            error.message()
        ));
        unsafe {
            MessageBoxW(None, &text, w!("Copilot Key Remapper"), MB_OK | MB_ICONERROR);
        }
    }
}

fn normalize_vietnamese(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        let ch = match c {
            'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ' | 'ấ'
            | 'ẩ' | 'ẫ' | 'ậ' => 'a',
            'À' | 'Á' | 'Ả' | 'Ã' | 'Ạ' | 'Ă' | 'Ằ' | 'Ắ' | 'Ẳ' | 'Ẵ' | 'Ặ' | 'Â' | 'Ầ' | 'Ấ'
            | 'Ẩ' | 'Ẫ' | 'Ậ' => 'a',
            'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => 'e',
            'È' | 'É' | 'Ẻ' | 'Ẽ' | 'Ẹ' | 'Ê' | 'Ề' | 'Ế' | 'Ể' | 'Ễ' | 'Ệ' => 'e',
            'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
            'Ì' | 'Í' | 'Ỉ' | 'Ĩ' | 'Ị' => 'i',
            'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ'
            | 'ở' | 'ỡ' | 'ợ' => 'o',
            'Ò' | 'Ó' | 'Ỏ' | 'Õ' | 'Ọ' | 'Ô' | 'Ồ' | 'Ố' | 'Ổ' | 'Ỗ' | 'Ộ' | 'Ơ' | 'Ờ' | 'Ớ'
            | 'Ở' | 'Ỡ' | 'Ợ' => 'o',
            'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => 'u',
            'Ù' | 'Ú' | 'Ủ' | 'Ũ' | 'Ụ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ử' | 'Ữ' | 'Ự' => 'u',
            'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
            'Ỳ' | 'Ý' | 'Ỷ' | 'Ỹ' | 'Ỵ' => 'y',
            'đ' => 'd',
            'Đ' => 'd',
            other => other.to_ascii_lowercase(),
        };
        result.push(ch);
    }
    result
}

/// Returns the apps whose name contains `query`, ignoring case and Vietnamese diacritics.
fn filter_apps<'a>(
    apps: &'a [InstalledApp],
    query: &str,
) -> impl Iterator<Item = &'a InstalledApp> {
    let query = normalize_vietnamese(query.trim());
    apps.iter()
        .filter(move |app| normalize_vietnamese(&app.name).contains(&query))
}

/// Shows the Shell file-open dialog and returns the chosen path, or `None` when cancelled.
fn pick_executable() -> Result<Option<PathBuf>, String> {
    // Shell dialogs need an STA thread; Reactor's background work runs on the thread pool.
    std::thread::spawn(|| unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok()?;
        let result = show_open_dialog();
        CoUninitialize();
        result
    })
    .join()
    .map_err(|_| "The file dialog thread panicked".to_string())?
    .map_err(|error| error.message())
}

unsafe fn show_open_dialog() -> windows::core::Result<Option<PathBuf>> {
    unsafe {
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetTitle(w!("Choose an application"))?;
        dialog.SetFileTypes(&[
            COMDLG_FILTERSPEC {
                pszName: w!("Executable / Shortcut"),
                pszSpec: w!("*.exe;*.lnk;*.bat;*.cmd"),
            },
            COMDLG_FILTERSPEC {
                pszName: w!("All Files"),
                pszSpec: w!("*.*"),
            },
        ])?;
        // Keep the chosen .lnk itself instead of resolving it to its target.
        dialog.SetOptions(dialog.GetOptions()? | FOS_NODEREFERENCELINKS)?;

        if let Err(error) = dialog.Show(None) {
            return if error.code() == ERROR_CANCELLED.to_hresult() {
                Ok(None)
            } else {
                Err(error)
            };
        }

        let raw = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = raw.to_string();
        CoTaskMemFree(Some(raw.0 as *const _));
        Ok(Some(PathBuf::from(path?)))
    }
}

#[derive(Clone)]
enum Message {
    ActionTypeChanged(Option<usize>),
    AppsLoaded(Vec<InstalledApp>),
    AppSearchChanged(String),
    AppSuggestionChosen(String),
    AppSelected(Option<usize>),
    AppPathChanged(String),
    BrowseApp,
    AppPicked(Result<Option<PathBuf>, String>),
    AppArgsChanged(String),
    AppDirChanged(String),
    UrlChanged(String),
    KeysChanged(String),
    CmdCommandChanged(String),
    CmdArgumentsChanged(String),
    CmdRunHiddenChanged(bool),
    ProviderStatus(bool),
    OpenWindowsSettings,
    TestAction,
    Save,
    DismissStatus,
}

struct Status {
    severity: InfoBarSeverity,
    message: String,
}

struct SettingsApp {
    action_type: ActionType,

    installed_apps: Option<Vec<InstalledApp>>,
    app_search: String,
    is_picking_file: bool,
    app_path: String,
    app_args: String,
    app_dir: String,

    url: String,

    keys: String,

    cmd_command: String,
    cmd_arguments: String,
    cmd_run_hidden: bool,

    provider_active: bool,
    status: Option<Status>,
}

impl SettingsApp {
    fn from_config(config: AppConfig) -> Self {
        Self {
            action_type: config.action_type,
            installed_apps: None,
            app_search: String::new(),
            is_picking_file: false,
            app_path: config.launch_app.path,
            app_args: config.launch_app.arguments,
            app_dir: config.launch_app.working_dir,
            url: config.open_url.url,
            keys: config.send_keys.keys.join(", "),
            cmd_command: config.custom_command.command,
            cmd_arguments: config.custom_command.arguments,
            cmd_run_hidden: config.custom_command.run_hidden,
            provider_active: registry_helper::get_copilot_key_status().is_copilot_remap_active,
            status: None,
        }
    }

    fn to_config(&self) -> AppConfig {
        let keys = self
            .keys
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        AppConfig {
            version: 1,
            action_type: self.action_type,
            launch_app: LaunchAppConfig {
                path: self.app_path.trim().to_string(),
                arguments: self.app_args.trim().to_string(),
                working_dir: self.app_dir.trim().to_string(),
            },
            open_url: OpenUrlConfig {
                url: self.url.trim().to_string(),
                browser: "Default".to_string(),
            },
            send_keys: SendKeysConfig { keys },
            custom_command: CustomCommandConfig {
                command: self.cmd_command.trim().to_string(),
                arguments: self.cmd_arguments.trim().to_string(),
                run_hidden: self.cmd_run_hidden,
            },
        }
    }

    fn apps(&self) -> &[InstalledApp] {
        self.installed_apps.as_deref().unwrap_or_default()
    }

    fn selected_app_index(&self) -> Option<usize> {
        let path = self.app_path.trim();
        if path.is_empty() {
            return None;
        }
        self.apps()
            .iter()
            .position(|app| app.path.eq_ignore_ascii_case(path) || app.name.eq_ignore_ascii_case(path))
    }

    fn set_status(&mut self, severity: InfoBarSeverity, message: impl Into<String>) {
        self.status = Some(Status {
            severity,
            message: message.into(),
        });
    }

    fn poll_provider_status(context: &ComponentContext<Self>, delay: Duration) {
        context.spawn_background(move |cancellation| {
            std::thread::sleep(delay);
            if cancellation.is_cancelled() {
                return Message::ProviderStatus(false);
            }
            Message::ProviderStatus(
                registry_helper::get_copilot_key_status().is_copilot_remap_active,
            )
        });
    }

    fn header_view(&self) -> View {
        StackPanel::new()
            .spacing(4.0)
            .children((
                TextBlock::new()
                    .text("Copilot Key Remapper")
                    .font_size(28.0)
                    .font_weight(FontWeight::SEMI_BOLD),
                TextBlock::new()
                    .text("Choose what happens when you press the Copilot key on your keyboard.")
                    .text_wrapping(TextWrapping::Wrap),
            ))
    }

    fn provider_view(&self) -> View {
        let (severity, title, message) = if self.provider_active {
            (
                InfoBarSeverity::Success,
                "Copilot key provider is active",
                "Windows sends Copilot key presses to Copilot Remap.",
            )
        } else {
            (
                InfoBarSeverity::Warning,
                "Copilot Remap is not the Copilot key provider",
                "Open Windows Settings and choose Copilot Remap as the app for the Copilot key.",
            )
        };
        InfoBar::new()
            .is_open(true)
            .is_closable(false)
            .severity(severity)
            .title(title)
            .message(message)
            .into()
    }

    fn action_type_view(&self, context: &ViewContext<Self>) -> View {
        let selected = ACTION_TYPES
            .iter()
            .position(|(action, _)| *action == self.action_type);
        card(
            "When the Copilot key is pressed",
            RadioButtons::new()
                .items_source(ACTION_TYPES.map(|(_, label)| label))
                .selected_index(selected)
                .max_columns(2)
                .on_selection_changed(context.callback(Message::ActionTypeChanged)),
        )
    }

    fn details_view(&self, context: &ViewContext<Self>) -> View {
        match self.action_type {
            ActionType::LaunchApp => card("Application", self.launch_app_view(context)),
            ActionType::OpenUrl => card("Website", self.open_url_view(context)),
            ActionType::SendKeys => card("Keyboard shortcut", self.send_keys_view(context)),
            ActionType::CustomCommand => card("Shell command", self.custom_command_view(context)),
        }
    }

    fn launch_app_view(&self, context: &ViewContext<Self>) -> View {
        let apps_loaded = self.installed_apps.is_some();
        let suggestions: Vec<&str> = if self.app_search.trim().is_empty() {
            Vec::new()
        } else {
            filter_apps(self.apps(), &self.app_search)
                .take(MAX_APP_SUGGESTIONS)
                .map(|app| app.name.as_str())
                .collect()
        };

        StackPanel::new().spacing(12.0).children((
            AutoSuggestBox::new()
                .text(self.app_search.clone())
                .placeholder_text("Type to filter, e.g. Claude, Terminal, Chrome, Cốc Cốc")
                .items_source(suggestions)
                .is_enabled(apps_loaded)
                .on_text_changed(context.callback(Message::AppSearchChanged))
                .on_suggestion_chosen(context.callback(Message::AppSuggestionChosen))
                .slot(AutoSuggestBoxSlot::Header, "Search installed applications"),
            ComboBox::new()
                .items_source(self.apps().iter().map(|app| app.name.as_str()))
                .selected_index(self.selected_app_index())
                .placeholder_text(if apps_loaded {
                    "Choose from installed applications"
                } else {
                    "Loading installed applications..."
                })
                .is_enabled(apps_loaded)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .on_selection_changed(context.callback(Message::AppSelected))
                .slot(ComboBoxSlot::Header, "Installed application"),
            Grid::new()
                .column_spacing(8.0)
                .columns([GridLength::STAR, GridLength::Auto])
                .children((
                    TextBox::new()
                        .text(self.app_path.clone())
                        .placeholder_text(r"C:\Path\To\App.exe")
                        .on_text_changed(context.callback(Message::AppPathChanged))
                        .slot(TextBoxSlot::Header, "Application path or executable"),
                    Button::new()
                        .is_enabled(!self.is_picking_file)
                        .vertical_alignment(VerticalAlignment::Bottom)
                        .grid_column(1)
                        .on_click(context.message(Message::BrowseApp))
                        .content("Browse..."),
                )),
            Grid::new()
                .column_spacing(12.0)
                .columns([GridLength::STAR, GridLength::STAR])
                .children((
                    TextBox::new()
                        .text(self.app_args.clone())
                        .on_text_changed(context.callback(Message::AppArgsChanged))
                        .slot(TextBoxSlot::Header, "Arguments (optional)"),
                    TextBox::new()
                        .text(self.app_dir.clone())
                        .grid_column(1)
                        .on_text_changed(context.callback(Message::AppDirChanged))
                        .slot(TextBoxSlot::Header, "Working directory (optional)"),
                )),
        ))
    }

    fn open_url_view(&self, context: &ViewContext<Self>) -> View {
        StackPanel::new().spacing(12.0).children((
            presets_view(context, URL_PRESETS, Message::UrlChanged),
            TextBox::new()
                .text(self.url.clone())
                .placeholder_text("https://")
                .on_text_changed(context.callback(Message::UrlChanged))
                .slot(TextBoxSlot::Header, "Website URL"),
        ))
    }

    fn send_keys_view(&self, context: &ViewContext<Self>) -> View {
        StackPanel::new().spacing(12.0).children((
            presets_view(context, KEY_PRESETS, Message::KeysChanged),
            TextBox::new()
                .text(self.keys.clone())
                .placeholder_text("Alt, Space")
                .on_text_changed(context.callback(Message::KeysChanged))
                .slot(TextBoxSlot::Header, "Shortcut keys (comma-separated)"),
        ))
    }

    fn custom_command_view(&self, context: &ViewContext<Self>) -> View {
        StackPanel::new().spacing(12.0).children((
            TextBox::new()
                .text(self.cmd_command.clone())
                .placeholder_text("wt.exe")
                .on_text_changed(context.callback(Message::CmdCommandChanged))
                .slot(TextBoxSlot::Header, "Command or executable"),
            TextBox::new()
                .text(self.cmd_arguments.clone())
                .on_text_changed(context.callback(Message::CmdArgumentsChanged))
                .slot(TextBoxSlot::Header, "Arguments (optional)"),
            CheckBox::new()
                .is_checked(self.cmd_run_hidden)
                .on_is_checked_changed(context.callback(Message::CmdRunHiddenChanged))
                .content("Run silently in the background (hide the console window)"),
        ))
    }

    fn status_view(&self, context: &ViewContext<Self>) -> View {
        let (severity, message) = self
            .status
            .as_ref()
            .map_or((InfoBarSeverity::Informational, ""), |status| {
                (status.severity, status.message.as_str())
            });
        InfoBar::new()
            .is_open(self.status.is_some())
            .is_closable(true)
            .severity(severity)
            .message(message)
            .on_closed(context.message(Message::DismissStatus))
            .into()
    }

    fn footer_view(&self, context: &ViewContext<Self>) -> View {
        Border::new()
            .background(ThemeBrush::CardBackground)
            .border_brush(ThemeBrush::CardStroke)
            .border_thickness(Thickness::new(0.0, 1.0, 0.0, 0.0))
            .padding(Thickness::xy(24.0, 16.0))
            .grid_row(1)
            .content(
                Grid::new()
                    .column_spacing(8.0)
                    .columns([
                        GridLength::Auto,
                        GridLength::STAR,
                        GridLength::Auto,
                        GridLength::Auto,
                    ])
                    .children((
                        Button::new()
                            .on_click(context.message(Message::OpenWindowsSettings))
                            .content("Windows Settings..."),
                        Button::new()
                            .grid_column(2)
                            .on_click(context.message(Message::TestAction))
                            .content("Test action"),
                        Button::new()
                            .style(ButtonStyle::Accent)
                            .grid_column(3)
                            .on_click(context.message(Message::Save))
                            .content("Save settings"),
                    )),
            )
    }
}

impl Component for SettingsApp {
    type Input = ();
    type Message = Message;

    fn create(_input: &(), context: &ComponentContext<Self>) -> Self {
        // Scanning initializes COM and walks the Start menu, so keep it off the UI thread.
        context.spawn_background(|_| {
            let apps = std::thread::spawn(app_scanner::get_installed_apps)
                .join()
                .unwrap_or_default();
            Message::AppsLoaded(apps)
        });
        Self::poll_provider_status(context, PROVIDER_POLL_INTERVAL);
        Self::from_config(AppConfig::load())
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        match message {
            Message::ActionTypeChanged(Some(index)) => {
                if let Some((action, _)) = ACTION_TYPES.get(index) {
                    self.action_type = *action;
                }
            }
            Message::ActionTypeChanged(None) => {}
            Message::AppsLoaded(apps) => self.installed_apps = Some(apps),
            Message::AppSearchChanged(text) => self.app_search = text,
            Message::AppSuggestionChosen(name) => {
                if let Some(app) = self.apps().iter().find(|app| app.name == name) {
                    self.app_path = app.path.clone();
                }
                self.app_search = name;
            }
            Message::AppSelected(Some(index)) => {
                if let Some(app) = self.apps().get(index) {
                    self.app_path = app.path.clone();
                }
            }
            Message::AppSelected(None) => {}
            Message::AppPathChanged(text) => self.app_path = text,
            Message::BrowseApp if !self.is_picking_file => {
                self.is_picking_file = true;
                context.spawn_background(|_| Message::AppPicked(pick_executable()));
            }
            Message::BrowseApp => {}
            Message::AppPicked(result) => {
                self.is_picking_file = false;
                match result {
                    Ok(Some(path)) => self.app_path = path.display().to_string(),
                    Ok(None) => {}
                    Err(error) => self.set_status(
                        InfoBarSeverity::Error,
                        format!("Could not open the file picker: {error}"),
                    ),
                }
            }
            Message::AppArgsChanged(text) => self.app_args = text,
            Message::AppDirChanged(text) => self.app_dir = text,
            Message::UrlChanged(text) => self.url = text,
            Message::KeysChanged(text) => self.keys = text,
            Message::CmdCommandChanged(text) => self.cmd_command = text,
            Message::CmdArgumentsChanged(text) => self.cmd_arguments = text,
            Message::CmdRunHiddenChanged(value) => self.cmd_run_hidden = value,
            Message::ProviderStatus(active) => {
                self.provider_active = active;
                Self::poll_provider_status(context, PROVIDER_POLL_INTERVAL);
            }
            Message::OpenWindowsSettings => registry_helper::open_windows_copilot_settings(),
            Message::TestAction => {
                executor::execute_action(&self.to_config());
                self.set_status(InfoBarSeverity::Success, "Test action executed.");
            }
            Message::Save => match self.to_config().save() {
                Ok(()) => {
                    if !context.window().request_close() {
                        self.set_status(InfoBarSeverity::Success, "Settings saved.");
                    }
                }
                Err(error) => self.set_status(
                    InfoBarSeverity::Error,
                    format!("Failed to save settings: {error}"),
                ),
            },
            Message::DismissStatus => self.status = None,
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title(WINDOW_TITLE);
        context.window_visuals(
            WindowVisuals::new()
                .backdrop(WindowBackdrop::Mica)
                .client_size(640.0, 720.0)
                .constraints(WindowConstraints {
                    min_width: Some(520.0),
                    min_height: Some(560.0),
                    ..WindowConstraints::default()
                }),
        );

        Grid::new()
            .rows([GridLength::STAR, GridLength::Auto])
            .children((
                ScrollViewer::new().content(
                    StackPanel::new()
                        .spacing(16.0)
                        .margin(Thickness::uniform(24.0))
                        .children((
                            self.header_view(),
                            self.provider_view(),
                            self.action_type_view(context),
                            self.details_view(context),
                            self.status_view(context),
                        )),
                ),
                self.footer_view(context),
            ))
    }
}

fn card(title: &str, body: impl Into<View>) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(1.0)
        .corner_radius(8.0)
        .padding(16.0)
        .content(StackPanel::new().spacing(12.0).children((
            TextBlock::new()
                .text(title)
                .font_size(16.0)
                .font_weight(FontWeight::SEMI_BOLD),
            body.into(),
        )))
}

/// Lays out four preset buttons in a 2x2 grid; clicking one sends `to_message(value)`.
fn presets_view(
    context: &ViewContext<SettingsApp>,
    presets: [(&'static str, &'static str); 4],
    to_message: fn(String) -> Message,
) -> View {
    let mut index = 0;
    let buttons = presets.map(|(label, value)| {
        let (row, column) = (index / 2, index % 2);
        index += 1;
        Button::new()
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .grid_row(row)
            .grid_column(column)
            .on_click(context.message(to_message(value.to_string())))
            .content(label)
    });
    StackPanel::new().spacing(8.0).children((
        TextBlock::new().text("Quick presets"),
        Grid::new()
            .row_spacing(8.0)
            .column_spacing(8.0)
            .columns([GridLength::STAR, GridLength::STAR])
            .rows([GridLength::Auto, GridLength::Auto])
            .children(buttons),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_vietnamese() {
        assert_eq!(normalize_vietnamese("Cốc Cốc"), "coc coc");
        assert_eq!(normalize_vietnamese("Bộ gõ tiếng Việt"), "bo go tieng viet");
        assert_eq!(normalize_vietnamese("Trợ giúp & Hỗ trợ"), "tro giup & ho tro");
        assert_eq!(normalize_vietnamese("Đại học"), "dai hoc");
    }

    fn app(name: &str) -> InstalledApp {
        InstalledApp {
            name: name.to_string(),
            path: format!("{name}.exe"),
            is_store_app: false,
        }
    }

    #[test]
    fn filter_apps_should_match_without_diacritics_or_case() {
        let apps = [app("Cốc Cốc"), app("Chrome"), app("Windows Terminal")];
        let names: Vec<&str> = filter_apps(&apps, "COC").map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Cốc Cốc"]);
    }

    #[test]
    fn filter_apps_should_match_diacritic_query() {
        let apps = [app("Coc Coc"), app("Chrome")];
        assert_eq!(filter_apps(&apps, "cốc").count(), 1);
    }
}
