use std::{env, path::PathBuf};

use ashpd::desktop::{
    CreateSessionOptions,
    global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, ListShortcutsOptions, NewShortcut},
};
use futures_util::StreamExt;

use super::{APP_ID, LauncherHandles, TOGGLE_SHORTCUT_ID, current_config, toggle_window};

pub(super) fn start_global_shortcut_registration(handles: LauncherHandles) {
    glib::MainContext::default().spawn_local(async move {
        if let Err(error) = register_global_shortcut(handles).await {
            if env::var_os("RIFT_DEBUG").is_some() {
                eprintln!("rift: global shortcut unavailable: {error}");
            }
        }
    });
}

async fn register_global_shortcut(handles: LauncherHandles) -> Result<(), String> {
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

    if !portal_has_toggle_shortcut(&shortcuts, &session, &preferred_trigger).await? {
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

        let response = request.response().map_err(|error| error.to_string())?;
        if response.shortcuts().is_empty() {
            return Err("no global shortcuts were granted by the portal".to_string());
        }
    }

    let mut activated = shortcuts
        .receive_activated()
        .await
        .map_err(|error| error.to_string())?;

    while let Some(signal) = activated.next().await {
        if signal.shortcut_id() == TOGGLE_SHORTCUT_ID {
            toggle_window(&handles);
        }
    }

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
