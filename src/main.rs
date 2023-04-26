#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{collections::HashMap, env, fs, path::PathBuf, thread};

use anyhow::{anyhow, Result};
use ascella_config::AscellaConfig;
use bytes::Bytes;
use config::{Config, Environment, File, FileFormat};
use eframe::{
    egui::{self, Button, Color32, Frame, Margin, RichText, Rounding, Window},
    epaint::Vec2,
};
use egui_file::FileDialog;
use egui_notify::{Toast, Toasts};
use egui_tracing::EventCollector;

use request_handler::handle_event;
use reqwest::{header::HeaderValue, Method, StatusCode};
use screenshots::ScreenshotType;
use serde::Deserialize;
use serde_json::Value;
use theme::{set_theme, THEME};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer,
};
use utils::ascella_dir;

mod ascella_config;
mod clipboard;
mod easy_mark;
mod request_handler;
mod screens;
mod screenshots;
mod theme;
mod utils;

pub enum RequestResponse {
    Request {
        content: Bytes,
        status: StatusCode,
        r_type: RequestType,
    },
    Toast(Toast),
}

pub enum SendScreenshot {
    Area,
    Screen,
    Window,
}

#[allow(clippy::enum_variant_names)]
pub enum Request {
    DoRequest {
        r_type: RequestType,
        request: reqwest::Request,
    },
    Screenshot {
        r_type: ScreenshotType,
        send: SendScreenshot,
        config: AscellaConfig,
    },
    SaveConfig(AscellaConfig),
}

#[derive(Clone)]
pub enum RequestType {
    RetrieveUser,
}

pub struct EventFilter(egui_tracing::EventCollector);

impl<S> Layer<S> for EventFilter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        if meta.level() == &Level::TRACE {
            return;
        }
        self.0.on_event(event, _ctx)
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct UploadResponse {
    url: String,
    delete: String,
    metadata: String,
}

fn main() -> Result<()> {
    let collector: EventCollector = egui_tracing::EventCollector::default();
    tracing_subscriber::registry()
        .with(EventFilter(collector.clone()))
        .init();

    fs::create_dir_all(ascella_dir())?;

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Request>();
    let (sender_1, receiver_1) = tokio::sync::mpsc::unbounded_channel::<RequestResponse>();

    // let (sender_1, receiver_1) = tokio::sync::broadcast::channel::<RequestResponse>(200);
    thread::Builder::new()
        .name("ascella-async".to_owned())
        .spawn(move || {
            let client = reqwest::Client::builder()
                .user_agent(format!(
                    "Ascella-uploader/{} ({})",
                    env!("CARGO_PKG_VERSION"),
                    env::consts::OS
                ))
                .build()
                .expect("Reqwest client did not built");
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_name("ascella-async")
                .max_blocking_threads(10)
                .build()
                .expect("How did we get here?")
                .block_on(async {
                    while let Some(data) = receiver.recv().await {
                        if let Err(e) = handle_event(data, &client, &sender_1).await {
                            tracing::error!("{e:?}");
                        };
                    }
                });
        })
        .ok();

    let config: AscellaConfig = Config::builder()
        .add_source(File::new("ascella.toml", FileFormat::Toml).required(false))
        .add_source(File::new("ascella.json", FileFormat::Json5).required(false))
        .add_source(File::new(ascella_dir().join("ascella.toml").to_str().unwrap(), FileFormat::Toml).required(false))
        .add_source(File::new(ascella_dir().join("ascella.json").to_str().unwrap(), FileFormat::Json5).required(false))
        .add_source(Environment::default())
        .set_default("api_url", "https://api.ascella.host/api/v3")?
        .set_default("request_url", "https://api.ascella.host/api/v3/upload")?
        .set_default("api_key", "")?
        .set_default("debug", false)?
        .set_default("headers", HashMap::<String, String>::default())?
        .set_default(
            "s_type",
            toml::from_str::<config::Value>(&toml::to_string(&ScreenshotType::Flameshot)?)?,
        )?
        .build()?
        .try_deserialize()?;

    let options = eframe::NativeOptions {
        min_window_size: Some(egui::vec2(620.0, 600.0)),
        initial_window_size: Some(egui::vec2(620.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Ascella GUI",
        options,
        Box::new(|_cc| Box::new(MyApp::new(config, sender, receiver_1, collector))),
    )
    .map_err(|e| anyhow!("{e}"))?;
    Ok(())
}
pub struct MyApp {
    menu: Menu,
    config: AscellaConfig,

    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,

    sender: UnboundedSender<Request>,
    receiver: UnboundedReceiver<RequestResponse>,

    user: Option<AscellaUser>,
    retrieving_user: bool,

    collector: EventCollector,

    toasts: Toasts,
}
#[derive(PartialEq, Default, Debug)]
enum Menu {
    #[default]
    Home,
    Settings,
    About,
    Screenshots,
}

impl MyApp {
    fn new(
        config: AscellaConfig,
        sender: UnboundedSender<Request>,
        receiver: UnboundedReceiver<RequestResponse>,
        collector: EventCollector,
    ) -> Self {
        Self {
            menu: if config.api_key.is_empty() {
                Menu::Settings
            } else {
                Menu::Home
            },
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
        }
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AscellaUser {
    id: i32,
    name: String,
    email: String,
    token: String,
    uuid: String,
    upload_limit: i64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AscellaUserEndpointResult<T> {
    status: u16,
    message: String,
    success: bool,
    data: T,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        set_theme(ctx, THEME);

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
                Menu::Screenshots => {
                    ui.heading("Screenshots");
                    ui.label("W.I.P");
                    ui.small("Check back later for screenshots!");
                }
                Menu::Settings => screens::settings::screen(self, ui, ctx).unwrap(),
            }
        });
        egui::TopBottomPanel::bottom("bottom_nav")
            .show_separator_line(false)
            .show(ctx, |ui| {
                fn btn(text: &str, active: bool) -> Button {
                    Button::new(text)
                        .min_size(Vec2::from((95.0, 30.0)))
                        .fill(if active { THEME.primary } else { THEME.neutral })
                        .rounding(3.0)
                }
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        Frame::none()
                            .fill(THEME.base_200)
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
                                                        ui.visuals_mut().override_text_color = Some(THEME.text_accent);
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
                                        (3, "Screenshots", Menu::Screenshots)
                                    }
                                });
                            });
                    },
                );
            });
        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    self.opened_file = Some(file.clone());
                    let raw: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();

                    self.config.headers = serde_json::from_value(raw["Headers"].clone()).unwrap();

                    self.config.request_url = raw["RequestURL"].as_str().unwrap().to_owned();

                    if let Some(token) = self.config.headers.remove("ascella-token") {
                        self.config.api_key = token.clone();
                        let mut req =
                            reqwest::Request::new(Method::GET, format!("{}/me", self.config.api_url).parse().unwrap());
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
            },
            Ok(RequestResponse::Toast(toast)) => {
                self.toasts.add(toast);
            }
            _ => {}
        }
    }
}
