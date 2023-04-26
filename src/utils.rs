use std::{env, path::PathBuf};

use home::home_dir;

pub fn ascella_dir() -> PathBuf {
    match env::var("ASCELLA_HOME") {
        Ok(var) => PathBuf::from(var),
        _ => home_dir().unwrap().join(".ascella"),
    }
}
