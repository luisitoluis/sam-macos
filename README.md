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
2. **Rust** (to build from source):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

## Installation

Installs the window, and Rust when it is missing. The command is `sam`.

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

The macOS Steam client ships a universal (arm64 and x86_64) `steamclient.dylib`. Sam:

1. **Finds it** at `~/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamclient.dylib`
2. **Loads it** with `dlopen`
3. **Talks to the running Steam client** through the same interfaces SamRewritten uses on Linux

A Linux `steamclient.so` is still picked up as a fallback if the dylib is missing.

## Known Issues

- **Intel Macs**: Should work, but untested

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
