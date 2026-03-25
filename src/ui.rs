/// ui.rs — UI rendering with Ratatui
///
/// React analogy: this file is your component tree.
/// `draw(frame, app)` is the root render() — called on every frame.
/// Ratatui uses immediate-mode rendering: every frame you redraw everything
/// from scratch. Widgets hold no state — all state lives in `App`.
///
/// frame.render_widget(widget, area)  ≈  root.render(<Widget />, domNode)

use crate::app::{App, LoginField, Screen};
use crate::bw::item_type_label;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, BorderType, Clear, List, ListItem, ListState,
        Padding, Paragraph,
    },
};

// ── Color palette ──────────────────────────────────────────────────────────

// ── Color palette (used by draw_help_popup which has no app access) ──────────

const COLOR_ACCENT: Color      = Color::Cyan;
#[allow(dead_code)]
const COLOR_ERROR: Color       = Color::Red;
#[allow(dead_code)]
const COLOR_SUCCESS: Color     = Color::Green;
const COLOR_DIM: Color         = Color::DarkGray;
#[allow(dead_code)]
const COLOR_INACTIVE: Color    = Color::Rgb(140, 140, 160);
#[allow(dead_code)]
const COLOR_SELECTED_BG: Color = Color::Rgb(30, 60, 80);

// ── Root render function ───────────────────────────────────────────────────

/// Entry point — called from the main loop on every frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Login  => draw_login(frame, app),
        Screen::Vault  => draw_vault(frame, app),
        Screen::Detail => draw_detail(frame, app),
        Screen::Help   => {
            draw_vault(frame, app);
            draw_help_popup(frame, frame.area());
        }
    }
}

// ── Login screen ───────────────────────────────────────────────────────────

