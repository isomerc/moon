use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const TELEMETRY_ENDPOINT: &str = "https://telemetry.illuminatedcorp.com/ping";
const TELEMETRY_TOKEN: Option<&str> = option_env!("MOON_TELEMETRY_TOKEN");

fn get_device_id_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("moon-calculator").join("device_id"))
}

fn get_or_create_device_id() -> Option<String> {
    let path = get_device_id_path()?;

    // Try to read existing ID
    if let Ok(id) = fs::read_to_string(&path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return Some(id);
        }
    }

    // Create new ID
    let id = Uuid::new_v4().to_string();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Save ID
    let _ = fs::write(&path, &id);

    Some(id)
}

pub fn send_launch_ping() {
    // Skip if no token configured
    let token = match TELEMETRY_TOKEN {
        Some(t) => t.to_string(),
        None => return,
    };

    let device_id = match get_or_create_device_id() {
        Some(id) => id,
        None => return,
    };

    let version = env!("CARGO_PKG_VERSION").to_string();
    let os = std::env::consts::OS.to_string();

    // Fire and forget in a separate thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        if let Ok(rt) = rt {
            let _ = rt.block_on(async {
                let client = reqwest::Client::new();
                client
                    .post(TELEMETRY_ENDPOINT)
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&serde_json::json!({
                        "device_id": device_id,
                        "version": version,
                        "os": os
                    }))
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
            });
        }
    });
}
