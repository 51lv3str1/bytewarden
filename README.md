# bytewarden

A terminal UI for [Bitwarden](https://bitwarden.com), built with [Ratatui](https://ratatui.rs).  
Wraps the official `bw` CLI — keyboard-driven, mouse-supported vault browser with full CRUD.

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

> The login wordmark uses the bundled `slant` FIGlet font via `figlet-rs` — no system `figlet` install needed.

### Install system dependencies

**Ubuntu / Debian**
```bash
npm install -g @bitwarden/cli   # or: snap install bw
sudo apt install wl-clipboard   # Wayland
sudo apt install xclip          # X11
```

**Arch Linux**
```bash
sudo pacman -S bitwarden-cli wl-clipboard   # Wayland
sudo pacman -S bitwarden-cli xclip          # X11
```

**macOS**
```bash
brew install bitwarden-cli
# pbcopy is built-in
```

**Rust (all platforms)**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Rust crate dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.30 | Terminal UI framework |
| `crossterm` | 0.29 | Cross-platform terminal control, keyboard & mouse |
| `serde` + `serde_json` | 1 | Parse `bw` CLI JSON output |
| `color-eyre` | 0.6 | Error reporting |
| `figlet-rs` | 0.1 | Login wordmark with bundled `slant` font |

---

## Installation

```bash
git clone https://github.com/51lv3str1/bytewarden
cd bytewarden
cargo build --release
./target/release/bytewarden
```

---

## Session resume

On every startup bytewarden runs `bw status` before showing the login screen and fast-paths the UI:

| `bw status` | What bytewarden does |
|-------------|----------------------|
| `unauthenticated` | Normal login — enter email and master password. |
| `locked` | Email pre-filled, cursor jumps to password field. |
| `unlocked` | Active session found in `BW_SESSION` — vault loads immediately, login screen skipped. |

A `⠋ Checking session…` spinner is visible while the check runs. The `unlocked` fast-path requires `BW_SESSION` to be exported in the shell:

```bash
export BW_SESSION=$(bw unlock --raw)
bytewarden
```

If `BW_SESSION` is missing or expired, bytewarden falls back to the `locked` path.

---

## Login screen

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle fields (Email → Password → Save email → Auto-lock → Email) |
| `←` `→` `Home` `End` | Move cursor in text fields |
| `Backspace` / `Delete` | Delete character |
| `Space` | Toggle checkbox (Save email / Auto-lock) |
| `Enter` | Login / Unlock |
| `Ctrl+C` | Quit |

The password is always masked. Email is pre-filled when **Save email** is enabled.

The feedback strip at the bottom shows login state:

| State | Display |
|-------|---------|
| Checking session | `⠋ Checking session…` (spinner) |
| Logging in | `⠋ Logging in…` (spinner) |
| Loading vault | `⠋ Loading vault…` (spinner) |
| Success | `✓ Loaded ✓` (green) |
| Wrong credentials | `✕ Invalid credentials. Please try again.` (red) |

---

## Vault screen

### Layout

| Panel | Label | Description |
|-------|-------|-------------|
| `[0]` | Status | Action feedback — spinner, ✓, ✕. Read-only. |
| `[1]` | Vaults | Vault selector (currently My Vault). |
| `[2]` | Items | Item type filter. |
| `[/]` | Search | Live fuzzy search bar. |
| `[3]` | Vault | Main item list. |
| `[4]` | Command Log | Log of every `bw` CLI call. |

### Panel navigation

| Key | Action |
|-----|--------|
| `0` | Focus **[0]-Status** |
| `1` | Focus **[1]-Vaults** |
| `2` | Focus **[2]-Items** |
| `3` | Focus **[3]-Vault** |
| `4` | Focus **[4]-Command Log** |
| `/` | Focus **[/]-Search** |
| `Tab` | Cycle: Search → Vaults → Items → List → CmdLog → Search |

Number keys `0`–`4` are disabled while Search is focused to allow typing.

### Vault list actions

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` `↓` | Navigate up / down |
| `PgUp` / `PgDn` | Scroll 10 items |
| `Enter` / `l` | Open item detail |
| `Alt+N` | **New item** — create a new vault item |
| `Alt+U` | Copy username to clipboard |
| `Alt+C` | Copy password to clipboard |
| `Alt+F` | Toggle favorite ★ |
| `Alt+S` | Sync vault with server |
| `Alt+D` | **Delete item** — opens confirmation popup |
| `Alt+L` | Lock vault |
| `Alt+Q` | Lock vault |
| `?` | Help popup |

All `Alt+` shortcuts also work while the **Search** bar is focused.

### Search

Press `/` to focus the search bar. Fuzzy search runs across name, username, and URL — results update live. `Esc` clears the query and returns to the list. While focused, `j`/`k` and `Enter` navigate and open items.

### Items filter `[2]`

Navigate with `j`/`k`, press `Enter` to apply, or click to apply immediately:
All Items · ★ Favorites · Login · Card · Identity · Secure Note · SSH Key

---

## Detail screen

### Read mode

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` `↓` | Move between fields |
| `Tab` / `Shift+Tab` | Move between fields (wraps) |
| `PgUp` / `PgDn` | Same as `k` / `j` |
| `F2` | Toggle reveal on selected hidden field |
| `Alt+C` | Copy selected field to clipboard |
| `Alt+E` | **Enter edit mode** |
| `Alt+D` | **Delete item** — opens confirmation popup |
| `Esc` / `h` | Back to vault |

Hidden fields (Password, Card Number, CVV, TOTP, SSN, Passport, License, custom hidden fields) show `●●●●●●●●` until `F2` is pressed. Navigating away re-hides the field.

### Edit mode

Press `e` from read mode to edit any item inline.

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field (wraps) |
| `↓` / `↑` | Next / previous field (clamps) |
| `←` `→` `Home` `End` | Move cursor within field |
| `Backspace` / `Delete` | Delete character |
| `F2` | Reveal / hide hidden field while editing |
| `Enter` | **Save** — calls `bw edit item` |
| `Esc` | Cancel — back to read mode (no changes saved) |

The **Type** field is read-only. All other fields are editable. Changes are saved atomically via `bw edit item` — the local item list is updated immediately on success.

---

## Create screen

Press `Alt+N` from the vault list (or search bar) to create a new item.

**Step 1 — choose type:**

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` `↓` | Navigate types |
| `Tab` / `Shift+Tab` | Navigate types (wraps) |
| `Enter` | Select type and go to fields |
| `Esc` | Cancel |

Supported types: **Login**, **Secure Note**, **Card**, **Identity**.

**Step 2 — fill fields:**

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field (wraps) |
| `↓` / `↑` | Next / previous field (clamps) |
| `←` `→` `Home` `End` | Move cursor |
| `Backspace` / `Delete` | Delete character |
| `F2` | Reveal / hide hidden field |
| `Enter` | **Create** — calls `bw create item` |
| `Esc` | Cancel |

The Name field is required. On success the item is inserted into the local list and the vault screen is shown with the new item selected.

---

## Delete confirmation

Press `D` from the vault list or detail screen to delete an item.

| Key | Action |
|-----|--------|
| `Enter` | Move to trash (`bw delete item`) |
| `D` | **Permanent delete** (`bw delete item --permanent`) |
| `Esc` / `n` | Cancel |

---

## Mouse support

| Action | Effect |
|--------|--------|
| Click panel | Focus that panel |
| Click list item | Select it |
| Click same item again | Open detail |
| Click filter | Apply filter immediately |
| Scroll wheel | Scroll the hovered panel |
| Click detail field | Select field |
| Click same field again | Toggle reveal |
| Click header row (detail) | Back to vault |

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

# Theme (all keys optional — omit to keep built-in default)
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

### Auto-lock

Enable on the login screen or set `auto_lock = true` in config. The vault locks after `lock_after_minutes` minutes of inactivity — any keypress resets the timer.

---

## Command Log `[4]`

Every `bw` CLI call is logged with its result. Session keys are always redacted as `***`. Passwords, TOTP codes, and clipboard values are logged as `[hidden]`. Keeps the last 50 entries.

Focus with `4`, scroll with `j`/`k` (1 line) or `PgUp`/`PgDn` (5 lines).

---

## Clipboard

Detected automatically at runtime:

| Environment | Tool |
|-------------|------|
| Wayland (`$WAYLAND_DISPLAY`) | `wl-copy` |
| X11 (`$DISPLAY`) | `xclip -selection clipboard` or `xsel` |
| macOS | `pbcopy` |

---

## Keyboard reference

```
LOGIN                      VAULT LIST / SEARCH           DETAIL (read)
─────────────────────      ────────────────────────────  ───────────────────────
Tab/S+Tab  next field      0-4     focus panel           j/k / Tab   prev/next field
Enter      login/unlock    /       focus search          F2          reveal/hide
Space      toggle check    j/k     navigate              Alt+C       copy field
←→         cursor          Enter   open detail           Alt+E       edit item
Ctrl+C     quit            Alt+N   new item              Alt+D       delete item
                           Alt+U   copy username         Esc/h       back to vault
                           Alt+C   copy password
                           Alt+F   toggle favorite       DETAIL (edit)
                           Alt+S   sync vault            ─────────────────────────
                           Alt+D   delete item           Tab/S+Tab   next/prev field
                           Alt+L   lock vault            ←→          cursor in field
                           ?       help                  F2          reveal/hide
                                                         Enter       save
CREATE (type select)        CREATE (fill fields)         Esc         cancel edit
────────────────────        ───────────────────
j/k / Tab  select type      Tab/S+Tab  next/prev field   CONFIRM DELETE
Enter      confirm          ←→         cursor            ──────────────────────
Esc        cancel           F2         reveal/hide       Enter   move to trash
                            Enter      create            D       permanent delete
                            Esc        cancel            Esc/n   cancel
```