fn draw_login(frame: &mut Frame, app: &mut App) {
    let t = &app.theme;
    let area = frame.area();

    // Form content: label(1)+input(3)+label(1)+input(3)+checkbox(1) = 9
    // Plus border top+bottom = 11. Error banner adds 2 more.
    let form_height: u16 = 12 + if app.login_error { 2 } else { 0 }; // +1 for auto-lock checkbox

    // Full layout: starfield fills everything except bottom bar
    let logo_art_height: u16 = 18;
    let show_logo = area.height >= (logo_art_height + form_height + 2 + 2);

    let chunks = if show_logo {
        Layout::vertical([
            Constraint::Length(logo_art_height), // figlet text + version
            Constraint::Length(form_height),     // form immediately below
            Constraint::Min(0),                  // fill
            Constraint::Length(2),               // command bar
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(form_height),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area)
    };

    let (logo_chunk, title_chunk, form_chunk, status_chunk) = if show_logo {
        (Some(chunks[0]), chunks[0], chunks[1], chunks[3])
    } else {
        (None, chunks[1], chunks[2], chunks[4])
    };

    // ── Star field + logo ─────────────────────────────────────────────────
    if let Some(logo_area) = logo_chunk {
        let w = logo_area.width as usize;
        let h = logo_area.height as usize;

        let s_dim    = Style::default().fg(Color::Rgb(38, 34, 72));
        let s_mid    = Style::default().fg(Color::Rgb(90, 84, 148));
        let s_bright = Style::default().fg(Color::Rgb(185, 178, 248));
        let acc_dim  = Style::default().fg(t.inactive);

        // Single accent color for the whole wordmark — clean and consistent
        let col_hi  = Style::default().fg(t.accent);
        let col_mid = Style::default().fg(t.accent);
        let col_lo  = Style::default().fg(t.accent);

        // Render via bundled slant font using figlet-rs (pure Rust, no system dep)
        let (fig1_str, fig2_str) = {
            use figlet_rs::FIGfont;
            let font_data = include_str!("assets/slant.flf");
            let font = FIGfont::from_content(font_data)
                .unwrap_or_else(|_| FIGfont::standard().unwrap());
            (
                font.convert("byte").map(|f| f.to_string()).unwrap_or_else(|| "byte".to_string()),
                font.convert("warden").map(|f| f.to_string()).unwrap_or_else(|| "warden".to_string()),
            )
        };

        let trim_lines = |s: &str| -> Vec<String> {
            let lines: Vec<&str> = s.lines().collect();
            let start = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
            let end   = lines.iter().rposition(|l| !l.trim().is_empty()).map(|i| i+1).unwrap_or(lines.len());
            lines[start..end].iter().map(|l| l.to_string()).collect()
        };

        let row1_owned = trim_lines(&fig1_str);
        let row2_owned = trim_lines(&fig2_str);
        let row1: Vec<&str> = row1_owned.iter().map(|s| s.as_str()).collect();
        let row2: Vec<&str> = row2_owned.iter().map(|s| s.as_str()).collect();
        let row1_start = 1usize;
        let row2_start = row1_start + row1.len();
        let text_end   = row2_start + row2.len(); // last row of figlet text
        // Place version halfway between text end and bottom of logo area
        let title_row  = text_end + (h.saturating_sub(text_end + 1)) / 2;
        let fig1_w     = row1.iter().map(|l| l.len()).max().unwrap_or(40);
        let fig2_w     = row2.iter().map(|l| l.len()).max().unwrap_or(60);
        let fig1_col   = if w > fig1_w { (w - fig1_w) / 2 } else { 0 };
        let fig2_col   = if w > fig2_w { (w - fig2_w) / 2 } else { 0 };

        let star_at = |row: usize, col: usize| -> (char, Style) {
            let hash = row.wrapping_mul(17)
                .wrapping_add(col.wrapping_mul(31))
                .wrapping_add(row.wrapping_mul(col)) % 120;
            match hash {
                0     => ('\u{2726}', s_bright),
                1 | 2 => ('\u{00b7}', s_mid),
                3     => ('\u{22c6}', s_dim),
                _     => (' ', s_dim),
            }
        };

        let render_line = |row: usize, fig: &str, fc: usize, fw: usize,
                           ri: usize, rh: usize| -> Line<'static> {
            let tone = match ri {
                0 | 1            => col_hi,
                r if r >= rh - 2 => col_lo,
                _                => col_mid,
            };
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut cs = s_dim;
            let mut ct = String::new();
            for col in 0..w {
                let fi = col.wrapping_sub(fc);
                let (ch, st) = if col >= fc && fi < fw {
                    let c = fig.chars().nth(fi).unwrap_or(' ');
                    if c != ' ' { (c, tone) } else { star_at(row, col) }
                } else { star_at(row, col) };
                if st == cs { ct.push(ch); }
                else {
                    if !ct.is_empty() { spans.push(Span::styled(ct.clone(), cs)); ct.clear(); }
                    cs = st; ct.push(ch);
                }
            }
            if !ct.is_empty() { spans.push(Span::styled(ct, cs)); }
            Line::from(spans)
        };

        let pure_stars = |row: usize| -> Line<'static> {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut cs = s_dim; let mut ct = String::new();
            for col in 0..w {
                let (ch, st) = star_at(row, col);
                if st == cs { ct.push(ch); }
                else {
                    if !ct.is_empty() { spans.push(Span::styled(ct.clone(), cs)); ct.clear(); }
                    cs = st; ct.push(ch);
                }
            }
            if !ct.is_empty() { spans.push(Span::styled(ct, cs)); }
            Line::from(spans)
        };

        let mut lines: Vec<Line> = Vec::with_capacity(h);
        for row in 0..h {
            if row == title_row {
                lines.push(Line::from(Span::styled("v0.1.0", acc_dim)).alignment(Alignment::Center));
                continue;
            }
            let r1 = row.wrapping_sub(row1_start);
            let r2 = row.wrapping_sub(row2_start);
            if row >= row1_start && r1 < row1.len() {
                lines.push(render_line(row, row1[r1], fig1_col, fig1_w, r1, row1.len()));
            } else if row >= row2_start && r2 < row2.len() {
                lines.push(render_line(row, row2[r2], fig2_col, fig2_w, r2, row2.len()));
            } else {
                lines.push(pure_stars(row));
            }
        }
        frame.render_widget(Paragraph::new(lines), logo_area);
    }
    // Fallback title when terminal too small for logo
    if logo_chunk.is_none() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("bytewarden", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
                Span::styled(" v0.1.0", Style::default().fg(t.inactive)),
            ])).alignment(Alignment::Center),
            title_chunk,
        );
    }

    // Form height: +2 for error banner when shown

    let form_area = center_rect(60, form_height, form_chunk);

    let form_border_style = if app.login_error {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(t.accent)
    };

    let block = Block::default()
        .title(" Login ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(form_border_style)
        .padding(Padding::horizontal(2));

    let inner = block.inner(form_area);
    frame.render_widget(block, form_area);

    let error_height: u16 = if app.login_error { 2 } else { 0 };

    let fields = Layout::vertical([
        Constraint::Length(1),            // email label
        Constraint::Length(3),            // email input
        Constraint::Length(1),            // password label
        Constraint::Length(3),            // password input
        Constraint::Length(1),            // save email checkbox
        Constraint::Length(1),            // auto-lock checkbox
        Constraint::Length(error_height), // error banner
    ])
    .split(inner);

    // ── Email field with inline cursor ────────────────────────────────────
    frame.render_widget(
        Paragraph::new("Email:").style(Style::default().fg(t.dim)),
        fields[0],
    );
    let email_focused = app.active_field == LoginField::Email;
    let email_line = input_with_cursor(&app.email_input, app.email_cursor, email_focused, t.accent);
    frame.render_widget(
        Paragraph::new(email_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style_themed(email_focused, t.accent)),
        ),
        fields[1],
    );

    // ── Password field with inline cursor ────────────────────────────────
    frame.render_widget(
        Paragraph::new("Master Password:").style(Style::default().fg(t.dim)),
        fields[2],
    );
    let pass_focused = app.active_field == LoginField::Password;
    // Build masked string with cursor
    let masked_before: String = "●".repeat(app.password_cursor);
    let masked_after: String  = "●".repeat(
        app.password_input.chars().count().saturating_sub(app.password_cursor)
    );
    let pass_line = if pass_focused {
        Line::from(vec![
            Span::raw(masked_before),
            Span::styled("█", Style::default().fg(t.accent)),
            Span::raw(masked_after),
        ])
    } else {
        let all_masked: String = "●".repeat(app.password_input.chars().count());
        Line::from(Span::raw(all_masked))
    };
    frame.render_widget(
        Paragraph::new(pass_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style_themed(pass_focused, t.accent)),
        ),
        fields[3],
    );

    // ── Save email checkbox ───────────────────────────────────────────────
    let checkbox_focused = app.active_field == crate::app::LoginField::SaveEmail;
    let (checkbox_icon, checkbox_color) = if app.save_email {
        ("☑", t.accent) } else { ("☐", t.inactive) };
    let checkbox_label_color = if checkbox_focused { t.accent } else { t.inactive };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(checkbox_icon, Style::default().fg(checkbox_color)),
            Span::styled(" Save email", Style::default().fg(checkbox_label_color)),
        ])),
        fields[4],
    );

    // ── Auto-lock checkbox ────────────────────────────────────────────────
    let al_focused = app.active_field == crate::app::LoginField::AutoLock;
    let (al_icon, al_color) = if app.auto_lock {
        ("☑", t.accent) } else { ("☐", t.inactive) };
    let al_label_color = if al_focused { t.accent } else { t.inactive };
    let lock_mins = app.lock_after_secs / 60;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(al_icon, Style::default().fg(al_color)),
            Span::styled(
                format!(" Auto-lock after {lock_mins} min"),
                Style::default().fg(al_label_color),
            ),
        ])),
        fields[5],
    );

    // ── Error banner ──────────────────────────────────────────────────────
    if app.login_error {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ✕ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("Invalid credentials. Please try again.", Style::default().fg(Color::Red)),
            ]))
            .block(Block::default().borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(60, 10, 10)))),
            fields[5],
        );
    }

    // Record login form area for mouse hit-testing
    app.mouse_areas.login = Some(form_area);

    // ── Status bar with truncation ────────────────────────────────────────
    let hints_full  = "Tab: field  |  Space: toggle save  |  Enter: login  |  ←→: cursor  |  Ctrl+C: quit";
    let hints_short = "Tab:field  Space:save  Enter:login  ←→:cursor  ^C:quit";
    let avail = area.width.saturating_sub(2) as usize;
    let hint = if hints_full.len() <= avail {
        hints_full.to_string()
    } else if hints_short.len() <= avail {
        hints_short.to_string()
    } else {
        format!("{}…", &hints_short[..avail.saturating_sub(1)])
    };
    frame.render_widget(
        Paragraph::new(format!(" {hint}"))
            .style(Style::default().fg(t.dim))
            .block(Block::default().borders(Borders::TOP)
                .border_style(Style::default().fg(t.dim))),
        status_chunk,
    );
}

