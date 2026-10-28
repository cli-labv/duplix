use crate::cli::KeepRule;
use crate::fingerprint::Fingerprint;
use crate::original_rule::choose_original;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io::{self, stdout};

pub fn decide_keep_windows(keep_rule: &KeepRule, groups: &[Vec<Fingerprint>]) -> io::Result<Option<Vec<usize>>> {
    if groups.is_empty() {
        return Ok(Some(vec![]));
    }

    let mut decisions: Vec<usize> = groups.iter().map(|g| choose_original(g, keep_rule)).collect();
    let mut skipped: Vec<bool> = vec![false; groups.len()];

    let mut current_group = 0usize;
    let mut selected_in_group = 0usize;

    enable_raw_mode()?;
    let mut out = stdout();
    out.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(3)].as_ref())
                .split(f.area());

            let group = &groups[current_group];
            let items: Vec<ListItem> = group
                .iter()
                .enumerate()
                .map(|(i, fp)| {
                    let is_keep = i == decisions[current_group];
                    let name = fp.path.file_name().unwrap_or_default().to_string_lossy();
                    let prefix = if is_keep { "●" } else { "○" };
                    let line = Line::from(vec![
                        Span::raw(format!("{prefix} ")),
                        Span::raw(name.to_string()),
                    ]);
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "Grupo {}/{} · {} copias",
                    current_group + 1,
                    groups.len(),
                    group.len()
                )))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

            f.render_widget(list, chunks[0]);

            let help = Paragraph::new("↑/↓ archivo  ←/→ grupo  Enter cambiar original  s omitir  a aprobar todo  q salir")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(help, chunks[1]);
        })?;

        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Up => {
                    selected_in_group = selected_in_group.saturating_sub(1);
                }
                KeyCode::Down => {
                    let max = groups[current_group].len().saturating_sub(1);
                    selected_in_group = selected_in_group.saturating_add(1).min(max);
                }
                KeyCode::Left => {
                    if current_group > 0 {
                        current_group -= 1;
                        selected_in_group = 0;
                    }
                }
                KeyCode::Right => {
                    if current_group + 1 < groups.len() {
                        current_group += 1;
                        selected_in_group = 0;
                    } else {
                        break;
                    }
                }
                KeyCode::Enter => {
                    decisions[current_group] = selected_in_group;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    skipped[current_group] = true;
                    if current_group + 1 < groups.len() {
                        current_group += 1;
                    } else {
                        break;
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    break;
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    disable_raw_mode()?;
                    std::io::stdout().execute(LeaveAlternateScreen)?;
                    return Ok(None);
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;

    let result = decisions
        .into_iter()
        .zip(skipped.into_iter())
        .map(|(d, skip)| if skip { usize::MAX } else { d })
        .collect();

    Ok(Some(result))
}
