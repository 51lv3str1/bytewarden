/// ui.rs — UI rendering with Ratatui

use crate::app::{App, LoginField, Screen, ActionState, Focus, ITEM_FILTERS, ItemFilter,
                  CREATE_ITEM_TYPES, build_full_name, identity_fields};
use figlet_rs::FIGfont;
use crate::bw::{item_type_label, Item};
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

// Only constants actually used by draw_help_popup (no app access there)
const COLOR_ACCENT: Color = Color::Cyan;
const COLOR_DIM:    Color = Color::DarkGray;

// ── Root ──────────────────────────────────────────────────────────────────

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Splash => draw_splash(frame, app),
        Screen::Login  => draw_login(frame, app),
        Screen::Vault  => draw_vault(frame, app),
        Screen::Detail => draw_detail(frame, app),
        Screen::Help   => { draw_vault(frame, app); draw_help_popup(frame, frame.area()); }
        Screen::Create => draw_create(frame, app),
        Screen::ConfirmDelete => {
            draw_vault(frame, app);
            draw_confirm_delete_popup(frame, frame.area(), app);
        }
    }
}

// ── Login ─────────────────────────────────────────────────────────────────

// ── Splash screen (shown during bw status check) ──────────────────────────

fn draw_splash(frame: &mut Frame, app: &mut App) {
    let t    = &app.theme;
    let area = frame.area();

    // Logo content is ~18 rows tall. Center it vertically (leave room for spinner below).
    let logo_h: u16 = 18;
    let top  = area.height.saturating_sub(logo_h + 3) / 2;
    let logo_area = ratatui::layout::Rect {
        x: 0, y: top,
        width:  area.width,
        height: logo_h.min(area.height.saturating_sub(top)),
    };

    // Fill rows above and below logo with pure starfield
    fill_stars(frame, ratatui::layout::Rect { x: 0, y: 0, width: area.width, height: top });
    render_logo(frame, app, logo_area);
    let below_y = top + logo_h;
    if below_y < area.height {
        fill_stars(frame, ratatui::layout::Rect { x: 0, y: below_y, width: area.width, height: area.height - below_y });
    }

    // Spinner centered just below logo
    let sp  = spinner_frame(app.action_tick);
    let msg = match &app.action_state {
        ActionState::Running(m) => format!(" {sp}  {m}"),
        ActionState::Done(m)    => format!(" ✓  {m}"),
        ActionState::Error(m)   => format!(" ✕  {m}"),
        ActionState::Idle       => String::new(),
    };
    let col = match &app.action_state {
        ActionState::Running(_) => t.accent,
        ActionState::Done(_)    => t.success,
        ActionState::Error(_)   => t.error,
        ActionState::Idle       => t.dim,
    };
    if !msg.is_empty() {
        let y = (top + logo_h + 1).min(area.height.saturating_sub(1));
        let w = msg.len() as u16;
        let x = area.width.saturating_sub(w) / 2;
        frame.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(col).add_modifier(Modifier::BOLD))),
            ratatui::layout::Rect { x, y, width: w, height: 1 },
        );
    }
}

// ── Login screen ──────────────────────────────────────────────────────────