// ── Vault screen ───────────────────────────────────────────────────────────

fn draw_vault(frame: &mut Frame, app: &mut App) {
    use crate::app::{Focus, ITEM_FILTERS, ItemFilter};
    let t = &app.theme;

    let area = frame.area();

    // Outer: body | keybindings
    let outer = Layout::vertical([
        Constraint::Min(0),      // body
        Constraint::Length(2),   // keybindings bar
    ])
    .split(area);

    // Body: sidebar | main
    let body = Layout::horizontal([
        Constraint::Percentage(26),
        Constraint::Percentage(74),
    ])
    .split(outer[0]);

    // Sidebar: [5]-Status (top, fixed 3 rows) | [1]-Vaults | [2]-Items
    let sidebar = Layout::vertical([
        Constraint::Length(3),   // [5]-Status pane — compact
        Constraint::Percentage(30),
        Constraint::Min(0),
    ])
    .split(body[0]);

    // Command log height: taller so more entries visible
    let cmd_log_height: u16 = if app.cmd_log.is_empty() { 4 } else { 9 };
    // Search bar: always visible (3 rows: title + input + border)
    let search_height: u16 = 3;
    let main = Layout::vertical([
        Constraint::Length(search_height), // [/]-Search bar (always visible)
        Constraint::Min(0),                // vault list
        Constraint::Length(cmd_log_height),// [4]-Command Log
    ])
    .split(body[1]);


    // ── Keybindings bar (bottom, hints only — status is in [5]-Status pane) ──
    let available = area.width.saturating_sub(2) as usize;
    let hints: &[(&str, &str)] = &[
        ("F1-F5", "panel"),
        ("/", "search"),
        ("j/k", "navigate"),
        ("PgUp/Dn", "scroll"),
        ("Enter", "detail"),
        ("u", "user"),
        ("c", "pass"),
        ("f", "fav"),
        ("s", "sync"),
        ("L", "lock"),
        ("?", "help"),
    ];
    let hint_full  = hints.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("  |  ");
    let hint_short = hints.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join("  ");
    let hint_line = if hint_full.len() <= available {
        hint_full
    } else if hint_short.len() <= available {
        hint_short
    } else {
        format!("{}…", &hint_short[..available.saturating_sub(1)])
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(format!(" {hint_line}"), Style::default().fg(t.dim))))
            .block(Block::default().borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(60, 62, 80)))),
        outer[1],
    );

    // ── [5]-Status pane ───────────────────────────────────────────────────
    // Only shown when an action is running or just finished.
    // Idle = empty pane (no text).
    let spinner_frames = ["-", "\\", "|", "/"];
    let spinner_idx = (app.action_tick / 3) as usize % spinner_frames.len();
    let (status_title_style, status_line) = match &app.action_state {
        crate::app::ActionState::Idle => (
            Style::default().fg(t.inactive),
            Line::from(""),  // blank — nothing to show
        ),
        crate::app::ActionState::Running(msg) => (
            Style::default().fg(t.accent),
            Line::from(vec![
                Span::styled(
                    format!(" {} ", spinner_frames[spinner_idx]),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(msg.as_str(), Style::default().fg(t.accent)),
            ]),
        ),
        crate::app::ActionState::Done(msg) => (
            Style::default().fg(t.success),
            Line::from(vec![
                Span::styled(" ✓ ", Style::default().fg(t.success).add_modifier(Modifier::BOLD)),
                Span::styled(msg.as_str(), Style::default().fg(t.success)),
            ]),
        ),
        crate::app::ActionState::Error(msg) => (
            Style::default().fg(t.error),
            Line::from(vec![
                Span::styled(" ✕ ", Style::default().fg(t.error).add_modifier(Modifier::BOLD)),
                Span::styled(msg.as_str(), Style::default().fg(t.error)),
            ]),
        ),
    };
    frame.render_widget(
        Paragraph::new(status_line)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled("─[5]-Status", status_title_style))
                .border_style(if app.focus == Focus::Status { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) })),
        sidebar[0],
    );

    // ── [1] Vaults panel ──────────────────────────────────────────────────
    let vaults_focused = app.focus == Focus::Vaults;
    let vaults_title_style = if vaults_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) };
    let vault_items = vec![
        ListItem::new(Line::from(vec![
            Span::styled("  My Vault", Style::default().fg(Color::White)),
            Span::styled(format!("  {}", app.items.len()), Style::default().fg(t.dim)),
        ])),
    ];
    let mut vault_state = ListState::default();
    vault_state.select(Some(0));
    frame.render_stateful_widget(
        List::new(vault_items)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled("─[1]-Vaults", vaults_title_style))
                .title_bottom(Line::from(Span::styled("─1 of 1─", Style::default().fg(t.dim))).right_aligned())
                .border_style(if vaults_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) }))
            .highlight_style(Style::default().bg(t.selected_bg).fg(Color::White)),
        sidebar[1],
        &mut vault_state,
    );

    // ── [2] Items filter panel ────────────────────────────────────────────
    let items_focused = app.focus == Focus::Items;
    let items_title_style = if items_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) };

    let filter_list_items: Vec<ListItem> = ITEM_FILTERS.iter().map(|f| {
        let count = app.count_for(f);
        let type_color = match f {
            ItemFilter::Login      => t.item_login,
            ItemFilter::Card       => t.item_card,
            ItemFilter::Identity   => t.item_identity,
            ItemFilter::SecureNote => t.item_note,
            ItemFilter::SshKey     => t.item_ssh,
            ItemFilter::Favorites  => t.item_favorite,
            ItemFilter::All        => Color::White,
        };
        let is_active = *f == app.active_filter;
        let label_style = if is_active {
            Style::default().fg(type_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(type_color)
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {}", f.label()), label_style),
            Span::styled(format!("  {count}"), Style::default().fg(t.dim)),
        ]))
    }).collect();

    let mut filter_state = ListState::default();
    filter_state.select(Some(app.filter_selected));
    let items_total = ITEM_FILTERS.len();
    let items_indicator = format!(" {} of {} ", app.filter_selected + 1, items_total);
    frame.render_stateful_widget(
        List::new(filter_list_items)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled("─[2]-Items", items_title_style))
                .title_bottom(Line::from(Span::styled(format!("─{}─", items_indicator.trim()), Style::default().fg(t.dim))).right_aligned())
                .border_style(if items_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) }))
            .highlight_style(Style::default().bg(t.selected_bg).fg(Color::White))
            .highlight_symbol("▶ "),
        sidebar[2],
        &mut filter_state,
    );

    // ── Main item list ────────────────────────────────────────────────────
    let list_focused = app.focus == Focus::List;
    let list_title_style = if list_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) };

    let filtered = app.filtered_items();
    let list_items: Vec<ListItem> = filtered.iter().map(|item| {
        let type_color = match item.item_type {
            1 => Color::Blue,
            2 => Color::Yellow,
            3 => Color::Magenta,
            4 => Color::Yellow,
            5 => Color::Rgb(160, 96, 224),
            _ => t.dim,
        };
        let spans = vec![
            // ★ for favorites, space for others — always same width
            if item.favorite {
                Span::styled("★ ", Style::default().fg(t.item_favorite))
            } else {
                Span::raw("  ")
            },
            Span::styled(format!("[{}]  ", item_type_label(item.item_type)), Style::default().fg(type_color)),
            Span::raw(item.name.as_str()),
            // Username intentionally not shown — still used for fuzzy search
        ];
        ListItem::new(Line::from(spans))
    }).collect();

    let mut list_state = ListState::default();
    let filtered_len = filtered.len();
    list_state.select(if filtered.is_empty() { None } else { Some(app.selected_index) });
    let vault_indicator = if filtered_len > 0 {
        format!(" {} of {} ", app.selected_index + 1, filtered_len)
    } else {
        " 0 of 0 ".to_string()
    };

    // ── [/] Search bar (always visible) ──────────────────────────────────
    let search_focused = app.focus == Focus::Search;
    let search_title_style = if search_focused {
        Style::default().fg(t.accent)
    } else {
        Style::default().fg(t.inactive)
    };
    // Same border style as other focused inputs — no custom dark color
    let search_border_style = border_style_themed(search_focused, t.accent);
    let search_content = if search_focused {
        // Focused: show text + cursor, NO extra slash prefix
        Line::from(vec![
            Span::styled(app.search_query.as_str(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(t.accent)),
        ])
    } else if !app.search_query.is_empty() {
        // Has query but not focused: show text dimly
        Line::from(Span::styled(app.search_query.as_str(), Style::default().fg(t.dim)))
    } else {
        // Empty and unfocused: placeholder
        Line::from(Span::styled(
            "type to filter…",
            Style::default().fg(Color::Rgb(80, 85, 120)),
        ))
    };
    frame.render_widget(
        Paragraph::new(search_content)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled("─[0]-Search", search_title_style))
                .border_style(search_border_style)),
        main[0],
    );

    frame.render_stateful_widget(
        List::new(list_items)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled("─[3]-Vault", list_title_style))
                .title_bottom(Line::from(Span::styled(format!("─{}─", vault_indicator.trim()), Style::default().fg(t.dim))).right_aligned())
                .border_style(if list_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) }))
            .highlight_style(Style::default().bg(t.selected_bg).fg(Color::White).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        main[1],
        &mut list_state,
    );

    // ── [4] Command Log ───────────────────────────────────────────────────
    let log_focused = app.focus == crate::app::Focus::CmdLog;
    let log_title_style = if log_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) };

    let all_lines: Vec<Line> = if app.cmd_log.is_empty() {
        vec![Line::from(Span::styled("  no commands yet", Style::default().fg(t.dim)))]
    } else {
        app.cmd_log.iter().flat_map(|e| {
            let result_color = if e.ok { t.success } else { t.error };
            let icon = if e.ok { "✓" } else { "✕" };
            vec![
                Line::from(Span::styled(format!("  $ {}", e.cmd), Style::default().fg(t.dim))),
                Line::from(Span::styled(format!("  {icon} {}", e.detail), Style::default().fg(result_color))),
            ]
        }).collect()
    };

    let visible = (cmd_log_height.saturating_sub(2)) as usize;
    let total = all_lines.len();
    let end = total.saturating_sub(app.cmd_log_scroll);
    let start = end.saturating_sub(visible);
    let visible_lines: Vec<Line> = all_lines[start..end].to_vec();

    let scroll_indicator = if app.cmd_log_scroll > 0 {
        "─[4]-Command Log  ↑".to_string()
    } else {
        "─[4]-Command Log".to_string()
    };

    frame.render_widget(
        Paragraph::new(visible_lines)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(scroll_indicator, log_title_style))
                .border_style(if log_focused { Style::default().fg(t.accent) } else { Style::default().fg(t.inactive) })),
        main[2],
    );
    // ── Record mouse hit areas (safe — draw_vault takes &mut App) ────────
    app.mouse_areas.status  = Some(sidebar[0]);
    app.mouse_areas.vaults  = Some(sidebar[1]);
    app.mouse_areas.items   = Some(sidebar[2]);
    app.mouse_areas.search  = Some(main[0]);
    app.mouse_areas.list    = Some(main[1]);
    app.mouse_areas.cmdlog  = Some(main[2]);
}

