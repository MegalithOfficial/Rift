use std::{
    cell::RefCell,
    env,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};

use adw::prelude::*;
use ashpd::desktop::{
    CreateSessionOptions,
    global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, ListShortcutsOptions, NewShortcut},
};
use futures_util::StreamExt;
use gio::ApplicationHoldGuard;
use gtk::{Align, Orientation, gdk};
use gtk4_layer_shell::{
    Edge, KeyboardMode, Layer, LayerShell, is_supported as layer_shell_supported,
};

use crate::model::{AppIndex, ResultAction, SearchResult};

const APP_ID: &str = "dev.rift.launcher";
const TOGGLE_SHORTCUT_ID: &str = "toggle-launcher";
const TOGGLE_SHORTCUT_TRIGGER: &str = "CTRL+space";
const WINDOW_WIDTH: i32 = 640;
const WINDOW_TOP_MARGIN: i32 = 44;
const EMPTY_HEIGHT: i32 = 64;
const NO_MATCHES_HEIGHT: i32 = 104;
const RESULTS_MAX_HEIGHT: i32 = 340;
const RESULT_ROW_HEIGHT: i32 = 56;
const RESULTS_BASE_HEIGHT: i32 = 108;
const SHEET_MARGIN: i32 = 0;
const FADE_DURATION_MS: u64 = 120;
const FADE_FRAME_MS: u64 = 16;

#[derive(Clone)]
struct LauncherHandles {
    animation_source: Rc<RefCell<Option<glib::SourceId>>>,
    settings_window: Rc<RefCell<Option<gtk::ApplicationWindow>>>,
    window: gtk::ApplicationWindow,
    sheet: gtk::Box,
    search_entry: gtk::SearchEntry,
    results: gtk::ListBox,
    status: gtk::Label,
    scroller: gtk::ScrolledWindow,
    index: Rc<AppIndex>,
    visible_entries: Rc<RefCell<Vec<SearchResult>>>,
}

pub fn build() -> adw::Application {
    let launcher = Rc::new(RefCell::new(None::<LauncherHandles>));
    let resident_hold = Rc::new(RefCell::new(None::<ApplicationHoldGuard>));
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate({
        let launcher = launcher.clone();
        let resident_hold = resident_hold.clone();

        move |app| {
            if let Some(handles) = launcher.borrow().as_ref().cloned() {
                toggle_window(&handles);
            } else {
                *resident_hold.borrow_mut() = Some(app.hold());
                let handles = build_ui(app);
                warn_if_dev_desktop_entry_missing(app);
                start_global_shortcut_registration(handles.clone());
                *launcher.borrow_mut() = Some(handles);
            }
        }
    });
    app
}

