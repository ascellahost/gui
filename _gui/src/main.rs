use std::{collections::HashMap, fs, path::PathBuf};

use ascella_cli::{
    clap::Parser,
    colored::Colorize,
    reqwest::Method,
    serde_json::Value,
    tabular::{Row, Table},
    user::User,
    util::{do_req, update_config, upload, AscellaError},
    *,
};
use ascella_desktop::app::AscellaDesktop;
use iced::{Application, Settings};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let res = Cli::parse();

    let res = match res.command {
        Commands::Area(Screenshot { delay }) => make_screenshot(ScreenshotKind::Area, delay).await,
        Commands::Window(Screenshot { delay }) => {
            make_screenshot(ScreenshotKind::Window, delay).await
        }
        Commands::Full(Screenshot { delay }) => make_screenshot(ScreenshotKind::Full, delay).await,
        Commands::Upload(Upload { path }) => {
            let file = PathBuf::from(path);
            let full_path = fs::canonicalize(&file).expect("File not found");
            println!(
                "{}",
                upload(full_path).await.expect("Failed to upload file")
            );
            println!("\nFile uploaded");
            println!("Have a nice day!");
            Ok(())
        }
        Commands::Profile => {
            let user = do_req(Method::POST, "verify")?.send().await?.text().await?;
            if let Ok(user) = serde_json::from_str::<User>(&user) {
                let mut table = Table::new("{:<}  {:<}");
                serde_json::from_slice::<HashMap<String, Value>>(&serde_json::to_vec(&user)?)?
                    .into_iter()
                    .for_each(|(k, v)| {
                        table.add_row(
                            Row::new()
                                .with_cell(k.green())
                                .with_cell(&v.to_string().red()),
                        );
                    });
                println!("{table}")
            } else {
                return Err(AscellaError::InvalidAuthToken.into());
            }
            Ok(())
        }
        Commands::Config(Config { file }) => {
            let file = PathBuf::from(file);
            match update_config(fs::canonicalize(&file).unwrap()) {
                Ok(()) => {
                    println!("Updated your config check ascella --help for more commands");
                    println!("Have a nice day!");
                }
                Err(e) => {
                    println!("Failed to update config please use a valid ascella config file,\n\n\nError {:?}\n", e);
                    println!("Have a nice day!");
                }
            };
            Ok(())
        }
        Commands::App => {
            AscellaDesktop::run(Settings::default()).unwrap();
            Ok(())
        }
    };
    if let Err(e) = res {
        eprintln!("{e:?}")
    }
    Ok(())
}
