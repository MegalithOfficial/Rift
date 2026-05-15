# Rift

Rift is an early Rust/GTK launcher for Linux. The goal is a small, keyboard-first command palette in the same category as Ulauncher, but built around native GTK, desktop portals, and fast local providers.

It is usable for basic testing today, but it is not packaged or stable yet.

## Current State

Rift currently has:

- a resident GTK launcher window
- GNOME/Wayland global shortcut support through the XDG GlobalShortcuts portal
- app search backed by `gio::AppInfo`
- weighted fuzzy ranking with acronym, word-boundary, subsequence, typo-tolerant, and usage-history scoring
- app launching with `Enter`
- calculator results for expressions such as `12 / 3 + 7`
- terminal command results for queries prefixed with `>`, such as `> flatpak update`
- persistent usage history stored under `~/.local/state/rift/history.tsv`
- dismiss on `Esc` and focus loss
- simple fade in/out animation

The UI is still in flux. The current work is focused on behavior and launcher ergonomics before final styling.

## Requirements

- Rust toolchain
- GTK4 development files
- Libadwaita development files
- a desktop portal backend with `org.freedesktop.portal.GlobalShortcuts` for the global shortcut

The shortcut flow has been tested against GNOME's portal path. Other desktops may behave differently or not support the portal yet.

## Run

For normal development:

```bash
cargo run
```

For resident/background testing:

```bash
cargo build
setsid nohup ./target/debug/rift >/tmp/rift.log 2>&1 < /dev/null &
```

## Global Shortcut Setup

In development, the portal needs a desktop entry for `dev.rift.launcher`. Build once, then install the local desktop entry:

```bash
cargo build
./scripts/install-dev-desktop-entry.sh
```

This writes:

```text
~/.local/share/applications/dev.rift.launcher.desktop
```

The entry points at the local debug binary in `target/debug/rift`.

Rift currently requests `Ctrl+Space` as the preferred summon shortcut. GNOME may show a portal dialog the first time and can accept, reject, or change the binding.

## Query Syntax

- Type an app name to search installed applications.
- Type a calculator expression to copy the result, for example `2 + 2`.
- Prefix a command with `>` to run it in a terminal, for example `> htop`.

## Known Limitations

- No file or folder provider yet.
- No configurable shortcut UI yet.
- No packaged install target yet.
- The calculator uses `meval`, which currently pulls an old `nom` version that emits a future-compat warning.
- Global shortcuts depend on the desktop portal implementation.
