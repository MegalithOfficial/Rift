use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use gtk::{Align, Orientation, gdk};

use crate::config::{AppConfig, RenderMonitor};
use crate::theme;

use super::{LauncherHandles, SaveOutcome, css, current_config, restart_application, save_config};

pub(super) fn present_settings_window(app: &adw::Application, handles: &LauncherHandles) {
    if let Some(window) = handles.settings_window.borrow().as_ref() {
        window.present();
        return;
    }

    let window = build_settings_window(app, handles);
    window.present();
    *handles.settings_window.borrow_mut() = Some(window);
}

fn build_settings_window(
    app: &adw::Application,
    handles: &LauncherHandles,
) -> gtk::ApplicationWindow {
    let config = current_config(handles);

    // --- Header -------------------------------------------------------------
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

    // --- Status banner ------------------------------------------------------
    let status_banner = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .visible(false)
        .css_classes(["settings-banner"])
        .build();
    let status_icon = gtk::Image::builder()
        .icon_name("emblem-synchronizing-symbolic")
        .pixel_size(14)
        .build();
    let status_label = gtk::Label::builder()
        .halign(Align::Start)
        .hexpand(true)
        .wrap(true)
        .css_classes(["settings-banner-text"])
        .build();
    let status_action = gtk::Button::builder()
        .label("Restart")
        .css_classes(["settings-banner-action"])
        .visible(false)
        .build();
    let status_dismiss = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Dismiss")
        .css_classes(["flat", "settings-banner-dismiss"])
        .build();
    status_banner.append(&status_icon);
    status_banner.append(&status_label);
    status_banner.append(&status_action);
    status_banner.append(&status_dismiss);

    let banner_state = SettingsBanner {
        container: status_banner.clone(),
        label: status_label.clone(),
        icon: status_icon.clone(),
        action: status_action.clone(),
    };

    status_dismiss.connect_clicked({
        let banner = banner_state.clone();
        move |_| banner.hide()
    });

    // --- Launcher controls --------------------------------------------------
    let shortcut_setup_card = build_shortcut_setup_card();

    let results_adjustment = gtk::Adjustment::new(
        config.launcher.max_visible_results as f64,
        1.0,
        12.0,
        1.0,
        1.0,
        0.0,
    );
    let results_spin = gtk::SpinButton::builder()
        .adjustment(&results_adjustment)
        .numeric(true)
        .css_classes(["settings-spin"])
        .build();
    results_spin.set_widget_name("results");

    let autostart_switch = gtk::Switch::builder()
        .active(config.launcher.launch_at_login)
        .valign(Align::Center)
        .build();
    autostart_switch.set_widget_name("launch-at-login");

    let clear_input_switch = gtk::Switch::builder()
        .active(config.launcher.clear_input_on_hide)
        .valign(Align::Center)
        .build();
    clear_input_switch.set_widget_name("clear-input-on-hide");

    let monitor_model = gtk::StringList::new(&["Monitor with cursor", "Default monitor"]);
    let monitor_dropdown = gtk::DropDown::builder()
        .model(&monitor_model)
        .css_classes(["settings-dropdown"])
        .build();
    monitor_dropdown.set_widget_name("render-monitor");
    monitor_dropdown.set_selected(match config.launcher.render_monitor {
        RenderMonitor::Cursor => 0,
        RenderMonitor::Default => 1,
    });

    let keep_focus_switch = gtk::Switch::builder()
        .active(config.launcher.keep_open_on_focus_loss)
        .valign(Align::Center)
        .build();
    keep_focus_switch.set_widget_name("keep-open-on-focus-loss");

    // --- Providers controls -------------------------------------------------
    let shell_switch = gtk::Switch::builder()
        .active(config.providers.shell_enabled)
        .valign(Align::Center)
        .build();
    shell_switch.set_widget_name("shell");

    let calculator_switch = gtk::Switch::builder()
        .active(config.providers.calculator_enabled)
        .valign(Align::Center)
        .build();
    calculator_switch.set_widget_name("calculator");

    // --- Theme controls -----------------------------------------------------
    let theme_entries = Rc::new(RefCell::new(theme::list_available_themes()));
    let theme_model = gtk::StringList::new(&[]);
    for entry in theme_entries.borrow().iter() {
        theme_model.append(&entry.manifest.name);
    }
    let theme_dropdown = gtk::DropDown::builder()
        .model(&theme_model)
        .css_classes(["settings-dropdown"])
        .build();
    theme_dropdown.set_widget_name("active-theme");
    let initial_index = theme_entries
        .borrow()
        .iter()
        .position(|entry| entry.manifest.id == config.theme.active)
        .unwrap_or(0) as u32;
    theme_dropdown.set_selected(initial_index);

    let theme_path_label = gtk::Label::builder()
        .label(
            theme_entries
                .borrow()
                .get(initial_index as usize)
                .map(|entry| entry.path.display().to_string())
                .unwrap_or_default(),
        )
        .halign(Align::Start)
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["settings-mono-value"])
        .build();

    let theme_meta_label = gtk::Label::builder()
        .label(theme_meta_text(
            theme_entries.borrow().get(initial_index as usize),
        ))
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["settings-row-helper"])
        .build();

    let validation_icon = gtk::Image::builder()
        .icon_name("emblem-ok-symbolic")
        .pixel_size(12)
        .build();
    let validation_text = gtk::Label::builder()
        .label("Valid")
        .halign(Align::Start)
        .xalign(0.0)
        .css_classes(["settings-validation-text"])
        .build();
    let validation_pill = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(Align::Start)
        .css_classes(["settings-validation-pill", "ok"])
        .build();
    validation_pill.append(&validation_icon);
    validation_pill.append(&validation_text);

    let reload_theme_btn = action_button("Reload", "view-refresh-symbolic");
    let validate_theme_btn = action_button("Validate", "object-select-symbolic");
    let open_folder_btn = action_button("Open Folder", "folder-symbolic");
    let refresh_default_btn = action_button("Reset Default", "edit-undo-symbolic");

    let theme_actions = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(Align::Start)
        .css_classes(["settings-theme-actions"])
        .build();
    theme_actions.append(&reload_theme_btn);
    theme_actions.append(&validate_theme_btn);
    theme_actions.append(&open_folder_btn);
    theme_actions.append(&refresh_default_btn);

    // --- Section panels (stacked) ------------------------------------------
    let launcher_panel = panel(&[
        wrap_section(
            "Shortcut setup",
            shortcut_setup_card.clone().upcast::<gtk::Widget>(),
        ),
        section(
            "General",
            group(&[
                labeled_row(
                    "Visible results",
                    Some("Maximum items shown without scrolling."),
                    results_spin.clone().upcast::<gtk::Widget>(),
                ),
                labeled_row(
                    "Launch at login",
                    Some("Start Rift automatically in the background."),
                    autostart_switch.clone().upcast::<gtk::Widget>(),
                ),
            ]),
        ),
        section(
            "Behavior",
            group(&[
                labeled_row(
                    "Clear input on hide",
                    Some("Reset the query whenever the launcher is dismissed."),
                    clear_input_switch.clone().upcast::<gtk::Widget>(),
                ),
                labeled_row(
                    "Render on",
                    Some("Best effort. Monitor targeting depends on the desktop/compositor."),
                    monitor_dropdown.clone().upcast::<gtk::Widget>(),
                ),
                labeled_row(
                    "Stay open on focus loss",
                    Some("Keep Rift open when another window takes focus."),
                    keep_focus_switch.clone().upcast::<gtk::Widget>(),
                ),
            ]),
        ),
    ]);

    let providers_panel = panel(&[section(
        "Providers",
        group(&[
            labeled_row(
                "Shell commands",
                Some("Prefix queries with > to run a command."),
                shell_switch.clone().upcast::<gtk::Widget>(),
            ),
            labeled_row(
                "Calculator",
                Some("Evaluate math expressions inline."),
                calculator_switch.clone().upcast::<gtk::Widget>(),
            ),
        ]),
    )]);
    providers_panel.add_css_class("spacious");

    let theme_panel = build_theme_panel(
        &theme_dropdown,
        &theme_path_label,
        &theme_meta_label,
        &validation_pill,
        &theme_actions,
    );

    // --- Sidebar + stack ----------------------------------------------------
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .transition_duration(150)
        .vexpand(true)
        .hexpand(true)
        .build();
    stack.add_named(&launcher_panel, Some("launcher"));
    stack.add_named(&providers_panel, Some("providers"));
    stack.add_named(&theme_panel, Some("theme"));

    let sidebar = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["settings-sidebar"])
        .build();
    sidebar.append(&sidebar_row("Launcher", "edit-find-symbolic", "launcher"));
    sidebar.append(&sidebar_row("Providers", "view-grid-symbolic", "providers"));
    sidebar.append(&sidebar_row(
        "Theme",
        "applications-graphics-symbolic",
        "theme",
    ));
    sidebar.select_row(sidebar.row_at_index(0).as_ref());

    sidebar.connect_row_selected({
        let stack = stack.clone();
        move |_, row| {
            if let Some(row) = row {
                let name = unsafe { row.data::<String>("panel-id") };
                if let Some(name) = name {
                    let name = unsafe { name.as_ref().clone() };
                    stack.set_visible_child_name(&name);
                }
            }
        }
    });

    let body = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(14)
        .vexpand(true)
        .hexpand(true)
        .css_classes(["settings-body"])
        .build();

    sidebar.set_width_request(180);
    sidebar.set_vexpand(true);

    let sidebar_wrap = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(["settings-sidebar-wrap"])
        .build();
    sidebar_wrap.append(&sidebar);
    body.append(&sidebar_wrap);

    let stack_wrap = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .hexpand(true)
        .child(&stack)
        .css_classes(["settings-content-scroll"])
        .build();
    body.append(&stack_wrap);

    // --- Compose window -----------------------------------------------------
    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(16)
        .margin_end(16)
        .build();
    content.append(&header);
    content.append(&status_banner);
    content.append(&body);
    content.append(&build_footer());

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rift Settings")
        .default_width(820)
        .default_height(560)
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

    // --- Wire up handlers ---------------------------------------------------
    close_button.connect_clicked({
        let window = window.clone();
        move |_| window.hide()
    });

    status_action.connect_clicked({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |_| {
            if let Err(error) = restart_application(&handles) {
                banner.show_error(&error);
            }
        }
    });

    results_spin.connect_value_changed({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |spin| {
            let value = spin.value() as u32;
            apply_change(&handles, &banner, |config| {
                config.launcher.max_visible_results = value;
            });
        }
    });

    autostart_switch.connect_active_notify({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |switch| {
            let active = switch.is_active();
            apply_change(&handles, &banner, |config| {
                config.launcher.launch_at_login = active;
            });
        }
    });

    clear_input_switch.connect_active_notify({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |switch| {
            let active = switch.is_active();
            apply_change(&handles, &banner, |config| {
                config.launcher.clear_input_on_hide = active;
            });
        }
    });

    monitor_dropdown.connect_selected_notify({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |dropdown| {
            let selection = match dropdown.selected() {
                1 => RenderMonitor::Default,
                _ => RenderMonitor::Cursor,
            };
            apply_change(&handles, &banner, |config| {
                config.launcher.render_monitor = selection;
            });
        }
    });

    keep_focus_switch.connect_active_notify({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |switch| {
            let active = switch.is_active();
            apply_change(&handles, &banner, |config| {
                config.launcher.keep_open_on_focus_loss = active;
            });
        }
    });

    shell_switch.connect_active_notify({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |switch| {
            let active = switch.is_active();
            apply_change(&handles, &banner, |config| {
                config.providers.shell_enabled = active;
            });
        }
    });

    calculator_switch.connect_active_notify({
        let handles = handles.clone();
        let banner = banner_state.clone();
        move |switch| {
            let active = switch.is_active();
            apply_change(&handles, &banner, |config| {
                config.providers.calculator_enabled = active;
            });
        }
    });

    theme_dropdown.connect_selected_notify({
        let handles = handles.clone();
        let banner = banner_state.clone();
        let entries = theme_entries.clone();
        let theme_path_label = theme_path_label.clone();
        let theme_meta_label = theme_meta_label.clone();
        let validation_pill = validation_pill.clone();
        let validation_text = validation_text.clone();
        let validation_icon = validation_icon.clone();
        move |dropdown| {
            let entries = entries.borrow();
            let index = dropdown.selected() as usize;
            let Some(entry) = entries.get(index) else {
                return;
            };
            let id = entry.manifest.id.clone();
            theme_path_label.set_label(&entry.path.display().to_string());
            theme_meta_label.set_label(&theme_meta_text(Some(entry)));
            apply_validation(
                &validation_pill,
                &validation_text,
                &validation_icon,
                theme::validate_theme_path(&entry.path).map(|_| ()),
            );
            apply_change(&handles, &banner, |config| {
                config.theme.active = id;
            });
            let theme = theme::load_theme(&entry.manifest.id);
            css::install_css(&theme.css);
        }
    });

    reload_theme_btn.connect_clicked({
        let handles = handles.clone();
        let banner = banner_state.clone();
        let entries = theme_entries.clone();
        let dropdown = theme_dropdown.clone();
        let model = theme_model.clone();
        let theme_path_label = theme_path_label.clone();
        let theme_meta_label = theme_meta_label.clone();
        let validation_pill = validation_pill.clone();
        let validation_text = validation_text.clone();
        let validation_icon = validation_icon.clone();
        move |_| {
            let new_entries = theme::list_available_themes();
            *entries.borrow_mut() = new_entries.clone();
            // Rebuild model
            while model.n_items() > 0 {
                model.remove(0);
            }
            for entry in new_entries.iter() {
                model.append(&entry.manifest.name);
            }
            let current = current_config(&handles).theme.active;
            let target_index = new_entries
                .iter()
                .position(|entry| entry.manifest.id == current)
                .unwrap_or(0);
            dropdown.set_selected(target_index as u32);
            if let Some(entry) = new_entries.get(target_index) {
                theme_path_label.set_label(&entry.path.display().to_string());
                theme_meta_label.set_label(&theme_meta_text(Some(entry)));
                apply_validation(
                    &validation_pill,
                    &validation_text,
                    &validation_icon,
                    theme::validate_theme_path(&entry.path).map(|_| ()),
                );
                let theme = theme::load_theme(&entry.manifest.id);
                css::install_css(&theme.css);
            }
            banner.show_info(&format!("Reloaded {} theme(s).", new_entries.len()));
        }
    });

    validate_theme_btn.connect_clicked({
        let entries = theme_entries.clone();
        let dropdown = theme_dropdown.clone();
        let validation_pill = validation_pill.clone();
        let validation_text = validation_text.clone();
        let validation_icon = validation_icon.clone();
        let banner = banner_state.clone();
        move |_| {
            let entries = entries.borrow();
            let index = dropdown.selected() as usize;
            if let Some(entry) = entries.get(index) {
                let result = theme::validate_theme_path(&entry.path).map(|_| ());
                apply_validation(
                    &validation_pill,
                    &validation_text,
                    &validation_icon,
                    result.clone(),
                );
                match result {
                    Ok(_) => banner.show_info(&format!(
                        "\u{201c}{}\u{201d} looks good.",
                        entry.manifest.name
                    )),
                    Err(err) => banner.show_error(&err),
                }
            }
        }
    });

    open_folder_btn.connect_clicked({
        let banner = banner_state.clone();
        move |_| {
            let path = theme::themes_dir_path();
            let uri = format!("file://{}", path.display());
            if let Err(error) =
                gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>)
            {
                banner.show_error(&error.to_string());
            }
        }
    });

    refresh_default_btn.connect_clicked({
        let handles = handles.clone();
        let banner = banner_state.clone();
        let entries = theme_entries.clone();
        let dropdown = theme_dropdown.clone();
        let model = theme_model.clone();
        let theme_path_label = theme_path_label.clone();
        let theme_meta_label = theme_meta_label.clone();
        let validation_pill = validation_pill.clone();
        let validation_text = validation_text.clone();
        let validation_icon = validation_icon.clone();
        move |_| match theme::rewrite_default_theme_file() {
            Ok(_) => {
                let new_entries = theme::list_available_themes();
                *entries.borrow_mut() = new_entries.clone();
                while model.n_items() > 0 {
                    model.remove(0);
                }
                for entry in new_entries.iter() {
                    model.append(&entry.manifest.name);
                }
                let target_index = new_entries
                    .iter()
                    .position(|entry| entry.manifest.id == theme::DEFAULT_THEME_ID)
                    .unwrap_or(0);
                dropdown.set_selected(target_index as u32);
                if let Some(entry) = new_entries.get(target_index) {
                    theme_path_label.set_label(&entry.path.display().to_string());
                    theme_meta_label.set_label(&theme_meta_text(Some(entry)));
                    apply_validation(
                        &validation_pill,
                        &validation_text,
                        &validation_icon,
                        theme::validate_theme_path(&entry.path).map(|_| ()),
                    );
                }
                apply_change(&handles, &banner, |config| {
                    config.theme.active = theme::DEFAULT_THEME_ID.to_string();
                });
                let theme = theme::load_theme(theme::DEFAULT_THEME_ID);
                css::install_css(&theme.css);
                banner.show_info("Default theme file rewritten from built-in CSS.");
            }
            Err(error) => banner.show_error(&error),
        }
    });

    apply_validation(
        &validation_pill,
        &validation_text,
        &validation_icon,
        theme::validate_theme_path(
            &theme_entries
                .borrow()
                .get(initial_index as usize)
                .map(|entry| entry.path.clone())
                .unwrap_or_else(theme::active_theme_path),
        )
        .map(|_| ()),
    );

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

