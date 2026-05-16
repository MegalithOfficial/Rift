mod animation;
pub(crate) mod css;
mod settings;
mod shortcuts;
mod terminal;
mod welcome;

use std::{cell::RefCell, env, ffi::OsString, process::Command, rc::Rc};

use adw::prelude::*;
use gio::{ApplicationFlags, ApplicationHoldGuard};
use gtk::{Align, Orientation, gdk};
use gtk4_layer_shell::{
    Edge, KeyboardMode, Layer, LayerShell, is_supported as layer_shell_supported,
};

use crate::{
    config::{AppConfig, RenderMonitor},
    model::{AppIndex, QueryOptions, ResultAction, SearchResult},
    theme,
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
    shortcut_task: Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    shortcut_session: Rc<
        RefCell<Option<ashpd::desktop::Session<ashpd::desktop::global_shortcuts::GlobalShortcuts>>>,
    >,
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

pub(super) enum SaveOutcome {
    Applied,
    RestartRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CliCommand {
    Activate,
    Background,
    Show,
    Hide,
    Toggle,
    Welcome,
    Quit,
}

pub fn build() -> adw::Application {
    let launcher = Rc::new(RefCell::new(None::<LauncherHandles>));
    let resident_hold = Rc::new(RefCell::new(None::<ApplicationHoldGuard>));
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.connect_activate({
        let launcher = launcher.clone();
        let resident_hold = resident_hold.clone();

        move |app| {
            let handles = ensure_launcher(app, &launcher, &resident_hold, true);
            toggle_window(&handles);
        }
    });
    app.connect_command_line({
        let launcher = launcher.clone();
        let resident_hold = resident_hold.clone();

        move |app, command_line| {
            let command = parse_cli_command(command_line.arguments());
            handle_cli_command(app, &launcher, &resident_hold, command);
            0.into()
        }
    });
    app
}

fn ensure_launcher(
    app: &adw::Application,
    launcher: &Rc<RefCell<Option<LauncherHandles>>>,
    resident_hold: &Rc<RefCell<Option<ApplicationHoldGuard>>>,
    show_on_create: bool,
) -> LauncherHandles {
    if let Some(handles) = launcher.borrow().as_ref().cloned() {
        return handles;
    }

    *resident_hold.borrow_mut() = Some(app.hold());
    let handles = build_ui(app, show_on_create);
    warn_if_dev_desktop_entry_missing(app);
    let _ = sync_launch_at_login(current_config(&handles).launcher.launch_at_login);
    shortcuts::start_global_shortcut_registration(handles.clone());
    *launcher.borrow_mut() = Some(handles.clone());

    if welcome::show_if_first_run(app, &handles) {
        // First-run: hide the launcher so the welcome window owns the screen.
        hide_launcher_now(&handles);
    }

    handles
}

fn handle_cli_command(
    app: &adw::Application,
    launcher: &Rc<RefCell<Option<LauncherHandles>>>,
    resident_hold: &Rc<RefCell<Option<ApplicationHoldGuard>>>,
    command: CliCommand,
) {
    match command {
        CliCommand::Activate | CliCommand::Toggle => {
            let handles = ensure_launcher(app, launcher, resident_hold, true);
            toggle_window(&handles);
        }
        CliCommand::Background => {
            let _ = ensure_launcher(app, launcher, resident_hold, false);
        }
        CliCommand::Show => {
            let handles = ensure_launcher(app, launcher, resident_hold, true);
            show_window(&handles);
        }
        CliCommand::Hide => {
            if let Some(handles) = launcher.borrow().as_ref().cloned() {
                dismiss_launcher(&handles);
            }
        }
        CliCommand::Quit => {
            if let Some(handles) = launcher.borrow().as_ref().cloned() {
                hide_launcher_now(&handles);
            }
            app.quit();
        }
        CliCommand::Welcome => {
            let handles = ensure_launcher(app, launcher, resident_hold, false);
            hide_launcher_now(&handles);
            welcome::present_now(app, &handles);
        }
    }
}

fn parse_cli_command(arguments: impl IntoIterator<Item = OsString>) -> CliCommand {
    let args = arguments
        .into_iter()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--welcome") {
        CliCommand::Welcome
    } else if args.iter().any(|arg| arg == "--show") {
        CliCommand::Show
    } else if args.iter().any(|arg| arg == "--background") {
        CliCommand::Background
    } else if args.iter().any(|arg| arg == "--hide") {
        CliCommand::Hide
    } else if args.iter().any(|arg| arg == "--toggle") {
        CliCommand::Toggle
    } else if args.iter().any(|arg| arg == "--quit") {
        CliCommand::Quit
    } else {
        CliCommand::Activate
    }
}

fn build_ui(app: &adw::Application, show_on_start: bool) -> LauncherHandles {
    let config = Rc::new(RefCell::new(AppConfig::load()));
    let runtime = config.borrow().runtime();
    let index = Rc::new(AppIndex::load());
    let animation_source = Rc::new(RefCell::new(None::<glib::SourceId>));
    let settings_window = Rc::new(RefCell::new(None::<gtk::ApplicationWindow>));
    let shortcut_task = Rc::new(RefCell::new(None::<glib::JoinHandle<()>>));
    let shortcut_session = Rc::new(RefCell::new(
        None::<ashpd::desktop::Session<ashpd::desktop::global_shortcuts::GlobalShortcuts>>,
    ));
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

    let active_theme = theme::load_theme(&config.borrow().theme.active);
    css::install_css(&active_theme.css);
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
        shortcut_task: shortcut_task.clone(),
        shortcut_session: shortcut_session.clone(),
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

            if current_config(&handles).launcher.keep_open_on_focus_loss {
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

    apply_runtime_config(&handles);
    handles.sheet.set_opacity(0.0);
    if show_on_start {
        search_entry.grab_focus();
        animation::animate_show(&handles);
    } else {
        handles.window.hide();
        handles.sheet.set_opacity(1.0);
    }

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
        handles.results.append(&build_row(entry, query));
    }

    *handles.visible_entries.borrow_mut() = matches;

    if let Some(row) = handles.results.row_at_index(0) {
        handles.results.select_row(Some(&row));
    }
}

fn build_row(result: &SearchResult, query: &str) -> gtk::ListBoxRow {
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
        .halign(Align::Start)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["result-title"])
        .build();
    title.set_use_markup(true);
    title.set_markup(&highlight_title_markup(result.title(), query));

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

    show_window(handles);
}