fn draw_login(frame: &mut Frame, app: &mut App) {
    let t = &app.theme;
    let area = frame.area();

    // form rows: pad(1)+email-lbl(1)+email-in(3)+pass-lbl(1)+pass-in(3)
    //            +save(1)+autolock(1)+strip(2)+border(2) = 15
    let form_height: u16 = 15;
    let logo_h: u16 = 18;
    let show_logo = area.height >= logo_h + form_height + 4;

    let (logo_chunk, title_chunk, form_chunk, bar_chunk) = if show_logo {
        let c = Layout::vertical([
            Constraint::Length(logo_h),
            Constraint::Length(form_height),
            Constraint::Min(0),
            Constraint::Length(2),
        ]).split(area);
        (Some(c[0]), c[0], c[1], c[3])
    } else {
        let c = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(form_height),
            Constraint::Min(0),
            Constraint::Length(2),
        ]).split(area);
        (None, c[1], c[2], c[4])
    };

    // ── Starfield + figlet logo ───────────────────────────────────────────
    if let Some(logo_area) = logo_chunk {
        render_logo(frame, app, logo_area);
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("bytewarden", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
                Span::styled(" v0.1.0",   Style::default().fg(t.inactive)),
            ])).alignment(Alignment::Center),
            title_chunk,
        );
    }

    // ── Form ─────────────────────────────────────────────────────────────
    let form_area = center_rect(60, form_height, form_chunk);
    let form_border = if app.login_error { Style::default().fg(t.error) }
                      else               { Style::default().fg(t.accent) };
    let block = Block::default()
        .title(" Login ").borders(Borders::ALL)
        .border_type(BorderType::Rounded).border_style(form_border)
        .padding(Padding::horizontal(2));
    let inner = block.inner(form_area);
    frame.render_widget(block, form_area);
    app.mouse_areas.login = Some(form_area);

    let f = Layout::vertical([
        Constraint::Length(1), // [0] padding
        Constraint::Length(1), // [1] email label
        Constraint::Length(3), // [2] email input
        Constraint::Length(1), // [3] pass label
        Constraint::Length(3), // [4] pass input
        Constraint::Length(1), // [5] save email
        Constraint::Length(1), // [6] auto-lock
        Constraint::Length(2), // [7] feedback/error strip
    ]).split(inner);

    // Email
    frame.render_widget(Paragraph::new("Email:").style(Style::default().fg(t.dim)), f[1]);
    let email_foc = app.active_field == LoginField::Email;
    frame.render_widget(
        Paragraph::new(input_with_cursor(&app.email_input, app.email_cursor, email_foc, t.accent))
            .block(rounded_block(focus_border(email_foc, t.accent))),
        f[2],
    );

    // Password
    frame.render_widget(Paragraph::new(Line::from(vec![
        Span::styled("Master Password:", Style::default().fg(t.dim)),
        Span::styled("  (F2: reveal)", Style::default().fg(
            if app.login_password_visible { t.accent } else { t.dim }
        )),
    ])), f[3]);
    let pass_foc = app.active_field == LoginField::Password;
    let pass_line = if app.login_password_visible {
        // Show plain text with cursor
        input_with_cursor(&app.password_input, app.password_cursor, pass_foc, t.accent)
    } else {
        // Masked
        let masked_before = "●".repeat(app.password_cursor);
        let masked_after  = "●".repeat(app.password_input.chars().count().saturating_sub(app.password_cursor));
        if pass_foc {
            Line::from(vec![
                Span::raw(masked_before),
                Span::styled("█", Style::default().fg(t.accent)),
                Span::raw(masked_after),
            ])
        } else {
            Line::from(Span::raw("●".repeat(app.password_input.chars().count())))
        }
    };
    frame.render_widget(
        Paragraph::new(pass_line).block(rounded_block(focus_border(pass_foc, t.accent))),
        f[4],
    );

    // Checkboxes
    render_checkbox(frame, "Save email",                app.save_email,  app.active_field == LoginField::SaveEmail, t.accent, t.inactive, f[5]);
    let lock_label = format!("Auto-lock after {} min", app.lock_after_secs / 60);
    render_checkbox(frame, &lock_label, app.auto_lock, app.active_field == LoginField::AutoLock, t.accent, t.inactive, f[6]);

    // Feedback/error strip
    let strip_block = Block::default().borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(60, 40, 80)));
    if app.login_error {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ✕ ", Style::default().fg(t.error).add_modifier(Modifier::BOLD)),
                Span::styled("Invalid credentials. Please try again.", Style::default().fg(t.error)),
            ])).block(strip_block),
            f[7],
        );
    } else if let Some(line) = action_line(app) {
        frame.render_widget(Paragraph::new(line).block(strip_block), f[7]);
    }

    // Bottom hints bar
    render_cmd_bar(frame, area, bar_chunk,
        "Tab: field  |  F2: reveal password  |  Space: toggle  |  Enter: login  |  ←→: cursor  |  Ctrl+C: quit",
        "Tab:field  F2:reveal  Enter:login  ^C:quit", t.dim);
}

// ── Vault ─────────────────────────────────────────────────────────────────