fn sidebar_row(label: &str, icon_name: &str, panel_id: &str) -> gtk::ListBoxRow {
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(12);
    icon.set_halign(Align::Center);
    icon.set_valign(Align::Center);
    icon.set_hexpand(true);
    icon.set_vexpand(true);

    let chip = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::Center)
        .valign(Align::Center)
        .width_request(22)
        .height_request(22)
        .css_classes(["settings-sidebar-chip", &format!("chip-{panel_id}")])
        .build();
    chip.set_hexpand(false);
    chip.set_vexpand(false);
    chip.set_hexpand_set(true);
    chip.set_vexpand_set(true);
    chip.append(&icon);

    let text = gtk::Label::builder()
        .label(label)
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["settings-sidebar-label"])
        .build();

    let row_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .css_classes(["settings-sidebar-row"])
        .build();
    row_box.append(&chip);
    row_box.append(&text);

    let row = gtk::ListBoxRow::builder().child(&row_box).build();
    unsafe {
        row.set_data("panel-id", panel_id.to_string());
    }
    row
}

fn panel(sections: &[gtk::Box]) -> gtk::Box {
    let panel = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .css_classes(["settings-panel"])
        .build();
    for section in sections {
        panel.append(section);
    }
    panel
}

fn build_theme_panel(
    theme_dropdown: &gtk::DropDown,
    theme_path_label: &gtk::Label,
    theme_meta_label: &gtk::Label,
    validation_pill: &gtk::Box,
    theme_actions: &gtk::Box,
) -> gtk::Box {
    let selector = section(
        "Active theme",
        group(&[labeled_row(
            "Theme",
            Some("Pick from any *.rift-theme file in your themes folder."),
            theme_dropdown.clone().upcast::<gtk::Widget>(),
        )]),
    );

    // Theme metadata card
    let detail_card = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .css_classes(["settings-theme-detail"])
        .build();

    let path_label = gtk::Label::builder()
        .label("File")
        .halign(Align::Start)
        .css_classes(["settings-theme-detail-label"])
        .build();
    detail_card.append(&path_label);
    detail_card.append(theme_path_label);

    let meta_heading = gtk::Label::builder()
        .label("Details")
        .halign(Align::Start)
        .css_classes(["settings-theme-detail-label"])
        .build();
    detail_card.append(&meta_heading);
    detail_card.append(theme_meta_label);

    let validation_heading = gtk::Label::builder()
        .label("Status")
        .halign(Align::Start)
        .css_classes(["settings-theme-detail-label"])
        .build();
    detail_card.append(&validation_heading);
    detail_card.append(validation_pill);

    let detail_section = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    let detail_heading = gtk::Label::builder()
        .label("Details")
        .halign(Align::Start)
        .css_classes(["settings-section-title"])
        .build();
    detail_section.append(&detail_heading);
    detail_section.append(&detail_card);

    let actions_section = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    let actions_heading = gtk::Label::builder()
        .label("Actions")
        .halign(Align::Start)
        .css_classes(["settings-section-title"])
        .build();
    actions_section.append(&actions_heading);
    actions_section.append(theme_actions);

    let panel = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .css_classes(["settings-panel"])
        .build();
    panel.append(&selector);
    panel.append(&detail_section);
    panel.append(&actions_section);
    panel
}

