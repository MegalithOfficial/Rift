use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const CONFIG_VERSION: u32 = 1;
const DEFAULT_SHORTCUT: &str = "CTRL+space";
const DEFAULT_WINDOW_WIDTH: i32 = 640;
const DEFAULT_WINDOW_TOP_MARGIN: i32 = 44;
const DEFAULT_MAX_VISIBLE_RESULTS: u32 = 4;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub window_width: i32,
    pub window_top_margin: i32,
    pub max_visible_results: usize,
    pub clear_input_on_hide: bool,
    pub render_monitor: RenderMonitor,
    pub shell_enabled: bool,
    pub calculator_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    #[serde(default)]
    pub launcher: LauncherConfig,
    #[serde(default)]
    pub providers: ProviderConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_id")]
    pub active: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            active: default_theme_id(),
        }
    }
}

fn default_theme_id() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default = "default_shortcut")]
    pub shortcut_trigger: String,
    #[serde(default = "default_window_width")]
    pub window_width: i32,
    #[serde(default = "default_window_top_margin")]
    pub window_top_margin: i32,
    #[serde(default = "default_max_visible_results")]
    pub max_visible_results: u32,
    #[serde(default = "default_false")]
    pub launch_at_login: bool,
    #[serde(default = "default_true")]
    pub clear_input_on_hide: bool,
    #[serde(default)]
    pub render_monitor: RenderMonitor,
    #[serde(default = "default_false")]
    pub keep_open_on_focus_loss: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RenderMonitor {
    #[default]
    Cursor,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default = "default_true")]
    pub shell_enabled: bool,
    #[serde(default = "default_true")]
    pub calculator_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            launcher: LauncherConfig::default(),
            providers: ProviderConfig::default(),
            theme: ThemeConfig::default(),
        }
    }
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            shortcut_trigger: default_shortcut(),
            window_width: default_window_width(),
            window_top_margin: default_window_top_margin(),
            max_visible_results: default_max_visible_results(),
            launch_at_login: false,
            clear_input_on_hide: true,
            render_monitor: RenderMonitor::default(),
            keep_open_on_focus_loss: false,
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            shell_enabled: true,
            calculator_enabled: true,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = path();
        let Ok(contents) = fs::read_to_string(&path) else {
            return Self::default();
        };

        toml::from_str::<Self>(&contents)
            .map(|config| config.sanitized())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = path();
        write_config(&path, self)
    }

    pub fn runtime(&self) -> RuntimeConfig {
        let sanitized = self.clone().sanitized();
        let launcher = sanitized.launcher;
        let providers = sanitized.providers;

        RuntimeConfig {
            window_width: launcher.window_width,
            window_top_margin: launcher.window_top_margin,
            max_visible_results: launcher.max_visible_results as usize,
            clear_input_on_hide: launcher.clear_input_on_hide,
            render_monitor: launcher.render_monitor,
            shell_enabled: providers.shell_enabled,
            calculator_enabled: providers.calculator_enabled,
        }
    }

    fn sanitized(mut self) -> Self {
        self.config_version = CONFIG_VERSION;
        self.launcher.window_width = self.launcher.window_width.clamp(420, 1200);
        self.launcher.window_top_margin = self.launcher.window_top_margin.clamp(0, 240);
        self.launcher.max_visible_results = self.launcher.max_visible_results.clamp(1, 8);
        self.launcher.shortcut_trigger = sanitize_shortcut(&self.launcher.shortcut_trigger);
        self
    }
}

pub fn path() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("rift").join("config.toml")
}

fn write_config(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let serialized =
        toml::to_string_pretty(&config.clone().sanitized()).map_err(|error| error.to_string())?;
    fs::write(path, serialized).map_err(|error| error.to_string())
}

fn sanitize_shortcut(shortcut: &str) -> String {
    let trimmed = shortcut.trim();
    if trimmed.is_empty() {
        default_shortcut()
    } else {
        trimmed.to_string()
    }
}

fn default_config_version() -> u32 {
    CONFIG_VERSION
}

fn default_shortcut() -> String {
    DEFAULT_SHORTCUT.to_string()
}

fn default_window_width() -> i32 {
    DEFAULT_WINDOW_WIDTH
}

fn default_window_top_margin() -> i32 {
    DEFAULT_WINDOW_TOP_MARGIN
}

fn default_max_visible_results() -> u32 {
    DEFAULT_MAX_VISIBLE_RESULTS
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}
