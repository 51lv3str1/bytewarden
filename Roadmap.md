# Bytewarden — Roadmap

Full feature parity with the Bitwarden CLI, organized by product area.  
~~Strikethrough~~ = fully implemented.

---

## 1. Authentication

| # | Feature | CLI |
|---|---------|-----|
| 1.1 | ~~Email + master password login~~ | `bw login` |
| 1.2 | ~~Vault unlock (locked session)~~ | `bw unlock` |
| 1.3 | ~~Lock vault~~ | `bw lock` |
| 1.4 | ~~Logout~~ | `bw logout` |
| 1.5 | ~~Session key management via `BW_SESSION`~~ | `--session` |
| 1.6 | ~~Auto-lock after inactivity~~ | `bw lock` |
| 1.7 | ~~Session detection on startup (`unlocked` / `locked` / `unauthenticated`)~~ | `bw status` |
| 1.8 | Two-factor authentication — Authenticator (method `0`), Email (`1`), YubiKey (`3`) | `bw login --method --code` |
| 1.9 | API key login | `bw login --apikey` |
| 1.10 | SSO login | `bw login --sso` |
| 1.11 | Master password re-prompt for items with `reprompt: 1` | — |
| 1.12 | Multiple account support via `BITWARDENCLI_APPDATA_DIR` | env var |

---

## 2. Vault Browsing & Search

| # | Feature | CLI |
|---|---------|-----|
| 2.1 | ~~List all vault items~~ | `bw list items` |
| 2.2 | ~~Live fuzzy search~~ | `--search` |
| 2.3 | ~~Filter by item type (Login, Card, Identity, Secure Note, SSH Key)~~ | — |
| 2.4 | ~~Filter favorites~~ | — |
| 2.5 | ~~Trash view~~ | `--trash` |
| 2.6 | Filter by folder | `--folderid` |
| 2.7 | Filter by collection | `--collectionid` |
| 2.8 | Filter by organization vault | `--organizationid` |
| 2.9 | Filter by URL | `--url` |
| 2.10 | Show `serverUrl` and `lastSync` in status pane | `bw status` |
| 2.11 | Display user fingerprint phrase | `bw get fingerprint me` |

---

## 3. Item Detail & Copy

| # | Feature | CLI |
|---|---------|-----|
| 3.1 | ~~View all fields for Login, Card, Identity, Secure Note~~ | `bw get item` |
| 3.2 | ~~Reveal hidden fields (F2)~~ | — |
| 3.3 | ~~Copy username from memory~~ | — |
| 3.4 | ~~Copy password from memory~~ | — |
| 3.5 | ~~Copy TOTP live code~~ | `bw get totp` |
| 3.6 | ~~Copy URI~~ | — |
| 3.7 | ~~Copy card fields (number, CVV, expiry)~~ | — |
| 3.8 | ~~Copy identity fields~~ | — |
| 3.9 | ~~Copy notes~~ | — |
| 3.10 | ~~Copy custom fields~~ | — |
| 3.11 | View SSH key fields (public key, private key, fingerprint) | — |

---

## 4. Item Create & Edit

| # | Feature | CLI |
|---|---------|-----|
| 4.1 | ~~Create Login item~~ | `bw create item` |
| 4.2 | ~~Create Secure Note~~ | `bw create item` |
| 4.3 | ~~Create Card item~~ | `bw create item` |
| 4.4 | ~~Create Identity item~~ | `bw create item` |
| 4.5 | ~~Edit all fields inline~~ | `bw edit item` |
| 4.6 | ~~Delete to trash~~ | `bw delete item` |
| 4.7 | ~~Permanent delete~~ | `bw delete item --permanent` |
| 4.8 | ~~Restore from trash~~ | `bw restore item` |
| 4.9 | ~~Toggle favorite~~ | `bw edit item` |
| 4.10 | Create SSH Key item | `bw create item` (type `5`) |
| 4.11 | Multiple URIs per Login item | — |
| 4.12 | URI match type per URL (Domain, Host, Exact, Regex, Never) | `match` field |
| 4.13 | Add / edit / remove custom fields (Text, Hidden, Boolean) | `fields[]` |
| 4.14 | Assign item to folder on create/edit | `folderId` |
| 4.15 | Move item to organization + collection | `bw move` |
| 4.16 | Clone item | `bw create item` |

---

## 5. Folders