fn action_button(label: &str, icon_name: &str) -> gtk::Button {
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(12);

    let text = gtk::Label::builder()
        .label(label)
        .css_classes(["settings-action-button-label"])
        .build();

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    row.append(&icon);
    row.append(&text);

    gtk::Button::builder()
        .child(&row)
        .css_classes(["settings-action-button"])
        .build()
}

fn theme_meta_text(entry: Option<&theme::ThemeEntry>) -> String {
    let Some(entry) = entry else {
        return "Unknown theme".to_string();
    };
    let manifest = &entry.manifest;
    let mut parts = vec![format!("v{}", manifest.version)];
    if let Some(author) = manifest.author.as_deref() {
        if !author.trim().is_empty() {
            parts.push(format!("by {}", author));
        }
    }
    parts.push(format!("schema v{}", manifest.rift_theme_version));
    let mut text = parts.join("  ·  ");
    if let Some(description) = manifest.description.as_deref() {
        if !description.trim().is_empty() {
            text.push('\n');
            text.push_str(description.trim());
        }
    }
    text
}

fn apply_validation(
    pill: &gtk::Box,
    text: &gtk::Label,
    icon: &gtk::Image,
    result: Result<(), String>,
) {
    pill.remove_css_class("ok");
    pill.remove_css_class("error");
    match result {
        Ok(_) => {
            pill.add_css_class("ok");
            icon.set_icon_name(Some("emblem-ok-symbolic"));
            text.set_label("Valid");
        }
        Err(message) => {
            pill.add_css_class("error");
            icon.set_icon_name(Some("dialog-warning-symbolic"));
            text.set_label(&message);
        }
    }
}

