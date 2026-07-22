use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use rand_core::OsRng;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fs,
    io::BufRead,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};
use tauri::Emitter;

#[derive(Serialize)]
struct BatteryInfo {
    percent: f32,
    charging: bool,
    state: String,
}

#[derive(Serialize)]
struct NetworkInfo {
    online: bool,
    kind: String,
    name: String,
}

#[derive(Serialize)]
struct PasswordStatus {
    has_password: bool,
}

#[derive(Serialize)]
struct LoginResult {
    ok: bool,
}

#[derive(Serialize)]
struct UpdateCheckResult {
    current_version: String,
    latest_version: String,
    latest_name: Option<String>,
    release_url: String,
    release_body: Option<String>,
    published_at: Option<String>,
    update_available: bool,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
}

#[derive(Clone, Serialize)]
struct PingOutput {
    request_id: String,
    text: String,
    done: bool,
    success: bool,
}

fn password_hash_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir().ok_or("Cannot find user data directory")?;
    Ok(data_dir.join("win12-desktop").join("password.hash"))
}

#[tauri::command]
fn get_login_password_status() -> Result<PasswordStatus, String> {
    Ok(PasswordStatus {
        has_password: password_hash_path()?.exists(),
    })
}

#[tauri::command]
fn verify_login_password(password: String) -> Result<LoginResult, String> {
    if password.is_empty() {
        return Err("Password cannot be empty".to_string());
    }

    let path = password_hash_path()?;

    if !path.exists() {
        return Ok(LoginResult { ok: true });
    }

    let hash = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let parsed_hash = PasswordHash::new(hash.trim()).map_err(|e| e.to_string())?;
    let ok = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(LoginResult { ok })
}

