#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{
    collections::HashMap,
    default, env, fs,
    io::ErrorKind,
    path::PathBuf,
    process::{ExitCode, ExitStatus},
    str::FromStr,
    thread,
};

use bytes::Bytes;
use config::{Config, Environment, File, FileFormat};
use eframe::{
    egui::{
        self, Button, Color32, FontDefinitions, FontFamily, Frame, Layout, Margin, RichText, Rounding, Stroke, Style,
        Window,
    },
    epaint::Vec2,
};
use egui_extras::{Column, TableBuilder};
use egui_file::FileDialog;
use egui_notify::{Toast, Toasts};
use egui_tracing::EventCollector;
use home::home_dir;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    multipart::{self, Part},
    Method, Request as OtherRequest, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use theme::{set_theme, Theme, THEME};
use tokio::{
    process::Command,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer,
};

mod clipboard;
mod easy_mark;
mod theme;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct AscellaConfig {
    #[serde(default)]
    api_url: String,
    #[serde(default)]
    api_key: String,
    #[serde(alias = "RequestURL")]
    request_url: String,
    #[serde(alias = "Headers")]
    headers: HashMap<String, String>,
    debug: bool,
    s_type: ScreenshotType,
}
fn ascella_dir() -> PathBuf {
    home_dir().unwrap().join(".ascella")
}

use anyhow::Result;

use crate::clipboard::copy;

