use std::{fs, path::PathBuf, time::Duration};

use eframe::{
    egui::{self, Button, Frame, Margin, RichText, Rounding, Window},
    epaint::Vec2,
};
use egui_file::FileDialog;
use egui_notify::Toasts;
use egui_tracing::EventCollector;
use reqwest::{header::HeaderValue, Method};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    ascella_config::AscellaConfig,
    easy_mark,
    screens::{self, history::AscellaFile},
    theme::{set_theme, Theme},
    utils::theme_number_to_theme,
    Request, RequestResponse, RequestType,
};

#[derive(PartialEq, Default, Debug)]
pub enum Menu {
    #[default]
    Home,
    Settings,
    About,
    History,
}

pub struct MyApp {
    pub menu: Menu,
    pub config: AscellaConfig,
    pub opened_file: Option<PathBuf>,
    pub open_file_dialog: Option<FileDialog>,
    pub theme: Theme,

    pub sender: UnboundedSender<Request>,
    pub receiver: UnboundedReceiver<RequestResponse>,

    pub user: Option<AscellaUser>,
    pub retrieving_user: bool,

    pub collector: EventCollector,

    pub toasts: Toasts,

    pub history: Vec<AscellaFile>,
    pub history_index: u64,
}

impl MyApp {
    pub fn new(
        config: AscellaConfig,
        sender: UnboundedSender<Request>,
        receiver: UnboundedReceiver<RequestResponse>,
        collector: EventCollector,
    ) -> Self {
        Self {
            menu: Menu::Home,
            theme: theme_number_to_theme(config.theme),
            config,
            sender,
            receiver,
            open_file_dialog: None,
            opened_file: None,
            user: None,
            collector,
            retrieving_user: false,
            toasts: Toasts::default()
                .with_padding(Vec2::from((5.0, 5.0)))
                .with_margin(Vec2::from((2.0, 2.0)))
                .with_anchor(egui_notify::Anchor::TopLeft),
            history: Vec::new(),
            history_index: 0,
        }
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct AscellaUser {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub token: String,
    pub uuid: String,
    pub upload_limit: i64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AscellaUserEndpointResult<T> {
    pub status: u16,
    pub message: String,
    pub success: bool,
    pub data: T,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.theme = theme_number_to_theme(self.config.theme);
        let theme = self.theme;
        set_theme(ctx, theme);

        ctx.set_pixels_per_point(1.25);

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "open-sans".to_string(),
            egui::FontData::from_static(include_bytes!("./OpenSans-Regular.ttf")),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "open-sans".to_string());
        ctx.set_fonts(fonts);

        self.toasts.show(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.config.debug {
                Window::new(RichText::new("Logs").strong().small())
                    .resizable(true)
                    .collapsible(true)
                    .default_open(false)
                    .constrain(true)
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.add(egui_tracing::Logs::new(self.collector.clone()));
                    });
            }

            match self.menu {
                Menu::Home => screens::home::screen(self, ui, ctx).unwrap(),
                Menu::About => easy_mark::easy_mark(ui, include_str!("../ABOUT.md")),
                Menu::History => screens::history::screen(self, ui, ctx).unwrap(),
                Menu::Settings => screens::settings::screen(self, ui, ctx).unwrap(),
            }
        });
        egui::TopBottomPanel::bottom("bottom_nav")
            .show_separator_line(false)
            .show(ctx, |ui| {
                let btn = |text: &str, active: bool| {
                    Button::new(text)
                        .min_size(Vec2::from((95.0, 30.0)))
                        .fill(if active { theme.primary } else { theme.neutral })
                        .rounding(3.0)
                };
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        Frame::none()
                            .fill(theme.base_200)
                            .inner_margin(Margin::symmetric(30.0, 10.0))
                            .rounding(Rounding {
                                nw: 20.0,
                                ne: 20.0,
                                sw: 0.0,
                                se: 0.0,
                            })
                            .show(ui, |ui| {
                                ui.columns(4, |columns| {
                                    macro_rules! add_columns {
                                        ( $( ($index:expr, $label:expr, $menu:expr) ),* ) => {
                                            $(
                                                columns[$index].vertical_centered(|ui| {
                                                    let active =self.menu == $menu;
                                                    if active {
                                                        ui.visuals_mut().override_text_color = Some(theme.text_accent);
                                                    }
                                                    if ui.add(btn($label, active)).clicked() {
                                                        self.menu = $menu;
                                                    }
                                                });
                                            )*
                                        };
                                    }
                                    add_columns! {
                                        (0, "Home", Menu::Home),
                                        (1, "Settings", Menu::Settings),
                                        (2, "About", Menu::About),
                                        (3, "History", Menu::History)
                                    }
                                });
                            });
                    },
                );
            });

        macro_rules! i_hate_borrow_checker {
            ($data:expr) => {
                let raw: Value = serde_json::from_slice($data).unwrap();

                self.config.headers = serde_json::from_value(raw["Headers"].clone()).unwrap();

                self.config.request_url = raw["RequestURL"].as_str().unwrap().to_owned();

                if let Some(token) = self.config.headers.remove("ascella-token") {
                    self.config.api_key = token.clone();
                    let mut req = reqwest::Request::new(
                        Method::GET,
                        format!("{}/me", self.config.api_url).parse().unwrap(),
                    );
                    req.headers_mut()
                        .append("ascella-token", HeaderValue::from_str(&token).unwrap());

                    self.sender
                        .send(Request::DoRequest {
                            r_type: RequestType::RetrieveUser,
                            request: req,
                        })
                        .ok();
                }
                self.toasts.info("Updated Config");

                self.sender.send(Request::SaveConfig(self.config.clone())).ok();
            };
        }

        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    self.opened_file = Some(file.clone());
                    let raw = &fs::read(file).unwrap();
                    i_hate_borrow_checker!(raw);
                }
            }
        }

        if !self.retrieving_user && self.user.is_none() {
            let mut req = reqwest::Request::new(Method::GET, format!("{}/me", self.config.api_url).parse().unwrap());
            req.headers_mut()
                .append("ascella-token", HeaderValue::from_str(&self.config.api_key).unwrap());

            self.sender
                .send(Request::DoRequest {
                    r_type: RequestType::RetrieveUser,
                    request: req,
                })
                .ok();
            self.retrieving_user = true;
        }

        match self.receiver.try_recv() {
            Ok(RequestResponse::Request {
                content,
                r_type,
                status,
            }) => match r_type {
                RequestType::RetrieveUser => {
                    if status.is_success() {
                        let data: AscellaUserEndpointResult<AscellaUser> = serde_json::from_slice(&content).unwrap();
                        self.user = Some(data.data);
                        self.toasts.success("Received user info");
                    } else {
                        self.toasts
                            .error(format!("Failed receiving user from token {}", status,));
                    }
                }
                RequestType::RequestPage => {
                    if status.is_success() {
                        let data: AscellaUserEndpointResult<Vec<AscellaFile>> =
                            serde_json::from_slice(&content).unwrap();
                        if data.data.is_empty() {
                            self.toasts
                                .error("Reached End....")
                                .set_duration(Some(Duration::from_secs(1)));
                            self.history_index -= 1;
                        } else {
                            self.toasts
                                .success("Files loaded....")
                                .set_duration(Some(Duration::from_secs(1)));
                        }
                        self.history.extend(data.data);
                    } else {
                        self.toasts
                            .error(format!("Failed receiving history from token {}", status,));
                    }
                }
            },
            Ok(RequestResponse::Toast(toast)) => {
                self.toasts.add(toast);
            }
            Ok(RequestResponse::UpdateConfigFromStringSxcu(data)) => {
                i_hate_borrow_checker!(&data);
            }
            _ => {}
        }
    }
}
