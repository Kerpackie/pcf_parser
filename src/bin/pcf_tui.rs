//! Interactive TUI viewer for PCF files.
//!
//! Keys: ↑/k/Mouse-Up  ↓/j/Mouse-Down   g-goto   q-quit

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::{Backend, CrosstermBackend}, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style}, text::{Line, Span}, widgets::{Block, Borders, Paragraph}, Frame, Terminal};
use std::{cmp, fs, io, path::PathBuf, time::Duration};

/// CLI arguments.
#[derive(Parser)]
struct Args {
    file_a: PathBuf,
    file_b: Option<PathBuf>,
}

/// One rendered line (offset, hex, ascii, per-byte diff flags)
struct HexLine {
    off: usize,
    hex_spans: Vec<Span<'static>>,
    ascii_spans: Vec<Span<'static>>,
}

fn build_lines(buf_a: &[u8], buf_b: Option<&[u8]>, bytes: usize) -> Vec<HexLine> {
    let mut out = Vec::new();
    for (row, chunk_a) in buf_a.chunks(bytes).enumerate() {
        let offset = row * bytes;
        let chunk_b = buf_b.and_then(|b| b.get(offset..offset + bytes)).unwrap_or(&[]);

        let mut hex_spans = Vec::with_capacity(bytes * 2);
        let mut ascii_spans = Vec::with_capacity(bytes);

        for i in 0..bytes {
            let a = *chunk_a.get(i).unwrap_or(&0);
            let b = *chunk_b.get(i).unwrap_or(&0);
            let diff = buf_b.is_some() && a != b;

            let fg = if diff { Color::Red } else { Color::White };
            hex_spans.push(Span::styled(format!("{:02X}", a), Style::default().fg(fg)));
            if i != bytes - 1 {
                hex_spans.push(Span::raw(" "));
            }

            let chr = if a.is_ascii_graphic() { a as char } else { '.' };
            ascii_spans.push(Span::styled(chr.to_string(), Style::default().fg(fg)));
        }

        out.push(HexLine { off: offset, hex_spans, ascii_spans });
    }
    out
}

enum Mode { View, Goto }

/// Menu options for the TUI
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum MenuItem {
    HexView,
    DiffView,
}

impl MenuItem {
    fn all() -> &'static [MenuItem] {
        &[MenuItem::HexView, MenuItem::DiffView]
    }
    fn title(&self) -> &'static str {
        match self {
            MenuItem::HexView => "Hex View",
            MenuItem::DiffView => "Diff View",
        }
    }
}

struct App<'a> {
    lines_a: Vec<HexLine>,
    lines_b: Option<Vec<HexLine>>,
    scroll: usize,
    bytes_per_line: usize,
    mode: Mode,
    goto_input: String,
    menu_selected: usize,
    _buf: &'a [u8],
}