fn draw_vault(frame: &mut Frame, app: &mut App) {
    let t = &app.theme;
    let area = frame.area();

    let outer = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(area);
    let body   = Layout::horizontal([Constraint::Percentage(26), Constraint::Percentage(74)]).split(outer[0]);
    let sidebar = Layout::vertical([
        Constraint::Length(3),
        Constraint::Percentage(30),
        Constraint::Min(0),
    ]).split(body[0]);
    let cmd_h = if app.cmd_log.is_empty() { 4u16 } else { 9 };
    let main = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(cmd_h),
    ]).split(body[1]);

    // Keybindings bar — dynamic based on focused panel and view context
    let hints_pairs: &[(&str, &str)] = match app.focus {
        Focus::Search => &[
            ("Esc","clear"), ("j/k","navigate"), ("Enter","open detail"), ("type","filter"),
        ],
        Focus::Items => &[
            ("j/k","select filter"), ("Enter","apply"), ("Tab","next panel"),
        ],
        Focus::Vaults => &[
            ("Tab","next panel"),
        ],
        Focus::CmdLog => &[
            ("j/k","scroll"), ("PgUp/Dn","scroll×5"), ("Tab","next panel"),
        ],
        Focus::List | Focus::Status => if app.is_trash_view() { &[
            ("j/k","navigate"), ("Enter","detail"),
            ("Alt+R","restore"), ("Alt+D","delete"), ("Alt+L","lock"), ("?","help"),
        ]} else { &[
            ("j/k","navigate"), ("Enter","detail"),
            ("Alt+N","new"), ("Alt+U","user"), ("Alt+C","pass"), ("Alt+F","fav"),
            ("Alt+S","sync"), ("Alt+D","delete"), ("Alt+L","lock"), ("?","help"),
        ]},
    };
    let full  = hints_pairs.iter().map(|(k,v)| format!("{k}: {v}")).collect::<Vec<_>>().join("  |  ");
    let short = hints_pairs.iter().map(|(k,v)| format!("{k}:{v}")).collect::<Vec<_>>().join("  ");
    render_cmd_bar(frame, area, outer[1], &full, &short, t.dim);

    // [5] Status pane
    let (status_title_style, status_line) = match &app.action_state {
        ActionState::Idle =>
            (Style::default().fg(t.inactive), Line::from("")),
        _ => (
            Style::default().fg(match &app.action_state {
                ActionState::Running(_) => t.accent,
                ActionState::Done(_)    => t.success,
                _                                    => t.error,
            }),
            action_line(app).unwrap_or_else(|| Line::from("")),
        ),
    };
    frame.render_widget(
        Paragraph::new(status_line)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled("─Status", status_title_style))
                .border_style(Style::default().fg(t.inactive))),
        sidebar[0],
    );

    // [1] Vaults panel
    let vf = app.focus == Focus::Vaults;
    let mut vs = ListState::default(); vs.select(Some(0));
    frame.render_stateful_widget(
        List::new(vec![ListItem::new(Line::from(vec![
            Span::styled("  My Vault", Style::default().fg(Color::White)),
            Span::styled(format!("  {}", app.items.len()), Style::default().fg(t.dim)),
        ]))])
        .block(titled_block("─[1]-Vaults", "─1 of 1─", focus_color(vf, t.accent, t.inactive)))
        .highlight_style(Style::default().bg(t.selected_bg).fg(Color::White)),
        sidebar[1], &mut vs,
    );

    // [2] Items filter panel
    let itf = app.focus == Focus::Items;
    let filter_items: Vec<ListItem> = ITEM_FILTERS.iter().map(|f| {
        let count = app.count_for(f);
        let col = match f {
            ItemFilter::Login      => t.item_login,
            ItemFilter::Card       => t.item_card,
            ItemFilter::Identity   => t.item_identity,
            ItemFilter::SecureNote => t.item_note,
            ItemFilter::SshKey     => t.item_ssh,
            ItemFilter::Favorites  => t.item_favorite,
            ItemFilter::Trash      => t.error,
            ItemFilter::All        => Color::White,
        };
        let icon = match f {
            ItemFilter::All        => "  ",
            ItemFilter::Favorites  => "★ ",
            ItemFilter::Login      => "󰌋 ",
            ItemFilter::Card       => "󰻷 ",
            ItemFilter::Identity   => "󰀉 ",
            ItemFilter::SecureNote => "󰎞 ",
            ItemFilter::SshKey     => "󰣀 ",
            ItemFilter::Trash      => "󰩺 ",
        };
        let active = *f == app.active_filter;
        let style = if active { Style::default().fg(col).add_modifier(Modifier::BOLD) }
                    else      { Style::default().fg(col) };
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {icon}{}", f.label()), style),
            Span::styled(format!("  {count}"), Style::default().fg(t.dim)),
        ]))
    }).collect();
    // Inject a visual separator before the Trash entry
    let mut filter_items_with_sep: Vec<ListItem> = Vec::with_capacity(filter_items.len() + 1);
    for (i, item) in filter_items.into_iter().enumerate() {
        // Trash is the last entry (index = ITEM_FILTERS.len() - 1)
        if i == ITEM_FILTERS.len() - 1 {
            filter_items_with_sep.push(ListItem::new(Line::from(
                Span::styled("  ─────────────────", Style::default().fg(Color::Rgb(60, 62, 80)))
            )));
        }
        filter_items_with_sep.push(item);
    }
    let mut fs = ListState::default();
    // Offset selection by 1 to account for the separator row before Trash
    let display_sel = if app.filter_selected == ITEM_FILTERS.len() - 1 {
        app.filter_selected + 1
    } else {
        app.filter_selected
    };
    fs.select(Some(display_sel));
    let fi_indicator = format!("{} of {}", app.filter_selected + 1, ITEM_FILTERS.len());
    frame.render_stateful_widget(
        List::new(filter_items_with_sep)
            .block(titled_block("─[2]-Items", &format!("─{fi_indicator}─"), focus_color(itf, t.accent, t.inactive)))
            .highlight_style(Style::default().bg(t.selected_bg).fg(Color::White))
            .highlight_symbol("▶ "),
        sidebar[2], &mut fs,
    );

    // [/] Search bar
    let sf = app.focus == Focus::Search;
    let search_line = if sf {
        Line::from(vec![
            Span::styled(app.search_query.as_str(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(t.accent)),
        ])
    } else if !app.search_query.is_empty() {
        Line::from(Span::styled(app.search_query.as_str(), Style::default().fg(t.dim)))
    } else {
        Line::from(Span::styled("type to filter…", Style::default().fg(Color::Rgb(80, 85, 120))))
    };
    frame.render_widget(
        Paragraph::new(search_line)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled("─[/]-Search", focus_color(sf, t.accent, t.inactive)))
                .border_style(focus_border(sf, t.accent))),
        main[0],
    );

    // [3] Vault list
    let lf = app.focus == Focus::List;
    let filtered = app.filtered_items();
    let list_items: Vec<ListItem> = filtered.iter().map(|item| {
        let col = match item.item_type {
            1 => t.item_login, 2 => t.item_note, 3 => t.item_card,
            4 => t.item_identity, 5 => t.item_ssh, _ => t.dim,
        };
        ListItem::new(Line::from(vec![
            if item.favorite { Span::styled("★ ", Style::default().fg(t.item_favorite)) }
            else             { Span::raw("  ") },
            Span::styled(format!("{:<14}", format!("[{}]", item_type_label(item.item_type))), Style::default().fg(col)),
            Span::raw(item.name.as_str()),
        ]))
    }).collect();
    let flen = filtered.len();
    let mut ls = ListState::default();
    ls.select(if flen == 0 { None } else { Some(app.selected_index) });
    let vi = if flen > 0 { format!("{} of {}", app.selected_index + 1, flen) }
             else        { "0 of 0".into() };
    frame.render_stateful_widget(
        List::new(list_items)
            .block(titled_block("─[3]-Vault", &format!("─{vi}─"), focus_color(lf, t.accent, t.inactive)))
            .highlight_style(Style::default().bg(t.selected_bg).fg(Color::White).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        main[1], &mut ls,
    );

    // [4] Command log
    let clf = app.focus == Focus::CmdLog;
    let all_log: Vec<Line> = if app.cmd_log.is_empty() {
        vec![Line::from(Span::styled("  no commands yet", Style::default().fg(t.dim)))]
    } else {
        app.cmd_log.iter().flat_map(|e| {
            let col  = if e.ok { t.success } else { t.error };
            let icon = if e.ok { "✓" } else { "✕" };
            vec![
                Line::from(Span::styled(format!("  $ {}", e.cmd), Style::default().fg(t.dim))),
                Line::from(Span::styled(format!("  {icon} {}", e.detail), Style::default().fg(col))),
            ]
        }).collect()
    };
    let visible = cmd_h.saturating_sub(2) as usize;
    let end     = all_log.len().saturating_sub(app.cmd_log_scroll);
    let start   = end.saturating_sub(visible);
    let log_title = if app.cmd_log_scroll > 0 { "─[4]-Command Log  ↑" } else { "─[4]-Command Log" };
    frame.render_widget(
        Paragraph::new(all_log[start..end].to_vec())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(log_title, focus_color(clf, t.accent, t.inactive)))
                .border_style(Style::default().fg(focus_color(clf, t.accent, t.inactive)))),
        main[2],
    );

    app.mouse_areas.status = Some(sidebar[0]);
    app.mouse_areas.vaults = Some(sidebar[1]);
    app.mouse_areas.items  = Some(sidebar[2]);
    app.mouse_areas.search = Some(main[0]);
    app.mouse_areas.list   = Some(main[1]);
    app.mouse_areas.cmdlog = Some(main[2]);
}

