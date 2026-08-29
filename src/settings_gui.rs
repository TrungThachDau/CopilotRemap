use crate::app_scanner::{self, InstalledApp};
use crate::config::{
    ActionType, AppConfig, CustomCommandConfig, LaunchAppConfig, OpenUrlConfig, SendKeysConfig,
};
use crate::executor;
use crate::registry_helper;
use eframe::egui;

fn setup_system_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Load Segoe UI from Windows Fonts for full Vietnamese & Unicode support
    if let Ok(font_data) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts.font_data.insert(
            "segoe_ui".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(font_data)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe_ui".to_owned());
    }

    if let Ok(font_bold) = std::fs::read(r"C:\Windows\Fonts\segoeuib.ttf") {
        fonts.font_data.insert(
            "segoe_ui_bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(font_bold)),
        );
    }

    if let Ok(font_mono) = std::fs::read(r"C:\Windows\Fonts\consola.ttf") {
        fonts.font_data.insert(
            "consolas".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(font_mono)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "consolas".to_owned());
    }

    ctx.set_fonts(fonts);
}

pub fn run_settings_window() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 560.0])
            .with_min_inner_size([520.0, 480.0])
            .with_title("Copilot Key Remapper - Settings"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Copilot Key Remapper - Settings",
        options,
        Box::new(|cc| {
            // Load Windows Segoe UI font
            setup_system_fonts(&cc.egui_ctx);
            // Light theme as requested
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(SettingsApp::new()))
        }),
    );
}

struct SettingsApp {
    action_type: ActionType,

    // App controls
    installed_apps: Vec<InstalledApp>,
    selected_app_idx: Option<usize>,
    app_search_filter: String,
    app_path: String,
    app_args: String,
    app_dir: String,

    // URL controls
    url: String,

    // Keys controls
    keys: String,

    // Command controls
    cmd_command: String,
    cmd_arguments: String,
    cmd_run_hidden: bool,

    // Status notification
    status_notification: Option<(String, bool)>, // (message, is_error)
}

impl SettingsApp {
    fn new() -> Self {
        let config = AppConfig::load();
        let installed_apps = app_scanner::get_installed_apps();

        let mut selected_app_idx = None;
        if !config.launch_app.path.is_empty() {
            for (idx, app) in installed_apps.iter().enumerate() {
                if app.path.eq_ignore_ascii_case(&config.launch_app.path)
                    || app.name.eq_ignore_ascii_case(&config.launch_app.path)
                {
                    selected_app_idx = Some(idx);
                    break;
                }
            }
        }

        Self {
            action_type: config.action_type,
            installed_apps,
            selected_app_idx,
            app_search_filter: String::new(),
            app_path: config.launch_app.path,
            app_args: config.launch_app.arguments,
            app_dir: config.launch_app.working_dir,
            url: config.open_url.url,
            keys: config.send_keys.keys.join(", "),
            cmd_command: config.custom_command.command,
            cmd_arguments: config.custom_command.arguments,
            cmd_run_hidden: config.custom_command.run_hidden,
            status_notification: None,
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

    fn save_and_close(&mut self, ctx: &egui::Context) {
        let config = self.to_config();
        match config.save() {
            Ok(()) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => {
                self.status_notification =
                    Some((format!("Failed to save settings: {}", e), true));
            }
        }
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keyboard quick-jump when not currently editing a text box
        if self.action_type == ActionType::LaunchApp && !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Text(t) = event {
                        for c in t.chars() {
                            if c.is_alphanumeric() {
                                let target = c.to_ascii_lowercase();
                                if let Some((idx, app)) = self
                                    .installed_apps
                                    .iter()
                                    .enumerate()
                                    .find(|(_, a)| a.name.to_lowercase().starts_with(target))
                                {
                                    self.selected_app_idx = Some(idx);
                                    self.app_path = app.path.clone();
                                }
                                break;
                            }
                        }
                    }
                }
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            // Header Title
            ui.horizontal(|ui| {
                ui.heading("⚡ Copilot Key Remapper");
            });
            ui.label("Configure what happens when the Copilot hardware key is pressed.");
            ui.separator();

            // 1. Action Mode Selector
            ui.label(egui::RichText::new("Action Type:").strong());
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(
                    &mut self.action_type,
                    ActionType::LaunchApp,
                    "🚀 Launch Application",
                );
                ui.selectable_value(
                    &mut self.action_type,
                    ActionType::OpenUrl,
                    "🌐 Open Website",
                );
                ui.selectable_value(
                    &mut self.action_type,
                    ActionType::SendKeys,
                    "⌨ Hotkey Shortcut",
                );
                ui.selectable_value(
                    &mut self.action_type,
                    ActionType::CustomCommand,
                    "📜 Shell Command",
                );
            });