| # | Feature | CLI |
|---|---------|-----|
| 5.1 | List folders in sidebar | `bw list folders` |
| 5.2 | Filter vault list by selected folder | `--folderid` |
| 5.3 | Create folder | `bw create folder` |
| 5.4 | Rename folder | `bw edit folder` |
| 5.5 | Delete folder | `bw delete folder` |

---

## 6. Organizations & Collections

| # | Feature | CLI |
|---|---------|-----|
| 6.1 | List organizations in sidebar | `bw list organizations` |
| 6.2 | List collections per organization | `bw list org-collections` |
| 6.3 | Filter items by collection | `--collectionid` |
| 6.4 | Create org-collection | `bw create org-collection` |
| 6.5 | Edit org-collection | `bw edit org-collection` |
| 6.6 | Delete org-collection | `bw delete org-collection` |
| 6.7 | Assign item to collections | `bw edit item-collections` |
| 6.8 | Move personal item to organization | `bw move` |
| 6.9 | List organization members | `bw list org-members` |
| 6.10 | Confirm pending member | `bw confirm org-member` |
| 6.11 | Device approval (list, approve, deny) | `bw device-approval` |

---

## 7. Attachments

| # | Feature | CLI |
|---|---------|-----|
| 7.1 | Show attachments section in item detail | — |
| 7.2 | Download attachment | `bw get attachment` |
| 7.3 | Upload attachment | `bw create attachment` |
| 7.4 | Delete attachment | `bw delete attachment` |

---

## 8. Bitwarden Send

| # | Feature | CLI |
|---|---------|-----|
| 8.1 | List sends | `bw send list` |
| 8.2 | Create text send | `bw send create` |
| 8.3 | Create file send | `bw send create -f` |
| 8.4 | Edit send (name, expiry, access count, password) | `bw send edit` |
| 8.5 | Delete send | `bw send delete` |
| 8.6 | Copy send link to clipboard after creation | — |
| 8.7 | Receive / access a send by URL | `bw send receive` |

---

## 9. Password Generator

| # | Feature | CLI |
|---|---------|-----|
| 9.1 | Generate password (length, uppercase, lowercase, numbers, symbols) | `bw generate` |
| 9.2 | Generate passphrase (words, separator, capitalize, include number) | `bw generate --passphrase` |
| 9.3 | Generator popup accessible from vault list and create/edit forms | — |
| 9.4 | One-key insert into focused field | — |

---

## 10. Import & Export

| # | Feature | CLI |
|---|---------|-----|
| 10.1 | Export vault as CSV | `bw export --format csv` |
| 10.2 | Export vault as JSON | `bw export --format json` |
| 10.3 | Export vault as encrypted JSON | `bw export --format encrypted_json` |
| 10.4 | Export vault as ZIP (includes attachments) | `bw export --format zip` |
| 10.5 | Export organization vault | `--organizationid` |
| 10.6 | Import from supported format | `bw import <format> <path>` |
| 10.7 | Format picker populated from `bw import --formats` | — |

---

## 11. Sync & Vault State

| # | Feature | CLI |
|---|---------|-----|
| 11.1 | ~~Sync vault manually~~ | `bw sync` |
| 11.2 | Show last sync timestamp in status pane | `bw sync --last` |
| 11.3 | Check for CLI updates | `bw update` |

---

## 12. Configuration & Server

| # | Feature | CLI |
|---|---------|-----|
| 12.1 | ~~Custom theme colors via `config.toml`~~ | — |
| 12.2 | ~~Save email across sessions~~ | — |
| 12.3 | Self-hosted server URL configuration | `bw config server <url>` |
| 12.4 | EU cloud preset (`vault.bitwarden.eu`) | `bw config server` |
| 12.5 | Individual service URL overrides (API, identity, icons…) | `bw config server --api` |
| 12.6 | Self-signed certificate support | `NODE_EXTRA_CA_CERTS` |

---

## 13. UI & Shell

| # | Feature | CLI |
|---|---------|-----|
| 13.1 | ~~Splash screen with session check spinner~~ | — |
| 13.2 | ~~Multi-panel vault layout (status, vaults, types, search, list, log)~~ | — |
| 13.3 | ~~Command log with redacted secrets~~ | — |
| 13.4 | ~~Mouse support (click, double-click, scroll)~~ | — |
| 13.5 | ~~Help popup with keybindings~~ | — |
| 13.6 | ~~Item type icons and favorite indicator~~ | — |
| 13.7 | ~~Clipboard support (wl-copy, xclip, pbcopy)~~ | — |
| 13.8 | Local REST API passthrough mode | `bw serve` |