impl AscellaConfig {
    pub async fn save(&self) -> Result<()> {
        let file = ascella_dir().join("ascella.toml");
        tokio::fs::write(file, toml_edit::ser::to_string_pretty(self)?).await?;

        Ok(())
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ScreenshotType {
    #[default]
    Flameshot,
    Spectacle,
    Scrot,
    Screencapture,
    Custom {
        area: String,
        screen: String,
        window: String,
    },
}

enum SendScreenshot {
    Area,
    Screen,
    Window,
}

impl ScreenshotType {
    fn name(&self) -> &'static str {
        match self {
            Self::Flameshot => "flameshot",
            Self::Spectacle => "spectacle",
            Self::Scrot => "scrot",
            Self::Screencapture => "screencapture",
            Self::Custom { .. } => "custom",
        }
    }
    fn generate_file_path() -> String {
        let filename = chrono::offset::Local::now().format("%Y-%m-%d_%H-%M-%S.png").to_string();
        home_dir()
            .unwrap()
            .join(".ascella")
            .join("images")
            .join(filename)
            .to_string_lossy()
            .to_string()
    }
    fn area_command(&self) -> (String, String) {
        let file = Self::generate_file_path();
        (
            file.clone(),
            match self {
                Self::Flameshot => format!("flameshot gui -c -p {}", file),
                Self::Spectacle => format!("spectacle -ar {}", file),
                Self::Scrot => format!("scrot -s {}", file),
                Self::Screencapture => format!("screencapture -R0,0,500,500 {}", file),
                Self::Custom { area, .. } => area.replace("{file}", &file),
            },
        )
    }
    // saves to .ascella/images with a chrono filename
    fn screen_command(&self) -> (String, String) {
        let file = Self::generate_file_path();
        (
            file.clone(),
            match self {
                Self::Flameshot => format!("flameshot gui -p {}", file),
                Self::Spectacle => format!("spectacle -b -p {}", file),
                Self::Scrot => format!("scrot {}", file),
                Self::Screencapture => format!("screencapture -m {}", file),
                Self::Custom { screen, .. } => screen.replace("{file}", &file),
            },
        )
    }
    // saves to .ascella/images with a chrono filename
    fn window_command(&self) -> (String, String) {
        let file = Self::generate_file_path();
        (
            file.clone(),
            match self {
                Self::Flameshot => format!("flameshot gui -p {} -w", file),
                Self::Spectacle => format!("spectacle -b -p {} -w", file),
                Self::Scrot => format!("scrot -u {}", file),
                Self::Screencapture => {
                    format!("screencapture -mw {}", file)
                }
                Self::Custom { window, .. } => window.replace("{file}", &file),
            },
        )
    }

    fn cmd_from_type(&self, send: SendScreenshot) -> (String, String) {
        match send {
            SendScreenshot::Area => self.area_command(),
            SendScreenshot::Window => self.window_command(),
            SendScreenshot::Screen => self.screen_command(),
        }
    }
}

enum RequestResponse {
    Request {
        content: Bytes,
        status: StatusCode,
        r_type: RequestType,
    },
    Toast(Toast),
}

#[allow(clippy::enum_variant_names)]
enum Request {
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
enum RequestType {
    RetrieveUser,
}

fn headermap_from_hashmap<'a, I, S>(headers: I) -> HeaderMap
where
    I: Iterator<Item = (S, S)> + 'a,
    S: AsRef<str> + 'a,
{
    headers
        .map(|(name, val)| (HeaderName::from_str(name.as_ref()), HeaderValue::from_str(val.as_ref())))
        // We ignore the errors here. If you want to get a list of failed conversions, you can use Iterator::partition
        // to help you out here
        .filter(|(k, v)| k.is_ok() && v.is_ok())
        .map(|(k, v)| (k.unwrap(), v.unwrap()))
        .collect()
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
struct UploadResponse {
    url: String,
    delete: String,
    metadata: String,
}

fn main() -> Result<(), eframe::Error> {
    let collector: EventCollector = egui_tracing::EventCollector::default();
    tracing_subscriber::registry()
        .with(EventFilter(collector.clone()))
        .init();

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
                .unwrap();
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_name("ascella-async")
                .max_blocking_threads(10)
                .build()
                .unwrap()
                .block_on(async {
                    while let Some(data) = receiver.recv().await {
                        match data {
                            Request::DoRequest { request, r_type } => {
                                let res = client.execute(request).await.unwrap();

                                sender_1
                                    .send(RequestResponse::Request {
                                        status: res.status(),
                                        content: res.bytes().await.unwrap(),
                                        r_type,
                                    })
                                    .ok();
                            }
                            Request::Screenshot { r_type, send, config } => {
                                let cmd = r_type.cmd_from_type(send);
                                let mut args = cmd.1.split_whitespace();

                                fn file_to_body(file: tokio::fs::File) -> reqwest::Body {
                                    let stream =
                                        tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());

                                    reqwest::Body::wrap_stream(stream)
                                }

                                let command = match Command::new(args.next().unwrap()).args(args).output().await {
                                    Ok(r) => r,
                                    Err(e) => {
                                        let msg = if e.kind() == ErrorKind::NotFound  {
                                            format!(
                                                "{} is not installed\nplease install it and make sure its added to your path",
                                                r_type.name()
                                            )
                                        } else {
                                            format!(
                                                "Failed executing screenshot command\n{:?}",
                                                e
                                            )
                                        };
                                        tracing::error!("Error starting screenshot process {e:?}, {}", cmd.1);
                                         sender_1
                                            .send(RequestResponse::Toast(Toast::error(msg)))
                                            .ok();

                                        continue;
                                    }
                                };
                                if !command.status.success() {
                                    sender_1
                                        .send(RequestResponse::Toast(Toast::error(format!(
                                            "Failed executing screenshot command\n{command:?}",
                                        ))))
                                        .ok();
                                    tracing::error!("Error executing screenshot command {command:?}");
                                    continue;
                                }

                                let file = file_to_body(tokio::fs::File::open(&cmd.0).await.unwrap());
                                let form = multipart::Form::new().part(
                                    "file",
                                    Part::stream(file)
                                        .file_name(
                                            PathBuf::from(cmd.0).file_name().unwrap().to_string_lossy().to_string(),
                                        )
                                        .mime_str("image/png")
                                        .unwrap(),
                                );

                                let mut headers = headermap_from_hashmap(config.headers.iter());
                                if !config.api_key.is_empty() {
                                    headers.insert("ascella-token", HeaderValue::from_str(&config.api_key).unwrap());
                                }

                                let res = client
                                    .post(config.request_url)
                                    // .post("http://127.0.0.1:8787/api/v3/upload")
                                    .headers(headers)
                                    .multipart(form)
                                    .send()
                                    .await
                                    .unwrap()
                                    .text()
                                    .await
                                    .unwrap();
                                tracing::debug!("Image uploaded {}", res);
                                let response: UploadResponse = serde_json::from_str(&res).unwrap();
                                copy(response.url.clone()).await;
                                sender_1
                                        .send(RequestResponse::Toast(Toast::success(format!(
                                            "Image uploaded {}",
                                            response.url
                                        ))))
                                        .ok();
                            },
                            Request::SaveConfig(config) => config.save().await.unwrap()
                        }
                    }
                });
        })
        .ok();

    let config: AscellaConfig = Config::builder()
        .add_source(File::new("ascella.toml", FileFormat::Toml).required(false))
        .add_source(File::new("ascella.json", FileFormat::Json5).required(false))
        .add_source(
            File::new(
                home::home_dir()
                    .expect("Failed Fetching home dir!")
                    .join(".ascella")
                    .join("ascella.toml")
                    .to_str()
                    .unwrap(),
                FileFormat::Toml,
            )
            .required(false),
        )
        .add_source(
            File::new(
                home::home_dir()
                    .expect("Failed Fetching home dir!")
                    .join(".ascella")
                    .join("ascella.json")
                    .to_str()
                    .unwrap(),
                FileFormat::Json5,
            )
            .required(false),
        )
        .add_source(Environment::default())
        .set_default("api_url", "https://api.ascella.host/api/v3")
        .unwrap()
        .set_default("request_url", "https://api.ascella.host/api/v3/upload")
        .unwrap()
        .set_default("api_key", "")
        .unwrap()
        .set_default("debug", false)
        .unwrap()
        .set_default("headers", HashMap::<String, String>::default())
        .unwrap()
        .set_default(
            "s_type",
            serde_json::from_str::<config::Value>(&serde_json::to_string(&ScreenshotType::Flameshot).unwrap()).unwrap(),
        )
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();

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
}
struct MyApp {
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
struct AscellaUser {
    id: i32,
    name: String,
    email: String,
    token: String,
    uuid: String,
    upload_limit: i64,
}

#[derive(Deserialize)]
struct AscellaUserEndpointResult<T> {
    status: u16,
    message: String,
    success: bool,
    data: T,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        set_theme(ctx, THEME);
        // ctx.set_pixels_per_point(1.25);
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