// ── Detail ────────────────────────────────────────────────────────────────

struct DetailField { label: String, value: String, hidden: bool }

fn build_detail_fields(item: &Item, show: bool, reveal_idx: usize) -> Vec<DetailField> {
    let mut f: Vec<DetailField> = vec![
        DetailField { label: "Name".into(), value: item.name.clone(), hidden: false },
        DetailField { label: "Type".into(), value: item_type_label(item.item_type).into(), hidden: false },
    ];

    if let Some(login) = &item.login {
        if let Some(u) = &login.username {
            f.push(DetailField { label: "Username".into(), value: u.clone(), hidden: false });
        }
        let pass = login.password.as_deref().unwrap_or("").to_string();
        let rev  = show && reveal_idx == f.len();
        f.push(DetailField {
            label: "Password".into(),
            value: if rev || pass.is_empty() { pass.clone() } else { "●".repeat(pass.chars().count().max(8)) },
            hidden: !rev && !pass.is_empty(),
        });
        for uri_d in login.uris.iter().flatten() {
            if let Some(uri) = &uri_d.uri {
                f.push(DetailField { label: "URL".into(), value: uri.clone(), hidden: false });
            }
        }
        if let Some(totp) = &login.totp {
            let rev = show && reveal_idx == f.len();
            f.push(DetailField {
                label: "TOTP".into(),
                value: if rev { totp.clone() } else { "●●●●●●".into() },
                hidden: !rev,
            });
        }
    }

    if let Some(card) = &item.card {
        push_opt_field(&mut f, "Cardholder", &card.cardholder_name, false, show, reveal_idx);
        push_opt_field(&mut f, "Brand",      &card.brand,           false, show, reveal_idx);
        push_opt_field(&mut f, "Number",     &card.number,          true,  show, reveal_idx);
        if card.exp_month.is_some() || card.exp_year.is_some() {
            f.push(DetailField {
                label: "Expiry".into(),
                value: format!("{}/{}", card.exp_month.as_deref().unwrap_or("?"), card.exp_year.as_deref().unwrap_or("?")),
                hidden: false,
            });
        }
        push_opt_field(&mut f, "CVV", &card.code, true, show, reveal_idx);
    }

    if let Some(id) = &item.identity {
        let full = build_full_name(id.title.as_deref(), id.first_name.as_deref(), id.middle_name.as_deref(), id.last_name.as_deref());
        if !full.is_empty() { f.push(DetailField { label: "Full Name".into(), value: full, hidden: false }); }
        let hidden_set = ["SSN", "Passport", "License"];
        for (lbl, val) in identity_fields(id) {
            push_opt_field(&mut f, lbl, val, hidden_set.contains(&lbl), show, reveal_idx);
        }
    }

    for field in &item.fields {
        let name  = field.name.as_deref().unwrap_or("Field").to_string();
        let value = field.value.as_deref().unwrap_or("").to_string();
        let is_h  = field.field_type == 1;
        let rev   = show && reveal_idx == f.len();
        f.push(DetailField {
            label: name,
            value: if is_h && !rev { "●".repeat(value.chars().count().max(4)) } else { value },
            hidden: is_h && !rev,
        });
    }

    if let Some(notes) = &item.notes {
        if !notes.is_empty() { f.push(DetailField { label: "Notes".into(), value: notes.clone(), hidden: false }); }
    }
    f
}

/// Push a field from an Option<String>, skipping if None or empty.
fn push_opt_field(fields: &mut Vec<DetailField>, label: &str, val: &Option<String>, is_hidden: bool, show: bool, reveal_idx: usize) {
    if let Some(v) = val { if !v.is_empty() {
        let rev = show && reveal_idx == fields.len();
        let hid = is_hidden && !rev;
        fields.push(DetailField {
            label: label.to_string(),
            value: if hid { "●".repeat(v.chars().count().max(4)) } else { v.clone() },
            hidden: hid,
        });
    }}
}

