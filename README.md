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

> **Note:** The login screen wordmark uses the bundled `slant` FIGlet font via `figlet-rs` — no system `figlet` install needed.

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
| `figlet-rs` | 0.1 | Render login wordmark with bundled `slant` font — no system dep |

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

The field cycle order is: **Email → Password → Save email → Auto-lock → Email**.

---

## Vault screen

### Layout

The vault screen is divided into five panels:

| Panel | Label | Description |
|-------|-------|-------------|
| `[5]` | Status | Action feedback (spinner, ✓, ✕). Read-only — not focusable. |
| `[1]` | Vaults | Vault selector (currently shows My Vault). |
| `[2]` | Items | Item type filter list. |
| `[/]` | Search | Live fuzzy search bar. |
| `[3]` | Vault | Main item list. |
| `[4]` | Command Log | Log of every `bw` CLI call and its result. |

### Navigation

| Key | Action |
|-----|--------|
| `F1` | Focus **[1]-Vaults** panel |
| `F2` | Focus **[2]-Items** filter panel |
| `F3` | Focus **[3]-Vault** list |
| `F4` | Focus **[4]-Command Log** |
| `/` | Focus **[/]-Search** bar |
| `Tab` | Cycle focus: Search → Vaults → Items → List → CmdLog → Search |
| `j` / `k` or `↑` `↓` | Navigate up/down in the focused panel |
| `PgUp` / `PgDn` | Scroll by 10 items in the list, or 5 entries in the log |

> **Note:** The `[5]-Status` pane is read-only and cannot be focused with `F5` or `Tab`. It updates automatically to reflect running actions, success, or errors.

### Actions (from vault list `[3]`)

| Key | Action |
|-----|--------|
| `Enter` / `l` | Open item detail |
| `u` | Copy username to clipboard |
| `c` | Copy password to clipboard |
| `f` | Toggle favorite ★ |
| `s` | Sync vault with server |
| `L` | **Lock vault** — runs `bw lock`, logs to Command Log, returns to login |
| `q` | Lock vault and return to login (no Command Log entry) |
| `?` | Show help popup |

### Search

Type `/` from anywhere on the vault screen to focus the search bar. The fuzzy search runs across item name, username, and URL. Results update live as you type. `Esc` clears the query and returns focus to the list. While the search bar is focused, `j`/`k` and `Enter` navigate and open items without leaving the search bar.

### Items filter panel `[2]`

Navigate with `j`/`k` then press `Enter` to apply, or click a filter to apply it immediately:

- All Items
- ★ Favorites
- Login / Card / Identity / Secure Note / SSH Key

---

## Detail screen

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` `↓` or `PgDn` / `PgUp` | Move to next / previous field (one field at a time) |
| `p` | Toggle show / hide for the selected hidden field |
| `c` | Copy the selected field to clipboard |
| `Esc` / `h` | Go back to vault |

**Hidden fields** (Password, Card Number, CVV, TOTP, SSN, Passport, License, and custom hidden fields) display `●●●●●●●●` until you press `p` while that field is selected. Navigating away from a field automatically hides it again.

All field types are displayed: Name, Type, Username, Password, URL(s), TOTP, Notes, Card fields, Identity fields, and any custom fields.

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
| Click header row (detail) | Go back to vault |

---

## Configuration

Config file: `~/.config/bytewarden/config.toml`

```toml
# Login
save_email = true
email = "you@example.com"

# Security
auto_lock = false
lock_after_minutes = 15

# Theme
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

All `[theme]` keys are optional — omit any key to keep its built-in default, so you can override only what you want.

### Auto-lock

Enable **Auto-lock** on the login screen (checkbox) or set `auto_lock = true` in `config.toml`.  
The vault locks automatically after `lock_after_minutes` minutes of inactivity (any keypress resets the timer).  
To change the timeout, edit `lock_after_minutes` in `config.toml` and restart.

---

## Command log `[4]`

Every `bw` CLI call is logged with its result. Session keys are always redacted as `***`. Passwords, TOTP codes, and clipboard values are logged as `[hidden]`. The log keeps the last 50 entries.

Scroll with `j`/`k` (1 entry) or `PgUp`/`PgDn` (5 entries) when the `[4]-Command Log` panel is focused.

---

## Clipboard

The clipboard tool is detected automatically at runtime:

| Environment | Tool used |
|-------------|-----------|
| Wayland (`$WAYLAND_DISPLAY`) | `wl-copy` |
| X11 (`$DISPLAY`) | `xclip -selection clipboard` or `xsel` |
| macOS | `pbcopy` |

---

## Keyboard reference card

```
LOGIN                  VAULT                         DETAIL
──────────────────     ──────────────────────────    ──────────────────
Tab    next field      F1-F4  focus panel            j/k    prev/next field
Enter  login/unlock    /      search                 PgUp/Dn  same as j/k
Space  toggle check    j/k    navigate               p      toggle reveal
←→     move cursor     PgUp/Dn scroll (10/5)         c      copy field
Ctrl+C quit            Enter  open detail            Esc/h  back to vault
                       u      copy username
                       c      copy password
                       f      toggle favorite
                       s      sync vault
                       L      lock (logged)
                       q      lock (silent)
                       ?      help popup
```