            ui.add_space(4.0);

            // 2. Configuration Details Box
            egui::Frame::group(ui.style())
                .inner_margin(12.0)
                .show(ui, |ui| {
                    match self.action_type {
                        ActionType::LaunchApp => {
                            ui.label(egui::RichText::new("Application Settings").strong());
                            ui.add_space(4.0);

                            // Installed apps search and filter
                            ui.label("Search or select installed application:");
                            ui.horizontal(|ui| {
                                let search_edit = ui.add(
                                    egui::TextEdit::singleline(&mut self.app_search_filter)
                                        .hint_text("🔍 Filter application name (e.g. Claude, Terminal, Chrome)...")
                                        .desired_width(ui.available_width() - 75.0),
                                );

                                if search_edit.changed()
                                    && !self.app_search_filter.trim().is_empty()
                                    && let filter = self.app_search_filter.trim().to_lowercase()
                                    && let Some((orig_idx, app)) = self
                                        .installed_apps
                                        .iter()
                                        .enumerate()
                                        .find(|(_, a)| a.name.to_lowercase().contains(&filter))
                                {
                                    self.selected_app_idx = Some(orig_idx);
                                    self.app_path = app.path.clone();
                                }

                                if !self.app_search_filter.is_empty()
                                    && ui.button("✖ Clear").clicked()
                                {
                                    self.app_search_filter.clear();
                                }
                            });

                            let filter = self.app_search_filter.trim().to_lowercase();
                            let filtered_apps: Vec<(usize, &InstalledApp)> = self
                                .installed_apps
                                .iter()
                                .enumerate()
                                .filter(|(_, a)| filter.is_empty() || a.name.to_lowercase().contains(&filter))
                                .collect();

                            let selected_text = self
                                .selected_app_idx
                                .and_then(|i| self.installed_apps.get(i))
                                .map(|a| a.name.as_str())
                                .unwrap_or("-- Choose from installed applications --");

                            egui::ComboBox::from_id_salt("installed_apps_combo")
                                .width(ui.available_width() - 10.0)
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    egui::ScrollArea::vertical()
                                        .max_height(240.0)
                                        .show(ui, |ui| {
                                            if filtered_apps.is_empty() {
                                                ui.label(
                                                    egui::RichText::new("No applications match your filter")
                                                        .italics()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            } else {
                                                for (orig_idx, app) in &filtered_apps {
                                                    let is_selected = self.selected_app_idx == Some(*orig_idx);
                                                    if ui.selectable_label(is_selected, &app.name).clicked() {
                                                        self.selected_app_idx = Some(*orig_idx);
                                                        self.app_path = app.path.clone();
                                                    }
                                                }
                                            }
                                        });
                                });

                            ui.add_space(4.0);
                            ui.label("Application path / executable:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.app_path)
                                        .desired_width(ui.available_width() - 85.0),
                                );
                                if ui.button("Browse...").clicked()
                                    && let Some(file) = rfd::FileDialog::new()
                                        .add_filter(
                                            "Executable / Shortcut",
                                            &["exe", "lnk", "bat", "cmd"],
                                        )
                                        .add_filter("All Files", &["*"])
                                        .pick_file()
                                {
                                    self.app_path = file.display().to_string();
                                }
                            });

                            ui.add_space(4.0);
                            ui.columns(2, |columns| {
                                columns[0].label("Arguments (optional):");
                                columns[0].add(
                                    egui::TextEdit::singleline(&mut self.app_args)
                                        .desired_width(columns[0].available_width()),
                                );

                                columns[1].label("Working directory (optional):");
                                columns[1].add(
                                    egui::TextEdit::singleline(&mut self.app_dir)
                                        .desired_width(columns[1].available_width()),
                                );
                            });
                        }

