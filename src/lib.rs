//! RigCraft inventory service: versioned private inventory with provenance.

pub mod api;
pub mod commit;
pub mod csv_import;
pub mod domain;
pub mod error;
pub mod store;

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

pub use api::{build_router, AppState};

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: String,
    pub data_dir: PathBuf,
    pub web_dir: PathBuf,
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Inventory lives outside the repository by default so a checkout never
/// carries private stock data.
pub fn default_data_dir() -> PathBuf {
    if let Some(dir) = non_empty_env("RIGCRAFT_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(xdg) = non_empty_env("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("rigcraft");
    }
    if let Some(home) = non_empty_env("HOME") {
        return PathBuf::from(home).join(".local/share/rigcraft");
    }
    env::temp_dir().join("rigcraft")
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            addr: non_empty_env("RIGCRAFT_ADDR").unwrap_or_else(|| "127.0.0.1:8080".to_string()),
            data_dir: default_data_dir(),
            web_dir: non_empty_env("RIGCRAFT_WEB_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("apps/web/dist")),
        }
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("inventory.sqlite3")
    }
}

pub async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(store::Store::open(&config.database_path())?);
    let state = AppState::new(store, config.web_dir.clone());
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(&config.addr).await?;
    tracing::info!(
        addr = %config.addr,
        data_dir = %config.data_dir.display(),
        "rigcraft inventory listening"
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