fn draw_detail(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let item = match app.selected_item() { Some(i) => i.clone(), None => return };
    let t    = &app.theme;

    let chunks = Layout::vertical([
        Constraint::Length(2), Constraint::Min(0), Constraint::Length(2),
    ]).split(area);

    // Header
    let (action_text, action_style) = action_text_style(app);
    let mode_tag = if app.edit_mode {
        Span::styled("  [EDIT]", Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
    } else { Span::raw("") };
    let pad = (area.width as usize).saturating_sub(
        4 + item.name.len() + 4 + item_type_label(item.item_type).len() + action_text.len() + 2
    );
    let padded = format!("{:>width$}", action_text, width = action_text.len() + pad);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ← ", Style::default().fg(t.dim)),
            Span::styled(item.name.as_str(), Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  [{}]", item_type_label(item.item_type)), Style::default().fg(t.inactive)),
            mode_tag,
            Span::styled(padded, action_style),
        ])).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(t.inactive))),
        chunks[0],
    );

    app.mouse_areas.detail = Some(chunks[1]);

    if app.edit_mode {
        let ef  = &app.edit_fields;
        let fas = field_areas(ef.len(), chunks[1]);
        for (i, field) in ef.iter().enumerate() {
            if i >= fas.len() { break; }
            let sel  = i == app.edit_field_idx;
            let bcol = if sel { t.accent } else { t.inactive };
            let hint = if field.read_only && sel       { " (read-only)" }
                       else if field.hidden && sel && !field.revealed { "  (F2: reveal)" }
                       else if field.hidden && sel &&  field.revealed { "  (F2: hide)"   }
                       else { "" };
            let display = if field.hidden && !field.revealed { "●".repeat(field.value.chars().count().max(8)) }
                          else { field.value.clone() };
            let vline = if sel && !field.read_only { cursor_line(&display, field.cursor, t.accent) }
                        else { Line::from(Span::styled(display, Style::default().fg(t.inactive))) };
            render_field_card(frame, &field.label, hint, vline, bcol, fas[i], t);
        }
        let fh = app.edit_fields.get(app.edit_field_idx).map_or(false, |f| f.hidden);
        render_cmd_bar(frame, area, chunks[2],
            if fh { "Tab/↑↓: field  |  ←→: cursor  |  F2: reveal/hide  |  Enter: save  |  Esc: cancel" }
            else  { "Tab/↑↓: field  |  ←→: cursor  |  Enter: save  |  Esc: cancel" },
            if fh { "Tab:field  F2:reveal  Enter:save  Esc:cancel" }
            else  { "Tab:field  ←→:cursor  Enter:save  Esc:cancel" },
            t.dim);
    } else {
        let fields = build_detail_fields(&item, app.show_password, app.detail_field);
        let sel    = app.detail_field.min(fields.len().saturating_sub(1));
        let fas    = field_areas(fields.len(), chunks[1]);
        for (i, field) in fields.iter().enumerate() {
            if i >= fas.len() { break; }
            let is_sel = i == sel;
            let bcol   = if is_sel { t.accent } else { t.inactive };
            let hint   = if field.hidden && is_sel { "  (F2: reveal)" } else { "" };
            let vline  = if is_sel { Line::from(Span::styled(field.value.as_str(), Style::default().fg(Color::White))) }
                         else      { Line::from(Span::styled(field.value.as_str(), Style::default().fg(t.inactive))) };
            render_field_card(frame, &field.label, hint, vline, bcol, fas[i], t);
        }
        let (detail_full, detail_short) = if app.is_trash_view() {
            ("j/k: field  |  F2: show/hide  |  Alt+R: restore  |  Alt+D: delete permanently  |  Esc: back",
             "j/k:field  F2:reveal  Alt+R:restore  Alt+D:del  Esc:back")
        } else {
            ("j/k: field  |  F2: show/hide  |  Alt+C: copy  |  Alt+E: edit  |  Alt+D: delete  |  Esc: back",
             "j/k:field  F2:reveal  Alt+C:copy  Alt+E:edit  Alt+D:del  Esc:back")
        };
        render_cmd_bar(frame, area, chunks[2], detail_full, detail_short, t.dim);
    }
}

// ── Create ────────────────────────────────────────────────────────────────

fn draw_create(frame: &mut Frame, app: &mut App) {
    let t    = &app.theme;
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(0), Constraint::Length(2)]).split(area);

    let title = if app.create_choosing_type { "New Item — choose type" } else { app.create_type.label() };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" + ", Style::default().fg(t.success).add_modifier(Modifier::BOLD)),
            Span::styled(title,  Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        ])).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(t.inactive))),
        chunks[0],
    );

    if app.create_choosing_type {
        let areas = Layout::vertical(
            (0..CREATE_ITEM_TYPES.len()).map(|_| Constraint::Length(3)).collect::<Vec<_>>()
        ).split(chunks[1]);
        for (i, ct) in CREATE_ITEM_TYPES.iter().enumerate() {
            if i >= areas.len() { break; }
            let sel = i == app.create_type_idx;
            let col = if sel { t.accent } else { t.inactive };
            let pfx = if sel { "▶ " } else { "  " };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!("{}{}", pfx, ct.label()),
                    Style::default().fg(col).add_modifier(if sel { Modifier::BOLD } else { Modifier::empty() }),
                ))).block(rounded_block(Style::default().fg(col))),
                areas[i],
            );
        }
    } else {
        // Feedback strip
        let fb = action_line(app);
        let fb_h: u16 = if fb.is_some() { 1 } else { 0 };
        let form_parts = Layout::vertical([Constraint::Length(fb_h), Constraint::Min(0)]).split(chunks[1]);
        if let Some(line) = fb { frame.render_widget(Paragraph::new(line), form_parts[0]); }

        let fas = field_areas(app.create_fields.len(), form_parts[1]);
        for (i, field) in app.create_fields.iter().enumerate() {
            if i >= fas.len() { break; }
            let sel  = i == app.create_field_idx;
            let bcol = if sel { t.accent } else { t.inactive };
            let hint = if field.hidden && sel && !field.revealed { "  (F2: reveal)" }
                       else if field.hidden && sel && field.revealed { "  (F2: hide)" }
                       else { "" };
            let display = if field.hidden && !field.revealed { "●".repeat(field.value.chars().count()) }
                          else { field.value.clone() };
            let vline = if sel { cursor_line(&display, field.cursor, t.accent) }
                        else   { Line::from(Span::styled(display, Style::default().fg(t.inactive))) };
            render_field_card(frame, &field.label, hint, vline, bcol, fas[i], t);
        }
    }

    let (hf, hs) = if app.create_choosing_type {
        ("j/k: select type  |  Enter: confirm  |  Esc: cancel",
         "j/k:select  Enter:ok  Esc:cancel")
    } else {
        ("Tab/↑↓: field  |  ←→: cursor  |  F2: reveal  |  Enter: create  |  Esc: cancel",
         "Tab:field  F2:reveal  Enter:create  Esc:cancel")
    };
    render_cmd_bar(frame, area, chunks[2], hf, hs, t.dim);
}

