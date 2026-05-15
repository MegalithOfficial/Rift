#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE_PATH="$ROOT_DIR/data/dev.rift.launcher.desktop.in"
TARGET_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
TARGET_PATH="$TARGET_DIR/dev.rift.launcher.desktop"
EXEC_PATH="$ROOT_DIR/target/debug/rift"

mkdir -p "$TARGET_DIR"

if [[ ! -x "$EXEC_PATH" ]]; then
  echo "rift binary not found at $EXEC_PATH" >&2
  echo "build it first with: cargo build" >&2
  exit 1
fi

python3 - <<'PY' "$TEMPLATE_PATH" "$TARGET_PATH" "$EXEC_PATH"
from pathlib import Path
import shlex
import sys

template_path = Path(sys.argv[1])
target_path = Path(sys.argv[2])
exec_path = Path(sys.argv[3])

template = template_path.read_text()
desktop = template.replace("__RIFT_EXEC__", shlex.quote(str(exec_path)))
target_path.write_text(desktop)
PY

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$TARGET_DIR" >/dev/null 2>&1 || true
fi

echo "installed $TARGET_PATH"
