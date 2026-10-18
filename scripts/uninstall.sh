#!/bin/bash
# Uninstall the Sam macOS window installed by install.sh.
#
#   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/uninstall.sh)"

# macOS /bin/bash is 3.2. Keep this script compatible with that.

set -eu
set -o pipefail

DEST_NAME="sam"
OTHER_NAME="sam-tui"
LABEL="Sam GUI"
APP_BUNDLE="Sam.app"
BUNDLE_ID="io.github.luisitoluis.sam"

PREFIX="${SAM_PREFIX:-$HOME/.sam}"
BIN_DIR="${SAM_BIN_DIR:-$HOME/.local/bin}"
APP_DIR="${SAM_APP_DIR:-$HOME/Applications}"
PURGE="${SAM_PURGE:-0}"

ohai() {
  printf "\033[1;34m==>\033[0m \033[1m%s\033[0m\n" "$*"
}

warn() {
  printf "\033[1;33mWarning:\033[0m %s\n" "$*" >&2
}

abort() {
  printf "\033[1;31mError:\033[0m %s\n" "$*" >&2
  exit 1
}

usage() {
  cat <<EOF
Uninstall the $LABEL ($DEST_NAME).

Options:
  --dir DIR         Command directory used at install time (default: \$HOME/.local/bin)
  --app-dir DIR     Where Sam.app was installed (default: \$HOME/Applications)
  --purge           Also delete the shared cache and settings, unless sam-tui is still installed
  -h, --help        Show this help

The same settings can be given as SAM_BIN_DIR, SAM_APP_DIR and SAM_PURGE=1
environment variables.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)
      [[ $# -ge 2 ]] || abort "--dir needs a value."
      BIN_DIR="$2"
      shift 2
      ;;
    --app-dir)
      [[ $# -ge 2 ]] || abort "--app-dir needs a value."
      APP_DIR="$2"
      shift 2
      ;;
    --purge)
      PURGE=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      abort "Unknown option: $1 (see --help)"
      ;;
  esac
done

if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
  abort "Run this as your user, not root."
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  abort "The $LABEL uninstall is macOS-only."
fi

removed=0

# Only touch files the installers wrote: the Sam binaries embed "steamclient" and the
# sam launcher names Sam.app. Anything else with the same name (AWS SAM CLI is also
# `sam`) is left alone.
is_sam_file() {
  [[ -f "$1" ]] && grep -aqE 'steamclient|Sam\.app' "$1"
}

remove_command() {
  local path="$BIN_DIR/$1"
  if [[ ! -e "$path" ]]; then
    return 0
  fi
  if is_sam_file "$path"; then
    rm -f "$path"
    ohai "Removed $path"
    removed=1
  else
    warn "$path is not a Sam command; left it in place."
  fi
}

remove_app() {
  local app="$APP_DIR/$APP_BUNDLE" id
  if [[ ! -e "$app" ]]; then
    return 0
  fi
  id="$(plutil -extract CFBundleIdentifier raw "$app/Contents/Info.plist" 2>/dev/null || true)"
  if [[ "$id" != "$BUNDLE_ID" ]]; then
    warn "$app is not this Sam ($id); left it in place."
    return 0
  fi
  osascript -e "quit app id \"$BUNDLE_ID\"" >/dev/null 2>&1 || true
  rm -rf "$app"
  ohai "Removed $app"
  removed=1
}

other_installed() {
  is_sam_file "$BIN_DIR/$OTHER_NAME"
}

# install.sh / install-tui.sh append "# <label>" plus an export line to the shell profile.
# Drop those lines once nothing is left in the command directory.
remove_path_lines() {
  if [[ -d "$BIN_DIR" ]] && [[ -n "$(ls -A "$BIN_DIR" 2>/dev/null)" ]]; then
    return 0
  fi
  local profile export_line
  export_line="export PATH=\"$BIN_DIR:\$PATH\""
  for profile in "$HOME/.zprofile" "$HOME/.bash_profile" "$HOME/.profile"; do
    [[ -f "$profile" ]] || continue
    if grep -qxF "$export_line" "$profile" && grep -qE '^# Sam (GUI|terminal UI)$' "$profile"; then
      # Drops each "# Sam ..." comment followed by our export line, plus the blank line
      # the installer put before it.
      awk -v want="$export_line" '
        held != "" { if ($0 == want) { held = ""; blank = 0; next } if (blank) print ""; blank = 0; print held; held = "" }
        /^$/ { if (blank) print ""; blank = 1; next }
        /^# Sam (GUI|terminal UI)$/ { held = $0; next }
        { if (blank) print ""; blank = 0; print }
        END { if (blank) print ""; if (held != "") print held }
      ' "$profile" >"$profile.sam-tmp"
      if ! cmp -s "$profile" "$profile.sam-tmp"; then
        # Write over the file instead of replacing it, so symlinked dotfiles stay symlinks.
        cat "$profile.sam-tmp" >"$profile"
        ohai "Removed $BIN_DIR from PATH in $profile"
      fi
      rm -f "$profile.sam-tmp"
    fi
  done
}

ohai "Uninstalling the $LABEL"

if [[ -n "$APP_BUNDLE" ]]; then
  remove_app
fi
remove_command "$DEST_NAME"

if other_installed; then
  printf "%s is still installed, so the shared files stay.\n" "$OTHER_NAME"
else
  if [[ -d "$PREFIX" ]]; then
    rm -rf "$PREFIX"
    ohai "Removed $PREFIX"
    removed=1
  fi
  remove_path_lines
fi

if [[ "$PURGE" == "1" ]]; then
  if other_installed; then
    warn "$OTHER_NAME is still installed, so the shared cache and settings stay."
  else
    for dir in "$HOME/Library/Caches/samrewritten" "$HOME/Library/Application Support/Sam"; do
      if [[ -e "$dir" ]]; then
        rm -rf "$dir"
        ohai "Removed $dir"
        removed=1
      fi
    done
  fi
fi

if command -v brew >/dev/null 2>&1 && brew list --formula sam >/dev/null 2>&1; then
  warn "The window is also installed with Homebrew. Remove that copy with: brew uninstall sam"
fi

if [[ "$removed" == "1" ]]; then
  ohai "$LABEL uninstalled"
else
  printf "Nothing to remove: the %s was not found in %s.\n" "$LABEL" "$BIN_DIR"
fi