// ── Confirm delete popup ──────────────────────────────────────────────────

fn draw_confirm_delete_popup(frame: &mut Frame, area: Rect, app: &App) {
    let t    = &app.theme;
    let name = app.selected_item().map(|i| i.name.as_str()).unwrap_or("this item");
    let popup = center_rect(50, 10, area);
    frame.render_widget(Clear, popup);

    // In trash view: Enter = permanent delete (already trashed, no second trash)
    // In vault view: Enter = trash, D = permanent
    let lines = if app.is_trash_view() {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Delete: ", Style::default().fg(t.inactive)),
                Span::styled(name,         Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Already in trash — this will delete permanently.", Style::default().fg(t.dim))),
            Line::from(""),
            Line::from(vec![Span::styled("  Enter", Style::default().fg(t.error).add_modifier(Modifier::BOLD)), Span::styled("  Delete permanently", Style::default().fg(t.error))]),
            Line::from(vec![Span::styled("  Esc  ", Style::default().fg(t.dim)),                                 Span::styled("  Cancel",             Style::default().fg(t.dim))]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Delete: ", Style::default().fg(t.inactive)),
                Span::styled(name,         Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled("  This action cannot be easily undone.", Style::default().fg(t.dim))),
            Line::from(""),
            Line::from(vec![Span::styled("  Enter", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)), Span::styled("  Move to trash",      Style::default().fg(Color::White))]),
            Line::from(vec![Span::styled("  D    ", Style::default().fg(t.error).add_modifier(Modifier::BOLD)),  Span::styled("  Delete permanently", Style::default().fg(t.error))]),
            Line::from(vec![Span::styled("  Esc  ", Style::default().fg(t.dim)),                                  Span::styled("  Cancel",             Style::default().fg(t.dim))]),
            Line::from(""),
        ]
    };

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().title(" Confirm Delete ")
            .borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(Style::default().fg(t.error))),
        popup,
    );
}

// ── Help popup ────────────────────────────────────────────────────────────

fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup = center_rect(60, 32, area);
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Login",  Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("Tab",       "Cycle fields"),
        help_line("F2",        "Reveal / hide master password"),
        help_line("Enter",     "Login / Unlock"),
        Line::from(""),
        Line::from(Span::styled("  Vault",  Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("j / ↓",    "Move down"),
        help_line("k / ↑",    "Move up"),
        help_line("Enter / l", "Open detail"),
        help_line("/",         "Search vault"),
        help_line("Alt+N",     "New item"),
        help_line("Alt+U",     "Copy username"),
        help_line("Alt+C",     "Copy password"),
        help_line("Alt+F",     "Toggle favorite ★"),
        help_line("Alt+S",     "Sync vault"),
        help_line("Alt+D",     "Delete item"),
        help_line("Alt+Q",     "Lock vault"),
        Line::from(""),
        Line::from(Span::styled("  Trash",  Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("Alt+R",     "Restore item to vault"),
        help_line("D / Enter", "Permanently delete"),
        Line::from(""),
        Line::from(Span::styled("  Detail", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("Alt+E",     "Edit item"),
        help_line("F2",        "Show / hide hidden field"),
        help_line("Alt+C",     "Copy field"),
        help_line("Alt+D",     "Delete item"),
        help_line("Esc / h",   "Back to vault"),
        Line::from(""),
        Line::from(Span::styled("  Global", Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD))),
        help_line("Ctrl+C",    "Quit"),
        help_line("?",         "This help screen"),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", Style::default().fg(COLOR_DIM))),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(Block::default()
            .title(" Help — Keyboard shortcuts ")
            .borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(Style::default().fg(COLOR_ACCENT))),
        popup,
    );
}

// ── Shared render helpers ─────────────────────────────────────────────────

const SPINNER: [&str; 4] = ["-", "\\", "|", "/"];

fn spinner_frame(tick: u8) -> &'static str {
    SPINNER[(tick / 3) as usize % SPINNER.len()]
}

/// Build a vertical layout of `count` equal rows of height 4 (label+box).
fn field_areas(count: usize, area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::vertical(
        (0..count).map(|_| Constraint::Length(4)).collect::<Vec<_>>()
    ).split(area)
}

/// Render a label row + bordered value box (4 rows total: 1 label + 3 box).
fn render_field_card(
    frame:  &mut Frame,
    label:  &str,
    hint:   &str,
    vline:  Line,
    bcol:   ratatui::style::Color,
    area:   Rect,
    t:      &crate::theme::Theme,
) {
    let fc = Layout::vertical([Constraint::Length(1), Constraint::Length(3)]).split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label}"), Style::default().fg(bcol)),
            Span::styled(hint, Style::default().fg(t.dim)),
        ])),
        fc[0],
    );
    frame.render_widget(
        Paragraph::new(vline).block(rounded_block(Style::default().fg(bcol))),
        fc[1],
    );
}

