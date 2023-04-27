use serde::{Deserialize, Serialize};

use crate::{utils::ascella_dir, SendScreenshot};

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "type")]
pub enum ScreenshotType {
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

impl ScreenshotType {
    pub fn name(&self) -> &'static str {
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
        ascella_dir()
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
                Self::Spectacle => format!("spectacle -rbno {}", file),
                Self::Scrot => format!("scrot --select {}", file),
                Self::Screencapture => format!("screencapture -S {}", file),
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
                Self::Spectacle => format!("spectacle -fbno {}", file),
                Self::Scrot => format!("scrot {}", file),
                Self::Screencapture => format!("screencapture -S {}", file),
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
                Self::Spectacle => format!("spectacle -abno {}", file),
                Self::Scrot => format!("scrot --border --focused {}", file),
                Self::Screencapture => {
                    format!("screencapture -w {}", file)
                }
                Self::Custom { window, .. } => window.replace("{file}", &file),
            },
        )
    }

    pub fn cmd_from_type(&self, send: SendScreenshot) -> (String, String) {
        match send {
            SendScreenshot::Area => self.area_command(),
            SendScreenshot::Window => self.window_command(),
            SendScreenshot::Screen => self.screen_command(),
        }
    }
}
