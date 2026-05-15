use gtk::gdk;

pub(super) fn install_css() {
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
            background-color: #232327;
            border: 1px solid #2e2e33;
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

        window.rift-shortcut-capture {
            background-color: #1c1c1f;
            color: #f4f4f5;
            border-radius: 14px;
            border: 1px solid #2e2e33;
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
        "#,
    );

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("display unavailable"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}
