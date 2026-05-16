use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use gtk::{Align, Orientation, gdk};

use super::{LauncherHandles, SaveOutcome, current_config, save_config};

// Welcome CSS is hardcoded here (not part of the themeable built-in CSS) so the
// onboarding window always looks identical regardless of the active theme.
const WELCOME_CSS: &str = r#"
window.rift-welcome-window {
    background-color: #18181c;
    color: #f4f4f5;
    border-radius: 14px;
    border: 1px solid #2e2e33;
}

window.rift-welcome-window label.welcome-title {
    color: #f4f4f5;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.015em;
}

window.rift-welcome-window label.welcome-subtitle {
    color: #85858c;
    font-size: 11px;
    letter-spacing: 0.01em;
}

window.rift-welcome-window box.welcome-page {
    padding: 4px 0 8px 0;
}

window.rift-welcome-window label.welcome-page-title {
    color: #f4f4f5;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.005em;
}

window.rift-welcome-window label.welcome-page-body {
    color: #b6b6bd;
    font-size: 12px;
    line-height: 1.45;
}

window.rift-welcome-window box.welcome-feature-list {
    margin-top: 6px;
}

window.rift-welcome-window box.welcome-feature-row {
    padding: 8px 12px;
    background-color: #232327;
    border: 1px solid #2e2e33;
    border-radius: 9px;
}

