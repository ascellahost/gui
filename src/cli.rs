use clap::{Parser, Subcommand};

/// Ascella GUI using no subcommand opens the gui?
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct AscellaCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Area { delay: Option<u64> },
    Window { delay: Option<u64> },
    Screen { delay: Option<u64> },
}

// macro_rules! subcommands {
//     ( $( ($name:ident, $s:tt, $desc:tt) ),* ) => {
//         $(
//             #[derive(FromArgs, PartialEq, Debug)]
//             #[doc = $desc]
//             #[argh(subcommand, name = $s)]
//             pub struct $name {
//                 /// how many ms to delay screenshotting
//                 #[argh(option)]
//                 pub delay: Option<i32>,
//             }
//         )*
//     };
// }

// subcommands![
//     (AreaCommand, "area", "Screenshot a area"),
//     (WindowCommand, "window", "Screenshot a window"),
//     (ScreenCommand, "screen", "Sreenshot a screen")
// ];
