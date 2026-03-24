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
        Padding, Paragraph, Wrap,
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
pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Login  => draw_login(frame, app),
        Screen::Vault  => draw_vault(frame, app),
        Screen::Detail => draw_detail(frame, app),
        Screen::Search => draw_vault(frame, app), // search is inline in vault now
        Screen::Help   => {
            draw_vault(frame, app);
            draw_help_popup(frame, frame.area());
        }
    }
}

// ── Login screen ───────────────────────────────────────────────────────────

fn draw_login(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = frame.area();

    // Form content: label(1)+input(3)+label(1)+input(3)+checkbox(1) = 9
    // Plus border top+bottom = 11. Error banner adds 2 more.
    let form_height: u16 = 11 + if app.login_error { 2 } else { 0 };

    let show_logo = area.height >= (9 + 1 + 1 + form_height + 2 + 2);

    let chunks = if show_logo {
        Layout::vertical([
            Constraint::Length(2),           // top padding
            Constraint::Length(9),           // logo art
            Constraint::Length(1),           // title line
            Constraint::Length(1),           // gap
            Constraint::Length(form_height), // form
            Constraint::Min(0),              // fill
            Constraint::Length(2),           // status bar
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Min(0),              // top fill
            Constraint::Length(1),           // title line
            Constraint::Length(1),           // gap
            Constraint::Length(form_height), // form
            Constraint::Min(0),              // bottom fill
            Constraint::Length(2),           // status bar
        ])
        .split(area)
    };

    // With logo:    chunks = [pad, art, title, gap, form, fill, status]
    // Without logo: chunks = [fill, title, gap, form, fill, status]
    let (logo_chunk, title_chunk, form_chunk, status_chunk) = if show_logo {
        (Some(chunks[1]), chunks[2], chunks[4], chunks[6])
    } else {
        (None, chunks[1], chunks[3], chunks[5])
    };

    // ── ASCII pixel-art shield logo
    let c  = Style::default().fg(t.accent);
    // Logo dark inner tone — use selected_bg color (dark, tinted) so it
    // always harmonizes with the accent regardless of theme
    let d  = Style::default().fg(t.selected_bg);
    let bg = Style::default().fg(Color::Rgb(0, 25, 25));

    if let Some(logo_area) = logo_chunk {
        let logo_lines: Vec<Line> = vec![
            Line::from(Span::styled("  ▄████████████▄  ", c)).alignment(Alignment::Center),
            Line::from(vec![Span::styled("  █", c), Span::styled("  ╔════════╗  ", d), Span::styled("█  ", c)]).alignment(Alignment::Center),
            Line::from(vec![Span::styled("  █", c), Span::styled("  ║ ", d), Span::styled("┌────┐", c), Span::styled(" ║  ", d), Span::styled("█  ", c)]).alignment(Alignment::Center),
            Line::from(vec![Span::styled("  █", c), Span::styled("  ║ ", d), Span::styled("│", c), Span::styled("░░░░", bg), Span::styled("│", c), Span::styled(" ║  ", d), Span::styled("█  ", c)]).alignment(Alignment::Center),
            Line::from(vec![Span::styled("  █", c), Span::styled("  ║ ", d), Span::styled("██████", c), Span::styled(" ║  ", d), Span::styled("█  ", c)]).alignment(Alignment::Center),
            Line::from(vec![Span::styled("  █", c), Span::styled("  ║ ", d), Span::styled("█", c), Span::styled("░░░░", bg), Span::styled("█", c), Span::styled(" ║  ", d), Span::styled("█  ", c)]).alignment(Alignment::Center),
            Line::from(vec![Span::styled("  █", c), Span::styled("  ╚════════╝  ", d), Span::styled("█  ", c)]).alignment(Alignment::Center),
            Line::from(vec![Span::styled("   ▀", c), Span::styled("████████████", d), Span::styled("▀   ", c)]).alignment(Alignment::Center),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(logo_lines), logo_area);
    }

    // ── Title line — always visible ────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("bytewarden", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" v0.1.0", Style::default().fg(t.dim)),
            Span::styled("  —  ", Style::default().fg(Color::Rgb(40, 44, 70))),
            Span::styled("Bitwarden TUI", Style::default().fg(t.dim)),
        ])).alignment(Alignment::Center),
        title_chunk,
    );

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
        ("☑", t.accent)   // checked → accent (matches theme, not success green)
    } else {
        ("☐", t.inactive) // unchecked → inactive gray
    };
    let checkbox_label_color = if checkbox_focused { t.accent } else { t.inactive };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(checkbox_icon, Style::default().fg(checkbox_color)),
            Span::styled(" Save email", Style::default().fg(checkbox_label_color)),
            if checkbox_focused {
                Span::styled("  (Space to toggle)", Style::default().fg(t.dim))
            } else {
                Span::raw("")
            },
        ])),
        fields[4],
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

fn draw_vault(frame: &mut Frame, app: &App) {
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
        ("^0-5/Tab", "panel"),
        ("//^0", "search"),
        ("j/k", "navigate"),
        ("Enter", "detail"),
        ("u", "copy user"),
        ("c", "copy pass"),
        ("f", "favorite"),
        ("s", "sync"),
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
}


// ── Detail screen ──────────────────────────────────────────────────────────

fn draw_detail(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);

    let item_name = app.selected_item().map(|i| i.name.as_str()).unwrap_or("?");
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" ← ", Style::default().fg(t.dim)),
        Span::styled(item_name, Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(t.dim)),
    );
    frame.render_widget(header, chunks[0]);

    if let Some(item) = app.selected_item() {
        let mut lines: Vec<Line> = vec![
            Line::from(""),
            detail_row("Name", &item.name),
            detail_row("Type", item_type_label(item.item_type)),
        ];

        if let Some(login) = &item.login {
            if let Some(username) = &login.username {
                lines.push(detail_row("Username", username));
            }

            // Password row — toggled by `p`
            let pass_text = if app.show_password {
                login.password.as_deref().unwrap_or("[no password]").to_string()
            } else {
                "●●●●●●●●".to_string()
            };
            lines.push(Line::from(vec![
                Span::styled("  Password   : ", Style::default().fg(t.dim)),
                Span::styled(pass_text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if app.show_password { " (visible)" } else { " (hidden)" },
                    Style::default().fg(t.dim),
                ),
            ]));

            if let Some(uris) = &login.uris {
                for uri_data in uris {
                    if let Some(uri) = &uri_data.uri {
                        lines.push(detail_row("URL", uri));
                    }
                }
            }

            if login.totp.is_some() {
                lines.push(detail_row("TOTP", "available"));
            }
        }

        if let Some(notes) = &item.notes {
            lines.push(Line::from(""));
            lines.push(detail_row("Notes", notes));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  p: show/hide password  |  c: copy password  |  Esc: back",
            Style::default().fg(t.dim),
        )));

        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" Detail "),
                )
                .wrap(Wrap { trim: true }),
            chunks[1],
        );
    }

    draw_status_bar(frame, app, chunks[2]);
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
                Screen::Search => "Search is now inline — press / from vault",
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