/// Render the bottom command-bar with graceful truncation.
fn render_cmd_bar(frame: &mut Frame, area: Rect, bar: Rect, full: &str, short: &str, col: ratatui::style::Color) {
    let avail = area.width.saturating_sub(2) as usize;
    let hint  = if full.len() <= avail { full }
                else if short.len() <= avail { short }
                else { &short[..avail.saturating_sub(1)] };
    frame.render_widget(
        Paragraph::new(format!(" {hint}")).style(Style::default().fg(col))
            .block(Block::default().borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(60, 62, 80)))),
        bar,
    );
}

/// Build a `Line` showing the current action state (spinner / ✓ / ✕).
/// Returns None when Idle.
fn action_line(app: &App) -> Option<Line<'static>> {
    let sp = spinner_frame(app.action_tick);
    let t  = &app.theme;
    match &app.action_state {
        ActionState::Idle => None,
        ActionState::Running(msg) => Some(Line::from(vec![
            Span::styled(format!(" {sp} "), Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled(msg.clone(), Style::default().fg(t.accent)),
        ])),
        ActionState::Done(msg) => Some(Line::from(vec![
            Span::styled(" ✓ ", Style::default().fg(t.success).add_modifier(Modifier::BOLD)),
            Span::styled(msg.clone(), Style::default().fg(t.success)),
        ])),
        ActionState::Error(msg) => Some(Line::from(vec![
            Span::styled(" ✕ ", Style::default().fg(t.error).add_modifier(Modifier::BOLD)),
            Span::styled(msg.clone(), Style::default().fg(t.error)),
        ])),
    }
}

/// Returns (text, style) for the action state — used in detail header.
fn action_text_style(app: &App) -> (String, Style) {
    let sp = spinner_frame(app.action_tick);
    let t  = &app.theme;
    match &app.action_state {
        ActionState::Idle          => (String::new(), Style::default()),
        ActionState::Running(msg)  => (format!("{sp} {msg}"), Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        ActionState::Done(msg)     => (format!("✓ {msg}"),   Style::default().fg(t.success).add_modifier(Modifier::BOLD)),
        ActionState::Error(msg)    => (format!("✕ {msg}"),   Style::default().fg(t.error).add_modifier(Modifier::BOLD)),
    }
}

/// Build a cursor-at-index line (█ block cursor).
fn cursor_line(display: &str, cursor: usize, accent: ratatui::style::Color) -> Line<'static> {
    let chars: Vec<char> = display.chars().collect();
    let pos    = cursor.min(chars.len());
    let before: String = chars[..pos].iter().collect();
    let after:  String = chars[pos..].iter().collect();
    Line::from(vec![
        Span::raw(before),
        Span::styled("█", Style::default().fg(accent)),
        Span::styled(after, Style::default().fg(Color::White)),
    ])
}

// ── Widget / style helpers ────────────────────────────────────────────────

/// Returns the accent color if focused, else inactive.
fn focus_color(focused: bool, accent: ratatui::style::Color, inactive: ratatui::style::Color) -> ratatui::style::Color {
    if focused { accent } else { inactive }
}

/// Returns a border Style using accent when focused, default otherwise.
fn focus_border(focused: bool, accent: ratatui::style::Color) -> Style {
    if focused { Style::default().fg(accent) } else { Style::default() }
}

/// Rounded-border block with a given border style.
fn rounded_block(border_style: Style) -> Block<'static> {
    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(border_style)
}

/// Rounded block with a title on top and a dim counter on the bottom-right.
fn titled_block(title: &str, bottom: &str, col: ratatui::style::Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .title(Span::styled(title.to_string(), Style::default().fg(col)))
        .title_bottom(Line::from(Span::styled(bottom.to_string(), Style::default().fg(Color::DarkGray))).right_aligned())
        .border_style(Style::default().fg(col))
}

// ── Login-specific helpers ────────────────────────────────────────────────

fn render_checkbox(frame: &mut Frame, label: &str, checked: bool, focused: bool,
                   accent: ratatui::style::Color, inactive: ratatui::style::Color, area: Rect) {
    let icon  = if checked { "☑" } else { "☐" };
    let icol  = if checked { accent } else { inactive };
    let lcol  = if focused { accent } else { inactive };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(icon, Style::default().fg(icol)),
            Span::styled(format!(" {label}"), Style::default().fg(lcol)),
        ])),
        area,
    );
}

fn input_with_cursor<'a>(text: &'a str, cursor_pos: usize, focused: bool, accent: ratatui::style::Color) -> Line<'a> {
    if !focused { return Line::from(Span::raw(text)); }
    let chars: Vec<char> = text.chars().collect();
    let before: String = chars[..cursor_pos].iter().collect();
    let after:  String = chars[cursor_pos..].iter().collect();
    Line::from(vec![
        Span::raw(before),
        Span::styled("█", Style::default().fg(accent)),
        Span::styled(after, Style::default().fg(Color::White)),
    ])
}

// ── Logo renderer ─────────────────────────────────────────────────────────