fn build_ui(app: &adw::Application) -> LauncherHandles {
    let index = Rc::new(AppIndex::load());
    let animation_source = Rc::new(RefCell::new(None::<glib::SourceId>));
    let settings_window = Rc::new(RefCell::new(None::<gtk::ApplicationWindow>));
    let visible_entries = Rc::new(RefCell::new(Vec::<SearchResult>::new()));
    let status = gtk::Label::builder()
        .halign(Align::Start)
        .css_classes(["dim-label", "caption", "result-meta"])
        .build();

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search")
        .hexpand(true)
        .css_classes(["search-field"])
        .build();

    let results = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["results-list"])
        .activate_on_single_click(true)
        .vexpand(true)
        .build();

    let scroller = gtk::ScrolledWindow::builder()
        .min_content_height(240)
        .max_content_height(240)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .css_classes(["results-scroll"])
        .child(&results)
        .build();

    let settings_icon = gtk::Image::builder()
        .icon_name("open-menu-symbolic")
        .pixel_size(14)
        .halign(Align::Center)
        .valign(Align::Center)
        .hexpand(true)
        .vexpand(true)
        .build();

    let settings_button = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["settings-button"])
        .tooltip_text("Settings")
        .focusable(false)
        .can_focus(false)
        .valign(Align::Center)
        .halign(Align::End)
        .width_request(28)
        .height_request(28)
        .build();
    settings_button.set_hexpand(false);
    settings_button.set_vexpand(false);
    settings_button.set_hexpand_set(true);
    settings_button.set_vexpand_set(true);
    settings_button.append(&settings_icon);

    let settings_hover = gtk::EventControllerMotion::new();
    {
        let btn = settings_button.clone();
        settings_hover.connect_enter(move |_, _, _| {
            btn.add_css_class("hover");
        });
    }
    {
        let btn = settings_button.clone();
        settings_hover.connect_leave(move |_| {
            btn.remove_css_class("hover");
        });
    }
    settings_button.add_controller(settings_hover);

    let settings_click = gtk::GestureClick::new();
    {
        let btn = settings_button.clone();
        settings_click.connect_pressed(move |_, _, _, _| {
            btn.add_css_class("active");
        });
    }
    {
        let btn = settings_button.clone();
        settings_click.connect_released(move |_, _, _, _| {
            btn.remove_css_class("active");
        });
    }
    settings_button.add_controller(settings_click);

    let search_row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["search-row"])
        .build();
    search_row.append(&search_entry);
    search_row.append(&settings_button);

    let sheet = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(SHEET_MARGIN)
        .margin_bottom(SHEET_MARGIN)
        .margin_start(SHEET_MARGIN)
        .margin_end(SHEET_MARGIN)
        .css_classes(["spotlight-sheet"])
        .build();
    sheet.append(&search_row);
    sheet.append(&status);
    sheet.append(&scroller);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rift")
        .default_width(WINDOW_WIDTH)
        .default_height(EMPTY_HEIGHT)
        .modal(true)
        .deletable(false)
        .resizable(false)
        .child(&sheet)
        .build();

    install_css();
    window.add_css_class("rift-window");
    window.set_hide_on_close(true);
    window.set_decorated(false);
    window.set_modal(true);
    window.set_deletable(false);
    window.set_startup_id(APP_ID);
    configure_layer_shell(&window);

    let handles = LauncherHandles {
        animation_source: animation_source.clone(),
        settings_window: settings_window.clone(),
        window: window.clone(),
        sheet: sheet.clone(),
        search_entry: search_entry.clone(),
        results: results.clone(),
        status: status.clone(),
        scroller: scroller.clone(),
        index: index.clone(),
        visible_entries: visible_entries.clone(),
    };

    let settings_open = gtk::GestureClick::new();
    settings_open.connect_released({
        let app = app.clone();
        let handles = handles.clone();
        let btn = settings_button.clone();

        move |_, _, _, _| {
            btn.remove_css_class("hover");
            btn.remove_css_class("active");
            hide_launcher_now(&handles);
            present_settings_window(&app, &handles.settings_window);
        }
    });
    settings_button.add_controller(settings_open);

    window.connect_hide({
        let btn = settings_button.clone();
        move |_| {
            btn.remove_css_class("hover");
            btn.remove_css_class("active");
        }
    });

    {
        let handles = handles.clone();
        refresh_results(
            &handles.index,
            "",
            &handles.sheet,
            &handles.results,
            &handles.status,
            &handles.scroller,
            &handles.window,
            &handles.visible_entries,
        );
    }

    search_entry.connect_search_changed({
        let handles = handles.clone();

        move |entry| {
            refresh_results(
                &handles.index,
                &entry.text(),
                &handles.sheet,
                &handles.results,
                &handles.status,
                &handles.scroller,
                &handles.window,
                &handles.visible_entries,
            );
        }
    });

    search_entry.connect_activate({
        let results = results.clone();

        move |_| {
            if let Some(row) = results.row_at_index(0) {
                results.emit_by_name::<()>("row-activated", &[&row]);
            }
        }
    });

    results.connect_row_activated({
        let handles = handles.clone();
        let app_index = index.clone();
        let visible_entries = visible_entries.clone();

        move |_, row| {
            let row_index = row.index();

            if row_index < 0 {
                return;
            }

            if let Some(result) = visible_entries.borrow().get(row_index as usize) {
                if let Err(error) = activate_result(result, &handles) {
                    eprintln!("failed to activate {}: {error}", result.title());
                } else {
                    app_index.record_usage(result);
                }
            }
        }
    });

    window.connect_is_active_notify({
        let handles = handles.clone();

        move |window| {
            if !window.is_visible() || window.is_active() {
                return;
            }

            dismiss_launcher(&handles);
        }
    });

    let escape = gtk::EventControllerKey::new();
    escape.set_propagation_phase(gtk::PropagationPhase::Capture);
    escape.connect_key_pressed({
        let handles = handles.clone();

        move |_, key, _, _| {
            if key == gdk::Key::Escape {
                dismiss_launcher(&handles);
                return true.into();
            }

            false.into()
        }
    });
    window.add_controller(escape);

    search_entry.connect_stop_search({
        let handles = handles.clone();

        move |_| {
            dismiss_launcher(&handles);
        }
    });

    search_entry.grab_focus();
    handles.sheet.set_opacity(0.0);
    animate_show(&handles);

    handles
}