#[tauri::command]
fn set_login_password(
    current_password: Option<String>,
    new_password: String,
) -> Result<(), String> {
    let path = password_hash_path()?;
    let has_password = path.exists();

    if has_password {
        let current_password =
            current_password.ok_or("Current password is required".to_string())?;
        if !verify_login_password(current_password)?.ok {
            return Err("Current password is incorrect".to_string());
        }
    }

    if new_password.is_empty() {
        if has_password {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(&path, password_hash).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn check_app_update() -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let release = reqwest::Client::new()
        .get("https://api.github.com/repos/win12-online/win12-desktop/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "win12-desktop-tauri-update-check")
        .send()
        .await
        .map_err(|e| format!("无法连接 GitHub: {e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub 返回错误: {e}"))?
        .json::<GitHubRelease>()
        .await
        .map_err(|e| format!("无法解析 GitHub 发布信息: {e}"))?;

    let update_available = compare_versions(&release.tag_name, &current_version)
        .map(|ordering| ordering == Ordering::Greater)
        .unwrap_or_else(|| normalize_version_tag(&release.tag_name) != current_version);

    Ok(UpdateCheckResult {
        current_version,
        latest_version: release.tag_name,
        latest_name: release.name,
        release_url: release.html_url,
        release_body: release.body.map(|body| body.chars().take(1200).collect()),
        published_at: release.published_at,
        update_available,
    })
}

fn compare_versions(remote: &str, current: &str) -> Option<Ordering> {
    let remote = Version::parse(&normalize_version_tag(remote)).ok()?;
    let current = Version::parse(&normalize_version_tag(current)).ok()?;
    Some(remote.cmp(&current))
}

fn normalize_version_tag(version: &str) -> String {
    version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

#[tauri::command]
fn get_battery_info() -> Result<BatteryInfo, String> {
    let manager = battery::Manager::new().map_err(|e| e.to_string())?;

    let mut batteries = manager.batteries().map_err(|e| e.to_string())?;

    let battery = batteries
        .next()
        .ok_or("No battery found")?
        .map_err(|e| e.to_string())?;

    let percent = battery
        .state_of_charge()
        .get::<battery::units::ratio::percent>();

    let state = format!("{:?}", battery.state());

    let charging = matches!(
        battery.state(),
        battery::State::Charging | battery::State::Full
    );

    Ok(BatteryInfo {
        percent,
        charging,
        state,
    })
}

#[tauri::command]
fn get_network_info() -> Result<NetworkInfo, String> {
    let interfaces = NetworkInterface::show().map_err(|e| e.to_string())?;

    for interface in interfaces {
        let name = interface.name.clone();

        // 跳过本机回环接口
        if name == "lo" || name.starts_with("lo") {
            continue;
        }

        // 没有 IP 地址的接口通常不是正在使用的网络
        if interface.addr.is_empty() {
            continue;
        }

        let kind = if name.starts_with("wl")
            || name.starts_with("wlan")
            || name.starts_with("wifi")
            || name.starts_with("wlp")
        {
            "wifi"
        } else if name.starts_with("en")
            || name.starts_with("eth")
            || name.starts_with("eno")
            || name.starts_with("ens")
            || name.starts_with("enp")
        {
            "ethernet"
        } else {
            "unknown"
        };

        return Ok(NetworkInfo {
            online: true,
            kind: kind.to_string(),
            name,
        });
    }

    Ok(NetworkInfo {
        online: false,
        kind: "offline".to_string(),
        name: String::new(),
    })
}

#[tauri::command]
fn ping_host(
    window: tauri::Window,
    host: String,
    ipv6: Option<bool>,
    request_id: String,
) -> Result<(), String> {
    let host = host.trim();
    let ipv6 = ipv6.unwrap_or(false);

    if host.is_empty() {
        return Err("Usage: ping <host>".to_string());
    }

    if host.split_whitespace().count() != 1 || host.starts_with('-') || host.starts_with('/') {
        return Err("Only a single host or IP address is supported".to_string());
    }

    let host = host.to_string();
    thread::spawn(move || {
        let mut command = if ipv6 && !cfg!(target_os = "windows") {
            Command::new("ping6")
        } else {
            Command::new("ping")
        };

        #[cfg(target_os = "windows")]
        {
            if ipv6 {
                command.args(["-6", "-n", "4", &host]);
            } else {
                command.args(["-n", "4", &host]);
            }
        }

        #[cfg(not(target_os = "windows"))]
        command.args(["-c", "4", &host]);

        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) => {
                emit_ping_output(&window, &request_id, format!("{}\n", e), true, false);
                return;
            }
        };

        let stdout_handle = child.stdout.take().map(|stdout| {
            let window = window.clone();
            let request_id = request_id.clone();
            thread::spawn(move || stream_ping_output(stdout, window, request_id))
        });

        let stderr_handle = child.stderr.take().map(|stderr| {
            let window = window.clone();
            let request_id = request_id.clone();
            thread::spawn(move || stream_ping_output(stderr, window, request_id))
        });

        let success = child.wait().map(|status| status.success()).unwrap_or(false);

        if let Some(handle) = stdout_handle {
            let _ = handle.join();
        }
        if let Some(handle) = stderr_handle {
            let _ = handle.join();
        }

        emit_ping_output(&window, &request_id, "", true, success);
    });

    Ok(())
}

fn stream_ping_output<R: std::io::Read>(stream: R, window: tauri::Window, request_id: String) {
    let mut reader = std::io::BufReader::new(stream);
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer).to_string();
                emit_ping_output(&window, &request_id, text, false, true);
            }
            Err(e) => {
                emit_ping_output(&window, &request_id, format!("{}\n", e), false, false);
                break;
            }
        }
    }
}

fn emit_ping_output(
    window: &tauri::Window,
    request_id: &str,
    text: impl Into<String>,
    done: bool,
    success: bool,
) {
    let _ = window.emit(
        "win12://ping-output",
        PingOutput {
            request_id: request_id.to_string(),
            text: text.into(),
            done,
            success,
        },
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_battery_info,
            get_network_info,
            get_login_password_status,
            verify_login_password,
            set_login_password,
            check_app_update,
            ping_host
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
