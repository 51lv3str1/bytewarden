# Bytewarden — Roadmap

Missing functionality from the Bitwarden CLI, ordered by priority.  
Each section maps directly to `bw` commands that exist but are not yet wrapped.

~~Strikethrough~~ = fully implemented.

---

## ~~P0 — Core vault CRUD~~ ✅

### ~~Item create~~
- ~~New screen for each item type: Login, Secure Note, Card, Identity~~
- ~~Type selector → field form → `bw create item`~~
- ~~Tab wraps through fields, F2 reveals hidden fields~~

### ~~Item edit~~
- ~~All fields editable inline in detail screen~~
- ~~`e` to enter edit mode, `Enter` to save, `Esc` to cancel~~
- ~~Maps to: `bw get item <id>` → patch JSON → `bw edit item <id>`~~

### ~~Item delete~~
- ~~`D` key from vault list or detail screen opens confirmation popup~~
- ~~`Enter` = trash (`bw delete item`), `D` = permanent (`bw delete item --permanent`)~~

### Two-step login during authentication _(remaining)_
- `bw login --raw` does not pass `--method` or `--code`
- Users with 2FA (Authenticator, Email, YubiKey) hit an interactive prompt bytewarden cannot handle
- Fix: add a 2FA code field to the login screen, shown only when `bw login` returns an auth challenge
- Supported methods: Authenticator (`0`), Email (`1`), YubiKey (`3`)

### Master password re-prompt _(remaining)_
- `reprompt` field exists on CLI item JSON but is not deserialized in `Item`
- Items with `reprompt: 1` should require password confirmation before revealing or copying sensitive fields
- Fix: add `reprompt: u8` to `Item`, check before `copy_selected_field` and `edit_toggle_reveal`

---

## P1 — Folder support

`folder_id` is already deserialized in `Item` but folders are never fetched or shown.

- Fetch folders on login: `bw list folders --session <key>`
- Add `Folder` struct: `{ id, name }`
- Show folders in the `[1]-Vaults` panel (currently hardcoded to "My Vault")
- Filter vault list by selected folder
- Folder CRUD:
  - Create: `bw get template folder | bw encode | bw create folder`
  - Rename: `bw get folder <id> | mutate | bw encode | bw edit folder <id>`
  - Delete: `bw delete folder <id>`

---

## P2 — Password & passphrase generator

`bw generate` is available in the CLI but has no UI surface.

- Generator popup accessible from vault list (`g`) and create/edit screens
- Options to expose:
  - `--uppercase` `-u`, `--lowercase` `-l`, `--number` `-n`, `--special` `-s`
  - `--length <n>` (min 5)
  - `--passphrase` mode: `--words <n>`, `--separator <char>`, `--capitalize`, `--includeNumber`
- One key to copy generated value to clipboard and optionally fill the focused field

---

## P3 — Attachments

`Item` struct has no `attachments` field — completely invisible.

- Add `attachments: Vec<Attachment>` to `Item`: `{ id, fileName, size, url }`
- Show attachments section in detail screen
- Download: `bw get attachment <filename> --itemid <id> --output <path>`
- Upload: `bw create attachment --file <path> --itemid <id>`
- Delete: `bw delete attachment <id> --itemid <id>`

---

## P4 — Bitwarden Send

Entirely absent — no data model, no screen, no `bw send` calls.

- Add `Send` struct: `{ id, name, type, text, file, key, deletionDate, maxAccessCount, … }`
- New Send screen / panel in vault layout
- Full subcommand coverage:
  - List: `bw send list`
  - Create text: `bw send template send.text | bw encode | bw send create`
  - Create file: `bw send template send.file | bw encode | bw send create`
  - Edit: `bw send get <id> | mutate | bw encode | bw send edit`
  - Delete: `bw send delete <id>`
  - Receive: `bw send receive [--password <pw>] <url>`
  - Copy Send link to clipboard after creation

---

## P5 — Organizations & collections

No org or collection data is fetched or shown.

- Add `Organization` struct: `{ id, name }`
- Add `Collection` struct: `{ id, name, organizationId }`
- Add `collectionIds: Vec<String>` and `organizationId: Option<String>` to `Item`
- Show org vaults in `[1]-Vaults` panel alongside personal vault
- Filter by collection in `[2]-Items` panel
- Commands to wrap:
  - `bw list organizations`
  - `bw list collections` / `bw list org-collections --organizationid <id>`
  - `bw move <itemid> <organizationid> <encodedCollectionIds>`

---

## P6 — Import & Export

Neither direction is implemented.

### Export
- `bw export --format csv --output <path>`
- `bw export --format json --output <path>`
- `bw export --format encrypted_json --password <pw> --output <path>`
- `bw export --format zip --output <path>` (includes attachments)
- Org export: add `--organizationid <id>`

### Import
- `bw import <format> <path>`
- Show format picker populated from `bw import --formats`
- Org import: add `--organizationid <id>`

---

## P7 — `bw status` on startup _(partially done)_

- ~~Call `bw status` at startup before showing login screen~~
- ~~Parse response: `{ serverUrl, lastSync, userEmail, userId, status }`~~
- ~~`unlocked` → skip login, load vault from `BW_SESSION`~~
- ~~`locked` → pre-fill email, prompt for master password only~~
- ~~`unauthenticated` → full login flow~~
- Show `serverUrl` and `lastSync` in the `[5]-Status` pane ← **remaining**

---

## P8 — `UriData` match field

`UriData` only deserializes `uri`. The `match` field is silently dropped.

- Add `match_type: Option<u8>` to `UriData`
- Show match type label next to each URL in detail view:
  `0` Domain · `1` Host · `2` Starts With · `3` Exact · `4` Regex · `5` Never
- Make match type editable in create/edit login forms

---

## P9 — API key login (`bw login --apikey`)

Only email + master password login is implemented.

- Add API key login option to the login screen (toggle or separate field set)
- Prompt for `client_id` and `client_secret`
- Support env var passthrough: `BW_CLIENTID` / `BW_CLIENTSECRET`
- After `--apikey` login, must still call `bw unlock` with master password to decrypt vault — handle this two-step flow in the UI
- Useful for accounts using FIDO2 or Duo 2FA (not supported interactively)

---

## P10 — Server configuration (`bw config server`)

No UI to switch servers — users must run `bw config server <url>` manually before launching.

- Add `server_url` key to `config.toml`
- On startup, if `server_url` is set, call `bw config server <url>` before login
- Presets: US cloud (default), EU cloud (`https://vault.bitwarden.eu`), custom self-hosted URL
- Show active server URL in `[5]-Status` pane (sourced from `bw status`)

---

## Summary

| Priority | Feature | Status |
|----------|---------|--------|
| P0 | ~~Item create~~ | ✅ done |
| P0 | ~~Item edit~~ | ✅ done |
| P0 | ~~Item delete (trash + permanent)~~ | ✅ done |
| P0 | Two-step login (2FA) | pending |
| P0 | Master password re-prompt | pending |
| P1 | Folder support | pending |
| P2 | Password / passphrase generator | pending |
| P3 | Attachments | pending |
| P4 | Bitwarden Send | pending |
| P5 | Organizations & collections | pending |
| P6 | Import & Export | pending |
| P7 | ~~`bw status` on startup~~ / show in Status pane | ✅ / pending |
| P8 | URI match type | pending |
| P9 | API key login | pending |
| P10 | Server configuration | pending |
