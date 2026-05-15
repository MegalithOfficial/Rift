use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::app::css;

const THEME_FORMAT_VERSION: u32 = 1;
pub const DEFAULT_THEME_ID: &str = "default";

#[derive(Debug, Clone)]
pub struct LoadedTheme {
    pub manifest: ThemeManifest,
    pub css: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub rift_theme_version: u32,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub manifest: ThemeManifest,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ThemeHeader {
    manifest: ThemeManifest,
}

pub fn load_theme(id: &str) -> LoadedTheme {
    let default_path = active_theme_path();
    let _ = ensure_default_theme_file(&default_path);

    if id != DEFAULT_THEME_ID {
        let path = themes_dir().join(format!("{id}.rift-theme"));
        if let Ok(theme) = read_theme_file(&path) {
            return theme;
        }
    }

    match read_theme_file(&default_path) {
        Ok(theme) => theme,
        Err(_) => built_in_theme(&default_path),
    }
}

pub fn list_available_themes() -> Vec<ThemeEntry> {
    let dir = themes_dir();
    let _ = ensure_default_theme_file(&dir.join(format!("{DEFAULT_THEME_ID}.rift-theme")));

    let mut entries = Vec::new();
    if let Ok(read) = fs::read_dir(&dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rift-theme") {
                continue;
            }
            if let Ok(theme) = read_theme_file(&path) {
                entries.push(ThemeEntry {
                    manifest: theme.manifest,
                    path: theme.path,
                });
            }
        }
    }

    entries.sort_by(|a, b| {
        if a.manifest.id == DEFAULT_THEME_ID {
            std::cmp::Ordering::Less
        } else if b.manifest.id == DEFAULT_THEME_ID {
            std::cmp::Ordering::Greater
        } else {
            a.manifest.name.to_lowercase().cmp(&b.manifest.name.to_lowercase())
        }
    });

    if !entries.iter().any(|entry| entry.manifest.id == DEFAULT_THEME_ID) {
        let path = active_theme_path();
        entries.insert(
            0,
            ThemeEntry {
                manifest: default_manifest(),
                path,
            },
        );
    }

    entries
}

pub fn validate_theme_path(path: &Path) -> Result<ThemeManifest, String> {
    let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let theme = parse_theme_file(path, &source)?;

    if theme.manifest.rift_theme_version > THEME_FORMAT_VERSION {
        return Err(format!(
            "theme requires rift_theme_version {} (this build supports {})",
            theme.manifest.rift_theme_version, THEME_FORMAT_VERSION
        ));
    }
    if theme.css.trim().is_empty() {
        return Err("theme [style] section is empty".to_string());
    }

    Ok(theme.manifest)
}

pub fn themes_dir_path() -> PathBuf {
    themes_dir()
}

pub fn active_theme_path() -> PathBuf {
    themes_dir().join(format!("{DEFAULT_THEME_ID}.rift-theme"))
}

pub fn rewrite_default_theme_file() -> Result<PathBuf, String> {
    let path = active_theme_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, default_theme_document()).map_err(|error| error.to_string())?;
    Ok(path)
}

fn read_theme_file(path: &Path) -> Result<LoadedTheme, String> {
    let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_theme_file(path, &source)
}

fn ensure_default_theme_file(path: &Path) -> Result<(), String> {
    if path.is_file() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    fs::write(path, default_theme_document()).map_err(|error| error.to_string())
}

fn parse_theme_file(path: &Path, source: &str) -> Result<LoadedTheme, String> {
    let (header, style) = source
        .split_once("\n[style]\n")
        .ok_or("theme file is missing a [style] section")?;
    let parsed: ThemeHeader = toml::from_str(header).map_err(|error| error.to_string())?;
    let manifest = sanitize_manifest(parsed.manifest);

    Ok(LoadedTheme {
        manifest,
        css: style.trim().to_string(),
        path: path.to_path_buf(),
    })
}

fn built_in_theme(path: &Path) -> LoadedTheme {
    LoadedTheme {
        manifest: default_manifest(),
        css: css::built_in_css(),
        path: path.to_path_buf(),
    }
}

fn default_theme_document() -> String {
    let manifest = default_manifest();
    let author = manifest.author.as_deref().unwrap_or("Rift");
    let description = manifest
        .description
        .as_deref()
        .unwrap_or("Built-in default Rift theme.");

    format!(
        "[manifest]\nid = \"{}\"\nname = \"{}\"\nversion = \"{}\"\nrift_theme_version = {}\nauthor = \"{}\"\ndescription = \"{}\"\n\n[style]\n{}\n",
        manifest.id,
        manifest.name,
        manifest.version,
        manifest.rift_theme_version,
        escape_toml(author),
        escape_toml(description),
        css::built_in_css()
    )
}

fn default_manifest() -> ThemeManifest {
    ThemeManifest {
        id: DEFAULT_THEME_ID.to_string(),
        name: "Rift Default".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rift_theme_version: THEME_FORMAT_VERSION,
        author: Some("Rift".to_string()),
        description: Some("Built-in default Rift theme.".to_string()),
    }
}

fn sanitize_manifest(mut manifest: ThemeManifest) -> ThemeManifest {
    if manifest.id.trim().is_empty() {
        manifest.id = DEFAULT_THEME_ID.to_string();
    }
    if manifest.name.trim().is_empty() {
        manifest.name = "Unnamed Theme".to_string();
    }
    if manifest.version.trim().is_empty() {
        manifest.version = "1.0.0".to_string();
    }
    if manifest.rift_theme_version == 0 {
        manifest.rift_theme_version = THEME_FORMAT_VERSION;
    }
    manifest
}

fn themes_dir() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("rift").join("themes")
}

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