// ── Detail screen ──────────────────────────────────────────────────────────

/// A field to display in the detail view.
struct DetailField {
    label:  String,
    value:  String,
    hidden: bool, // if true, show ●●● unless revealed
}

fn build_detail_fields(item: &crate::bw::Item, show: bool, reveal_idx: usize) -> Vec<DetailField> {
    let mut fields = vec![
        DetailField { label: "Name".into(), value: item.name.clone(), hidden: false },
        DetailField { label: "Type".into(), value: item_type_label(item.item_type).to_string(), hidden: false },
    ];

    // ── Login ─────────────────────────────────────────────────────────────
    if let Some(login) = &item.login {
        if let Some(u) = &login.username {
            fields.push(DetailField { label: "Username".into(), value: u.clone(), hidden: false });
        }
        let pass = login.password.as_deref().unwrap_or("").to_string();
        let reveal = show && reveal_idx == fields.len();
        fields.push(DetailField {
            label: "Password".into(),
            value: if reveal || pass.is_empty() { pass.clone() } else { "●".repeat(pass.chars().count().max(8)) },
            hidden: !reveal && !pass.is_empty(),
        });
        if let Some(uris) = &login.uris {
            for uri_data in uris {
                if let Some(uri) = &uri_data.uri {
                    fields.push(DetailField { label: "URL".into(), value: uri.clone(), hidden: false });
                }
            }
        }
        if let Some(totp) = &login.totp {
            let reveal = show && reveal_idx == fields.len();
            fields.push(DetailField {
                label: "TOTP".into(),
                value: if reveal { totp.clone() } else { "●●●●●●".to_string() },
                hidden: !reveal,
            });
        }
    }

    // ── Card ──────────────────────────────────────────────────────────────
    if let Some(card) = &item.card {
        // Helper fn (not closure) avoids double-borrow of fields
        fn card_push(fields: &mut Vec<DetailField>, label: &str, val: &Option<String>, is_hidden: bool, show: bool, reveal_idx: usize) {
            if let Some(v) = val {
                if !v.is_empty() {
                    let reveal = show && reveal_idx == fields.len();
                    let hidden = is_hidden && !reveal;
                    fields.push(DetailField {
                        label: label.to_string(),
                        value: if hidden { "●".repeat(v.chars().count().max(4)) } else { v.clone() },
                        hidden,
                    });
                }
            }
        }
        card_push(&mut fields, "Cardholder", &card.cardholder_name, false, show, reveal_idx);
        card_push(&mut fields, "Brand",      &card.brand,           false, show, reveal_idx);
        card_push(&mut fields, "Number",     &card.number,          true,  show, reveal_idx);
        if card.exp_month.is_some() || card.exp_year.is_some() {
            let exp = format!("{}/{}", card.exp_month.as_deref().unwrap_or("?"), card.exp_year.as_deref().unwrap_or("?"));
            fields.push(DetailField { label: "Expiry".into(), value: exp, hidden: false });
        }
        card_push(&mut fields, "CVV", &card.code, true, show, reveal_idx);
    }

    // ── Identity ──────────────────────────────────────────────────────────
    if let Some(id) = &item.identity {
        fn id_push(fields: &mut Vec<DetailField>, label: &str, val: &Option<String>, is_hidden: bool, show: bool, reveal_idx: usize) {
            if let Some(v) = val {
                if !v.is_empty() {
                    let reveal = show && reveal_idx == fields.len();
                    let hidden = is_hidden && !reveal;
                    fields.push(DetailField {
                        label: label.to_string(),
                        value: if hidden { "●".repeat(v.chars().count().max(4)) } else { v.clone() },
                        hidden,
                    });
                }
            }
        }
        let full_name = crate::app::build_full_name(id.title.as_deref(), id.first_name.as_deref(), id.middle_name.as_deref(), id.last_name.as_deref());
        if !full_name.is_empty() {
            fields.push(DetailField { label: "Full Name".into(), value: full_name, hidden: false });
        }
        let hidden_fields = ["SSN", "Passport", "License"];
        for (lbl, val) in crate::app::identity_fields(id) {
            id_push(&mut fields, lbl, val, hidden_fields.contains(&lbl), show, reveal_idx);
        }
    }

    // ── Custom fields (field_type: 0=text, 1=hidden, 2=boolean) ──────────
    for field in &item.fields {
        let name  = field.name.as_deref().unwrap_or("Field").to_string();
        let value = field.value.as_deref().unwrap_or("").to_string();
        let is_hidden = field.field_type == 1;
        let reveal = show && reveal_idx == fields.len();
        let hidden = is_hidden && !reveal;
        fields.push(DetailField {
            label: name,
            value: if hidden { "●".repeat(value.chars().count().max(4)) } else { value },
            hidden,
        });
    }

    // ── Notes ─────────────────────────────────────────────────────────────
    if let Some(notes) = &item.notes {
        if !notes.is_empty() {
            fields.push(DetailField { label: "Notes".into(), value: notes.clone(), hidden: false });
        }
    }

    fields
}

