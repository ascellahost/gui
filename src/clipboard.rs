#[cfg(not(target_os = "linux"))]
pub async fn copy(t: String) {
    use clipboard2::{Clipboard, SystemClipboard};
    let clipboard = SystemClipboard::new().unwrap();
    clipboard.set_string_contents(t).unwrap();
}

#[cfg(target_os = "linux")]
pub async fn copy(t: String) {
    use std::{env, process::Stdio};

    use tokio::{io::AsyncWriteExt, process::Command};

    let wayland = match env::var("XDG_SESSION_TYPE") {
        Ok(ok) => matches!(ok.to_lowercase().as_ref(), "wayland"),
        Err(_) => false,
    };

    match wayland {
        true => {
            Command::new("wl-copy").arg(&t).spawn().ok();
        }
        false => {
            let child = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn();
            if let Ok(mut child) = child {
                {
                    let child_stdin = child.stdin.as_mut();
                    if let Some(child_stdin) = child_stdin {
                        child_stdin.write_all(t.to_string().as_bytes()).await.ok();
                    }
                }
                let _ = child.wait().await.ok();
            }
        }
    }
}
