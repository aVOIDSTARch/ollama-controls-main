use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

fn is_listening(addr: &str) -> bool {
    let Ok(sock): Result<SocketAddr, _> = addr.parse() else {
        return false;
    };
    TcpStream::connect_timeout(&sock, Duration::from_millis(250)).is_ok()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn spawn_bg(workspace: &PathBuf, package: &str, bin: &str, log_name: &str) -> Result<(), String> {
    let log_path = workspace.join(log_name);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("open {}: {e}", log_path.display()))?;

    Command::new("cargo")
        .current_dir(workspace)
        .arg("run")
        .arg("-p")
        .arg(package)
        .arg("--bin")
        .arg(bin)
        .stdout(Stdio::from(
            log.try_clone()
                .map_err(|e| format!("clone log fd {}: {e}", log_path.display()))?,
        ))
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|e| format!("spawn {package}/{bin}: {e}"))?;
    Ok(())
}

fn main() {
    let ws = workspace_root();
    let api_addr = "127.0.0.1:3000";
    let web_addr = "127.0.0.1:3005";

    if !is_listening(api_addr) {
        eprintln!("starting ollama-controls-api on {api_addr}...");
        if let Err(e) = spawn_bg(&ws, "ollama-controls-api", "ollama-controls-api", "ollama-api.log") {
            eprintln!("{e}");
            std::process::exit(1);
        }
    } else {
        eprintln!("ollama-controls-api already running on {api_addr}");
    }

    if !is_listening(web_addr) {
        eprintln!("starting fail-academy website on {web_addr}...");
        if let Err(e) = spawn_bg(&ws, "fail-academy", "fail-academy", "fail-academy.log") {
            eprintln!("{e}");
            std::process::exit(1);
        }
    } else {
        eprintln!("fail-academy website already running on {web_addr}");
    }

    eprintln!("launch sequence complete.");
}