fn apply_change(
    handles: &LauncherHandles,
    banner: &SettingsBanner,
    mutate: impl FnOnce(&mut AppConfig),
) {
    let mut next = current_config(handles);
    mutate(&mut next);
    match save_config(handles, next) {
        Ok(SaveOutcome::Applied) => banner.hide(),
        Ok(SaveOutcome::RestartRequired) => banner.show_restart_required(),
        Err(error) => banner.show_error(&error),
    }
}

#[derive(Clone)]
struct SettingsBanner {
    container: gtk::Box,
    label: gtk::Label,
    icon: gtk::Image,
    action: gtk::Button,
}

impl SettingsBanner {
    fn hide(&self) {
        self.container.set_visible(false);
        self.container.remove_css_class("error");
    }

    fn show_info(&self, message: &str) {
        self.icon
            .set_icon_name(Some("emblem-synchronizing-symbolic"));
        self.label.set_label(message);
        self.container.remove_css_class("error");
        self.action.set_visible(false);
        self.container.set_visible(true);
    }

    fn show_restart_required(&self) {
        self.icon
            .set_icon_name(Some("emblem-synchronizing-symbolic"));
        self.label
            .set_label("Restart Rift to apply the new shortcut.");
        self.container.remove_css_class("error");
        self.action.set_visible(true);
        self.container.set_visible(true);
    }

