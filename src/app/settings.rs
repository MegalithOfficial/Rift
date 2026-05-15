use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use gtk::{Align, Orientation, gdk};

use super::TOGGLE_SHORTCUT_TRIGGER;

pub(super) fn present_settings_window(
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