fn configure_layer_shell(window: &gtk::ApplicationWindow) {
    if is_gnome_session() || !layer_shell_supported() {
        return;
    }

    window.init_layer_shell();
    window.set_namespace(Some("rift"));
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(0);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_margin(Edge::Top, WINDOW_TOP_MARGIN);
}

fn is_gnome_session() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("XDG_SESSION_DESKTOP"))
        .map(|desktop| desktop.to_ascii_lowercase().contains("gnome"))
        .unwrap_or(false)
}

fn refresh_results(
    index: &AppIndex,
    query: &str,
    sheet: &gtk::Box,
    results: &gtk::ListBox,
    status: &gtk::Label,
    scroller: &gtk::ScrolledWindow,
    window: &gtk::ApplicationWindow,
    visible_entries: &Rc<RefCell<Vec<SearchResult>>>,
) {
    while let Some(child) = results.first_child() {
        results.remove(&child);
    }

    let matches = index.query(query);
    let query = query.trim();

    if query.is_empty() {
        sheet.add_css_class("collapsed");
        sheet.remove_css_class("expanded");
        set_sheet_margins(sheet, SHEET_MARGIN);
        status.set_text(&format!("{} apps", index.len()));
        status.set_visible(false);
        scroller.set_visible(false);
        scroller.set_height_request(0);
        window.set_default_size(WINDOW_WIDTH, EMPTY_HEIGHT);
        window.set_size_request(WINDOW_WIDTH, EMPTY_HEIGHT);
        window.queue_resize();
    } else if matches.is_empty() {
        sheet.remove_css_class("collapsed");
        sheet.add_css_class("expanded");
        set_sheet_margins(sheet, SHEET_MARGIN);
        status.set_text("No matches");
        status.set_visible(true);
        scroller.set_visible(false);
        scroller.set_height_request(0);
        window.set_default_size(WINDOW_WIDTH, NO_MATCHES_HEIGHT);
        window.set_size_request(WINDOW_WIDTH, NO_MATCHES_HEIGHT);
        window.queue_resize();
    } else {
        sheet.remove_css_class("collapsed");
        sheet.add_css_class("expanded");
        set_sheet_margins(sheet, SHEET_MARGIN);
        status.set_text(&format!("{} results", matches.len()));
        status.set_visible(true);
        scroller.set_visible(true);
        let visible_rows = matches.len().min(4) as i32;
        let results_height =
            (RESULTS_BASE_HEIGHT + visible_rows * RESULT_ROW_HEIGHT).min(RESULTS_MAX_HEIGHT);
        let scroller_height = (visible_rows * RESULT_ROW_HEIGHT).min(240);
        scroller.set_height_request(scroller_height);
        window.set_default_size(WINDOW_WIDTH, results_height);
        window.set_size_request(WINDOW_WIDTH, results_height);
        window.queue_resize();
    }

    for entry in &matches {
        results.append(&build_row(entry));
    }

    *visible_entries.borrow_mut() = matches;

    if let Some(row) = results.row_at_index(0) {
        results.select_row(Some(&row));
    }
}

