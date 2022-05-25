use std::fmt::Display;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use crate::{session_type, take_ss, ScreenshotKind, SessionKind};
use anyhow::Result;
use clap::crate_version;
use home::home_dir;
use lazy_static::lazy_static;
use native_dialog::{MessageDialog, MessageType};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;

lazy_static! {
    static ref CLIENT: OnceCell<Client> = OnceCell::new();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AscellaConfig {
    #[serde(rename = "authorization")]
    pub auth: Option<String>,
    pub headers: Option<String>,
    pub command: Option<String>,
}

pub fn update_config<T: Into<PathBuf>>(path: T) -> Result<(), Error> {
    let path: PathBuf = path.into();
    let r: Value = std::fs::read_to_string(&path)
        .map(|r| serde_json::from_str(&r))
        .map_err(|x| Error::new(ErrorKind::Other, x.to_string()))?
        .map_err(|x| Error::new(ErrorKind::Other, x.to_string()))?;

    let config: AscellaConfig = serde_json::from_str(
        &serde_json::to_string(&r["Headers"])
            .map_err(|x| Error::new(ErrorKind::Other, x.to_string()))?,
    )
    .map_err(|x| Error::new(ErrorKind::Other, x.to_string()))?;

    let mut write_path = home_dir().unwrap();

    write_path.extend(&[".ascella", "config.toml"]);
    std::fs::write(
        &write_path,
        toml::to_string_pretty(&config).map_err(|x| Error::new(ErrorKind::Other, x.to_string()))?,
    )
    .map_err(|x| Error::new(ErrorKind::Other, x.to_string()))?;
    Ok(())
}

pub async fn screenshot(t: ScreenshotKind) -> Result<()> {
    let mut write_path = home_dir().unwrap();
    write_path.extend(&[".ascella", "config.toml"]);

    let config: AscellaConfig = if let Ok(config_raw) = std::fs::read_to_string(write_path) {
        if let Ok(config) = toml::from_str(&config_raw) {
            config
        } else {
            println!("Your config is invalid please use a valid ascella config");
            MessageDialog::new()
                .set_type(MessageType::Info)
                .set_title("invalid config")
                .set_text("Your config is invalid please use a valid ascella config")
                .show_alert()
                .unwrap();
            return Ok(());
        }
    } else {
        println!("config not detected please upload your config");
        MessageDialog::new()
      .set_type(MessageType::Info)
      .set_title("config not detected please upload your config")
      .set_text("config not detected please upload your config\n\nPlease add a config file you can do this using the gui")
      .show_alert()
      .unwrap();
        return Ok(());
    };

    let mut path = home_dir().unwrap();

    path.extend(&[".ascella", "images"]);
    std::fs::create_dir_all(&path).unwrap();
    let filename = chrono::offset::Local::now()
        .format("%Y-%m-%d_%H-%M-%S.png")
        .to_string();
    path.extend(&[&filename]);
    if let Some(command) = config.command {
        let replaced = command.replace(
            "%image",
            &path.clone().into_os_string().into_string().unwrap(),
        );
        let mut parts = replaced.trim().split_whitespace();

        let command = parts.next().unwrap();

        let args = parts;

        Command::new(command).args(args).spawn().unwrap();
    } else {
        take_ss(
            t,
            path.clone().into_os_string().into_string().unwrap(),
            true,
        );
    }
    upload(path).await.unwrap();
    Ok(())
}

use thiserror::Error;
use tokio::sync::OnceCell;

#[derive(Error, Debug)]
pub enum AscellaError {
    #[error("Invalid config, config file not found. use the config subcommand to set the config!")]
    NoInvalidConfig,
    #[error("Config is not valid toml!")]
    ConfigParsingError,
    #[error("Invalid auth token! please upload your new config!")]
    InvalidAuthToken,
}

pub fn get_config() -> Result<AscellaConfig, AscellaError> {
    let mut write_path = home_dir().unwrap();
    write_path.extend(&[".ascella", "config.toml"]);

    let config_raw = if let Ok(config_raw) = std::fs::read_to_string(write_path) {
        config_raw
    } else {
        return Err(AscellaError::NoInvalidConfig);
    };
    if let Ok(config) = toml::from_str(&config_raw) {
        Ok(config)
    } else {
        Err(AscellaError::ConfigParsingError)
    }
}

pub fn get_client() -> Result<Client> {
    match CLIENT.get() {
        Some(client) => Ok(client.clone()),
        None => {
            let config = get_config()?;
            let mut headers = HeaderMap::new();
            headers.append(
                "authorization",
                HeaderValue::from_str(&config.auth.expect("No auth"))?,
            );

            let client = reqwest::ClientBuilder::new()
                .default_headers(headers)
                .user_agent(format!(
                    "Ascella-uploader/{} ( {} )",
                    crate_version!(),
                    env::consts::OS
                ))
                .build()?;
            CLIENT.set(client)?;
            get_client()
        }
    }
}
const PATH: &str = "https://ascella.wtf/v2/ascella";

#[inline]
pub fn do_req<T: Display>(method: Method, path: T) -> Result<RequestBuilder> {
    let req = get_client()?.request(method, format!("https://ascella.wtf/v2/ascella/{}", path));

    Ok(req)
}

pub async fn upload<P: AsRef<Path>>(path: P) -> Result<String> {
    let bytes = fs::read(path);
    if bytes.is_err() {
        return Ok(String::new());
    }

    let form = Form::new().part("file", Part::bytes(bytes.unwrap()));

    let resp = do_req(Method::POST, "upload")?
        .multipart(form)
        .send()
        .await?;

    let text = resp.text().await.unwrap();
    let r: Value = serde_json::from_str(&text).unwrap();
    let url = r["url"].as_str().expect("Invalid image type");
    println!("{url}");

    let session_kid = session_type();

    let backend = match session_kid {
        SessionKind::Wayland => Some(ClipboardBackend::Wayland),
        SessionKind::X11 => Some(ClipboardBackend::Xorg),
        _ => None,
    };

    copy(url.to_owned(), backend);

    Ok(url.to_owned())
}

pub enum ClipboardBackend {
    Wayland,
    Xorg,
}

#[cfg(not(target_os = "linux"))]
fn copy(t: String, backend: Option<ClipboardBackend>) {
    use clipboard2::{Clipboard, SystemClipboard};
    let clipboard = SystemClipboard::new().unwrap();
    clipboard.set_string_contents(t).unwrap();
}

#[cfg(target_os = "linux")]
fn copy(t: String, backend: Option<ClipboardBackend>) {
    if let Some(backend) = backend {
        match backend {
            ClipboardBackend::Xorg => {
                let child = Command::new("xclip")
                    .arg("-selection")
                    .arg("clipboard")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn();
                if let Ok(mut child) = child {
                    {
                        let child_stdin = child.stdin.as_mut();
                        if let Some(child_stdin) = child_stdin {
                            child_stdin.write_all((&t).to_string().as_bytes()).ok();
                        }
                    }
                    let _ = child.wait().ok();
                }
            }
            ClipboardBackend::Wayland => {
                Command::new("wl-copy").arg(&t).spawn().ok();
            }
        }
    }
}
