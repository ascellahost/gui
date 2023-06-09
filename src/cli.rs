use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Ascella GUI using no subcommand opens the gui
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct AscellaCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// SCreenshot a area
    Area { delay: Option<u64> },
    /// Screenshot a window
    Window { delay: Option<u64> },
    /// Screenshot a screen
    Screen { delay: Option<u64> },
    /// Upload a file
    Upload { file: PathBuf }
}