fn draw_detail(frame: &mut Frame, app: &mut App) {
    let t = &app.theme;
    let area = frame.area();

    let Some(item) = app.selected_item() else { return; };

    // Layout: header | fields area | command bar
    let chunks = Layout::vertical([
        Constraint::Length(2),  // header: ← Item name
        Constraint::Min(0),     // fields
        Constraint::Length(2),  // command bar
    ])
    .split(area);

    // ── Header with action feedback right-aligned ─────────────────────────
    let spinner_frames = ["-", "\\", "|", "/"];
    let spinner_idx = (app.action_tick / 3) as usize % spinner_frames.len();
    let (action_text, action_style) = match &app.action_state {
        crate::app::ActionState::Idle => (String::new(), Style::default()),
        crate::app::ActionState::Running(msg) => (
            format!("{} {}", spinner_frames[spinner_idx], msg),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        crate::app::ActionState::Done(msg) => (
            format!("✓ {}", msg),
            Style::default().fg(t.success).add_modifier(Modifier::BOLD),
        ),
        crate::app::ActionState::Error(msg) => (
            format!("✕ {}", msg),
            Style::default().fg(t.error).add_modifier(Modifier::BOLD),
        ),
    };

    // Right-pad action text to fill remaining width
    let left_len = 4 + item.name.len() + 4 + item_type_label(item.item_type).len(); // " ← name  [Type]"
    let right_len = action_text.len();
    let pad = (area.width as usize).saturating_sub(left_len + right_len + 2);
    let padded_action = format!("{:>width$}", action_text, width = right_len + pad);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ← ", Style::default().fg(t.dim)),
            Span::styled(item.name.as_str(), Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("  [{}]", item_type_label(item.item_type)),
                Style::default().fg(t.inactive),
            ),
            Span::styled(padded_action, action_style),
        ]))
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(t.inactive))),
        chunks[0],
    );

    // ── Fields ────────────────────────────────────────────────────────────
    let fields = build_detail_fields(item, app.show_password, app.detail_field);
    let field_count = fields.len();
    let selected = app.detail_field.min(field_count.saturating_sub(1));

    // Each field = label row (1) + input box (3) = 4 rows
    let field_height: u16 = 4;
    let constraints: Vec<Constraint> = (0..field_count)
        .map(|_| Constraint::Length(field_height))
        .collect();

    let field_areas = Layout::vertical(constraints).split(chunks[1]);

    // Record the fields area for mouse hit-testing
    app.mouse_areas.detail = Some(chunks[1]);

    for (i, field) in fields.iter().enumerate() {
        if i >= field_areas.len() { break; }
        let fa = field_areas[i];
        // Split: label (1 row) | box (3 rows)
        let fc = Layout::vertical([Constraint::Length(1), Constraint::Length(3)]).split(fa);

        let is_selected = i == selected;
        let border_style = if is_selected {
            Style::default().fg(t.accent)
        } else {
            Style::default().fg(t.inactive)
        };
        let label_style = if is_selected {
            Style::default().fg(t.accent)
        } else {
            Style::default().fg(t.inactive)
        };

        // Label
        let label_suffix = if field.hidden && is_selected {
            "  (p: reveal)"
        } else if !field.hidden && is_selected && i == app.detail_field {
            ""
        } else {
            ""
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" {}", field.label), label_style),
                Span::styled(label_suffix, Style::default().fg(t.dim)),
            ])),
            fc[0],
        );

        // Value box
        let value_line = if is_selected {
            Line::from(vec![
                Span::styled(field.value.as_str(), Style::default().fg(Color::White)),
            ])
        } else {
            Line::from(Span::styled(field.value.as_str(), Style::default().fg(t.inactive)))
        };
        frame.render_widget(
            Paragraph::new(value_line)
                .block(Block::default().borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style)),
            fc[1],
        );
    }

    // ── Command bar ───────────────────────────────────────────────────────
    let hints_full  = "j/k: field  |  p: show/hide  |  c: copy field  |  Esc: back";
    let hints_short = "j/k:field  p:reveal  c:copy  Esc:back";
    let avail = area.width.saturating_sub(2) as usize;
    let hint = if hints_full.len() <= avail { hints_full }
               else if hints_short.len() <= avail { hints_short }
               else { &hints_short[..avail.saturating_sub(1)] };
    frame.render_widget(
        Paragraph::new(format!(" {hint}"))
            .style(Style::default().fg(t.dim))
            .block(Block::default().borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(60, 62, 80)))),
        chunks[2],
    );
}
// ── Help popup ─────────────────────────────────────────────────────────────

fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup = center_rect(60, 22, area);
    frame.render_widget(Clear, popup);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Vault", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("j / ↓",     "Move down"),
        help_line("k / ↑",     "Move up"),
        help_line("Enter / l", "Open detail"),
        help_line("/",         "Search vault"),
        help_line("u",         "Copy username to clipboard"),
        help_line("c",         "Copy password to clipboard"),
        help_line("f",         "Toggle favorite ★"),
        help_line("s",         "Sync vault with server"),
        help_line("s",         "Sync vault with server"),
        help_line("q",         "Lock and go to login"),
        Line::from(""),
        Line::from(Span::styled("  Detail", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("p",         "Show / hide password"),
        help_line("c",         "Copy password to clipboard"),
        help_line("Esc / h",   "Back to vault"),
        Line::from(""),
        Line::from(Span::styled("  Global", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("Ctrl+C",    "Quit"),
        help_line("?",         "This help screen"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to close",
            Style::default().fg(COLOR_DIM),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" Help — Keyboard shortcuts ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(COLOR_ACCENT)),
        ),
        popup,
    );
}

// ── Status bar ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let (text, color) = match &app.status {
        Some(msg) => (
            msg.text.clone(),
            if msg.is_error { t.error } else { t.success },
        ),
        None => {
            let hint = match app.screen {
                Screen::Login  => "Tab: switch  |  Enter: login  |  Ctrl+C: quit",
                Screen::Vault  => "Tab: switch panel  |  j/k: navigate  |  Enter: detail  |  u: copy user  |  c: copy pass  |  ?: help",
                Screen::Detail => "p: password  |  c: copy  |  Esc: back",
                Screen::Help   => "Any key to close",
            };
            (hint.to_string(), t.dim)
        }
    };

    frame.render_widget(
        Paragraph::new(format!(" {text}"))
            .style(Style::default().fg(color))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(t.dim)),
            ),
        area,
    );
}