        let send_save = |config: AscellaConfig| self.sender.send(Request::SaveConfig(config));

        egui::CentralPanel::default().show(ctx, |ui| {
            //self.collector.clone()
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
                Menu::Home => {
                    ui.heading("Home");
                    if self.config.api_key.is_empty() {
                        ui.small("No access key set gallery will be local only!");
                    }
                    if let Some(data) = &self.user {
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
                                        self.sender
                                            .send(Request::Screenshot {
                                                r_type: self.config.s_type.clone(),
                                                send: SendScreenshot::Area,
                                                config: self.config.clone(),
                                            })
                                            .ok();
                                    }
                                });
                                columns[1].vertical_centered(|ui| {
                                    if ui.button("Screenshot Window").clicked() {
                                        self.sender
                                            .send(Request::Screenshot {
                                                r_type: self.config.s_type.clone(),
                                                send: SendScreenshot::Window,
                                                config: self.config.clone(),
                                            })
                                            .ok();
                                    }
                                });
                                columns[2].vertical_centered(|ui| {
                                    if ui.button("Screenshot Screen").clicked() {
                                        self.sender
                                            .send(Request::Screenshot {
                                                r_type: self.config.s_type.clone(),
                                                send: SendScreenshot::Screen,
                                                config: self.config.clone(),
                                            })
                                            .ok();
                                    }
                                });
                            })
                        });
                }
                Menu::About => easy_mark::easy_mark(ui, include_str!("../about.md")),

                Menu::Screenshots => {
                    ui.heading("Screenshots");
                }
                Menu::Settings => {
                    ui.heading("Settings");
                    ui.hyperlink_to("Config Creator", "https://ascella.host/config_wizard/");
                    if ui.button("Import Config from file").clicked() {
                        let mut dialog = FileDialog::open_file(self.opened_file.clone())
                            .resizable(false)
                            .show_rename(false)
                            .filter(Box::new(|f| {
                                f.extension().map_or(false, |ext| ext == "json" || ext == "sxcu")
                            }));
                        dialog.open();
                        self.open_file_dialog = Some(dialog);
                    }
                    ui.horizontal(|ui| {
                        let token_label = ui.label("Ascella Token (Optional) ");
                        ui.text_edit_singleline(&mut self.config.api_key)
                            .labelled_by(token_label.id);
                    });
                    ui.horizontal(|ui| {
                        let screenshot_label = ui.label("Screenshot tool ");
                        egui::ComboBox::from_id_source(screenshot_label.id)
                            .selected_text(self.config.s_type.name())
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.config.s_type, ScreenshotType::Flameshot, "Flameshot");
                                ui.selectable_value(
                                    &mut self.config.s_type,
                                    ScreenshotType::Screencapture,
                                    "Screencapture",
                                );
                                ui.selectable_value(&mut self.config.s_type, ScreenshotType::Scrot, "Scrot");
                                ui.selectable_value(&mut self.config.s_type, ScreenshotType::Spectacle, "Spectacle");
                            })
                            .response
                    });
                    egui::CollapsingHeader::new("Advanced").show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let url_label = ui.label("Ascella API URL ");
                            ui.text_edit_singleline(&mut self.config.api_url)
                                .labelled_by(url_label.id);
                        });

                        ui.horizontal(|ui| ui.checkbox(&mut self.config.debug, "Debug Mode"));
                        ui.heading(RichText::new("Headers").size(15.0));

                        TableBuilder::new(ui)
                            .striped(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::initial(30.0))
                            .column(Column::remainder())
                            .min_scrolled_height(0.0)
                            .body(|mut body| {
                                for (name, value) in &mut self.config.headers.iter_mut() {
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
                        send_save(self.config.clone()).ok();
                    }
                }
            }
            if let Some(dialog) = &mut self.open_file_dialog {
                if dialog.show(ctx).selected() {
                    if let Some(file) = dialog.path() {
                        self.opened_file = Some(file.clone());
                        let raw: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();

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

                        send_save(self.config.clone()).ok();
                    }
                }
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