window.rift-welcome-window box.welcome-feature-chip {
    background-color: alpha(#ffffff, 0.08);
    border-radius: 6px;
}

window.rift-welcome-window box.welcome-feature-chip image {
    color: #f4f4f5;
}

window.rift-welcome-window label.welcome-feature-title {
    color: #ececf0;
    font-size: 12px;
    font-weight: 600;
}

window.rift-welcome-window label.welcome-feature-body {
    color: #85858c;
    font-size: 11px;
}

window.rift-welcome-window box.welcome-shortcut-card {
    background-color: #232327;
    border: 1px solid #2e2e33;
    border-radius: 10px;
    padding: 10px 14px;
}

window.rift-welcome-window box.welcome-steps {
    margin-top: 4px;
}

window.rift-welcome-window box.welcome-step-row {
    padding: 4px 0;
}

window.rift-welcome-window label.welcome-step-number {
    color: #f4f4f5;
    background-color: alpha(#ffffff, 0.10);
    border-radius: 999px;
    font-size: 11px;
    font-weight: 700;
    padding: 2px 0;
    margin-top: 1px;
}

window.rift-welcome-window label.welcome-step-title {
    color: #ececf0;
    font-size: 12px;
    font-weight: 600;
}

window.rift-welcome-window label.welcome-step-body {
    color: #85858c;
    font-size: 11px;
    line-height: 1.45;
}

window.rift-welcome-window label.welcome-command {
    color: #f4f4f5;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    font-weight: 500;
    background-color: alpha(#ffffff, 0.05);
    border: 1px solid alpha(#ffffff, 0.06);
    border-radius: 5px;
    padding: 2px 8px;
}

window.rift-welcome-window label.welcome-command-desc {
    color: #85858c;
    font-size: 11px;
}

window.rift-welcome-window label.welcome-command-badge {
    color: #b9e0c8;
    background-color: alpha(#62c089, 0.14);
    border: 1px solid alpha(#62c089, 0.32);
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
}

window.rift-welcome-window button.welcome-command-copy {
    min-width: 26px;
    min-height: 26px;
    padding: 0;
    border-radius: 6px;
    background-color: transparent;
    border: none;
    color: #9a9aa1;
}

window.rift-welcome-window button.welcome-command-copy:hover {
    background-color: alpha(#ffffff, 0.08);
    color: #f4f4f5;
}

window.rift-welcome-window button.welcome-command-copy image {
    -gtk-icon-size: 12px;
}

window.rift-welcome-window label.welcome-tip {
    color: #85858c;
    font-size: 11px;
    line-height: 1.45;
}

window.rift-welcome-window box.welcome-finish-card {
    background-color: #232327;
    border: 1px solid #2e2e33;
    border-radius: 10px;
    padding: 10px 14px;
}

window.rift-welcome-window box.welcome-finish-row {
    padding: 6px 0;
}

window.rift-welcome-window label.welcome-finish-title {
    color: #ececf0;
    font-size: 12px;
    font-weight: 600;
}

window.rift-welcome-window label.welcome-finish-body {
    color: #85858c;
    font-size: 11px;
    line-height: 1.45;
}

window.rift-welcome-window box.welcome-dots {
    margin-top: 4px;
}

window.rift-welcome-window box.welcome-dot {
    background-color: alpha(#ffffff, 0.12);
    border-radius: 999px;
    min-width: 7px;
    min-height: 7px;
}

window.rift-welcome-window box.welcome-dot.active {
    background-color: #f4f4f5;
}

window.rift-welcome-window button.welcome-button-primary,
window.rift-welcome-window button.welcome-button-secondary {
    min-height: 32px;
    padding: 0 18px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
}

window.rift-welcome-window button.welcome-button-secondary {
    background-color: #232327;
    border: 1px solid #34343a;
    color: #e4e4e7;
}

window.rift-welcome-window button.welcome-button-secondary:hover {
    background-color: #2a2a2f;
}

window.rift-welcome-window button.welcome-button-secondary:disabled {
    opacity: 0.4;
}
"#;

thread_local! {
    static WELCOME_PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

fn install_welcome_css() {
    WELCOME_PROVIDER.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let provider = gtk::CssProvider::new();
        provider.load_from_data(WELCOME_CSS);
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("display unavailable"),
            &provider,
            // One above USER so themes can't override the onboarding window.
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
        );
        *slot.borrow_mut() = Some(provider);
    });
}

pub(super) fn show_if_first_run(app: &adw::Application, handles: &LauncherHandles) -> bool {
    if current_config(handles).welcomed {
        return false;
    }
    install_welcome_css();
    present(app, handles);
    true
}

pub(super) fn present_now(app: &adw::Application, handles: &LauncherHandles) {
    install_welcome_css();
    present(app, handles);
}

fn present(app: &adw::Application, handles: &LauncherHandles) {
    let window = build_window(app, handles);
    window.present();
}

fn build_window(app: &adw::Application, handles: &LauncherHandles) -> gtk::ApplicationWindow {
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::SlideLeftRight)
        .transition_duration(180)
        .vexpand(true)
        .hexpand(true)
        .build();

    stack.add_named(&build_intro_page(), Some("intro"));
    stack.add_named(&build_shortcut_page(), Some("shortcut"));
    stack.add_named(&build_finish_page(), Some("finish"));

    let dots = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(Align::Center)
        .css_classes(["welcome-dots"])
        .build();
    let dot_widgets: Vec<gtk::Box> = (0..3)
        .map(|i| {
            let dot = gtk::Box::builder()
                .css_classes(["welcome-dot"])
                .width_request(7)
                .height_request(7)
                .build();
            if i == 0 {
                dot.add_css_class("active");
            }
            dots.append(&dot);
            dot
        })
        .collect();

    let back_button = gtk::Button::builder()
        .label("Back")
        .css_classes(["welcome-button-secondary"])
        .sensitive(false)
        .build();
    let primary_button = gtk::Button::builder()
        .label("Next")
        .css_classes(["suggested-action", "welcome-button-primary"])
        .build();

    let actions = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .css_classes(["welcome-actions"])
        .build();
    actions.append(&back_button);
    let spacer = gtk::Box::builder().hexpand(true).build();
    actions.append(&spacer);
    actions.append(&primary_button);

    let footer = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .build();
    footer.append(&dots);
    footer.append(&actions);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(22)
        .margin_end(22)
        .build();
    let header = build_header();
    content.append(&header);
    content.append(&stack);
    content.append(&footer);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Welcome to Rift")
        .default_width(540)
        .default_height(460)
        .resizable(false)
        .modal(true)
        .decorated(false)
        .child(&content)
        .build();
    window.add_css_class("rift-welcome-window");
    window.set_hide_on_close(false);

    let page_index = Rc::new(RefCell::new(0usize));
    let pages = ["intro", "shortcut", "finish"];

    let update_dots = {
        let dot_widgets = dot_widgets.clone();
        move |index: usize| {
            for (i, dot) in dot_widgets.iter().enumerate() {
                if i == index {
                    dot.add_css_class("active");
                } else {
                    dot.remove_css_class("active");
                }
            }
        }
    };

    let update_buttons = {
        let back_button = back_button.clone();
        let primary_button = primary_button.clone();
        move |index: usize| {
            back_button.set_sensitive(index > 0);
            primary_button.set_label(if index == 2 { "Get started" } else { "Next" });
        }
    };

    back_button.connect_clicked({
        let page_index = page_index.clone();
        let stack = stack.clone();
        let update_dots = update_dots.clone();
        let update_buttons = update_buttons.clone();
        move |_| {
            let mut index = page_index.borrow_mut();
            if *index > 0 {
                *index -= 1;
                stack.set_visible_child_name(pages[*index]);
                update_dots(*index);
                update_buttons(*index);
            }
        }
    });

    primary_button.connect_clicked({
        let page_index = page_index.clone();
        let stack = stack.clone();
        let handles = handles.clone();
        let window = window.clone();
        let update_dots = update_dots.clone();
        let update_buttons = update_buttons.clone();
        move |_| {
            let mut index = page_index.borrow_mut();
            if *index < pages.len() - 1 {
                *index += 1;
                stack.set_visible_child_name(pages[*index]);
                update_dots(*index);
                update_buttons(*index);
            } else {
                mark_welcomed(&handles);
                window.close();
            }
        }
    });

    let escape = gtk::EventControllerKey::new();
    escape.set_propagation_phase(gtk::PropagationPhase::Capture);
    escape.connect_key_pressed({
        let window = window.clone();
        let handles = handles.clone();
        move |_, key, _, _| {
            if key == gdk::Key::Escape {
                mark_welcomed(&handles);
                window.close();
                return true.into();
            }
            false.into()
        }
    });
    window.add_controller(escape);

    window
}

fn build_header() -> gtk::Box {
    let title = gtk::Label::builder()
        .label("Welcome to Rift")
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["welcome-title"])
        .build();
    let subtitle = gtk::Label::builder()
        .label(&format!("v{}", env!("CARGO_PKG_VERSION")))
        .halign(Align::Start)
        .css_classes(["welcome-subtitle"])
        .build();

    let text = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    text.append(&title);
    text.append(&subtitle);

    let header = gtk::WindowHandle::builder().child(&text).build();
    let wrapper = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .build();
    wrapper.append(&header);
    wrapper
}

fn build_intro_page() -> gtk::Box {
    let page = page_box();

    page.append(&page_heading(
        "Hi.",
        "Rift is a launcher you open with a hotkey. Type, hit Enter. \
         That's the whole loop.",
    ));

    let feature_list = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .css_classes(["welcome-feature-list"])
        .build();
    feature_list.append(&feature_row(
        "edit-find-symbolic",
        "Apps",
        "Start typing a name to find it.",
    ));
    feature_list.append(&feature_row(
        "utilities-terminal-symbolic",
        "Shell",
        "Start the query with > to run it as a command.",
    ));
    feature_list.append(&feature_row(
        "accessories-calculator-symbolic",
        "Math",
        "Type an expression to get the answer.",
    ));
    page.append(&feature_list);

    let note = gtk::Label::builder()
        .label("Next: set up the hotkey that opens it.")
        .halign(Align::Start)
        .xalign(0.0)
        .css_classes(["welcome-tip"])
        .build();
    page.append(&note);

    page
}

fn build_shortcut_page() -> gtk::Box {
    let page = page_box();

    page.append(&page_heading(
        "Pick a hotkey.",
        "Rift doesn't grab keys itself. Bind one in your system's keyboard \
         settings. It runs the command below.",
    ));

    let command_card = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .css_classes(["welcome-shortcut-card"])
        .build();
    let command_label = gtk::Label::builder()
        .label("rift --toggle")
        .halign(Align::Start)
        .hexpand(true)
        .selectable(true)
        .css_classes(["welcome-command"])
        .build();
    let copy_button = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy command")
        .valign(Align::Center)
        .css_classes(["flat", "welcome-command-copy"])
        .build();
    copy_button.connect_clicked(|_| {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text("rift --toggle");
        }
    });
    command_card.append(&command_label);
    command_card.append(&copy_button);
    page.append(&command_card);

    let steps = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .css_classes(["welcome-steps"])
        .build();
    steps.append(&step_row(
        "1",
        "Open keyboard settings",
        "GNOME: Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts.\n\
         KDE: System Settings → Shortcuts → Add Command.\n\
         Hyprland / Sway: edit your config file.",
    ));
    steps.append(&step_row(
        "2",
        "Add a new shortcut",
        "Paste the command above and pick a name (\"Rift\" works).",
    ));
    steps.append(&step_row(
        "3",
        "Bind your key",
        "Super+Space and Ctrl+Space are both common picks.",
    ));
    page.append(&steps);

    page
}

fn step_row(number: &str, title: &str, body: &str) -> gtk::Box {
    let number_label = gtk::Label::builder()
        .label(number)
        .halign(Align::Center)
        .valign(Align::Start)
        .width_request(22)
        .height_request(22)
        .css_classes(["welcome-step-number"])
        .build();

    let title_label = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["welcome-step-title"])
        .build();
    let body_label = gtk::Label::builder()
        .label(body)
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["welcome-step-body"])
        .build();
    let text = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    text.append(&title_label);
    text.append(&body_label);

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .css_classes(["welcome-step-row"])
        .build();
    row.append(&number_label);
    row.append(&text);
    row
}

fn build_finish_page() -> gtk::Box {
    let page = page_box();

    page.append(&page_heading(
        "Try it.",
        "Press the key you just bound. Rift will pop up wherever you are.",
    ));

    let card = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .css_classes(["welcome-finish-card"])
        .build();
    card.append(&finish_row("Need to change something?", "Open the launcher, click the menu icon, hit Settings."));
    card.append(&finish_row("Want a different look?", "Drop a .rift-theme file into ~/.config/rift/themes/."));
    card.append(&finish_row("Hotkey not working?", "Some desktops only register the binding after you log out and back in."));
    page.append(&card);

    page
}

fn page_box() -> gtk::Box {
    gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(14)
        .css_classes(["welcome-page"])
        .build()
}

fn page_heading(title: &str, body: &str) -> gtk::Box {
    let title_label = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .xalign(0.0)
        .css_classes(["welcome-page-title"])
        .build();
    let body_label = gtk::Label::builder()
        .label(body)
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["welcome-page-body"])
        .build();
    let box_ = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();
    box_.append(&title_label);
    box_.append(&body_label);
    box_
}

