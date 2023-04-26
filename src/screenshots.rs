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

    pub fn cmd_from_type(&self, send: SendScreenshot) -> (String, String) {
        match send {
            SendScreenshot::Area => self.area_command(),
            SendScreenshot::Window => self.window_command(),
            SendScreenshot::Screen => self.screen_command(),
        }
    }
}
