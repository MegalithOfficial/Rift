use gtk::gdk;
use std::cell::RefCell;

thread_local! {
    static CSS_PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

const LAUNCHER_WINDOW_CSS: &str = r#"
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
    background-color: alpha(#16161a, 0.82);
    border: 1px solid alpha(#4a4a52, 0.58);
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
"#;

const LAUNCHER_SEARCH_CSS: &str = r#"
window.rift-window box.search-row {
    min-height: 52px;
    border-radius: 26px;
    background-color: alpha(#202026, 0.74);
    border: 1px solid alpha(#4a4a52, 0.48);
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
"#;

const LAUNCHER_RESULTS_CSS: &str = r#"
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
    background-color: alpha(#42424b, 0.82);
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
"#;

const SETTINGS_WINDOW_CSS: &str = r#"
window.rift-settings-window {
    background-color: #18181c;
    color: #f4f4f5;
    border-radius: 14px;
    border: 1px solid #2e2e33;
}

window.rift-settings-window box.settings-header {
    margin-bottom: 6px;
    min-height: 30px;
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

window.rift-settings-window box.settings-banner {
    background-color: alpha(#5b8bd6, 0.12);
    border: 1px solid alpha(#5b8bd6, 0.32);
    border-radius: 8px;
    padding: 9px 12px;
    color: #cfdcef;
}

window.rift-settings-window box.settings-banner.error {
    background-color: alpha(#e07b9b, 0.12);
    border-color: alpha(#e07b9b, 0.4);
    color: #f3c2d0;
}

window.rift-settings-window box.settings-banner image {
    color: inherit;
}

window.rift-settings-window label.settings-banner-text {
    font-size: 12px;
    color: inherit;
}

window.rift-settings-window button.settings-banner-action {
    min-height: 24px;
    padding: 0 10px;
    border-radius: 6px;
    background-color: alpha(#ffffff, 0.06);
    border: 1px solid alpha(#ffffff, 0.10);
    color: #e4e4e7;
    font-size: 12px;
    font-weight: 500;
}

window.rift-settings-window button.settings-banner-action:hover {
    background-color: alpha(#ffffff, 0.12);
}

window.rift-settings-window button.settings-banner-dismiss {
    min-width: 22px;
    min-height: 22px;
    padding: 0;
    border-radius: 999px;
    background-color: transparent;
    border: none;
    color: inherit;
    margin-left: 2px;
}

window.rift-settings-window button.settings-banner-dismiss:hover {
    background-color: alpha(#ffffff, 0.10);
}

window.rift-settings-window button.settings-banner-dismiss image {
    -gtk-icon-size: 10px;
    color: inherit;
}

window.rift-settings-window box.settings-section {
    background-color: transparent;
    border: none;
    padding: 0;
}

window.rift-settings-window label.settings-section-title {
    color: #8b8b94;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin: 0 2px 8px 2px;
}

window.rift-settings-window box.settings-group {
    background-color: alpha(#232327, 0.76);
    border: 1px solid alpha(#45454d, 0.42);
    border-radius: 10px;
}

window.rift-settings-window box.settings-group > box.settings-row {
    padding: 12px 14px;
    min-height: 34px;
    border-bottom: 1px solid #2e2e33;
}

window.rift-settings-window box.settings-group > box.settings-row:last-child {
    border-bottom: none;
}

window.rift-settings-window label.settings-row-label {
    color: #ececf0;
    font-size: 13px;
}

window.rift-settings-window label.settings-row-helper {
    color: #85858c;
    font-size: 11px;
}

window.rift-settings-window label.settings-row-value {
    color: #d8d8dc;
    font-size: 11px;
}

window.rift-settings-window label.settings-row-value.path {
    color: #c7cad2;
    font-family: monospace;
}

window.rift-settings-window spinbutton.settings-spin {
    min-height: 28px;
    min-width: 84px;
    padding: 0;
    border-radius: 7px;
    background-color: #161619;
    border: 1px solid #34343a;
    color: #f4f4f5;
    font-size: 13px;
}

window.rift-settings-window dropdown.settings-dropdown {
    min-height: 28px;
    min-width: 150px;
    border-radius: 7px;
    background-color: #161619;
    border: 1px solid #34343a;
    color: #f4f4f5;
    font-size: 12px;
}

window.rift-settings-window dropdown.settings-dropdown button {
    min-height: 28px;
    padding: 0 10px;
    border-radius: 7px;
    background-color: transparent;
    border: none;
    color: inherit;
}

window.rift-settings-window dropdown.settings-dropdown:hover {
    border-color: #44444c;
}

window.rift-settings-window spinbutton.settings-spin text {
    background-color: transparent;
    color: inherit;
    min-height: 26px;
    padding: 0 6px;
}

window.rift-settings-window spinbutton.settings-spin button {
    min-height: 26px;
    min-width: 20px;
    padding: 0;
    background-color: transparent;
    border: none;
    color: #9a9aa1;
}

window.rift-settings-window spinbutton.settings-spin button:hover {
    background-color: alpha(#ffffff, 0.06);
    color: #f4f4f5;
}

window.rift-settings-window button.settings-shortcut-button {
    min-height: 28px;
    min-width: 120px;
    padding: 0 12px;
    border-radius: 7px;
    background-color: #161619;
    border: 1px solid #34343a;
    color: #e4e4e7;
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.02em;
}

window.rift-settings-window button.settings-shortcut-button:hover {
    background-color: #1a1a1e;
    border-color: #44444c;
    color: #f4f4f5;
}

window.rift-settings-window switch {
    margin: 0;
    min-width: 36px;
    min-height: 20px;
}

window.rift-settings-window box.settings-footer-box {
    margin-top: 4px;
}

window.rift-settings-window label.settings-footer {
    color: #6a6a72;
    font-size: 11px;
    letter-spacing: 0.02em;
}

/* ─── Sidebar layout ──────────────────────────────────────────── */

window.rift-settings-window box.settings-body {
    margin: 4px 0 6px 0;
}

window.rift-settings-window box.settings-sidebar-wrap {
    background-color: alpha(#ffffff, 0.03);
    border: 1px solid alpha(#ffffff, 0.06);
    border-radius: 12px;
    padding: 4px;
}

window.rift-settings-window list.settings-sidebar {
    background-color: transparent;
    min-width: 170px;
    padding: 2px;
}

window.rift-settings-window list.settings-sidebar row {
    border-radius: 8px;
    padding: 0;
    margin: 1px 0;
    background-color: transparent;
    transition: background-color 120ms ease;
}

window.rift-settings-window list.settings-sidebar row:hover {
    background-color: alpha(#ffffff, 0.04);
}

window.rift-settings-window list.settings-sidebar row:selected {
    background-color: alpha(#ffffff, 0.07);
}

window.rift-settings-window list.settings-sidebar row:selected:hover {
    background-color: alpha(#ffffff, 0.10);
}

window.rift-settings-window box.settings-sidebar-row {
    padding: 6px 8px;
}

window.rift-settings-window label.settings-sidebar-label {
    color: #e4e4e7;
    font-size: 13px;
    font-weight: 500;
    letter-spacing: -0.005em;
}

/* Sidebar icons (monochrome) */

window.rift-settings-window box.settings-sidebar-chip {
    background-color: transparent;
}

window.rift-settings-window box.settings-sidebar-chip image {
    color: #9a9aa1;
    -gtk-icon-size: 14px;
}

window.rift-settings-window list.settings-sidebar row:selected box.settings-sidebar-chip image,
window.rift-settings-window list.settings-sidebar row:hover box.settings-sidebar-chip image {
    color: #f4f4f5;
}

window.rift-settings-window scrolledwindow.settings-content-scroll {
    background-color: transparent;
    border: none;
}

window.rift-settings-window scrolledwindow.settings-content-scroll > viewport {
    background-color: transparent;
}

window.rift-settings-window box.settings-panel {
    padding: 4px 18px 12px 18px;
}

window.rift-settings-window box.settings-panel.spacious {
    padding: 14px 24px 16px 24px;
}

window.rift-settings-window box.settings-panel.spacious box.settings-group > box.settings-row {
    padding: 16px 16px;
}

/* ─── Theme panel ─────────────────────────────────────────────── */

window.rift-settings-window box.settings-theme-detail {
    background-color: alpha(#232327, 0.74);
    border: 1px solid alpha(#45454d, 0.42);
    border-radius: 10px;
    padding: 12px 14px;
}

window.rift-settings-window label.settings-theme-detail-label {
    color: #8b8b94;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    margin-top: 4px;
}

window.rift-settings-window label.settings-theme-detail-label:first-child {
    margin-top: 0;
}

window.rift-settings-window label.settings-mono-value {
    color: #cfcfd6;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 11px;
}

window.rift-settings-window box.settings-validation-pill {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid transparent;
}

window.rift-settings-window box.settings-validation-pill.ok {
    background-color: alpha(#62c089, 0.14);
    border-color: alpha(#62c089, 0.36);
    color: #b5e9c8;
}

window.rift-settings-window box.settings-validation-pill.error {
    background-color: alpha(#e07b9b, 0.14);
    border-color: alpha(#e07b9b, 0.42);
    color: #f4c4d2;
}

window.rift-settings-window box.settings-validation-pill image {
    color: inherit;
}

window.rift-settings-window label.settings-validation-text {
    color: inherit;
    font-size: 11px;
    font-weight: 500;
}

window.rift-settings-window box.settings-theme-actions {
    margin-top: 2px;
}

window.rift-settings-window button.settings-action-button {
    min-height: 28px;
    padding: 0 12px;
    border-radius: 7px;
    background-color: alpha(#ffffff, 0.04);
    border: 1px solid alpha(#ffffff, 0.08);
    color: #e4e4e7;
}

window.rift-settings-window button.settings-action-button:hover {
    background-color: alpha(#ffffff, 0.09);
    border-color: alpha(#ffffff, 0.16);
    color: #f4f4f5;
}

window.rift-settings-window button.settings-action-button image {
    color: inherit;
}

window.rift-settings-window label.settings-action-button-label {
    color: inherit;
    font-size: 12px;
    font-weight: 500;
}
"#;

const SHORTCUT_CAPTURE_CSS: &str = r#"
window.rift-shortcut-capture {
    background-color: alpha(#18181c, 0.92);
    color: #f4f4f5;
    border-radius: 14px;
    border: 1px solid alpha(#414149, 0.64);
}

window.rift-shortcut-capture box.settings-capture-box {
    background-color: transparent;
}

window.rift-shortcut-capture label.settings-capture-title {
    color: #f4f4f5;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
}

window.rift-shortcut-capture label.settings-capture-hint {
    color: #85858c;
    font-size: 12px;
}

window.rift-shortcut-capture box.settings-capture-keycap {
    background-color: #161619;
    border: 1px solid #34343a;
    border-radius: 10px;
    padding: 14px 22px;
    margin: 4px 0;
    min-height: 44px;
}

window.rift-shortcut-capture label.settings-capture-value {
    color: #f4f4f5;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: 0.04em;
}

window.rift-shortcut-capture box.settings-capture-actions {
    margin-top: 6px;
}

window.rift-shortcut-capture button.settings-capture-cancel,
window.rift-shortcut-capture button.settings-capture-confirm {
    min-height: 30px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
}

window.rift-shortcut-capture button.settings-capture-cancel {
    background-color: #232327;
    border: 1px solid #34343a;
    color: #e4e4e7;
}

window.rift-shortcut-capture button.settings-capture-cancel:hover {
    background-color: #2a2a2f;
}
"#;

pub(crate) fn built_in_css() -> String {
    [
        LAUNCHER_WINDOW_CSS,
        LAUNCHER_SEARCH_CSS,
        LAUNCHER_RESULTS_CSS,
        SETTINGS_WINDOW_CSS,
        SHORTCUT_CAPTURE_CSS,
    ]
    .join("\n")
}

pub(super) fn install_css(css: &str) {
    CSS_PROVIDER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let provider = slot.get_or_insert_with(|| {
            let provider = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().expect("display unavailable"),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
            provider
        });
        provider.load_from_data(css);
    });
}