fn feature_row(icon_name: &str, title: &str, body: &str) -> gtk::Box {
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(14);
    icon.set_valign(Align::Center);

    let chip = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::Center)
        .valign(Align::Center)
        .width_request(26)
        .height_request(26)
        .css_classes(["welcome-feature-chip"])
        .build();
    chip.set_hexpand(false);
    chip.set_hexpand_set(true);
    chip.append(&icon);
    icon.set_halign(Align::Center);
    icon.set_hexpand(true);
    icon.set_vexpand(true);

    let title_label = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["welcome-feature-title"])
        .build();
    let body_label = gtk::Label::builder()
        .label(body)
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["welcome-feature-body"])
        .build();
    let text = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .hexpand(true)
        .build();
    text.append(&title_label);
    text.append(&body_label);

    let row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .css_classes(["welcome-feature-row"])
        .build();
    row.append(&chip);
    row.append(&text);
    row
}

fn finish_row(title: &str, body: &str) -> gtk::Box {
    let title_label = gtk::Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["welcome-finish-title"])
        .build();
    let body_label = gtk::Label::builder()
        .label(body)
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["welcome-finish-body"])
        .build();
    let row = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .css_classes(["welcome-finish-row"])
        .build();
    row.append(&title_label);
    row.append(&body_label);
    row
}

fn mark_welcomed(handles: &LauncherHandles) {
    let mut config = current_config(handles);
    if config.welcomed {
        return;
    }
    config.welcomed = true;
    let _ = save_config(handles, config).unwrap_or(SaveOutcome::Applied);
}
