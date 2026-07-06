use std::{path::PathBuf, sync::Mutex};

use crate::{commands::selection::PanelContext, config::AppConfig};

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub settings_path: Option<PathBuf>,
    pub latest_selection: Mutex<Option<PanelContext>>,
    latest_selection_window_handle: Mutex<Option<isize>>,
    latest_source_text: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Mutex::new(config),
            settings_path: None,
            latest_selection: Mutex::new(None),
            latest_selection_window_handle: Mutex::new(None),
            latest_source_text: Mutex::new(None),
        }
    }

    pub fn new_with_settings_path(config: AppConfig, settings_path: PathBuf) -> Self {
        Self {
            config: Mutex::new(config),
            settings_path: Some(settings_path),
            latest_selection: Mutex::new(None),
            latest_selection_window_handle: Mutex::new(None),
            latest_source_text: Mutex::new(None),
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
        let source_text = context.selection.text.clone();
        *self
            .latest_selection
            .lock()
            .expect("latest selection mutex poisoned") = Some(context);
        self.store_latest_source_text(source_text);
    }

    pub fn latest_selection(&self) -> Option<PanelContext> {
        self.latest_selection
            .lock()
            .expect("latest selection mutex poisoned")
            .clone()
    }

    pub fn store_latest_selection_window_handle(&self, handle: isize) {
        *self
            .latest_selection_window_handle
            .lock()
            .expect("latest selection window handle mutex poisoned") = Some(handle);
    }

    pub fn latest_selection_window_handle(&self) -> Option<isize> {
        *self
            .latest_selection_window_handle
            .lock()
            .expect("latest selection window handle mutex poisoned")
    }

    pub fn clear_latest_selection(&self) {
        *self
            .latest_selection
            .lock()
            .expect("latest selection mutex poisoned") = None;
        *self
            .latest_selection_window_handle
            .lock()
            .expect("latest selection window handle mutex poisoned") = None;
    }

    pub fn store_latest_source_text(&self, text: String) {
        *self
            .latest_source_text
            .lock()
            .expect("latest source text mutex poisoned") = Some(text);
    }

    pub fn latest_source_text(&self) -> Option<String> {
        self.latest_source_text
            .lock()
            .expect("latest source text mutex poisoned")
            .clone()
    }
}