fn build_row(result: &SearchResult) -> gtk::ListBoxRow {
    let icon = if let Some(icon) = result.icon() {
        gtk::Image::builder()
            .gicon(icon)
            .pixel_size(24)
            .icon_size(gtk::IconSize::Normal)
            .build()
    } else {
        gtk::Image::from_icon_name(result.fallback_icon_name())
    };

    let title = gtk::Label::builder()
        .label(result.title())
        .halign(Align::Start)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["result-title"])
        .build();

    let subtitle_text = if result.executable().is_empty() {
        result.subtitle().to_string()
    } else {
        format!("{}  •  {}", result.subtitle(), result.executable())
    };

    let subtitle = gtk::Label::builder()
        .label(subtitle_text)
        .halign(Align::Start)
        .wrap(false)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["dim-label", "result-subtitle"])
        .build();

    let text = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .build();
    text.append(&title);
    text.append(&subtitle);

    let shortcut = gtk::Label::builder()
        .label("Enter")
        .halign(Align::End)
        .valign(Align::Center)
        .css_classes(["dim-label", "caption", "result-shortcut"])
        .build();

    let row_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    row_box.append(&icon);
    row_box.append(&text);
    row_box.append(&shortcut);

    gtk::ListBoxRow::builder()
        .child(&row_box)
        .activatable(true)
        .selectable(true)
        .build()
}

fn toggle_window(handles: &LauncherHandles) {
    if handles.window.is_visible() {
        dismiss_launcher(handles);
        return;
    }

    reset_launcher(handles);
    animate_show(handles);
}

fn reset_launcher(handles: &LauncherHandles) {
    handles.search_entry.set_text("");
    reset_launcher_results(handles);
    handles.search_entry.grab_focus();
}

fn reset_launcher_results(handles: &LauncherHandles) {
    refresh_results(
        &handles.index,
        "",
        &handles.sheet,
        &handles.results,
        &handles.status,
        &handles.scroller,
        &handles.window,
        &handles.visible_entries,
    );
}

fn dismiss_launcher(handles: &LauncherHandles) {
    if !handles.window.is_visible() {
        return;
    }

    animate_hide(handles);
}

fn hide_launcher_now(handles: &LauncherHandles) {
    cancel_animation(handles);
    handles.window.hide();
    handles.sheet.set_opacity(1.0);
    handles.search_entry.set_text("");
    reset_launcher_results(handles);
}

fn present_settings_window(
    app: &adw::Application,
    settings_window: &Rc<RefCell<Option<gtk::ApplicationWindow>>>,
) {
    if let Some(window) = settings_window.borrow().as_ref() {
        window.present();
        return;
    }

    let window = build_settings_window(app);
    window.present();
    *settings_window.borrow_mut() = Some(window);
}

fn build_settings_window(app: &adw::Application) -> gtk::ApplicationWindow {
    let title = gtk::Label::builder()
        .label("Settings")
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["settings-title"])
        .build();

    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Close")
        .css_classes(["flat", "settings-close"])
        .build();

    let header_inner = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .css_classes(["settings-header"])
        .build();
    header_inner.append(&title);
    header_inner.append(&close_button);

    let header = gtk::WindowHandle::builder().child(&header_inner).build();

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(20)
        .margin_top(20)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(&header);
    content.append(&settings_section(
        "General",
        &[("Global shortcut", TOGGLE_SHORTCUT_TRIGGER)],
    ));
    content.append(&settings_section(
        "About",
        &[
            ("Version", env!("CARGO_PKG_VERSION")),
            ("License", "MPL-2.0"),
        ],
    ));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rift Settings")
        .default_width(420)
        .default_height(280)
        .resizable(false)
        .modal(false)
        .decorated(false)
        .child(&content)
        .build();
    window.add_css_class("rift-settings-window");
    window.set_hide_on_close(true);
    window.connect_close_request(|window| {
        window.hide();
        glib::Propagation::Stop
    });

    close_button.connect_clicked({
        let window = window.clone();

        move |_| {
            window.hide();
        }
    });

    let escape = gtk::EventControllerKey::new();
    escape.set_propagation_phase(gtk::PropagationPhase::Capture);
    escape.connect_key_pressed({
        let window = window.clone();

        move |_, key, _, _| {
            if key == gdk::Key::Escape {
                window.hide();
                return true.into();
            }

            false.into()
        }
    });
    window.add_controller(escape);

    window
}

fn settings_section(title: &str, rows: &[(&str, &str)]) -> gtk::Box {
    let heading = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["settings-section-title"])
        .build();

    let group = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(["settings-group"])
        .build();
    for (label, value) in rows {
        group.append(&settings_row(label, value));
    }

    let section = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .css_classes(["settings-section"])
        .build();
    section.append(&heading);
    section.append(&group);

    section
}

