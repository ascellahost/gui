#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{collections::HashMap, env, fs, process, thread, time::Duration};

use anyhow::{anyhow, Result};
use ascella_config::AscellaConfig;
use bytes::Bytes;
use clap::Parser;
use cli::{AscellaCli, Commands};
use config::{Config, Environment, File, FileFormat};
use eframe::egui::{self, Color32};

use egui_notify::Toast;
use egui_tracing::EventCollector;
use request_handler::handle_event;
use reqwest::StatusCode;
use screenshots::ScreenshotType;
use serde::Deserialize;

use tokio::runtime::Runtime;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer,
};
use utils::ascella_dir;
use webserver::start_server;

mod ascella_config;
mod cli;
mod clipboard;
mod easy_mark;
mod request_handler;
mod screens;
mod screenshots;
mod theme;
mod ui;
mod utils;
mod webserver;
pub enum RequestResponse {
    Request {
        content: Bytes,
        status: StatusCode,
        r_type: RequestType,
    },
    Toast(Toast),
    UpdateConfigFromStringSxcu(Vec<u8>),
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
        print: bool,
    },
    SaveConfig(AscellaConfig),
}

#[derive(Clone)]
pub enum RequestType {
    RetrieveUser,
    RequestPage,
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
pub struct UploadResponse {
    url: String,
    delete: String,
    metadata: String,
}

fn create_rt() -> Result<Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .thread_name("ascella-async")
        .max_blocking_threads(10)
        .build()?)
}

fn main() -> Result<()> {
    let arg = AscellaCli::parse();

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
        .set_default("webserver", true)?
        .set_default("theme", 2)?
        .set_default("optimize_png", false)?
        .set_default("optimize_timeout", 100)?
        .set_default("console_logging", false)?
        .set_default("notifications_enabled", true)?
        .set_default(
            "s_type",
            toml::from_str::<config::Value>(&toml::to_string(&ScreenshotType::Flameshot)?)?,
        )?
        .build()?
        .try_deserialize()?;

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "Ascella-uploader/{} ({})",
            env!("CARGO_PKG_VERSION"),
            env::consts::OS
        ))
        .build()
        .expect("Reqwest client did not built");

    // subcommand branch
    if let Some(sub) = arg.command {
        create_rt()?.block_on(async {
            let (sender, _) = tokio::sync::mpsc::unbounded_channel::<RequestResponse>();
            let (delay, send) = match sub {
                Commands::Area { delay } => (delay, SendScreenshot::Area),
                Commands::Window { delay } => (delay, SendScreenshot::Window),
                Commands::Screen { delay } => (delay, SendScreenshot::Screen),
                Commands::Upload { file } => match request_handler::upload_file(file, &config, &client, true).await {
                    Ok(_) => {
                        process::exit(0);
                    }
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
                },
            };

            if let Some(delay) = delay {
                std::thread::sleep(Duration::from_millis(delay))
            }
            let data = Request::Screenshot {
                r_type: config.s_type.clone(),
                print: true,
                send,
                config,
            };
            if let Err(e) = handle_event(data, &client, &sender).await {
                tracing::error!("{e:?}");
            };
        });
        return Ok(());
    }

    let collector: EventCollector = egui_tracing::EventCollector::default();
    if !config.console_logging {
        tracing_subscriber::registry()
            .with(EventFilter(collector.clone()))
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .compact()
            .init();
    }

    fs::create_dir_all(ascella_dir().join("images"))?;

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Request>();
    let (sender_1, receiver_1) = tokio::sync::mpsc::unbounded_channel::<RequestResponse>();
    let webserver = config.webserver;
    thread::Builder::new()
        .name("ascella-async".to_owned())
        .spawn(move || {
            create_rt().expect("How did this happen").block_on(async {
                if webserver {
                    tokio::spawn(start_server(sender_1.clone()));
                }

                while let Some(data) = receiver.recv().await {
                    if let Err(e) = handle_event(data, &client, &sender_1).await {
                        tracing::error!("{e:?}");
                    };
                }
            });
        })
        .ok();

    let options = eframe::NativeOptions {
        min_window_size: Some(egui::vec2(620.0, 600.0)),
        initial_window_size: Some(egui::vec2(620.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Ascella GUI",
        options,
        Box::new(|_cc| Box::new(ui::MyApp::new(config, sender, receiver_1, collector))),
    )
    .map_err(|e| anyhow!("{e}"))?;
    Ok(())
}
