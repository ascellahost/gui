use anyhow::Result;
use eframe::egui::{self, Frame, Margin, RichText, Ui};
use egui_extras::{Column, TableBuilder};

use crate::{theme::THEME, ui::MyApp, Request, SendScreenshot};

pub fn screen(app: &mut MyApp, ui: &mut Ui, _ctx: &egui::Context) -> Result<()> {
    ui.heading("Home");
    if app.config.api_key.is_empty() {
        ui.small("No access key set gallery will be local only!");
    }
    if let Some(data) = &app.user {
        ui.heading(RichText::new(format!("Welcome {}!", data.name)).size(15.0));
        egui::CollapsingHeader::new("User Info").show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(30.0))
                .column(Column::remainder())
                .min_scrolled_height(0.0)
                .body(|mut body| {
                    macro_rules! create_tables {
                                        ( $(( $name:expr, $value:expr) ),* ) => {
                                            $(
                                                body.row(18.0, |mut f| {
                                                    f.col(|ui| {
                                                        ui.label(stringify!($name));
                                                    });
                                                    f.col(|ui| {
                                                        ui.label(&$value.to_string());
                                                    });
                                                });
                                            )*
                                        };
                                    }
                    create_tables![
                        (name, data.name),
                        (id, data.id),
                        (email, data.email),
                        (uuid, data.uuid),
                        (upload_limit, "2.5mb")
                    ];
                });
        });
    } else {
        ui.small("No user info available");
    }
    Frame::none()
        .fill(THEME.neutral)
        .inner_margin(Margin::symmetric(30.0, 12.0))
        .show(ui, |ui| {
            ui.columns(3, |columns| {
                columns[0].vertical_centered(|ui| {
                    if ui.button("Screenshot Area").clicked() {
                        app.sender
                            .send(Request::Screenshot {
                                r_type: app.config.s_type.clone(),
                                send: SendScreenshot::Area,
                                config: app.config.clone(),
                                print: false,
                            })
                            .ok();
                    }
                });
                columns[1].vertical_centered(|ui| {
                    if ui.button("Screenshot Window").clicked() {
                        app.sender
                            .send(Request::Screenshot {
                                r_type: app.config.s_type.clone(),
                                send: SendScreenshot::Window,
                                config: app.config.clone(),
                                print: false,
                            })
                            .ok();
                    }
                });
                columns[2].vertical_centered(|ui| {
                    if ui.button("Screenshot Screen").clicked() {
                        app.sender
                            .send(Request::Screenshot {
                                r_type: app.config.s_type.clone(),
                                send: SendScreenshot::Screen,
                                config: app.config.clone(),
                                print: false,
                            })
                            .ok();
                    }
                });
            })
        });
    Ok(())
}
