use std::{ffi::OsStr, io::ErrorKind, os::unix::prelude::OsStrExt, path::PathBuf, time::Duration};

use egui_notify::Toast;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    multipart::{self, Part},
};
use tokio::{process::Command, sync::mpsc::UnboundedSender};
use tracing::info;

use crate::{
    ascella_config::AscellaConfig, clipboard::copy, utils::ascella_notif, Request, RequestResponse, UploadResponse,
};

pub async fn handle_event(
    data: Request,
    client: &reqwest::Client,
    sender: &UnboundedSender<RequestResponse>,
) -> anyhow::Result<()> {
    match data {
        Request::DoRequest { request, r_type } => {
            let res = client.execute(request).await?;

            sender
                .send(RequestResponse::Request {
                    status: res.status(),
                    content: res.bytes().await?,
                    r_type,
                })
                .ok();
        }
        Request::Screenshot {
            r_type,
            send,
            config,
            print,
        } => {
            let cmd = r_type.cmd_from_type(send);
            let mut args = cmd.1.split_whitespace();

            let command = match Command::new(args.next().unwrap()).args(args).output().await {
                Ok(r) => r,
                Err(e) => {
                    let msg = if e.kind() == ErrorKind::NotFound {
                        format!(
                            "{} is not installed\nplease install it and make sure its added to your path",
                            r_type.name()
                        )
                    } else {
                        format!("Failed executing screenshot command\n{:?}", e)
                    };
                    tracing::error!("Error starting screenshot process {e:?}, {}", cmd.1);
                    sender.send(RequestResponse::Toast(Toast::error(msg))).ok();
                    return Ok(());
                }
            };
            if !command.status.success() {
                sender
                    .send(RequestResponse::Toast(Toast::error(format!(
                        "Failed executing screenshot command\n{command:?}",
                    ))))
                    .ok();
                tracing::error!("Error executing screenshot command {command:?}");
                return Ok(());
            }

            match upload_file(PathBuf::from(cmd.0), &config, client, print).await {
                Ok(res) => {
                    sender
                        .send(RequestResponse::Toast(Toast::success(format!(
                            "Image uploaded {}",
                            res.url
                        ))))
                        .ok();
                }
                Err(e) => {
                    sender
                        .send(RequestResponse::Toast(Toast::error(format!(
                            "Failed uploading image\n{:?}",
                            e
                        ))))
                        .ok();
                }
            }
        }
        Request::SaveConfig(config) => {
            config.save().await?;
            sender
                .send(RequestResponse::Toast(Toast::success("Config saved".to_string())))
                .ok();
        }
    };
    Ok(())
}

pub async fn upload_file(
    path: PathBuf,
    config: &AscellaConfig,
    client: &reqwest::Client,
    print: bool,
) -> anyhow::Result<UploadResponse> {
    let mut form = multipart::Form::new();
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    if path.extension() == Some(OsStr::from_bytes(b"png")) && config.optimize_png {
        info!("Optimizing PNG");
        let file = std::fs::read(&path)?;
        let file_ln = file.len();
        let now = std::time::Instant::now();
        let buf = oxipng::optimize_from_memory(
            &file,
            &oxipng::Options {
                timeout: Some(Duration::from_millis(config.optimize_timeout)),
                strip: oxipng::Headers::Safe,
                force: true,
                ..oxipng::Options::from_preset(4)
            },
        )?;
        info!(
            "Optimized image in {}ms before: {file_ln} after: {}",
            now.elapsed().as_millis(),
            buf.len()
        );
        form = form.part("file", Part::bytes(buf).file_name(filename).mime_str("image/png")?);
    } else {
        fn file_to_body(file: tokio::fs::File) -> reqwest::Body {
            let stream = tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());

            reqwest::Body::wrap_stream(stream)
        }

        let file = file_to_body(tokio::fs::File::open(&path).await?);
        form = form.part(
            "file",
            Part::stream(file)
                .file_name(filename)
                //TODO infer mime
                .mime_str("image/png")?,
        );
    }
    let mut headers = headermap_from_hashmap(config.headers.iter());
    if !config.api_key.is_empty() {
        headers.insert("ascella-token", HeaderValue::from_str(&config.api_key)?);
    }

    let res = client
        .post(&config.request_url)
        .headers(headers)
        .multipart(form)
        .send()
        .await?
        .text()
        .await?;

    tracing::debug!("Image uploaded {}", res);
    let response: UploadResponse = serde_json::from_str(&res).unwrap();
    copy(response.url.clone()).await;

    if print {
        println!("Image uploaded {}", response.url);
        println!("Delete URL: {}", response.delete);
    }
    if config.notifications_enabled {
        let mut notif = &mut ascella_notif();
        notif = notif.body("Upload success, url copied to clipboard!");
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            notif = notif.image_path(&path.to_string_lossy());
        };
        notif.show()?;
    }
    Ok(response)
}

fn headermap_from_hashmap<'a, I, S>(headers: I) -> HeaderMap
where
    I: Iterator<Item = (S, S)> + 'a,
    S: AsRef<str> + 'a,
{
    headers
        .map(|(name, val)| {
            (
                HeaderName::from_bytes(name.as_ref().as_bytes()),
                HeaderValue::from_str(val.as_ref()),
            )
        })
        // We ignore the errors here. If you want to get a list of failed conversions, you can use Iterator::partition
        // to help you out here
        .filter(|(k, v)| k.is_ok() && v.is_ok())
        .map(|(k, v)| (k.unwrap(), v.unwrap()))
        .collect()
}
