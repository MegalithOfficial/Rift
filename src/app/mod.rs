mod animation;
mod css;
mod settings;
mod shortcuts;
mod terminal;

use std::{cell::RefCell, env, rc::Rc};

use adw::prelude::*;
use gio::ApplicationHoldGuard;
use gtk::{Align, Orientation, gdk};
use gtk4_layer_shell::{
    Edge, KeyboardMode, Layer, LayerShell, is_supported as layer_shell_supported,
};

use crate::{
    config::AppConfig,
    model::{AppIndex, QueryOptions, ResultAction, SearchResult},
};

const APP_ID: &str = "dev.rift.launcher";
const TOGGLE_SHORTCUT_ID: &str = "toggle-launcher";
const EMPTY_HEIGHT: i32 = 64;
const NO_MATCHES_HEIGHT: i32 = 104;
const RESULT_ROW_HEIGHT: i32 = 60;
const RESULTS_BASE_HEIGHT: i32 = 108;
const SHEET_MARGIN: i32 = 0;
const FADE_DURATION_MS: u64 = 120;
const FADE_FRAME_MS: u64 = 16;

#[derive(Clone)]
pub(super) struct LauncherHandles {
    animation_source: Rc<RefCell<Option<glib::SourceId>>>,
    settings_window: Rc<RefCell<Option<gtk::ApplicationWindow>>>,
    config: Rc<RefCell<AppConfig>>,
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
                shortcuts::start_global_shortcut_registration(handles.clone());
                *launcher.borrow_mut() = Some(handles);
            }
        }
    });
    app
}

fn build_ui(app: &adw::Application) -> LauncherHandles {
    let config = Rc::new(RefCell::new(AppConfig::load()));
    let runtime = config.borrow().runtime();
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
        .default_width(runtime.window_width)
        .default_height(EMPTY_HEIGHT)
        .modal(true)
        .deletable(false)
        .resizable(false)
        .child(&sheet)
        .build();

    css::install_css();
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
        config: config.clone(),
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
            settings::present_settings_window(&app, &handles);
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
        refresh_results(&handles, "");
    }

    search_entry.connect_search_changed({
        let handles = handles.clone();

        move |entry| {
            refresh_results(&handles, &entry.text());
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
    apply_runtime_config(&handles);
    animation::animate_show(&handles);

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
    window.set_margin(Edge::Top, 0);
}

fn is_gnome_session() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("XDG_SESSION_DESKTOP"))
        .map(|desktop| desktop.to_ascii_lowercase().contains("gnome"))
        .unwrap_or(false)
}