fn settings_row(label: &str, value: &str) -> gtk::Box {
    let label = gtk::Label::builder()
        .label(label)
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["settings-row-label"])
        .build();

    let value = gtk::Label::builder()
        .label(value)
        .halign(Align::End)
        .css_classes(["settings-row-value"])
        .build();

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["settings-row"])
        .build();
    row.append(&label);
    row.append(&value);
    row
}

fn activate_result(result: &SearchResult, handles: &LauncherHandles) -> Result<(), String> {
    match result.action() {
        ResultAction::LaunchApp(app_info) => app_info
            .launch(&[], None::<&gio::AppLaunchContext>)
            .map(|_| {
                handles.window.hide();
            })
            .map_err(|error| error.to_string()),
        ResultAction::RunShell(command) => {
            launch_in_terminal(command)?;
            handles.window.hide();
            Ok(())
        }
        ResultAction::CopyText(text) => {
            let display = gdk::Display::default().ok_or("display unavailable")?;
            display.clipboard().set_text(text);
            Ok(())
        }
    }
}

fn start_global_shortcut_registration(handles: LauncherHandles) {
    glib::MainContext::default().spawn_local(async move {
        if let Err(error) = register_global_shortcut(handles).await {
            if env::var_os("RIFT_DEBUG").is_some() {
                eprintln!("rift: global shortcut unavailable: {error}");
            }
        }
    });
}

async fn register_global_shortcut(handles: LauncherHandles) -> Result<(), String> {
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

    if !portal_has_toggle_shortcut(&shortcuts, &session).await? {
        let request = shortcuts
            .bind_shortcuts(
                &session,
                &[NewShortcut::new(TOGGLE_SHORTCUT_ID, "Toggle Rift launcher")
                    .preferred_trigger(Some(TOGGLE_SHORTCUT_TRIGGER))],
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
) -> Result<bool, String> {
    let request = shortcuts
        .list_shortcuts(session, ListShortcutsOptions::default())
        .await
        .map_err(|error| error.to_string())?;
    let response = request.response().map_err(|error| error.to_string())?;

    Ok(response
        .shortcuts()
        .iter()
        .any(|shortcut| shortcut.id() == TOGGLE_SHORTCUT_ID))
}

fn launch_in_terminal(command: &str) -> Result<(), String> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let wrapped = format!("{command}; printf '\\n'; exec {}", shell_quote(&shell));
    let shell_args = shell_invocation_args(&shell, &wrapped);
    let xfce_command = format!(
        "{} {}",
        shell_quote(&shell),
        shell_args
            .iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" "),
    );

    let candidates: [(&str, &[&str]); 9] = [
        ("kgx", &["--"]),
        ("ptyxis", &["--"]),
        ("gnome-terminal", &["--"]),
        ("xfce4-terminal", &["--command"]),
        ("konsole", &["-e"]),
        ("kitty", &["-e"]),
        ("alacritty", &["-e"]),
        ("foot", &["-e"]),
        ("xterm", &["-e"]),
    ];

    for (program, prefix) in candidates {
        if !command_exists(program) {
            continue;
        }

        let mut process = Command::new(program);

        if program == "xfce4-terminal" {
            process.arg("--command").arg(&xfce_command);
        } else {
            for arg in prefix {
                process.arg(arg);
            }

            process.arg(&shell);
            for arg in &shell_args {
                process.arg(arg);
            }
        }

        if process.spawn().is_ok() {
            return Ok(());
        }
    }

    Err("no supported terminal emulator found".to_string())
}

fn command_exists(name: &str) -> bool {
    if name.contains('/') {
        return Path::new(name).is_file();
    }

    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths).any(|directory| directory.join(name).is_file())
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn shell_invocation_args(shell: &str, command: &str) -> Vec<String> {
    match Path::new(shell).file_name().and_then(|name| name.to_str()) {
        Some("fish") => vec!["-l".to_string(), "-c".to_string(), command.to_string()],
        _ => vec!["-lc".to_string(), command.to_string()],
    }
}

fn set_sheet_margins(sheet: &gtk::Box, margin: i32) {
    sheet.set_margin_top(margin);
    sheet.set_margin_bottom(margin);
    sheet.set_margin_start(margin);
    sheet.set_margin_end(margin);
}

