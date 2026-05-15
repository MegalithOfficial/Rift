use std::{cell::RefCell, collections::HashMap, path::PathBuf};

use gio::prelude::*;

use crate::model::{
    ResultAction, SearchResult, calculator,
    history::{self, bonus as history_bonus},
    ranking::{WeightedText, weighted_match_score},
};

const MAX_RESULTS: usize = 8;

#[derive(Clone, Copy)]
pub struct QueryOptions {
    pub shell_enabled: bool,
    pub calculator_enabled: bool,
}

#[derive(Clone)]
struct DesktopEntry {
    app_info: gio::AppInfo,
    app_id: String,
    display_name: String,
    description: String,
    executable: String,
    icon: Option<gio::Icon>,
}

impl DesktopEntry {
    fn load() -> Vec<Self> {
        let mut apps = gio::AppInfo::all()
            .into_iter()
            .filter(|app| app.should_show())
            .map(|app| {
                let display_name = app.display_name().to_string();
                let description = app
                    .description()
                    .map(|text| text.to_string())
                    .unwrap_or_else(|| app.name().to_string());
                let executable = app.executable().to_string_lossy().into_owned();
                let icon = app.icon();
                let app_id = app
                    .id()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| executable.clone());

                Self {
                    app_info: app,
                    app_id,
                    display_name,
                    description,
                    executable,
                    icon,
                }
            })
            .collect::<Vec<_>>();

        apps.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        apps
    }

    fn into_result(self) -> SearchResult {
        SearchResult::new(
            self.display_name,
            self.description,
            self.executable,
            self.icon,
            "application-x-executable-symbolic",
            format!("app:{}", self.app_id),
            ResultAction::LaunchApp(self.app_info),
        )
    }

    fn score(&self, query: &str) -> Option<i32> {
        if query.is_empty() {
            return Some(0);
        }

        weighted_match_score(
            query,
            &[
                WeightedText::new(&self.display_name, 760),
                WeightedText::new(&self.app_id, 360),
                WeightedText::new(&self.executable, 260),
                WeightedText::new(&self.description, 80),
            ],
        )
    }
}

pub struct AppIndex {
    entries: Vec<DesktopEntry>,
    history: RefCell<HashMap<String, u32>>,
    history_path: PathBuf,
}

impl AppIndex {
    pub fn load() -> Self {
        let history_path = history::path();
        let history = history::read(&history_path);

        Self {
            entries: DesktopEntry::load(),
            history: RefCell::new(history),
            history_path,
        }
    }

    pub fn query(&self, term: &str, options: QueryOptions) -> Vec<SearchResult> {
        let trimmed = term.trim();

        if trimmed.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        if options.shell_enabled
            && let Some(result) = shell_result(trimmed)
        {
            results.push((1_400, result));
        }

        if options.calculator_enabled
            && let Some(result) = calculator::calculator_result(trimmed)
        {
            results.push((10_000, result));
        }

        let history = self.history.borrow();
        let remaining_slots = MAX_RESULTS.saturating_sub(results.len());
        if remaining_slots > 0 {
            let mut app_matches = self
                .entries
                .iter()
                .filter_map(|entry| {
                    entry.score(trimmed).map(|score| {
                        let usage_bonus =
                            history_bonus(history.get(&format!("app:{}", entry.app_id)));
                        (
                            score + usage_bonus - (entry.display_name.len() as i32 / 3),
                            entry.clone(),
                        )
                    })
                })
                .collect::<Vec<_>>();

            app_matches.sort_by(|(left_score, left), (right_score, right)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| left.display_name.cmp(&right.display_name))
            });

            results.extend(
                app_matches
                    .into_iter()
                    .map(|(score, entry)| (score, entry.into_result()))
                    .take(remaining_slots),
            );
        }

        results.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.title().cmp(right.title()))
        });

        results
            .into_iter()
            .map(|(_, result)| result)
            .take(MAX_RESULTS)
            .collect()
    }

    pub fn record_usage(&self, result: &SearchResult) {
        let mut history = self.history.borrow_mut();
        let entry = history.entry(result.usage_key().to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        let snapshot = history.clone();
        drop(history);
        let _ = history::write(&self.history_path, &snapshot);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

fn shell_result(query: &str) -> Option<SearchResult> {
    let command = query.strip_prefix('>')?.trim();

    if command.is_empty() {
        return None;
    }

    Some(SearchResult::new(
        command.to_string(),
        "Run in terminal".to_string(),
        command.to_string(),
        None,
        "utilities-terminal-symbolic",
        format!("shell:{command}"),
        ResultAction::RunShell(command.to_string()),
    ))
}
