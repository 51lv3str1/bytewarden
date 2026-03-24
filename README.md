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
npm install -g @bitwarden/cli   # preferred
# or: download binary from https://bitwarden.com/download/?app=cli&platform=linux

# 3. Verify bw is working
bw --version

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