fn refresh_results(handles: &LauncherHandles, query: &str) {
    while let Some(child) = handles.results.first_child() {
        handles.results.remove(&child);
    }

    let runtime = handles.config.borrow().runtime();
    let matches = handles.index.query(
        query,
        QueryOptions {
            shell_enabled: runtime.shell_enabled,
            calculator_enabled: runtime.calculator_enabled,
        },
    );
    let query = query.trim();

    if query.is_empty() {
        handles.sheet.add_css_class("collapsed");
        handles.sheet.remove_css_class("expanded");
        set_sheet_margins(&handles.sheet, SHEET_MARGIN);
        handles
            .status
            .set_text(&format!("{} apps", handles.index.len()));
        handles.status.set_visible(false);
        handles.scroller.set_visible(false);
        handles.scroller.set_height_request(0);
        handles
            .window
            .set_default_size(runtime.window_width, EMPTY_HEIGHT);
        handles
            .window
            .set_size_request(runtime.window_width, EMPTY_HEIGHT);
        handles.window.queue_resize();
    } else if matches.is_empty() {
        handles.sheet.remove_css_class("collapsed");
        handles.sheet.add_css_class("expanded");
        set_sheet_margins(&handles.sheet, SHEET_MARGIN);
        handles.status.set_text("No matches");
        handles.status.set_visible(true);
        handles.scroller.set_visible(false);
        handles.scroller.set_height_request(0);
        handles
            .window
            .set_default_size(runtime.window_width, NO_MATCHES_HEIGHT);
        handles
            .window
            .set_size_request(runtime.window_width, NO_MATCHES_HEIGHT);
        handles.window.queue_resize();
    } else {
        handles.sheet.remove_css_class("collapsed");
        handles.sheet.add_css_class("expanded");
        set_sheet_margins(&handles.sheet, SHEET_MARGIN);
        handles
            .status
            .set_text(&format!("{} results", matches.len()));
        handles.status.set_visible(true);
        handles.scroller.set_visible(true);
        let visible_rows = matches.len().min(runtime.max_visible_results) as i32;
        let max_results_height =
            RESULTS_BASE_HEIGHT + runtime.max_visible_results as i32 * RESULT_ROW_HEIGHT;
        let results_height =
            (RESULTS_BASE_HEIGHT + visible_rows * RESULT_ROW_HEIGHT).min(max_results_height);
        let scroller_height =
            (visible_rows * RESULT_ROW_HEIGHT).min(max_results_height - RESULTS_BASE_HEIGHT);
        handles.scroller.set_height_request(scroller_height);
        handles
            .window
            .set_default_size(runtime.window_width, results_height);
        handles
            .window
            .set_size_request(runtime.window_width, results_height);
        handles.window.queue_resize();
    }

    for entry in &matches {
        handles.results.append(&build_row(entry));
    }

    *handles.visible_entries.borrow_mut() = matches;

    if let Some(row) = handles.results.row_at_index(0) {
        handles.results.select_row(Some(&row));
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

pub(super) fn toggle_window(handles: &LauncherHandles) {
    if handles.window.is_visible() {
        dismiss_launcher(handles);
        return;
    }

    reset_launcher(handles);
    animation::animate_show(handles);
}

pub(super) fn reset_launcher(handles: &LauncherHandles) {
    handles.search_entry.set_text("");
    reset_launcher_results(handles);
    handles.search_entry.grab_focus();
}

fn reset_launcher_results(handles: &LauncherHandles) {
    refresh_results(handles, "");
}

fn dismiss_launcher(handles: &LauncherHandles) {
    if !handles.window.is_visible() {
        return;
    }

    animation::animate_hide(handles);
}

fn hide_launcher_now(handles: &LauncherHandles) {
    animation::cancel_animation(handles);
    handles.window.hide();
    handles.sheet.set_opacity(1.0);
    handles.search_entry.set_text("");
    reset_launcher_results(handles);
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
            terminal::launch_in_terminal(command)?;
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

fn set_sheet_margins(sheet: &gtk::Box, margin: i32) {
    sheet.set_margin_top(margin);
    sheet.set_margin_bottom(margin);
    sheet.set_margin_start(margin);
    sheet.set_margin_end(margin);
}

fn warn_if_dev_desktop_entry_missing(app: &adw::Application) {
    if !cfg!(debug_assertions) {
        return;
    }

    let desktop_entry = shortcuts::desktop_entry_path(APP_ID);
    if desktop_entry.is_file() {
        return;
    }

    let notification = gio::Notification::new("Rift setup required");
    notification.set_body(Some(
        "Install the dev desktop entry with ./scripts/install-dev-desktop-entry.sh to enable GNOME global shortcuts.",
    ));
    app.send_notification(Some("dev-desktop-entry"), &notification);
}

pub(super) fn current_config(handles: &LauncherHandles) -> AppConfig {
    handles.config.borrow().clone()
}

pub(super) fn save_config(handles: &LauncherHandles, config: AppConfig) -> Result<(), String> {
    config.save()?;
    *handles.config.borrow_mut() = config;
    apply_runtime_config(handles);
    Ok(())
}

pub(super) fn apply_runtime_config(handles: &LauncherHandles) {
    let runtime = handles.config.borrow().runtime();
    if !is_gnome_session() && layer_shell_supported() {
        handles
            .window
            .set_margin(Edge::Top, runtime.window_top_margin);
    }
    reset_launcher_results(handles);
}
