## Install

Homebrew, installs the window (`sam`) and the terminal UI (`sam-tui`):

```bash
brew install luisitoluis/tap/sam
```

Script (no Homebrew needed), the window. The command is `sam`:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install.sh)"
```

Script, the terminal UI. The command is `sam-tui`:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install-tui.sh)"
```

All of them install without the macOS "Apple could not verify" warning.

### Downloading the archive from this page

Sam is not notarized by Apple, so a copy downloaded with a browser is blocked the first time
("Apple could not verify Sam..."). This applies to both `Sam.app` and `sam-tui`. After extracting
the archive, either run:

```bash
xattr -dr com.apple.quarantine Sam.app sam-tui
```

or try to open each one once (`./sam-tui` from Terminal for the TUI), then go to
**System Settings → Privacy & Security** and click **Open Anyway**.
