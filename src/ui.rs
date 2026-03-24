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

const COLOR_ACCENT: Color      = Color::Cyan;
const COLOR_ERROR: Color       = Color::Red;
const COLOR_SUCCESS: Color     = Color::Green;
const COLOR_DIM: Color         = Color::DarkGray;
const COLOR_SELECTED_BG: Color = Color::Rgb(30, 60, 80);

// ── Root render function ───────────────────────────────────────────────────

/// Entry point — called from the main loop on every frame.
pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Login  => draw_login(frame, app),
        Screen::Vault  => draw_vault(frame, app),
        Screen::Detail => draw_detail(frame, app),
        Screen::Search => draw_search(frame, app),
        Screen::Help   => {
            draw_vault(frame, app);
            draw_help_popup(frame, frame.area());
        }
    }
}

// ── Login screen ───────────────────────────────────────────────────────────

fn draw_login(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let form_height: u16 = if app.login_error { 12 } else { 10 };

    // Show logo only if terminal is tall enough (logo=11 + gap=1 + form + status=3 + padding=2)
    let show_logo = area.height >= (11 + 1 + form_height + 3 + 2);

    let chunks = if show_logo {
        Layout::vertical([
            Constraint::Length(2),           // top padding
            Constraint::Length(9),           // logo art only (no title line)
            Constraint::Length(1),           // title line
            Constraint::Length(1),           // gap
            Constraint::Length(form_height), // form
            Constraint::Min(0),              // fill
            Constraint::Length(3),           // status bar
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Min(0),              // top fill
            Constraint::Length(1),           // title line (always shown)
            Constraint::Length(1),           // gap
            Constraint::Length(form_height), // form
            Constraint::Min(0),              // bottom fill
            Constraint::Length(3),           // status bar
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

    let form_area = center_rect(60, form_height, form_chunk);

    // ── ASCII pixel-art shield logo (only when tall enough) ────────────────
    let c  = Style::default().fg(COLOR_ACCENT);
    let d  = Style::default().fg(Color::Rgb(0, 90, 90));
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
            Span::styled("bytewarden", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" v0.1.0", Style::default().fg(COLOR_DIM)),
            Span::styled("  —  ", Style::default().fg(Color::Rgb(40, 44, 70))),
            Span::styled("Bitwarden TUI", Style::default().fg(COLOR_DIM)),
        ])).alignment(Alignment::Center),
        title_chunk,
    );

    // Form block — border turns red on invalid credentials
    let form_border_style = if app.login_error {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(COLOR_ACCENT)
    };

    let block = Block::default()
        .title(" Login ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(form_border_style)
        .padding(Padding::horizontal(2));

    let inner = block.inner(form_area);
    frame.render_widget(block, form_area);

    // Error banner inside form (only visible on failed attempt)
    let error_height: u16 = if app.login_error { 2 } else { 0 };

    let fields = Layout::vertical([
        Constraint::Length(1),            // email label
        Constraint::Length(3),            // email input
        Constraint::Length(1),            // password label
        Constraint::Length(3),            // password input
        Constraint::Length(error_height), // error banner (0 when hidden)
    ])
    .split(inner);

    // Email label + input
    frame.render_widget(
        Paragraph::new("Email:").style(Style::default().fg(COLOR_DIM)),
        fields[0],
    );
    let email_border = border_style(app.active_field == LoginField::Email);
    frame.render_widget(
        Paragraph::new(app.email_input.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(email_border),
        ),
        fields[1],
    );

    // Password label + input (masked)
    frame.render_widget(
        Paragraph::new("Master Password:").style(Style::default().fg(COLOR_DIM)),
        fields[2],
    );
    let pass_border = border_style(app.active_field == LoginField::Password);
    let masked: String = "●".repeat(app.password_input.len());
    frame.render_widget(
        Paragraph::new(masked).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(pass_border),
        ),
        fields[3],
    );

    // Error banner at the bottom
    if app.login_error {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ✕ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(
                    "Invalid credentials. Please try again.",
                    Style::default().fg(Color::Red),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::Rgb(60, 10, 10))),
            ),
            fields[4],
        );
    }

    draw_status_bar(frame, app, status_chunk);
}

