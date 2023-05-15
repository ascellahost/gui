use std::{env, path::PathBuf};

use home::home_dir;

use crate::theme::{themes, Theme};

pub fn ascella_dir() -> PathBuf {
    match env::var("ASCELLA_HOME") {
        Ok(var) => PathBuf::from(var),
        _ => home_dir().unwrap().join(".ascella"),
    }
}

pub fn theme_number_to_theme(theme: u8) -> Theme {
    match theme {
        0 => themes::DARK_THEME,
        1 => themes::LIGHT_THEME,
        2 => themes::TWILIGHT_THEME,
        3 => themes::SUNRISE_THEME,
        4 => themes::OCEANIC_THEME,
        5 => themes::GALACTIC_THEME,
        _ => panic!("Invalid theme"),
    }
}

pub fn theme_to_name(theme: u8) -> String {
    match theme {
        0 => "Dark".to_string(),
        1 => "Light".to_string(),
        2 => "Twilight".to_string(),
        3 => "Sunrise".to_string(),
        4 => "Oceanic".to_string(),
        5 => "Galactic".to_string(),
        _ => panic!("Invalid theme"),
    }
}