    fn show_error(&self, message: &str) {
        self.icon.set_icon_name(Some("dialog-warning-symbolic"));
        self.label.set_label(message);
        self.container.add_css_class("error");
        self.action.set_visible(false);
        self.container.set_visible(true);
    }
}

fn section(title: &str, group: gtk::Box) -> gtk::Box {
    let heading = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["settings-section-title"])
        .build();

    let section = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .css_classes(["settings-section"])
        .build();
    section.append(&heading);
    section.append(&group);
    section
}

fn wrap_section(title: &str, body: gtk::Widget) -> gtk::Box {
    let heading = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["settings-section-title"])
        .build();

    let section = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .css_classes(["settings-section"])
        .build();
    section.append(&heading);
    section.append(&body);
    section
}

fn build_shortcut_setup_card() -> gtk::Box {
    let hint = gtk::Label::builder()
        .label(
            "Rift must be running in the background before these commands can control it. \
             Enable Launch at login or start it manually with `rift --background`, then bind a key \
             in your desktop's keyboard settings. Rift won't capture global keys itself.",
        )
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["settings-shortcut-card-hint"])
        .build();

    let list = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(["settings-shortcut-card-list"])
        .build();
    for (command, description, badge) in [
        (
            "rift --background",
            "Start Rift in the background",
            Some("run first"),
        ),
        ("rift --toggle", "Toggle visibility", Some("recommended")),
        ("rift --show", "Open the launcher", None),
        ("rift --hide", "Close the launcher", None),
        ("rift --quit", "Stop the background process", None),
    ] {
        list.append(&shortcut_command_row(command, description, badge));
    }

    let card = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .css_classes(["settings-shortcut-card"])
        .build();
    card.append(&hint);
    card.append(&list);
    card
}

