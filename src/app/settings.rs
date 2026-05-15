use adw::prelude::*;
use gtk::{Align, Orientation, gdk};

use crate::config::AppConfig;

use super::{LauncherHandles, current_config, save_config};

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
        .spacing(18)
        .margin_top(20)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(&header);
    content.append(&build_launcher_section(&config));
    content.append(&build_provider_section(&config));
    content.append(&settings_note(
        "Shortcut changes are saved immediately but may require restarting Rift and re-approving the shortcut in your desktop portal.",
    ));
    content.append(&settings_section(
        "About",
        &[
            ("Version", env!("CARGO_PKG_VERSION")),
            ("License", "MPL-2.0"),
        ],
    ));
    let feedback = gtk::Label::builder()
        .halign(Align::Start)
        .wrap(true)
        .visible(false)
        .css_classes(["settings-feedback"])
        .build();
    content.append(&feedback);

    let actions = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .halign(Align::End)
        .css_classes(["settings-actions"])
        .build();
    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .css_classes(["settings-secondary-button"])
        .build();
    let save_button = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action", "settings-primary-button"])
        .build();
    actions.append(&cancel_button);
    actions.append(&save_button);
    content.append(&actions);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rift Settings")
        .default_width(420)
        .default_height(420)
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

    cancel_button.connect_clicked({
        let window = window.clone();

        move |_| {
            window.hide();
        }
    });

    save_button.connect_clicked({
        let window = window.clone();
        let handles = handles.clone();
        let feedback = feedback.clone();
        let shortcut = find_entry_value(&content, "shortcut");
        let result_count = find_spin_value(&content, "results");
        let shell_enabled = find_switch_value(&content, "shell");
        let calculator_enabled = find_switch_value(&content, "calculator");

        move |_| {
            let mut next = current_config(&handles);
            next.launcher.shortcut_trigger = shortcut.text().to_string();
            next.launcher.max_visible_results = result_count.value() as u32;
            next.providers.shell_enabled = shell_enabled.is_active();
            next.providers.calculator_enabled = calculator_enabled.is_active();

            match save_config(&handles, next) {
                Ok(()) => {
                    feedback.set_label("Saved.");
                    feedback.remove_css_class("error");
                    feedback.set_visible(true);
                    window.hide();
                }
                Err(error) => {
                    feedback.set_label(&error);
                    feedback.add_css_class("error");
                    feedback.set_visible(true);
                }
            }
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

fn build_launcher_section(config: &AppConfig) -> gtk::Box {
    settings_section_widget(
        "Launcher",
        &[
            editable_entry_row(
                "Global shortcut",
                &config.launcher.shortcut_trigger,
                "shortcut",
            ),
            editable_spin_row(
                "Visible results",
                config.launcher.max_visible_results as f64,
                1.0,
                8.0,
                1.0,
                "results",
            ),
        ],
    )
}

fn build_provider_section(config: &AppConfig) -> gtk::Box {
    settings_section_widget(
        "Providers",
        &[
            editable_switch_row(
                "Shell commands (>)",
                config.providers.shell_enabled,
                "shell",
            ),
            editable_switch_row(
                "Calculator",
                config.providers.calculator_enabled,
                "calculator",
            ),
        ],
    )
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

fn settings_section_widget(title: &str, rows: &[gtk::Box]) -> gtk::Box {
    let heading = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["settings-section-title"])
        .build();

    let group = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(["settings-group"])
        .build();
    for row in rows {
        group.append(row);
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

fn editable_entry_row(label: &str, value: &str, key: &str) -> gtk::Box {
    let label = gtk::Label::builder()
        .label(label)
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["settings-row-label"])
        .build();

    let entry = gtk::Entry::builder()
        .text(value)
        .width_chars(18)
        .css_classes(["settings-entry"])
        .build();
    entry.set_widget_name(key);

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["settings-row"])
        .build();
    row.append(&label);
    row.append(&entry);
    row
}

fn editable_spin_row(
    label: &str,
    value: f64,
    lower: f64,
    upper: f64,
    step: f64,
    key: &str,
) -> gtk::Box {
    let label = gtk::Label::builder()
        .label(label)
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["settings-row-label"])
        .build();
    let adjustment = gtk::Adjustment::new(value, lower, upper, step, step, 0.0);
    let spin = gtk::SpinButton::builder()
        .adjustment(&adjustment)
        .numeric(true)
        .css_classes(["settings-spin"])
        .build();
    spin.set_widget_name(key);

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["settings-row"])
        .build();
    row.append(&label);
    row.append(&spin);
    row
}

fn editable_switch_row(label: &str, active: bool, key: &str) -> gtk::Box {
    let label = gtk::Label::builder()
        .label(label)
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["settings-row-label"])
        .build();
    let switch = gtk::Switch::builder().active(active).build();
    switch.set_widget_name(key);

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["settings-row"])
        .build();
    row.append(&label);
    row.append(&switch);
    row
}

fn settings_note(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .halign(Align::Start)
        .wrap(true)
        .css_classes(["settings-note"])
        .build()
}

fn find_entry_value(root: &gtk::Box, key: &str) -> gtk::Entry {
    find_named_widget(root, key)
}

fn find_spin_value(root: &gtk::Box, key: &str) -> gtk::SpinButton {
    find_named_widget(root, key)
}

fn find_switch_value(root: &gtk::Box, key: &str) -> gtk::Switch {
    find_named_widget(root, key)
}

fn find_named_widget<T: glib::object::IsA<gtk::Widget> + Clone + 'static>(
    root: &gtk::Box,
    key: &str,
) -> T {
    let mut stack = vec![root.clone().upcast::<gtk::Widget>()];
    while let Some(widget) = stack.pop() {
        if widget.widget_name() == key {
            return widget
                .downcast::<T>()
                .ok()
                .expect("settings widget type mismatch");
        }

        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }

    panic!("missing settings widget: {key}");
}
