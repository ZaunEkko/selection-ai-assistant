use std::{path::PathBuf, sync::Mutex};

use crate::{commands::selection::PanelContext, config::AppConfig};

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub settings_path: Option<PathBuf>,
    pub latest_selection: Mutex<Option<PanelContext>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Mutex::new(config),
            settings_path: None,
            latest_selection: Mutex::new(None),
        }
    }

    pub fn new_with_settings_path(config: AppConfig, settings_path: PathBuf) -> Self {
        Self {
            config: Mutex::new(config),
            settings_path: Some(settings_path),
            latest_selection: Mutex::new(None),
        }
    }

    pub fn load_or_default_from_path(settings_path: PathBuf) -> std::io::Result<Self> {
        let config = if settings_path.exists() {
            AppConfig::load_from_path(&settings_path).unwrap_or_default()
        } else {
            AppConfig::default()
        };
        Ok(Self::new_with_settings_path(config, settings_path))
    }

    pub fn load_or_default() -> Self {
        match AppConfig::settings_path().and_then(|path| Self::load_or_default_from_path(path)) {
            Ok(state) => state,
            Err(_) => Self::new(AppConfig::default()),
        }
    }

    pub fn store_latest_selection(&self, context: PanelContext) {
        *self
            .latest_selection
            .lock()
            .expect("latest selection mutex poisoned") = Some(context);
    }

    pub fn latest_selection(&self) -> Option<PanelContext> {
        self.latest_selection
            .lock()
            .expect("latest selection mutex poisoned")
            .clone()
    }

    pub fn clear_latest_selection(&self) {
        *self
            .latest_selection
            .lock()
            .expect("latest selection mutex poisoned") = None;
    }
}
