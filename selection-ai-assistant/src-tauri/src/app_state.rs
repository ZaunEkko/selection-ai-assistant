use std::sync::Mutex;

use crate::config::AppConfig;

pub struct AppState {
    pub config: Mutex<AppConfig>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Mutex::new(config),
        }
    }
}
