use std::{io::ErrorKind, path::PathBuf};

use egui_notify::Toast;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    multipart::{self, Part},
};
use tokio::{process::Command, sync::mpsc::UnboundedSender};

use crate::{clipboard::copy, Request, RequestResponse, UploadResponse, ascella_config::AscellaConfig};

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
                },
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

pub async fn upload_file(path: PathBuf, config: &AscellaConfig, client: &reqwest::Client, print: bool)
-> anyhow::Result<UploadResponse> {
    fn file_to_body(file: tokio::fs::File) -> reqwest::Body {
        let stream = tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());

        reqwest::Body::wrap_stream(stream)
    }

    let file = file_to_body(tokio::fs::File::open(&path).await?);
    let form = multipart::Form::new().part(
        "file",
        Part::stream(file)
            .file_name(path.file_name().unwrap().to_string_lossy().to_string())
            .mime_str("image/png")?,
    );

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