fn render_logo(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let w = area.width as usize;
    let h = area.height as usize;

    let s_dim    = Style::default().fg(Color::Rgb(38, 34, 72));
    let s_mid    = Style::default().fg(Color::Rgb(90, 84, 148));
    let s_bright = Style::default().fg(Color::Rgb(185, 178, 248));
    let acc_dim  = Style::default().fg(t.inactive);
    let col_acc  = Style::default().fg(t.accent);

    let (fig1_str, fig2_str) = {
        let font = FIGfont::from_content(include_str!("assets/slant.flf"))
            .unwrap_or_else(|_| FIGfont::standard().unwrap());
        (
            font.convert("byte").map(|f| f.to_string()).unwrap_or_else(|| "byte".into()),
            font.convert("warden").map(|f| f.to_string()).unwrap_or_else(|| "warden".into()),
        )
    };

    let trim = |s: &str| -> Vec<String> {
        let ls: Vec<&str> = s.lines().collect();
        let a = ls.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
        let b = ls.iter().rposition(|l| !l.trim().is_empty()).map(|i| i+1).unwrap_or(ls.len());
        ls[a..b].iter().map(|l| l.to_string()).collect()
    };

    let r1o = trim(&fig1_str); let r2o = trim(&fig2_str);
    let r1: Vec<&str> = r1o.iter().map(|s| s.as_str()).collect();
    let r2: Vec<&str> = r2o.iter().map(|s| s.as_str()).collect();
    let r1s = 1usize; let r2s = r1s + r1.len();
    let tend = r2s + r2.len();
    let trow = tend + (h.saturating_sub(tend + 1)) / 2;
    let f1w = r1.iter().map(|l| l.len()).max().unwrap_or(40);
    let f2w = r2.iter().map(|l| l.len()).max().unwrap_or(60);
    let f1c = if w > f1w { (w - f1w) / 2 } else { 0 };
    let f2c = if w > f2w { (w - f2w) / 2 } else { 0 };

    let star_at = |row: usize, col: usize| -> (char, Style) {
        let h = row.wrapping_mul(17).wrapping_add(col.wrapping_mul(31)).wrapping_add(row.wrapping_mul(col)) % 120;
        match h { 0 => ('\u{2726}', s_bright), 1|2 => ('\u{00b7}', s_mid), 3 => ('\u{22c6}', s_dim), _ => (' ', s_dim) }
    };

    let spans_from = |row: usize, fig: &str, fc: usize, fw: usize| -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut cs = s_dim; let mut ct = String::new();
        for col in 0..w {
            let fi = col.wrapping_sub(fc);
            let (ch, st) = if col >= fc && fi < fw {
                let c = fig.chars().nth(fi).unwrap_or(' ');
                if c != ' ' { (c, col_acc) } else { star_at(row, col) }
            } else { star_at(row, col) };
            if st == cs { ct.push(ch); } else {
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
            if st == cs { ct.push(ch); } else {
                if !ct.is_empty() { spans.push(Span::styled(ct.clone(), cs)); ct.clear(); }
                cs = st; ct.push(ch);
            }
        }
        if !ct.is_empty() { spans.push(Span::styled(ct, cs)); }
        Line::from(spans)
    };

    let mut lines: Vec<Line> = Vec::with_capacity(h);
    for row in 0..h {
        if row == trow {
            lines.push(Line::from(Span::styled("v0.1.0", acc_dim)).alignment(Alignment::Center));
            continue;
        }
        let r = row.wrapping_sub(r1s);
        let s = row.wrapping_sub(r2s);
        if row >= r1s && r < r1.len()      { lines.push(spans_from(row, r1[r], f1c, f1w)); }
        else if row >= r2s && s < r2.len() { lines.push(spans_from(row, r2[s], f2c, f2w)); }
        else                               { lines.push(pure_stars(row)); }
    }
    frame.render_widget(Paragraph::new(lines), area);
}

// ── Layout helpers ────────────────────────────────────────────────────────

fn center_rect(width_pct: u16, height: u16, area: Rect) -> Rect {
    let v = Layout::vertical([Constraint::Fill(1), Constraint::Length(height), Constraint::Fill(1)]).split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - width_pct) / 2),
        Constraint::Percentage(width_pct),
        Constraint::Percentage((100 - width_pct) / 2),
    ]).split(v[1])[1]
}

fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{key:<14}"), Style::default().fg(COLOR_ACCENT)),
        Span::styled(desc, Style::default().fg(Color::White)),
    ])
}

/// Fill a rect with the pure star pattern (no logo text).
fn fill_stars(frame: &mut Frame, area: Rect) {
    if area.height == 0 || area.width == 0 { return; }
    let w = area.width as usize;
    let h = area.height as usize;
    let s_dim    = Style::default().fg(Color::Rgb(38, 34, 72));
    let s_mid    = Style::default().fg(Color::Rgb(90, 84, 148));
    let s_bright = Style::default().fg(Color::Rgb(185, 178, 248));
    let star_at = |row: usize, col: usize| -> (char, Style) {
        let hash = row.wrapping_mul(17).wrapping_add(col.wrapping_mul(31))
            .wrapping_add(row.wrapping_mul(col)) % 120;
        match hash {
            0     => ('\u{2726}', s_bright),
            1 | 2 => ('\u{00b7}', s_mid),
            3     => ('\u{22c6}', s_dim),
            _     => (' ',        s_dim),
        }
    };
    // offset rows by area.y so stars align with the full-screen pattern
    let lines: Vec<ratatui::text::Line> = (0..h).map(|r| {
        let row = area.y as usize + r;
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut cs = s_dim; let mut ct = String::new();
        for col in 0..w {
            let (ch, st) = star_at(row, col);
            if st == cs { ct.push(ch); } else {
                if !ct.is_empty() { spans.push(Span::styled(ct.clone(), cs)); ct.clear(); }
                cs = st; ct.push(ch);
            }
        }
        if !ct.is_empty() { spans.push(Span::styled(ct, cs)); }
        ratatui::text::Line::from(spans)
    }).collect();
    frame.render_widget(Paragraph::new(lines), area);
}