fn shortcut_command_row(command: &str, description: &str, badge: Option<&str>) -> gtk::Box {
    let cmd_label = gtk::Label::builder()
        .label(command)
        .halign(Align::Start)
        .selectable(true)
        .css_classes(["settings-shortcut-command"])
        .build();

    let desc_label = gtk::Label::builder()
        .label(description)
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["settings-shortcut-command-desc"])
        .build();

    let text_column = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(Align::Center)
        .build();

    let header = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    header.append(&cmd_label);
    if let Some(badge_text) = badge {
        let badge_label = gtk::Label::builder()
            .label(badge_text)
            .halign(Align::Start)
            .valign(Align::Center)
            .css_classes(["settings-shortcut-badge"])
            .build();
        header.append(&badge_label);
    }
    text_column.append(&header);
    text_column.append(&desc_label);

    let copy_button = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy command")
        .valign(Align::Center)
        .css_classes(["flat", "settings-shortcut-copy"])
        .build();
    let to_copy = command.to_string();
    copy_button.connect_clicked(move |_| {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(&to_copy);
        }
    });

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .css_classes(["settings-shortcut-command-row"])
        .build();
    row.append(&text_column);
    row.append(&copy_button);
    row
}

fn group(rows: &[gtk::Box]) -> gtk::Box {
    let group = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(["settings-group"])
        .build();
    for row in rows {
        group.append(row);
    }
    group
}

fn labeled_row(label: &str, helper: Option<&str>, control: gtk::Widget) -> gtk::Box {
    let label_widget = gtk::Label::builder()
        .label(label)
        .halign(Align::Start)
        .css_classes(["settings-row-label"])
        .build();

    let text_column = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(Align::Center)
        .build();
    text_column.append(&label_widget);

    if let Some(helper_text) = helper {
        let helper_widget = gtk::Label::builder()
            .label(helper_text)
            .halign(Align::Start)
            .wrap(true)
            .css_classes(["settings-row-helper"])
            .build();
        text_column.append(&helper_widget);
    }

    control.set_valign(Align::Center);
    control.set_halign(Align::End);

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(14)
        .css_classes(["settings-row"])
        .build();
    row.append(&text_column);
    row.append(&control);
    row
}

fn build_footer() -> gtk::Box {
    let version = gtk::Label::builder()
        .label(&format!("Rift v{}  ·  MPL-2.0", env!("CARGO_PKG_VERSION")))
        .halign(Align::Center)
        .hexpand(true)
        .css_classes(["settings-footer"])
        .build();

    let footer = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["settings-footer-box"])
        .build();
    footer.append(&version);
    footer
}
