use std::{env, path::PathBuf};

use ashpd::desktop::{
    CreateSessionOptions,
    global_shortcuts::{
        BindShortcutsOptions, ConfigureShortcutsOptions, GlobalShortcuts, ListShortcutsOptions,
        NewShortcut,
    },
};
use futures_util::StreamExt;
use gio::prelude::*;

use super::{APP_ID, LauncherHandles, TOGGLE_SHORTCUT_ID, current_config, toggle_window};

pub(super) fn start_global_shortcut_registration(handles: LauncherHandles) {
    spawn_global_shortcut_registration(handles, false);
}

pub(super) fn restart_global_shortcut_registration(handles: &LauncherHandles) {
    if let Some(task) = handles.shortcut_task.borrow_mut().take() {
        task.abort();
    }
    if let Some(session) = handles.shortcut_session.borrow_mut().take() {
        glib::MainContext::default().spawn_local(async move {
            let _ = session.close().await;
        });
    }

    spawn_global_shortcut_registration(handles.clone(), true);
}

fn spawn_global_shortcut_registration(handles: LauncherHandles, force_rebind: bool) {
    let task_handles = handles.clone();
    let join = glib::MainContext::default().spawn_local(async move {
        if let Err(error) = register_global_shortcut(task_handles, force_rebind).await {
            if env::var_os("RIFT_DEBUG").is_some() {
                eprintln!("rift: global shortcut unavailable: {error}");
            }
        }
    });
    *handles.shortcut_task.borrow_mut() = Some(join);
}

async fn register_global_shortcut(
    handles: LauncherHandles,
    force_rebind: bool,
) -> Result<(), String> {
    let preferred_trigger = current_config(&handles).launcher.shortcut_trigger;
    let connection = ashpd::zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?;
    let app_id = APP_ID
        .parse()
        .map_err(|error: ashpd::Error| error.to_string())?;
    ashpd::register_host_app_with_connection(connection.clone(), app_id)
        .await
        .map_err(|error| error.to_string())?;

    let shortcuts = GlobalShortcuts::with_connection(connection)
        .await
        .map_err(|error| error.to_string())?;
    let session = shortcuts
        .create_session(CreateSessionOptions::default())
        .await
        .map_err(|error| error.to_string())?;
    let gnome_v1_fallback = is_gnome_session() && shortcuts.version() < 2;

    let needs_reconfigure = force_rebind
        || !portal_has_toggle_shortcut(&shortcuts, &session, &preferred_trigger).await?;
    if needs_reconfigure {
        let request = shortcuts
            .bind_shortcuts(
                &session,
                &[NewShortcut::new(TOGGLE_SHORTCUT_ID, "Toggle Rift launcher")
                    .preferred_trigger(Some(preferred_trigger.as_str()))],
                None,
                BindShortcutsOptions::default(),
            )
            .await
            .map_err(|error| error.to_string())?;

        if gnome_v1_fallback {
            sync_gnome_v1_shortcut(&preferred_trigger)?;
            if env::var_os("RIFT_DEBUG").is_some() {
                eprintln!(
                    "rift: synced GNOME GlobalShortcuts v1 fallback for {}",
                    preferred_trigger
                );
            }
        }

        let response = request.response().map_err(|error| error.to_string())?;
        if response.shortcuts().is_empty()
            && !(gnome_v1_fallback && gnome_v1_shortcut_matches(&preferred_trigger)?)
        {
            return Err("no global shortcuts were granted by the portal".to_string());
        }

        if shortcuts.version() >= 2 {
            shortcuts
                .configure_shortcuts(&session, None, ConfigureShortcutsOptions::default())
                .await
                .map_err(|error| error.to_string())?;
        }
    }

    *handles.shortcut_session.borrow_mut() = Some(session);

    let mut activated = shortcuts
        .receive_activated()
        .await
        .map_err(|error| error.to_string())?;

    while let Some(signal) = activated.next().await {
        if signal.shortcut_id() == TOGGLE_SHORTCUT_ID {
            toggle_window(&handles);
        }
    }

    handles.shortcut_session.borrow_mut().take();

    Ok(())
}

async fn portal_has_toggle_shortcut(
    shortcuts: &GlobalShortcuts,
    session: &ashpd::desktop::Session<GlobalShortcuts>,
    preferred_trigger: &str,
) -> Result<bool, String> {
    let request = shortcuts
        .list_shortcuts(session, ListShortcutsOptions::default())
        .await
        .map_err(|error| error.to_string())?;
    let response = request.response().map_err(|error| error.to_string())?;

    Ok(response.shortcuts().iter().any(|shortcut| {
        shortcut.id() == TOGGLE_SHORTCUT_ID
            && shortcut
                .trigger_description()
                .eq_ignore_ascii_case(preferred_trigger)
    }))
}

pub(super) fn desktop_entry_path(app_id: &str) -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("applications")
        .join(format!("{app_id}.desktop"))
}

pub(super) fn shortcut_change_needs_restart() -> bool {
    is_gnome_session()
}

fn sync_gnome_v1_shortcut(preferred_trigger: &str) -> Result<(), String> {
    if !is_gnome_session() {
        return Ok(());
    }

    let shortcut = to_gnome_shortcut(preferred_trigger);
    let settings = gnome_v1_settings();
    let variant_text = format!(
        "[('toggle-launcher', {{'shortcuts': <['{}']>, 'description': <'Toggle Rift launcher'>}})]",
        shortcut
    );
    let variant = glib::Variant::parse(None, &variant_text).map_err(|error| error.to_string())?;
    settings
        .set_value("shortcuts", &variant)
        .map_err(|error| error.to_string())
}

fn gnome_v1_shortcut_matches(preferred_trigger: &str) -> Result<bool, String> {
    if !is_gnome_session() {
        return Ok(false);
    }

    let expected = to_gnome_shortcut(preferred_trigger);
    let printed = gnome_v1_settings()
        .value("shortcuts")
        .print(false)
        .to_string();
    Ok(printed.contains("toggle-launcher") && printed.contains(&format!("'{}'", expected)))
}

fn gnome_v1_settings() -> gio::Settings {
    gio::Settings::with_path(
        "org.gnome.settings-daemon.global-shortcuts.application",
        &format!("/org/gnome/settings-daemon/global-shortcuts/{APP_ID}/"),
    )
}

fn to_gnome_shortcut(trigger: &str) -> String {
    let mut modifiers = String::new();
    let mut primary = None::<String>;

    for part in trigger
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => modifiers.push_str("<Control>"),
            "ALT" => modifiers.push_str("<Alt>"),
            "SHIFT" => modifiers.push_str("<Shift>"),
            "SUPER" | "META" => modifiers.push_str("<Super>"),
            other => primary = Some(normalize_gnome_key_name(other)),
        }
    }

    format!("{modifiers}{}", primary.unwrap_or_default())
}

fn normalize_gnome_key_name(key: &str) -> String {
    match key {
        "SPACE" => "space".to_string(),
        "ESC" => "Escape".to_string(),
        "ENTER" => "Return".to_string(),
        "TAB" => "Tab".to_string(),
        single if single.chars().count() == 1 => single.to_ascii_lowercase(),
        other => {
            let lower = other.to_ascii_lowercase();
            let mut chars = lower.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

fn is_gnome_session() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("XDG_SESSION_DESKTOP"))
        .map(|desktop| desktop.to_ascii_lowercase().contains("gnome"))
        .unwrap_or(false)
}