fn animate_show(handles: &LauncherHandles) {
    cancel_animation(handles);
    handles.sheet.set_opacity(0.0);
    handles.window.present();
    handles.search_entry.grab_focus();

    let sheet = handles.sheet.clone();
    let animation_source = handles.animation_source.clone();
    let started_at = Instant::now();

    let source = glib::timeout_add_local(Duration::from_millis(FADE_FRAME_MS), move || {
        let progress = (started_at.elapsed().as_millis() as f64 / FADE_DURATION_MS as f64).min(1.0);
        sheet.set_opacity(progress);

        if progress >= 1.0 {
            *animation_source.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    *handles.animation_source.borrow_mut() = Some(source);
}

fn animate_hide(handles: &LauncherHandles) {
    cancel_animation(handles);

    let sheet = handles.sheet.clone();
    let fade_handles = handles.clone();
    let animation_source = handles.animation_source.clone();
    let start_opacity = sheet.opacity();
    let started_at = Instant::now();

    let source = glib::timeout_add_local(Duration::from_millis(FADE_FRAME_MS), move || {
        let progress = (started_at.elapsed().as_millis() as f64 / FADE_DURATION_MS as f64).min(1.0);
        let opacity = (start_opacity * (1.0 - progress)).max(0.0);
        sheet.set_opacity(opacity);

        if progress >= 1.0 {
            fade_handles.window.hide();
            reset_launcher(&fade_handles);
            fade_handles.sheet.set_opacity(1.0);
            *animation_source.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    *handles.animation_source.borrow_mut() = Some(source);
}

fn cancel_animation(handles: &LauncherHandles) {
    if let Some(source) = handles.animation_source.borrow_mut().take() {
        source.remove();
    }
}

fn warn_if_dev_desktop_entry_missing(app: &adw::Application) {
    if !cfg!(debug_assertions) {
        return;
    }

    let desktop_entry = desktop_entry_path();
    if desktop_entry.is_file() {
        return;
    }

    let notification = gio::Notification::new("Rift setup required");
    notification.set_body(Some(
        "Install the dev desktop entry with ./scripts/install-dev-desktop-entry.sh to enable GNOME global shortcuts.",
    ));
    app.send_notification(Some("dev-desktop-entry"), &notification);
}

fn desktop_entry_path() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("applications")
        .join(format!("{APP_ID}.desktop"))
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        r#"
        window.rift-window,
        window.rift-window.background,
        window.rift-window .background,
        window.rift-window decoration,
        window.rift-window contents,
        window.rift-window > box,
        window.rift-window box.spotlight-sheet {
            background-color: transparent;
            border: none;
            border-radius: 0;
        }

        window.rift-window box.spotlight-sheet,
        window.rift-window box.spotlight-sheet.collapsed {
            min-width: 620px;
            background-color: transparent;
            border: none;
            padding: 0;
            margin: 0;
        }

        window.rift-window box.spotlight-sheet.expanded {
            min-width: 620px;
            background-color: #1f1f22;
            border: 1px solid #34343a;
            border-radius: 18px;
            padding: 0;
            margin: 0;
        }

        window.rift-window > widget,
        window.rift-window > box,
        window.rift-window > box > box {
            margin: 0;
            padding: 0;
        }

        window.rift-window box.search-row {
            min-height: 52px;
            border-radius: 26px;
            background-color: #1f1f22;
            border: 1px solid #34343a;
            padding: 0;
        }

        window.rift-window box.spotlight-sheet.expanded box.search-row {
            min-height: 58px;
            border-radius: 0;
            background-color: transparent;
            border: none;
            padding: 0;
        }

        window.rift-window entry.search-field {
            min-height: 50px;
            padding: 0 12px 0 18px;
            margin: 0;
            border-radius: 26px;
            background-color: transparent;
            color: #f5f5f6;
            caret-color: #f5f5f6;
            border: none;
            box-shadow: none;
            font-size: 18px;
            font-weight: 400;
            letter-spacing: -0.01em;
        }

        window.rift-window box.spotlight-sheet.expanded entry.search-field {
            min-height: 58px;
            padding: 0 12px 0 20px;
            border-radius: 0;
        }

        window.rift-window box.settings-button {
            min-width: 28px;
            min-height: 28px;
            margin-top: 0;
            margin-bottom: 0;
            margin-left: 6px;
            margin-right: 12px;
            padding: 0;
            border-radius: 14px;
            background-color: transparent;
            border: none;
            color: #9a9aa1;
            transition: background-color 120ms ease, color 120ms ease;
        }

        window.rift-window box.settings-button.hover {
            background-color: alpha(#ffffff, 0.08);
            color: #f3f4f6;
        }

        window.rift-window box.settings-button.active {
            background-color: alpha(#ffffff, 0.14);
            color: #ffffff;
        }

        window.rift-window box.settings-button image {
            -gtk-icon-size: 14px;
            margin: 0;
            padding: 0;
            color: inherit;
        }

        window.rift-window box.spotlight-sheet.expanded box.settings-button {
            margin-right: 16px;
        }

        window.rift-window entry.search-field > image {
            color: #85858c;
            margin-right: 8px;
            -gtk-icon-size: 16px;
        }

        window.rift-window entry.search-field > text > placeholder {
            color: #77777e;
        }

        window.rift-window entry.search-field:focus,
        window.rift-window entry.search-field:focus-within {
            outline: none;
            background-color: transparent;
            border: none;
            box-shadow: none;
        }

        window.rift-window box.search-row:focus-within {
            border: 1px solid #44444c;
        }

        window.rift-window box.spotlight-sheet.expanded box.search-row:focus-within {
            border: none;
        }

        window.rift-window entry.search-field > text {
            background-color: transparent;
        }
        window.rift-window entry.search-field > text > selection {
            background-color: #3f5273;
        }

        window.rift-window label.result-meta {
            color: #85858c;
            margin: 0 14px 6px 14px;
        }

        window.rift-window scrolledwindow.results-scroll {
            background-color: transparent;
            border: none;
            margin: 0 6px 6px 6px;
        }

        window.rift-window list.results-list {
            background-color: transparent;
        }

        window.rift-window list.results-list row {
            border-radius: 10px;
            margin: 0;
            background-color: transparent;
        }

        window.rift-window list.results-list row:selected,
        window.rift-window list.results-list row:hover {
            background-color: #34343a;
        }

        window.rift-window label.result-title {
            color: #f3f4f6;
            font-size: 16px;
            font-weight: 500;
        }

        window.rift-window label.result-subtitle {
            color: #7e7e86;
            font-size: 12px;
        }

        window.rift-window label.result-shortcut {
            color: #77777e;
        }

        window.rift-settings-window {
            background-color: #1c1c1f;
            color: #f4f4f5;
            border-radius: 12px;
            border: 1px solid #2e2e33;
        }

        window.rift-settings-window box.settings-header {
            margin-bottom: 8px;
            min-height: 28px;
        }

        window.rift-settings-window label.settings-title {
            color: #f4f4f5;
            font-size: 15px;
            font-weight: 600;
            letter-spacing: -0.01em;
        }

        window.rift-settings-window button.settings-close {
            min-width: 22px;
            min-height: 22px;
            padding: 0;
            border-radius: 999px;
            background-color: transparent;
            border: none;
            color: #85858c;
        }

        window.rift-settings-window button.settings-close:hover {
            background-color: alpha(#ffffff, 0.08);
            color: #f4f4f5;
        }

        window.rift-settings-window button.settings-close image {
            -gtk-icon-size: 12px;
        }

        window.rift-settings-window box.settings-section {
            background-color: transparent;
            border: none;
            padding: 0;
        }

        window.rift-settings-window label.settings-section-title {
            color: #8b8b94;
            font-size: 11px;
            font-weight: 500;
            letter-spacing: 0.04em;
            margin: 4px 2px 8px 2px;
        }

        window.rift-settings-window box.settings-group {
            background-color: #232327;
            border: 1px solid #2e2e33;
            border-radius: 8px;
        }

        window.rift-settings-window box.settings-group > box.settings-row {
            padding: 10px 14px;
            min-height: 24px;
            border-bottom: 1px solid #2e2e33;
        }

        window.rift-settings-window box.settings-group > box.settings-row:last-child {
            border-bottom: none;
        }

        window.rift-settings-window label.settings-row-label {
            color: #e4e4e7;
            font-size: 13px;
        }

        window.rift-settings-window label.settings-row-value {
            color: #85858c;
            font-size: 12px;
        }
        "#,
    );

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("display unavailable"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}
