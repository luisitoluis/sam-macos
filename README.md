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
- ✅ Uses Steam Linux client via Rosetta 2
- ✅ No .NET Framework required
- ✅ Native macOS window 

## Requirements

1. **Steam installed** on your Mac
2. **Rosetta 2** (for Apple Silicon users):
   ```bash
   /usr/sbin/softwareupdate --install-rosetta
   ```
3. **Rust** (to build from source):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

## Installation

Installs the window, Rust, and Rosetta 2 when they are missing. The command is `sam`.

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install.sh)"
```

Installs the terminal UI. The command is `sam-tui`.

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/luisitoluis/sam-macos-rs/master/scripts/install-tui.sh)"
```

To build them yourself:

```bash
git clone https://github.com/luisitoluis/sam-macos-rs.git
cd sam-macos-rs
```

GUI, written to `target/release/Sam`:

```bash
cargo build --bin Sam --no-default-features --features gui
```

TUI, written to `target/release/sam-tui`:

```bash
cargo build --bin sam-tui --no-default-features --features cli
```

After building, `sam-tui` from any directory opens the terminal interface. `sam-tui --version` prints the version.

## How it works on macOS

Since Valve never released a native `libsteam_api.dylib` for macOS (especially for Apple Silicon), this fork:

1. **Locates the Steam installation** in standard macOS locations
2. **Uses the Linux `steamclient.so`** library
3. **Runs via Rosetta 2** to translate Linux binaries to ARM64
4. **Loads the library** via standard `dlopen`

This is the same approach that many other Steam tools use on macOS.

## Known Issues

- **Apple Silicon**: Requires Rosetta 2 - the Steam Linux client runs in userspace translation
- **Intel Macs**: Should work more directly but untested
- **First launch**: May take a few seconds as Rosetta translates the binary

## Troubleshooting

### "Could not find steam client library"
```bash
# Make sure Steam has been launched at least once
open -a Steam

# Or set the path manually:
export SAM_STEAMCLIENT_PATH=~/.local/share/Steam/linux64/steamclient.so
```

### "Rosetta 2 not installed"
```bash
/usr/sbin/softwareupdate --install-rosetta
```

### "Failed to open library"
Ensure your Steam is installed and you've launched it at least once to generate the necessary files.

## License

 GPL-3.0 - See LICENSE file.
