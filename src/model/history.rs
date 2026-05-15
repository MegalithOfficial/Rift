use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

pub fn bonus(count: Option<&u32>) -> i32 {
    count.copied().unwrap_or(0).min(8).saturating_mul(45) as i32
}

pub fn path() -> PathBuf {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("rift").join("history.tsv")
}

pub fn read(path: &Path) -> HashMap<String, u32> {
    let mut history = HashMap::new();
    let Ok(contents) = fs::read_to_string(path) else {
        return history;
    };

    for line in contents.lines() {
        let Some((key, count)) = line.split_once('\t') else {
            continue;
        };

        if let Ok(count) = count.parse::<u32>() {
            history.insert(key.to_string(), count);
        }
    }

    history
}

pub fn write(path: &Path, history: &HashMap<String, u32>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut lines = history
        .iter()
        .map(|(key, count)| format!("{key}\t{count}"))
        .collect::<Vec<_>>();
    lines.sort();
    fs::write(path, lines.join("\n"))
}