// ── Vault screen ───────────────────────────────────────────────────────────

fn draw_vault(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" 🔐 bytewarden", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  —  {} items", app.items.len()),
            Style::default().fg(COLOR_DIM),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(COLOR_DIM)),
    );
    frame.render_widget(header, chunks[0]);

    // Item list
    let list_items: Vec<ListItem> = app
        .items
        .iter()
        .map(|item| {
            let type_color = match item.item_type {
                1 => Color::Blue,
                2 => Color::Yellow,
                3 => Color::Magenta,
                _ => COLOR_DIM,
            };
            let mut spans = vec![
                Span::styled(
                    format!("[{}] ", item_type_label(item.item_type)),
                    Style::default().fg(type_color),
                ),
                Span::raw(&item.name),
            ];
            if let Some(login) = &item.login {
                if let Some(user) = &login.username {
                    spans.push(Span::styled(
                        format!("  {user}"),
                        Style::default().fg(COLOR_DIM),
                    ));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Vault "),
        )
        .highlight_style(
            Style::default()
                .bg(COLOR_SELECTED_BG)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    draw_status_bar(frame, app, chunks[2]);
}

// ── Detail screen ──────────────────────────────────────────────────────────

fn draw_detail(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    let item_name = app.selected_item().map(|i| i.name.as_str()).unwrap_or("?");
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" ← ", Style::default().fg(COLOR_DIM)),
        Span::styled(item_name, Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(COLOR_DIM)),
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
                Span::styled("  Password   : ", Style::default().fg(COLOR_DIM)),
                Span::styled(pass_text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if app.show_password { " (visible)" } else { " (hidden)" },
                    Style::default().fg(COLOR_DIM),
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
            Style::default().fg(COLOR_DIM),
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

// ── Search screen ──────────────────────────────────────────────────────────

fn draw_search(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    // Search input — shows a blinking-cursor-style underscore
    frame.render_widget(
        Paragraph::new(format!("/ {}_", app.search_query))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Search ")
                    .border_style(Style::default().fg(COLOR_ACCENT)),
            )
            .style(Style::default().fg(Color::White)),
        chunks[0],
    );

    // Results list
    let results: Vec<ListItem> = app
        .search_results
        .iter()
        .map(|item| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", item_type_label(item.item_type)),
                    Style::default().fg(COLOR_DIM),
                ),
                Span::raw(&item.name),
                // Show username as secondary text
                if let Some(login) = &item.login {
                    if let Some(user) = &login.username {
                        Span::styled(format!("  {user}"), Style::default().fg(COLOR_DIM))
                    } else {
                        Span::raw("")
                    }
                } else {
                    Span::raw("")
                },
            ]))
        })
        .collect();

    let count_title = format!(" {} result(s) ", app.search_results.len());
    let mut list_state = ListState::default();
    let sel = if app.search_results.is_empty() { None } else { Some(app.selected_index) };
    list_state.select(sel);

    frame.render_stateful_widget(
        List::new(results)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(count_title.as_str()),
            )
            .highlight_style(Style::default().bg(COLOR_SELECTED_BG).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        chunks[1],
        &mut list_state,
    );

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
        help_line("c",         "Copy password to clipboard"),
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
    let (text, color) = match &app.status {
        Some(msg) => (
            msg.text.clone(),
            if msg.is_error { COLOR_ERROR } else { COLOR_SUCCESS },
        ),
        None => {
            let hint = match app.screen {
                Screen::Login  => "Tab: switch  |  Enter: login  |  Ctrl+C: quit",
                Screen::Vault  => "j/k: navigate  |  Enter: detail  |  /: search  |  c: copy  |  ?: help",
                Screen::Detail => "p: password  |  c: copy  |  Esc: back",
                Screen::Search => "Type to search  |  Enter: open  |  Esc: back",
                Screen::Help   => "Any key to close",
            };
            (hint.to_string(), COLOR_DIM)
        }
    };

    frame.render_widget(
        Paragraph::new(format!(" {text}"))
            .style(Style::default().fg(color))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(COLOR_DIM)),
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
fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(COLOR_ACCENT)
    } else {
        Style::default()
    }
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