                        ActionType::OpenUrl => {
                            ui.label(egui::RichText::new("Website Settings").strong());
                            ui.add_space(4.0);

                            ui.label("Quick Presets:");
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("🤖 ChatGPT").clicked() {
                                    self.url = "https://chatgpt.com".to_string();
                                }
                                if ui.button("🧠 Claude AI").clicked() {
                                    self.url = "https://claude.ai".to_string();
                                }
                                if ui.button("✨ Google Gemini").clicked() {
                                    self.url = "https://gemini.google.com".to_string();
                                }
                                if ui.button("🔍 Perplexity").clicked() {
                                    self.url = "https://www.perplexity.ai".to_string();
                                }
                            });

                            ui.add_space(4.0);
                            ui.label("Target Website URL:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.url)
                                    .desired_width(ui.available_width() - 10.0),
                            );
                        }

                        ActionType::SendKeys => {
                            ui.label(egui::RichText::new("Hotkey Shortcut Settings").strong());
                            ui.add_space(4.0);

                            ui.label("Quick Presets:");
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("⚡ PowerToys Run (Alt+Space)").clicked() {
                                    self.keys = "Alt, Space".to_string();
                                }
                                if ui.button("✂ Snipping Tool (Win+Shift+S)").clicked() {
                                    self.keys = "Win, Shift, S".to_string();
                                }
                                if ui.button("⚙ Task Manager (Ctrl+Shift+Esc)").clicked() {
                                    self.keys = "Ctrl, Shift, Esc".to_string();
                                }
                                if ui.button("🗔 Task View (Win+Tab)").clicked() {
                                    self.keys = "Win, Tab".to_string();
                                }
                            });

                            ui.add_space(4.0);
                            ui.label("Shortcut keys (comma-separated, e.g. Alt, Space):");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.keys)
                                    .desired_width(ui.available_width() - 10.0),
                            );
                        }

                        ActionType::CustomCommand => {
                            ui.label(egui::RichText::new("Custom Shell Command Settings").strong());
                            ui.add_space(4.0);

                            ui.label("Command or Executable:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.cmd_command)
                                    .desired_width(ui.available_width() - 10.0),
                            );

                            ui.add_space(4.0);
                            ui.label("Arguments (optional):");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.cmd_arguments)
                                    .desired_width(ui.available_width() - 10.0),
                            );

                            ui.add_space(4.0);
                            ui.checkbox(
                                &mut self.cmd_run_hidden,
                                "Run in background silently (hide console window)",
                            );
                        }
                    }
                });

            // 3. Status Notification / Info Banner
            if let Some((msg, is_error)) = &self.status_notification {
                let color = if *is_error {
                    egui::Color32::from_rgb(180, 40, 40)
                } else {
                    egui::Color32::from_rgb(40, 140, 40)
                };
                ui.colored_label(color, msg);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                // Bottom Button Bar
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("⚙ Windows Settings...").clicked() {
                        registry_helper::open_windows_copilot_settings();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("💾 Save Settings").strong(),
                                )
                                .min_size(egui::vec2(120.0, 28.0)),
                            )
                            .clicked()
                        {
                            self.save_and_close(ctx);
                        }

                        if ui
                            .add(
                                egui::Button::new("▶ Test Action")
                                    .min_size(egui::vec2(100.0, 28.0)),
                            )
                            .clicked()
                        {
                            let config = self.to_config();
                            executor::execute_action(&config);
                            self.status_notification =
                                Some(("Action executed for test.".to_string(), false));
                        }
                    });
                });

                ui.separator();

                // Provider Status check
                let status = registry_helper::get_copilot_key_status();
                ui.horizontal(|ui| {
                    if status.is_copilot_remap_active {
                        ui.label(
                            egui::RichText::new("✓ Copilot Key Provider is active")
                                .color(egui::Color32::from_rgb(30, 130, 30)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "⚠ CopilotRemap is not currently selected as Windows Copilot Provider",
                            )
                            .color(egui::Color32::from_rgb(180, 100, 20)),
                        );
                    }
                });
            });
        });
    }
}
