Steam Achievement Manager For macOS 
===

<p align=center>
    <img src="assets/icon_256.png" alt="SamRewrittenLogo">
</p>

<p align="center">A Steam Achievement Manager for macOS.</p>
<p align="center">
    <a href="https://github.com/luisitoluis/sam-rust-mac">DOWNLOAD</a>
</p>

<p align=center>
    <em>
        This fork adds native macOS Apple Silicon support to SamRewritten.
    </em>
</p>

## What is this fork?

This is a fork of [SamRewritten](https://github.com/PaulCombal/SamRewritten) that adds macOS support (Apple Silicon & Intel).

**Key features:**
- ✅ Native macOS support (All Apple Silicon)
- ✅ Uses the steamclient.dylib that ships with Steam for macOS
- ✅ No .NET Framework required
- ✅ Native macOS window 

## Requirements

1. **Steam installed** on your Mac, open and signed in
2. **Rust**, only if you build from source:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

## Installation

### Homebrew

Installs the window (`sam`) and the terminal UI (`sam-tui`):

```bash
brew install luisitoluis/tap/sam
```

`Sam.app` stays inside Homebrew's folder. To also open it from Finder, Launchpad or Spotlight:

```bash
ln -sfn "$(brew --prefix sam)/Sam.app" ~/Applications/Sam.app
```

### Script

The installers download the prebuilt binary for your Mac (Apple Silicon or Intel) from the
[latest release](https://github.com/luisitoluis/sam-macos-rs/releases/latest), check it against
the release's `checksums.txt` and install it. No Rust needed.

Installs the window as `Sam.app` in `~/Applications`, so it opens from Launchpad, Spotlight or
Finder without a Terminal window. The `sam` command opens the same app.

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install.sh)"
```

Installs the terminal UI. The command is `sam-tui`.

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install-tui.sh)"
```

If a release has no binary for your Mac, the installer builds it from source instead (installing
Rust when it is missing). Settings are passed as environment variables in front of the command:

| Variable | Effect |
| --- | --- |
| `SAM_METHOD=source` | Always build from source. `binary` never falls back. |
| `SAM_VERSION=v1.0.2` | Install that release instead of the latest. |
| `SAM_BIN_DIR=/path` | Put the `sam` / `sam-tui` command somewhere other than `~/.local/bin`. |
| `SAM_APP_DIR=/path` | Put `Sam.app` somewhere other than `~/Applications`. |
```bash
SAM_METHOD=source /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install.sh)"
```

### Uninstall

Removes what the matching script installed: `Sam.app` and the `sam` command, or `sam-tui`. The
cloned source in `~/.sam` and the `PATH` line in your shell profile go too, once neither the
window nor the terminal UI is left.

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/uninstall.sh)"
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/uninstall-tui.sh)"
```

To also delete Sam's cache and settings, which the window and the terminal UI share:

```bash
SAM_PURGE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/uninstall.sh)"
```

Installed with Homebrew? Use `brew uninstall sam` instead.

### Downloading the release with a browser

Sam is not notarized by Apple, so a copy downloaded from the releases page with a browser is
blocked the first time ("Apple could not verify Sam..."). Homebrew and the script above download
with `curl`, which does not trigger this. If you downloaded the archive anyway, after extracting it
either run:

```bash
xattr -dr com.apple.quarantine Sam.app sam-tui
```

or try to open Sam once, then go to **System Settings → Privacy & Security** and click
**Open Anyway**.

### Build from source

To build them yourself:

```bash
git clone https://github.com/luisitoluis/sam-macos-rs.git
cd sam-macos-rs
```

Both at once, written to `target/release/Sam.app` (GUI) and `target/release/sam-tui` (TUI):

```bash
cargo build --release
```

Only the GUI, or only the TUI:

```bash
cargo build --release --bin sam --no-default-features --features gui
cargo build --release --bin sam-tui --no-default-features --features cli
```

`Sam.app` opens without Terminal. Cargo still writes the bare `sam` executable next to it, but it
is hidden in Finder because double-clicking it would open Terminal. Drag `Sam.app` to
`~/Applications` to keep it.

After building, `sam-tui` from any directory opens the terminal interface. `sam-tui --version` prints the version.

## How it works on macOS

The macOS Steam client ships a universal (arm64 and x86_64) `steamclient.dylib`. Sam:

1. **Finds it** at `~/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamclient.dylib`
2. **Loads it** with `dlopen`
3. **Talks to the running Steam client** through the same interfaces SamRewritten uses on Linux

A Linux `steamclient.so` is still picked up as a fallback if the dylib is missing.

## Known Issues

- **Intel Macs**: Binaries are published for every release, but untested

## Troubleshooting

### "Steam connection failed"
Open Steam and sign in before starting Sam.

### "Could not find steam client library"
```bash
# Make sure Steam has been launched at least once
open -a Steam

# Or set the path manually:
export SAM_STEAMCLIENT_PATH="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamclient.dylib"
```

### "Failed to open library"
Ensure your Steam is installed and you've launched it at least once to generate the necessary files.

## License

 GPL-3.0 - See LICENSE file.
