use std::{ffi::OsStr, io::ErrorKind, os::unix::prelude::OsStrExt, path::PathBuf, time::Duration};

use egui_notify::Toast;
use tokio::{process::Command, sync::mpsc::UnboundedSender};
use tracing::info;

use crate::{
    ascella_config::AscellaConfig, clipboard::copy, upload_s3, utils::ascella_notif, Request, RequestResponse,
    UploadResponse,
};

pub async fn handle_event(data: Request, sender: &UnboundedSender<RequestResponse>) -> anyhow::Result<()> {
    match data {
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

            match upload_file(PathBuf::from(cmd.0), &config, print).await {
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

pub async fn upload_file(path: PathBuf, config: &AscellaConfig, print: bool) -> anyhow::Result<UploadResponse> {
    let filename = path.file_name().unwrap().to_string_lossy().to_string();

    let buf = if path.extension() == Some(OsStr::from_bytes(b"png")) && config.optimize_png {
        info!("Optimizing PNG");
        let file = std::fs::read(&path)?;
        let file_ln = file.len();
        let now = std::time::Instant::now();
        let buf = oxipng::optimize_from_memory(
            &file,
            &oxipng::Options {
                timeout: Some(Duration::from_millis(config.optimize_timeout)),
                strip: oxipng::StripChunks::Safe,
                force: true,
                ..oxipng::Options::from_preset(4)
            },
        )?;
        info!(
            "Optimized image in {}ms before: {file_ln} after: {}",
            now.elapsed().as_millis(),
            buf.len()
        );
        buf
    } else {
        // fn file_to_body(file: tokio::fs::File) -> reqwest::Body {
        //     let stream = tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());

        //     reqwest::Body::wrap_stream(stream)
        // }

        // let file = file_to_body(tokio::fs::File::open(&path).await?);
        // form = form.part(
        //     "file",
        //     Part::stream(file)
        //         .file_name(filename)
        //         //TODO infer mime
        //         .mime_str("image/png")?,
        // );

        tokio::fs::read(&path).await?
    };

    upload_s3::s3_upload(config, filename.clone(), buf).await?;
    let response = UploadResponse {
        url: format!("{}/{filename}", config.public_url),
        delete: String::new(),
        metadata: String::new(),
    };

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
