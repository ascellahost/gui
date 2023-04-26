use anyhow::Result;
use eframe::egui::{self, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use egui_file::FileDialog;

use crate::{MyApp, Request, ScreenshotType};

pub fn screen(app: &mut MyApp, ui: &mut Ui, _ctx: &egui::Context) -> Result<()> {
    ui.heading("Settings");
    ui.hyperlink_to("Config Creator", "https://ascella.host/config_wizard/");
    if ui.button("Import Config from file").clicked() {
        let mut dialog = FileDialog::open_file(app.opened_file.clone())
            .resizable(false)
            .show_rename(false)
            .filter(Box::new(|f| {
                f.extension().map_or(false, |ext| ext == "json" || ext == "sxcu")
            }));
        dialog.open();
        app.open_file_dialog = Some(dialog);
    }
    ui.horizontal(|ui| {
        let token_label = ui.label("Ascella Token (Optional) ");
        ui.text_edit_singleline(&mut app.config.api_key)
            .labelled_by(token_label.id);
    });
    ui.horizontal(|ui| {
        let screenshot_label = ui.label("Screenshot tool ");
        egui::ComboBox::from_id_source(screenshot_label.id)
            .selected_text(app.config.s_type.name())
            .width(120.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.config.s_type, ScreenshotType::Flameshot, "Flameshot");
                #[cfg(target_os = "macos")]
                ui.selectable_value(&mut app.config.s_type, ScreenshotType::Screencapture, "Screencapture");
                #[cfg(target_os = "linux")]
                ui.selectable_value(&mut app.config.s_type, ScreenshotType::Scrot, "Scrot");
                #[cfg(target_os = "linux")]
                ui.selectable_value(&mut app.config.s_type, ScreenshotType::Spectacle, "Spectacle");
                ui.selectable_value(
                    &mut app.config.s_type,
                    ScreenshotType::Custom {
                        area: String::new(),
                        screen: String::new(),
                        window: String::new(),
                    },
                    "Custom",
                );
            })
            .response
    });

    if let ScreenshotType::Custom { area, screen, window } = &mut app.config.s_type {
        ui.heading("Custom Config use {file} to set the output dir");

        ui.horizontal(|ui| {
            let token_label = ui.label("Area command ");
            ui.text_edit_singleline(area).labelled_by(token_label.id);
        });
        ui.horizontal(|ui| {
            let token_label = ui.label("Screen command ");
            ui.text_edit_singleline(screen).labelled_by(token_label.id);
        });
        ui.horizontal(|ui| {
            let token_label = ui.label("Window command ");
            ui.text_edit_singleline(window).labelled_by(token_label.id);
        });
    }

    egui::CollapsingHeader::new("Advanced").show(ui, |ui| {
        ui.horizontal(|ui| {
            let url_label = ui.label("Ascella API URL ");
            ui.text_edit_singleline(&mut app.config.api_url)
                .labelled_by(url_label.id);
        });

        ui.horizontal(|ui| ui.checkbox(&mut app.config.debug, "Debug Mode"));
        ui.heading(RichText::new("Headers").size(15.0));

        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(30.0))
            .column(Column::remainder())
            .min_scrolled_height(0.0)
            .body(|mut body| {
                for (name, value) in &mut app.config.headers.iter_mut() {
                    body.row(24.0, |mut row| {
                        row.col(|ui| {
                            ui.label(name);
                        });
                        row.col(|ui| {
                            ui.text_edit_singleline(value);
                        });
                    });
                }
            });
    });

    if ui.button("save").clicked() {
        app.sender.send(Request::SaveConfig(app.config.clone())).ok();
    }
    Ok(())
}