impl<'a> App<'a> {
    fn try_jump(&mut self) -> Result<()> {
        let s = self.goto_input.trim();
        if s.is_empty() { return Ok(()); }
        let off = if let Some(hex) = s.strip_prefix("0x") {
            usize::from_str_radix(hex, 16)?
        } else if let Some(hex) = s.strip_suffix('h').or_else(|| s.strip_suffix('H')) {
            usize::from_str_radix(hex, 16)?
        } else { s.parse()? };
        self.scroll = off / self.bytes_per_line;
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let buf_a = fs::read(&args.file_a).with_context(|| format!("Reading {:?}", args.file_a))?;
    let buf_b = if let Some(p) = &args.file_b {
        Some(fs::read(p).with_context(|| format!("Reading {:?}", p))?)
    } else { None };

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let res = run(&mut term, &buf_a, buf_b.as_deref());

    terminal::disable_raw_mode()?;
    execute!(term.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    term.show_cursor()?;
    res
}

fn run(term: &mut Terminal<CrosstermBackend<io::Stdout>>, buf_a: &[u8], buf_b: Option<&[u8]>) -> Result<()> {
    let bytes = 16;
    let lines_a = build_lines(buf_a, buf_b, bytes);
    let lines_b = buf_b.map(|b| build_lines(b, Some(buf_a), bytes));

    let mut app = App { lines_a, lines_b, scroll: 0, bytes_per_line: bytes, mode: Mode::View, goto_input: String::new(), menu_selected: 0, _buf: buf_a };

    loop {
        let mut should_quit = false;

        term.draw(|f: &mut Frame| {
            // Draw menu bar
            let menu_items = MenuItem::all();
            let menu_spans: Vec<Span> = menu_items.iter().enumerate().map(|(i, item)| {
                if i == app.menu_selected {
                    Span::styled(
                        format!(" {} ", item.title()),
                        Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD),
                    )
                } else {
                    Span::styled(
                        format!(" {} ", item.title()),
                        Style::default().fg(Color::Yellow),
                    )
                }
            }).collect();
            let menu = Paragraph::new(Line::from(menu_spans)).block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(menu, Rect { x: 0, y: 0, width: f.size().width, height: 3 });

            // Adjust layout to leave space for menu
            let rows = if matches!(app.mode, Mode::Goto) {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(3), Constraint::Length(1)])
                    .split(f.size())
                    .to_vec()
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
                    .split(f.size())
                    .to_vec()
            };
            let viewer_area = rows[1];
            let panes = if app.lines_b.is_some() {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(viewer_area)
                    .to_vec()
            } else { vec![viewer_area] };

            // Show view based on menu selection
            match menu_items[app.menu_selected] {
                MenuItem::HexView => {
                    draw_side::<CrosstermBackend<io::Stdout>>(f, panes[0], &app.lines_a, "File A", app.scroll);
                    if let (Some(lines), Some(area)) = (app.lines_b.as_ref(), panes.get(1)) {
                        draw_side::<CrosstermBackend<io::Stdout>>(f, *area, lines, "File B", app.scroll);
                    }
                }
                MenuItem::DiffView => {
                    // Placeholder: show a message for now
                    let diff_msg = Paragraph::new("Diff view coming soon!").block(Block::default().borders(Borders::ALL).title("Diff"));
                    f.render_widget(diff_msg, panes[0]);
                }
            }
    
            if matches!(app.mode, Mode::Goto) {
                let prompt = Paragraph::new(Line::from(vec![
                    Span::styled("Goto offset: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&app.goto_input),
                ]))
                    .block(Block::default().borders(Borders::ALL).title("Input"));
                f.render_widget(prompt, rows[2]);
            }

            let help = Line::from(vec![
                Span::styled("↑/k", Style::default().fg(Color::Cyan)), Span::raw(" Scroll   "),
                Span::styled("g", Style::default().fg(Color::Cyan)), Span::raw(" Goto   "),
                Span::styled("q", Style::default().fg(Color::Cyan)), Span::raw(" Quit"),
            ]);
            let bar = Paragraph::new(help).block(Block::default().borders(Borders::TOP));
            if let Some(help_area) = rows.last() {
                f.render_widget(bar, *help_area);
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => match app.mode {
                    Mode::View => match k.code {
                        KeyCode::Char('q') => should_quit = true,
                        KeyCode::Up | KeyCode::Char('k') => app.scroll = app.scroll.saturating_sub(1),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll += 1,
                        KeyCode::Char('g') | KeyCode::Char('G') => { app.mode = Mode::Goto; app.goto_input.clear(); }
                        KeyCode::Left => app.menu_selected = app.menu_selected.saturating_sub(1),
                        KeyCode::Right => app.menu_selected = (app.menu_selected + 1).min(MenuItem::all().len() - 1),
                        _ => {}
                    },
                    Mode::Goto => match k.code {
                        KeyCode::Esc => app.mode = Mode::View,
                        KeyCode::Enter => if app.try_jump().is_ok() { app.mode = Mode::View },
                        KeyCode::Backspace => { app.goto_input.pop(); },
                        KeyCode::Char(c) => app.goto_input.push(c),
                        _ => {}
                    },
                },
                Event::Mouse(m) if matches!(app.mode, Mode::View) => match m.kind {
                    MouseEventKind::ScrollUp => app.scroll = app.scroll.saturating_sub(1),
                    MouseEventKind::ScrollDown => app.scroll += 1,
                    _ => {}
                },
                _ => {}
            }
        }

        if should_quit { break; }
    }

    Ok(())
}

/// Draws a single pane (file view) at the given `area`.
fn draw_side<B: Backend>(
    f: &mut Frame,
    area: Rect,
    lines: &[HexLine],
    title: &str,
    scroll: usize,
) {
    let max_rows = area.height.saturating_sub(2) as usize;
    let start = cmp::min(scroll, lines.len().saturating_sub(max_rows));
    let slice = &lines[start..cmp::min(start + max_rows, lines.len())];

    let header = Span::styled(
        format!(" {} ", title),
        Style::default().fg(Color::Magenta).add_modifier(ratatui::style::Modifier::BOLD),
    );
    let block = Block::default().borders(Borders::ALL).title(header);

    let body: Vec<Line> = slice
        .iter()
        .map(|l| {
            let mut spans = Vec::with_capacity(l.hex_spans.len() + l.ascii_spans.len() + 4);
            spans.push(Span::styled(format!("{:06X}", l.off), Style::default().fg(Color::DarkGray)));
            spans.push(Span::raw("  "));
            spans.extend(l.hex_spans.clone());
            spans.push(Span::raw("  |"));
            spans.extend(l.ascii_spans.clone());
            spans.push(Span::raw("|"));
            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(body).block(block);
    f.render_widget(paragraph, area);
}
