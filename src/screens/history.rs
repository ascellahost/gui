use std::time::Duration;

use anyhow::Result;
use eframe::egui::{self, Ui};

use reqwest::{header::HeaderValue, Method};
use serde::{Deserialize, Serialize};

use crate::{ui::MyApp, Request, RequestType};

#[derive(Default, Serialize, Deserialize)]
pub struct AscellaFile {
    name: String,
    vanity: String,
    raw: String,
}

pub fn screen(app: &mut MyApp, ui: &mut Ui, _ctx: &egui::Context) -> Result<()> {
    ui.heading("History");

    for file in &app.history {
        ui.horizontal(|ui| {
            ui.label(&file.name);
            ui.hyperlink(format!("https://picup.click/v/{}", file.vanity));
            // if ui.hyperlink(format!("https://ascella.host/v/{}".file.vanity)) {
            // open_url
            // }
        });
    }

    ui.horizontal(|ui| {
        if ui.button("Reset").clicked() {
            app.history.clear();
            app.history_index = 0;
        }

        if ui.button("Load more").clicked() {
            let mut req = reqwest::Request::new(
                Method::GET,
                format!("{}/me/files?page={}", app.config.api_url, app.history_index)
                    .parse()
                    .unwrap(),
            );
            req.headers_mut()
                .append("ascella-token", HeaderValue::from_str(&app.config.api_key).unwrap());

            app.sender
                .send(Request::DoRequest {
                    r_type: RequestType::RequestPage,
                    request: req,
                })
                .ok();
            app.history_index += 1;

            app.toasts
                .basic("Fetching images")
                .set_duration(Some(Duration::from_millis(600)));
        }
    });

    Ok(())
}
