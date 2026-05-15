use std::{cell::RefCell, collections::HashSet, rc::Rc};

use adw::prelude::*;
use gtk::{Align, Orientation, gdk};

use crate::config::AppConfig;

use super::{LauncherHandles, SaveOutcome, current_config, restart_application, save_config};

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
    status_banner.append(&status_icon);
    status_banner.append(&status_label);
    status_banner.append(&status_action);

    let shortcut_button = gtk::Button::builder()
        .label(&config.launcher.shortcut_trigger)
        .css_classes(["settings-shortcut-button"])
        .build();
    shortcut_button.set_widget_name("shortcut");

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

    let launcher_group = group(&[
        labeled_row(
            "Global shortcut",
            Some("Triggers the launcher anywhere."),
            shortcut_button.clone().upcast::<gtk::Widget>(),
        ),
        labeled_row(
            "Visible results",
            Some("Maximum items shown without scrolling."),
            results_spin.clone().upcast::<gtk::Widget>(),
        ),
    ]);

    let providers_group = group(&[
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
    ]);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .margin_top(16)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(20)
        .build();
    content.append(&header);
    content.append(&status_banner);
    content.append(&section("Launcher", &launcher_group));
    content.append(&section("Providers", &providers_group));

    let footer = build_footer();
    content.append(&footer);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rift Settings")
        .default_width(440)
        .default_height(440)
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

    let banner_state = SettingsBanner {
        container: status_banner.clone(),
        label: status_label.clone(),
        icon: status_icon.clone(),
        action: status_action.clone(),
    };

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

    shortcut_button.connect_clicked({
        let window = window.clone();
        let handles = handles.clone();
        let banner = banner_state.clone();
        let button = shortcut_button.clone();
        move |_| {
            present_shortcut_capture(&window, &button, &handles, &banner);
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

fn section(title: &str, group: &gtk::Box) -> gtk::Box {
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
    section.append(group);
    section
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

fn present_shortcut_capture(
    parent: &gtk::ApplicationWindow,
    target_button: &gtk::Button,
    handles: &LauncherHandles,
    banner: &SettingsBanner,
) {
    let title = gtk::Label::builder()
        .label("Set Shortcut")
        .halign(Align::Center)
        .css_classes(["settings-capture-title"])
        .build();
    let hint = gtk::Label::builder()
        .label("Hold modifiers and press a key. Esc to cancel.")
        .halign(Align::Center)
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes(["settings-capture-hint"])
        .build();
    let value = gtk::Label::builder()
        .label("Listening…")
        .halign(Align::Center)
        .css_classes(["settings-capture-value"])
        .build();

    let value_frame = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::Center)
        .css_classes(["settings-capture-keycap"])
        .build();
    value_frame.append(&value);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(14)
        .margin_top(24)
        .margin_bottom(20)
        .margin_start(24)
        .margin_end(24)
        .css_classes(["settings-capture-box"])
        .build();
    content.append(&title);
    content.append(&hint);
    content.append(&value_frame);

    let actions = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .halign(Align::Fill)
        .homogeneous(true)
        .css_classes(["settings-capture-actions"])
        .build();
    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .css_classes(["settings-capture-cancel"])
        .build();
    let confirm_button = gtk::Button::builder()
        .label("Set Shortcut")
        .sensitive(false)
        .css_classes(["suggested-action", "settings-capture-confirm"])
        .build();
    actions.append(&cancel_button);
    actions.append(&confirm_button);
    content.append(&actions);

    let dialog = gtk::Window::builder()
        .title("Capture Shortcut")
        .transient_for(parent)
        .modal(true)
        .decorated(false)
        .resizable(false)
        .default_width(340)
        .default_height(220)
        .child(&content)
        .build();
    dialog.add_css_class("rift-shortcut-capture");

    let pressed_keys = Rc::new(RefCell::new(HashSet::<u32>::new()));
    let draft = Rc::new(RefCell::new(ShortcutDraft::default()));

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    key_controller.connect_key_pressed({
        let dialog = dialog.clone();
        let value = value.clone();
        let confirm_button = confirm_button.clone();
        let pressed_keys = pressed_keys.clone();
        let draft = draft.clone();

        move |_, key, keycode, state| {
            if key == gdk::Key::Escape {
                dialog.close();
                return true.into();
            }

            if !pressed_keys.borrow_mut().insert(keycode) {
                return true.into();
            }

            {
                let mut draft = draft.borrow_mut();
                draft.ctrl = state.contains(gdk::ModifierType::CONTROL_MASK)
                    || matches!(key, gdk::Key::Control_L | gdk::Key::Control_R);
                draft.alt = state.contains(gdk::ModifierType::ALT_MASK)
                    || matches!(key, gdk::Key::Alt_L | gdk::Key::Alt_R);
                draft.alt_gr = matches!(key, gdk::Key::ISO_Level3_Shift | gdk::Key::Mode_switch);
                draft.shift = state.contains(gdk::ModifierType::SHIFT_MASK)
                    || matches!(key, gdk::Key::Shift_L | gdk::Key::Shift_R);
                draft.super_key = state.contains(gdk::ModifierType::SUPER_MASK)
                    || matches!(
                        key,
                        gdk::Key::Super_L | gdk::Key::Super_R | gdk::Key::Meta_L | gdk::Key::Meta_R
                    );

                if !is_modifier_key(key) {
                    draft.primary = format_key_name(key);
                }
            }

            let display = draft.borrow().display();
            value.set_label(if display.is_empty() {
                "Listening…"
            } else {
                &display
            });
            confirm_button.set_sensitive(draft.borrow().is_complete());

            true.into()
        }
    });
    key_controller.connect_key_released({
        let pressed_keys = pressed_keys.clone();
        let value = value.clone();
        let confirm_button = confirm_button.clone();
        let draft = draft.clone();

        move |_, key, keycode, state| {
            pressed_keys.borrow_mut().remove(&keycode);
            if is_modifier_key(key) && draft.borrow().primary.is_none() {
                let mut draft = draft.borrow_mut();
                draft.ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
                draft.alt = state.contains(gdk::ModifierType::ALT_MASK);
                draft.alt_gr = false;
                draft.shift = state.contains(gdk::ModifierType::SHIFT_MASK);
                draft.super_key = state.contains(gdk::ModifierType::SUPER_MASK);
                let display = draft.display();
                value.set_label(if display.is_empty() {
                    "Listening…"
                } else {
                    &display
                });
                confirm_button.set_sensitive(draft.is_complete());
            }
        }
    });
    dialog.add_controller(key_controller);

    cancel_button.connect_clicked({
        let dialog = dialog.clone();
        move |_| dialog.close()
    });

    confirm_button.connect_clicked({
        let dialog = dialog.clone();
        let target_button = target_button.clone();
        let draft = draft.clone();
        let handles = handles.clone();
        let banner = banner.clone();
        move |_| {
            let display = draft.borrow().display();
            if !display.is_empty() {
                target_button.set_label(&display);
                let next_shortcut = display.clone();
                apply_change(&handles, &banner, |config| {
                    config.launcher.shortcut_trigger = next_shortcut;
                });
            }
            dialog.close();
        }
    });

    dialog.present();
}

#[derive(Default)]
struct ShortcutDraft {
    ctrl: bool,
    alt: bool,
    alt_gr: bool,
    shift: bool,
    super_key: bool,
    primary: Option<String>,
}

impl ShortcutDraft {
    fn display(&self) -> String {
        let mut parts = Vec::new();

        if self.ctrl {
            parts.push("CTRL".to_string());
        }
        if self.alt {
            parts.push("ALT".to_string());
        }
        if self.alt_gr {
            parts.push("ALTGR".to_string());
        }
        if self.shift {
            parts.push("SHIFT".to_string());
        }
        if self.super_key {
            parts.push("SUPER".to_string());
        }
        if let Some(primary) = &self.primary {
            parts.push(primary.clone());
        }

        parts.join("+")
    }

    fn is_complete(&self) -> bool {
        self.primary.is_some()
    }
}

fn format_key_name(key: gdk::Key) -> Option<String> {
    let name = key.name()?;
    if name.starts_with("0x") || name.starts_with("0X") {
        return None;
    }

    match name.as_str() {
        "space" => Some("SPACE".to_string()),
        "Return" => Some("ENTER".to_string()),
        "Escape" => Some("ESC".to_string()),
        "BackSpace" => Some("BACKSPACE".to_string()),
        "Tab" => Some("TAB".to_string()),
        "Delete" => Some("DELETE".to_string()),
        "Insert" => Some("INSERT".to_string()),
        "Home" => Some("HOME".to_string()),
        "End" => Some("END".to_string()),
        "Page_Up" => Some("PAGEUP".to_string()),
        "Page_Down" => Some("PAGEDOWN".to_string()),
        "Left" => Some("LEFT".to_string()),
        "Right" => Some("RIGHT".to_string()),
        "Up" => Some("UP".to_string()),
        "Down" => Some("DOWN".to_string()),
        "minus" => Some("-".to_string()),
        "equal" => Some("=".to_string()),
        "comma" => Some(",".to_string()),
        "period" => Some(".".to_string()),
        "slash" => Some("/".to_string()),
        "backslash" => Some("\\".to_string()),
        "semicolon" => Some(";".to_string()),
        "apostrophe" => Some("'".to_string()),
        "bracketleft" => Some("[".to_string()),
        "bracketright" => Some("]".to_string()),
        "grave" => Some("`".to_string()),
        "KP_Enter" => Some("NUMENTER".to_string()),
        other
            if other.starts_with('F')
                && other[1..]
                    .chars()
                    .all(|character| character.is_ascii_digit()) =>
        {
            Some(other.to_string())
        }
        other if other.starts_with("XF86") => Some(other.to_uppercase()),
        _ => key
            .to_unicode()
            .filter(|character| !character.is_control())
            .map(|character| character.to_uppercase().collect())
            .or_else(|| Some(name.to_uppercase())),
    }
}

fn is_modifier_key(key: gdk::Key) -> bool {
    matches!(
        key,
        gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::ISO_Level3_Shift
            | gdk::Key::Mode_switch
    )
}
