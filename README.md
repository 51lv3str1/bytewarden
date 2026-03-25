# bytewarden

A terminal UI for [Bitwarden](https://bitwarden.com), built with [Ratatui](https://ratatui.rs).  
Wraps the official `bw` CLI to provide a keyboard-driven, mouse-supported vault browser.

```
    __          __                              __
   / /_  __  __/ /____ _      ______ __________/ /__  ____
  / __ \/ / / / __/ _ \ | /| / / __ `/ ___/ __  / _ \/ __ \
 / /_/ / /_/ / /_/  __/ |/ |/ / /_/ / /  / /_/ /  __/ / / /
/_.___/\__, /\__/\___/|__/|__/\__,_/_/   \__,_/\___/_/ /_/
      /____/
```

---

## Requirements

- [Bitwarden CLI](https://bitwarden.com/help/cli/) (`bw`) installed and on `$PATH`
- [Rust toolchain](https://rustup.rs) (`cargo`) to build from source
- Clipboard tool: `wl-copy` (Wayland), `xclip` / `xsel` (X11), or `pbcopy` (macOS)

> **Note:** The login screen font (`mono12`) is bundled inside the binary via `figlet-rs` — no system `figlet` install needed.

### Install system dependencies

**Ubuntu / Debian**
```bash
# Bitwarden CLI (via npm)
npm install -g @bitwarden/cli

# Or via snap
snap install bw

# Clipboard (pick one)
sudo apt install wl-clipboard      # Wayland
sudo apt install xclip             # X11
```

**Arch Linux**
```bash
sudo pacman -S bitwarden-cli wl-clipboard   # Wayland
sudo pacman -S bitwarden-cli xclip          # X11
```

**macOS**
```bash
brew install bitwarden-cli
# pbcopy is built-in — no clipboard install needed
```

**Rust (all platforms)**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Rust crate dependencies (auto-installed by cargo)

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.30 | Terminal UI framework |
| `crossterm` | 0.29 | Cross-platform terminal control, keyboard & mouse |
| `serde` + `serde_json` | 1 | Parse `bw` CLI JSON output |
| `color-eyre` | 0.6 | Error reporting |
| `figlet-rs` | 0.1 | Render login wordmark — bundles font, no system dep |

---

## Installation

```bash
git clone https://github.com/51lv3str1/bytewarden
cd bytewarden
cargo build --release
./target/release/bytewarden
```

---

## Login screen

| Action | Key / Input |
|--------|-------------|
| Switch field | `Tab` |
| Move cursor | `←` `→` `Home` `End` |
| Delete | `Backspace` / `Delete` |
| Toggle Save email | `Space` (on checkbox) or click |
| Toggle Auto-lock | `Space` (on checkbox) or click |
| Login / Unlock | `Enter` |
| Quit | `Ctrl+C` |

The password field is always masked. Email is pre-filled if **Save email** is enabled.

---

## Vault screen

### Navigation

| Key | Action |
|-----|--------|
| `F1` | Focus **[1]-Vaults** panel |
| `F2` | Focus **[2]-Items** filter panel |
| `F3` | Focus **[3]-Vault** list |
| `F4` | Focus **[4]-Command Log** |
| `F5` | Focus **[5]-Status** pane |
| `/` | Focus **[0]-Search** bar |
| `Tab` | Cycle focus through all panels |
| `j` / `k` or `↑` `↓` | Navigate up/down in focused panel |
| `PgUp` / `PgDn` | Scroll by 10 items (list) or 5 lines (log) |

### Actions (from vault list)

| Key | Action |
|-----|--------|
| `Enter` / `l` | Open item detail |
| `u` | Copy username to clipboard |
| `c` | Copy password to clipboard |
| `f` | Toggle favorite |
| `s` | Sync vault with server |
| `L` | **Lock vault** and return to login |
| `q` | Lock vault and return to login |
| `?` | Show help popup |

### Search

Type `/` from anywhere to focus the search bar. Fuzzy-searches across name, username, and URL. Results update live. `Esc` clears the query and returns focus to the list.

### Items filter panel `[2]`

Click or navigate with `j`/`k` to filter by type:

- All Items
- ★ Favorites
- Login / Card / Identity / Secure Note / SSH Key

---

## Detail screen

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` `↓` | Navigate between fields |
| `PgUp` / `PgDn` | Jump fields faster |
| `p` | Show / hide selected hidden field |
| `c` | Copy selected field to clipboard |
| `Esc` / `h` | Go back to vault |

**Hidden fields** (password, CVV, SSN, TOTP, etc.) show `●●●●●●●●` until you press `p` while that field is selected. Moving to another field auto-hides it again.

All field types are shown: Name, Type, Username, Password, URL(s), TOTP, Notes, Card fields, Identity fields, and any custom fields.

---

## Mouse support

| Action | Effect |
|--------|--------|
| Click panel | Focuses that panel |
| Click list item | Selects it |
| Click same item again | Opens detail |
| Click filter | Applies filter immediately |
| Scroll wheel | Scrolls the hovered panel |
| Click detail field | Selects field |
| Click same field again | Toggles reveal |
| Click `←` header (detail) | Go back |

---

## Configuration

Config file: `~/.config/bytewarden/config.toml`

```toml
# Login
save_email = true
email = "you@example.com"

# Security
auto_lock = false
lock_after_minutes = 15   # only read at startup; edit manually to change

# Theme — named preset
theme = "catppuccin"

# Theme — custom colors (inline comments supported)
[theme]
accent        = "#cba6f7"   # active borders, cursor, highlights
inactive      = "#6c7086"   # inactive panel borders
selected_bg   = "#313244"   # selected row background
success       = "#a6e3a1"   # success messages
error         = "#f38ba8"   # error messages
dim           = "#585b70"   # secondary text
item_login    = "#89b4fa"
item_card     = "#cba6f7"
item_identity = "#f9e2af"
item_note     = "#a6e3a1"
item_ssh      = "#b4befe"
item_favorite = "#f9e2af"
```

### Built-in theme presets

| Value | Description |
|-------|-------------|
| `"default"` | Classic dark blue/cyan |
| `"catppuccin"` | Catppuccin Macchiato |

To use a preset: `theme = "catppuccin"` (top-level key, not inside `[theme]`).

### Auto-lock

Enable **Auto-lock** on the login screen (or set `auto_lock = true` in config).  
The vault locks automatically after `lock_after_minutes` minutes of inactivity.  
To change the timeout, edit `lock_after_minutes` in `config.toml` and restart.

---

## Command log `[4]`

Every `bw` CLI call is logged with its result. Session keys are always redacted as `***`. Passwords and sensitive values are shown as `[hidden]`. Scroll with `j`/`k` or `PgUp`/`PgDn` when the log panel is focused.

---

## Clipboard

Clipboard tool is detected automatically at runtime:

| Environment | Tool used |
|-------------|-----------|
| Wayland (`$WAYLAND_DISPLAY`) | `wl-copy` |
| X11 (`$DISPLAY`) | `xclip -selection clipboard` or `xsel` |
| macOS | `pbcopy` |

---

## Keyboard reference card

```
LOGIN             VAULT                    DETAIL
─────────         ──────────────────────   ──────────────
Tab   field       F1-F5   panel            j/k    field
Enter login       /       search           p      reveal
Space toggle      j/k     navigate         c      copy
                  PgUp/Dn scroll           Esc    back
                  Enter   detail
                  u       copy user
                  c       copy pass
                  f       favorite
                  s       sync
                  L       lock
                  ?       help
```