// ── Layout helpers ─────────────────────────────────────────────────────────

/// Centers a rect of `width_pct`% wide and `height` tall inside `area`.
/// CSS equivalent: position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%)
fn center_rect(width_pct: u16, height: u16, area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - width_pct) / 2),
        Constraint::Percentage(width_pct),
        Constraint::Percentage((100 - width_pct) / 2),
    ])
    .split(vert[1])[1]
}

/// Returns the border style for an input field — highlighted when focused.
#[allow(dead_code)]
fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(COLOR_ACCENT)
    } else {
        Style::default()
    }
}

/// Returns the border style using a themed accent color.
fn border_style_themed(focused: bool, accent: Color) -> Style {
    if focused { Style::default().fg(accent) } else { Style::default() }
}

/// A label:value row for the detail screen.
#[allow(dead_code)]
fn detail_row<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {label:<12}: "), Style::default().fg(COLOR_DIM)),
        Span::styled(value, Style::default().fg(Color::White)),
    ])
}

/// A key + description row for the help popup.
fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{key:<14}"), Style::default().fg(COLOR_ACCENT)),
        Span::styled(desc, Style::default().fg(Color::White)),
    ])
}

/// Renders a text input with a block cursor at `cursor_pos`.
/// When focused, splits the string at the cursor and inserts a █ character.
/// When unfocused, renders the string as-is.
fn input_with_cursor<'a>(text: &'a str, cursor_pos: usize, focused: bool, accent: Color) -> Line<'a> {
    if !focused {
        return Line::from(Span::raw(text));
    }
    let chars: Vec<char> = text.chars().collect();
    let before: String = chars[..cursor_pos].iter().collect();
    let after: String  = chars[cursor_pos..].iter().collect();
    Line::from(vec![
        Span::raw(before),
        Span::styled("█", Style::default().fg(accent)),
        Span::styled(after, Style::default().fg(Color::White)),
    ])
}
