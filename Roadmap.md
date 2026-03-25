# Bytewarden — Roadmap

Missing functionality from the Bitwarden CLI, ordered by priority.  
Each section maps directly to `bw` commands that exist but are not yet wrapped.

---

## P0 — Core vault CRUD

The most critical gap. Users can browse and copy but cannot create, edit, or delete anything.

### Item create
- New screen / modal for each item type: Login, Secure Note, Card, Identity, SSH Key
- Maps to: `bw get template <type> | jq ... | bw encode | bw create item`
- Needs inline password generator (see P2)

### Item edit
- Only `favorite` toggle is currently implemented via `bw edit`
- All other fields (name, username, password, URLs, notes, custom fields) are read-only
- Maps to: `bw get item <id> | jq ... | bw encode | bw edit item <id>`

### Item delete
- Send to trash: `bw delete item <id>`
- Permanent delete: `bw delete item <id> --permanent`
- Restore from trash: `bw restore item <id>`
- Needs confirmation prompt before permanent delete

### Two-step login during authentication
- `bw login` is called with `--raw` only — `--method` and `--code` are never passed
- Users with 2FA (Authenticator, Email, YubiKey) hit an interactive prompt
  bytewarden cannot handle, causing login to hang or fail silently
- Fix: add a 2FA code field to the login screen, shown only when needed
- Supported methods: Authenticator (`0`), Email (`1`), YubiKey (`3`)

### Master password re-prompt
- `reprompt` field exists on CLI item JSON but is not deserialized in the `Item` struct
- Items marked with re-prompt should require password confirmation before
  revealing or copying any sensitive field
- Fix: add `reprompt: u8` to `Item`, check it before `copy_selected_field` and reveal

---

## P1 — Folder support

`folder_id` is already deserialized in `Item` but folders are never fetched or shown.

- Fetch folders on login: `bw list folders --session <key>`
- Add `Folder` struct: `{ id, name }`
- Show folders in the `[1]-Vaults` panel (currently hardcoded to "My Vault")
- Filter vault list by selected folder
- Folder CRUD:
  - Create: `bw get template folder | jq '.name="..."' | bw encode | bw create folder`
  - Rename: `bw get folder <id> | jq '.name="..."' | bw encode | bw edit folder <id>`
  - Delete: `bw delete folder <id>`

---

## P2 — Password & passphrase generator

`generate_password()` already exists in `bw.rs` with `#[allow(dead_code)]` — never called from UI.

- Wire it to a generator popup accessible from vault list and create/edit screens
- Expose all CLI options not yet surfaced:
  - `--uppercase` `-u`, `--lowercase` `-l`, `--number` `-n`, `--special` `-s`
  - `--length <n>` (min 5)
  - `--passphrase` mode with `--words <n>`, `--separator <char>`,
    `--capitalize`, `--includeNumber`
- Copy generated value to clipboard with one key
- Maps to: `bw generate [options]`

---

## P3 — Attachments

Completely invisible — `Item` struct has no `attachments` field.

- Add `attachments: Vec<Attachment>` to `Item` struct: `{ id, fileName, size, url }`
- Show attachments section in detail screen
- Download attachment: `bw get attachment <filename> --itemid <id> --output <path>`
- Upload attachment: `bw create attachment --file <path> --itemid <id>`
- Delete attachment: `bw delete attachment <id> --itemid <id>`

---

## P4 — Bitwarden Send

Entirely absent — no data model, no screen, no `bw send` calls.

- Add `Send` struct: `{ id, name, type, text, file, key, deletionDate, maxAccessCount, ... }`
- New Send screen / panel in vault layout
- Full subcommand coverage:
  - List: `bw send list`
  - Create text: `bw send template send.text | jq ... | bw encode | bw send create`
  - Create file: `bw send template send.file | jq ... | bw encode | bw send create`
  - Edit: `bw send get <id> | jq ... | bw encode | bw send edit`
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
  - `bw move <itemid> <organizationid> <encodedCollectionIds>` — move item to org
  - `bw list org-members --organizationid <id>`
  - `bw confirm org-member <id> --organizationid <id>`

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
- Show format picker (use `bw import --formats` to populate)
- Org import: add `--organizationid <id>`

---

## P7 — `bw status` on startup

`bw status` is never called. The app has no knowledge of server URL, last sync
time, or whether a session is already active before the user attempts login.

- Call `bw status` at startup before showing login screen
- Parse response: `{ serverUrl, lastSync, userEmail, userId, status }`
- If `status == "unlocked"` → skip login, go straight to vault with existing session
- If `status == "locked"` → pre-fill email, go to unlock (password only)
- If `status == "unauthenticated"` → full login flow
- Show `serverUrl` and `lastSync` in the `[5]-Status` pane

---

## P8 — `UriData` match field

`UriData` only deserializes `uri`. The `match` field is silently dropped.

- Add `match_type: Option<u8>` to `UriData`
- Show match type label in detail view next to each URL:
  - `0` Domain, `1` Host, `2` Starts With, `3` Exact, `4` Regex, `5` Never
- Make it editable when creating/editing login items

---

## P9 — API key login (`bw login --apikey`)

Only email + password login is implemented.

- Add API key login option to login screen (toggle or separate field set)
- Prompt for `client_id` and `client_secret`
- Support env var passthrough: `BW_CLIENTID` / `BW_CLIENTSECRET`
- Useful for: accounts using FIDO2 or Duo 2FA (not supported interactively),
  automated/scripted usage
- After `--apikey` login, must still call `bw unlock` with master password to
  decrypt vault — handle this two-step flow in the UI

---

## P10 — Server configuration (`bw config server`)

No UI to switch servers. Users must run `bw config server <url>` manually
before launching bytewarden.

- Add `server_url` key to `config.toml`
- On startup, if `server_url` is set, call `bw config server <url>` before login
- Presets to support: US cloud (default), EU cloud (`https://vault.bitwarden.eu`),
  custom self-hosted URL
- Show active server URL in `[5]-Status` pane (sourced from `bw status`)

---

## Summary

| Priority | Feature | Key `bw` commands |
|----------|---------|-------------------|
| P0 | Item create / edit / delete | `bw create item`, `bw edit item`, `bw delete item`, `bw restore item` |
| P0 | Two-step login | `bw login --method <n> --code <code>` |
| P0 | Master password re-prompt | `reprompt` field on item |
| P1 | Folder support | `bw list folders`, `bw create folder`, `bw edit folder`, `bw delete folder` |
| P2 | Password / passphrase generator | `bw generate` |
| P3 | Attachments | `bw get attachment`, `bw create attachment`, `bw delete attachment` |
| P4 | Bitwarden Send | `bw send create/get/edit/list/delete/receive` |
| P5 | Organizations & collections | `bw list organizations`, `bw list collections`, `bw move` |
| P6 | Import & Export | `bw import`, `bw export` |
| P7 | Status on startup | `bw status` |
| P8 | URI match type | `UriData.match` field |
| P9 | API key login | `bw login --apikey` |
| P10 | Server configuration | `bw config server` |