fn show_window(handles: &LauncherHandles) {
    if handles.window.is_visible() {
        handles.search_entry.grab_focus();
        return;
    }

    apply_runtime_config(handles);
    let runtime = current_config(handles).runtime();
    if runtime.clear_input_on_hide {
        reset_launcher(handles);
    } else {
        refresh_results(handles, &handles.search_entry.text());
        handles.search_entry.grab_focus();
    }
    apply_render_monitor(handles, runtime.render_monitor);
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
    if current_config(handles).launcher.clear_input_on_hide {
        handles.search_entry.set_text("");
        reset_launcher_results(handles);
    }
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

fn highlight_title_markup(title: &str, query: &str) -> String {
    let matched_indices = matched_title_indices(title, query);
    if matched_indices.is_empty() {
        return glib::markup_escape_text(title).to_string();
    }

    let mut markup = String::new();
    for (index, character) in title.chars().enumerate() {
        let escaped = glib::markup_escape_text(&character.to_string());
        if matched_indices.contains(&index) {
            markup.push_str("<span foreground=\"#ffffff\" weight=\"700\">");
            markup.push_str(&escaped);
            markup.push_str("</span>");
        } else {
            markup.push_str(&escaped);
        }
    }

    markup
}

fn matched_title_indices(title: &str, query: &str) -> Vec<usize> {
    let query_chars = query
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect::<Vec<_>>();

    if query_chars.is_empty() {
        return Vec::new();
    }

    let mut matched = Vec::new();
    let mut query_index = 0usize;

    for (index, character) in title.chars().enumerate() {
        if query_index >= query_chars.len() {
            break;
        }

        if !character.is_alphanumeric() {
            continue;
        }

        let lower = character.to_lowercase().next().unwrap_or(character);
        if lower == query_chars[query_index] {
            matched.push(index);
            query_index += 1;
        }
    }

    if query_index == query_chars.len() {
        return matched;
    }

    let lowered_title = title.to_lowercase();
    let lowered_query = query.trim().to_lowercase();
    if lowered_query.is_empty() {
        return Vec::new();
    }

    lowered_title
        .find(&lowered_query)
        .map(|byte_start| {
            title[..byte_start].chars().count()
                ..title[..byte_start + lowered_query.len()].chars().count()
        })
        .map(|range| range.collect())
        .unwrap_or_default()
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

pub(super) fn save_config(
    handles: &LauncherHandles,
    config: AppConfig,
) -> Result<SaveOutcome, String> {
    let shortcut_changed =
        handles.config.borrow().launcher.shortcut_trigger != config.launcher.shortcut_trigger;
    config.save()?;
    *handles.config.borrow_mut() = config;
    sync_launch_at_login(handles.config.borrow().launcher.launch_at_login)?;
    apply_runtime_config(handles);
    if shortcut_changed {
        shortcuts::restart_global_shortcut_registration(handles);
        if shortcuts::shortcut_change_needs_restart() {
            return Ok(SaveOutcome::RestartRequired);
        }
    }
    Ok(SaveOutcome::Applied)
}

pub(super) fn apply_runtime_config(handles: &LauncherHandles) {
    let runtime = handles.config.borrow().runtime();
    if !is_gnome_session() && layer_shell_supported() {
        handles
            .window
            .set_margin(Edge::Top, runtime.window_top_margin);
    }
    apply_render_monitor(handles, runtime.render_monitor);
    reset_launcher_results(handles);
}

fn apply_render_monitor(handles: &LauncherHandles, preference: RenderMonitor) {
    if is_gnome_session() || !layer_shell_supported() || !handles.window.is_layer_window() {
        return;
    }

    let monitor = match preference {
        RenderMonitor::Cursor => cursor_monitor().or_else(default_monitor),
        RenderMonitor::Default => default_monitor(),
    };
    handles.window.set_monitor(monitor.as_ref());
}

fn default_monitor() -> Option<gdk::Monitor> {
    let display = gdk::Display::default()?;
    let monitors = display.monitors();
    let first = monitors.item(0)?;
    first.downcast::<gdk::Monitor>().ok()
}

fn cursor_monitor() -> Option<gdk::Monitor> {
    let display = gdk::Display::default()?;
    let seat = display.default_seat()?;
    let pointer = seat.pointer()?;
    let (surface, _, _) = pointer.surface_at_position();
    surface.and_then(|surface| display.monitor_at_surface(&surface))
}

fn sync_launch_at_login(enabled: bool) -> Result<(), String> {
    let autostart_dir = env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("autostart");
    let autostart_file = autostart_dir.join(format!("{APP_ID}.desktop"));

    if !enabled {
        if autostart_file.exists() {
            std::fs::remove_file(&autostart_file).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }

    std::fs::create_dir_all(&autostart_dir).map_err(|error| error.to_string())?;
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let exec = format!(
        "{} --background",
        glib::shell_quote(executable).to_string_lossy()
    );
    let desktop_entry = format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Rift\nComment=Start Rift in the background\nExec={exec}\nIcon=system-search-symbolic\nTerminal=false\nNoDisplay=true\nStartupNotify=false\nX-GNOME-Autostart-enabled=true\n"
    );
    std::fs::write(autostart_file, desktop_entry).map_err(|error| error.to_string())
}

pub(super) fn restart_application(handles: &LauncherHandles) -> Result<(), String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if let Some(task) = handles.shortcut_task.borrow_mut().take() {
        task.abort();
    }
    if let Some(session) = handles.shortcut_session.borrow_mut().take() {
        glib::MainContext::default().spawn_local(async move {
            let _ = session.close().await;
        });
    }
    animation::cancel_animation(handles);
    handles.window.hide();

    Command::new("sh")
        .arg("-lc")
        .arg("sleep 0.35; exec \"$@\"")
        .arg("rift-relaunch")
        .arg(&executable)
        .args(&args)
        .spawn()
        .map_err(|error| error.to_string())?;

    if let Some(app) = handles.window.application() {
        app.quit();
    } else {
        handles.window.close();
    }

    glib::timeout_add_local_once(std::time::Duration::from_millis(800), || {
        std::process::exit(0);
    });

    Ok(())
}
