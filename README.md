# bytewarden

A fast terminal UI for the Bitwarden CLI, built with Rust + Ratatui.

## Requirements

- [Rust](https://rustup.rs/) 1.74+
- [Bitwarden CLI](https://bitwarden.com/help/cli/) (`bw` in PATH)

## Setup on Debian

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Install the Bitwarden CLI
npm install -g @bitwarden/cli   # installs as "bw", then alias:
alias bytewarden=bw
# or: download binary from https://bitwarden.com/download/?app=cli&platform=linux

# 3. Verify bw is working
bytewarden --version

# 4. Build and run
cargo run           # dev build
cargo run --release # optimized build
```

## Keyboard shortcuts

| Screen     | Key           | Action                        |
|------------|---------------|-------------------------------|
| **Vault**  | `j` / `↓`    | Move cursor down              |
| **Vault**  | `k` / `↑`    | Move cursor up                |
| **Vault**  | `Enter` / `l` | Open item detail              |
| **Vault**  | `/`           | Open search                   |
| **Vault**  | `c`           | Copy password to clipboard    |
| **Vault**  | `s`           | Sync vault with server        |
| **Vault**  | `q`           | Lock vault, go to login       |
| **Detail** | `p`           | Show / hide password          |
| **Detail** | `c`           | Copy password to clipboard    |
| **Detail** | `Esc` / `h`  | Back to vault                 |
| **Search** | type          | Filter instantly (in-memory)  |
| **Search** | `j` / `k`    | Navigate results              |
| **Search** | `Enter`       | Open selected result          |
| **Search** | `Esc`         | Back to vault                 |
| **Global** | `?`           | Help screen                   |
| **Global** | `Ctrl+C`      | Quit                          |

## How search works

Search is **instant** and runs entirely in memory — no subprocess calls per keystroke.

On login, all vault items are loaded once with `bw list items`. The search screen
then filters that in-memory `Vec<Item>` using a fuzzy scoring algorithm:

| Score | Condition                                      |
|-------|------------------------------------------------|
| +100  | Query is a substring of the item name          |
| +20   | Bonus: name starts with the query              |
| +50   | Query characters appear in order (subsequence) |
| +30   | Match found in username                        |
| +10   | Match found in URL                             |
| +5    | Match found in notes                           |

Results are sorted by score descending (best match first).

## Project structure

```
src/
├── main.rs    — Main loop: render → handle events → repeat
├── app.rs     — Global state (App struct), actions, fuzzy search logic
├── bw.rs      — BwClient: wraps the bw CLI subprocess
├── ui.rs      — Ratatui widget rendering for all screens
└── events.rs  — Keyboard event dispatch via crossterm
```

## Theming

bytewarden reads `~/.config/bytewarden/config.toml` for theme configuration.

### Built-in presets

```toml
# Use Catppuccin Macchiato
theme = "catppuccin"

# Use the default theme (same as not setting theme)
theme = "default"
```

### Custom colors (hex only)

```toml
[theme]
accent        = "#00d4d4"   # active panel borders, cursor
inactive      = "#8c8ca0"   # inactive panel titles
selected_bg   = "#1e3c50"   # selected list row background
success       = "#00c896"   # success messages
error         = "#e05060"   # error messages
dim           = "#888888"   # secondary text
item_login    = "#5b8fff"   # [Login] type label
item_card     = "#c060e0"   # [Card] type label
item_identity = "#e0b840"   # [Identity] type label
item_note     = "#00c896"   # [Note] type label
item_ssh      = "#a060e0"   # [SSH] type label
item_favorite = "#ffc800"   # ★ favorite star
```

You can mix a preset with overrides — `theme = "catppuccin"` loads the preset, and individual `[theme]` keys override specific colors.
