//! The thread as a plain terminal stream (the top pane of a split thread).
//!
//! No alternate screen, no raw mode, no mouse capture: finished rows are
//! printed once and become ordinary scrollback, so the terminal scrolls,
//! searches and selects them itself.  Only the **live tail** — rows still
//! streaming or running, a pending approval, the "working" line — is redrawn
//! in place at the bottom (cursor up + clear), the way inline CLIs do it.

use std::collections::HashSet;
use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::glue;
use crate::model::{ItemBody, ItemKind, ItemStatus, ThreadItem};
use crate::theme;
use crate::thread::{Row, ThreadView};
use crate::util::now_secs;
use crate::view::thread_view::row_lines;

const TICK: Duration = Duration::from_millis(250);
const SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];

/// Follow `thread_id` until the process is killed (the pane closes with its window).
pub fn run_stream(store: &axel_store::Store, owner_pk: &str, thread_id: &str) -> Result<()> {
    let mut out = std::io::stdout();
    let data = glue::load_thread(store, owner_pk, thread_id, now_secs())?;
    let mut view = ThreadView::new(thread_id, data);
    let mut s = Stream::default();
    write!(out, "\x1b[?25l")?; // the cursor would blink under the live tail
    let header = header_line(&view);
    writeln!(out, "{}\r", to_ansi(&header))?;
    loop {
        let (w, h) = crossterm::terminal::size().unwrap_or((100, 40));
        let frame = s.frame(&view, w as usize, h as usize);
        if !frame.is_empty() {
            out.write_all(frame.as_bytes())?;
            out.flush()?;
        }
        std::thread::sleep(TICK);
        s.tick += 1;
        if let Ok(d) = glue::load_thread(store, owner_pk, thread_id, now_secs()) {
            view.set_data(d);
        }
    }
}

#[derive(Default)]
struct Stream {
    /// Row keys already printed to scrollback.
    committed: HashSet<String>,
    /// Lines the live tail occupies right now (to erase before redrawing).
    live_lines: usize,
    /// The live tail as last drawn, to skip identical redraws.
    last_live: Vec<String>,
    tick: usize,
}

impl Stream {
    /// The bytes to write this tick: erase the old tail, print newly finished
    /// rows, draw the new tail.  Empty when nothing changed.
    fn frame(&mut self, v: &ThreadView, width: usize, height: usize) -> String {
        let width = width.saturating_sub(1).max(20);
        let rows = v.rows();
        let items = &v.data.items;
        let working = v.is_working();
        let mut commit: Vec<String> = Vec::new();
        let mut live_rows: Vec<&Row> = Vec::new();
        let mut blocked = false;
        for (i, row) in rows.iter().enumerate() {
            let key = row.key(items);
            if self.committed.contains(&key) {
                continue;
            }
            // Commit in order: once one row is still moving, the rest wait.
            if !blocked && is_settled(row, items, i + 1 == rows.len(), working) {
                commit.extend(render_row(v, row, width).iter().map(to_ansi));
                self.committed.insert(key);
            } else {
                blocked = true;
                live_rows.push(row);
            }
        }
        let mut live: Vec<String> = live_rows.iter().flat_map(|r| render_row(v, r, width)).map(|l| to_ansi(&l)).collect();
        if working {
            let who = v.data.agents.iter().find(|a| a.2 == crate::model::StatusState::Working).map(|a| a.0.clone()).unwrap_or_else(|| "agent".into());
            let frame = SPINNER[self.tick % SPINNER.len()];
            live.push(to_ansi(&Line::from(vec![
                Span::styled(format!("{frame} "), theme::accent()),
                Span::styled(format!("{who} is working…"), theme::dim()),
            ])));
        }
        // Never draw a tail taller than the pane: cursor-up can't go past the top.
        let cap = height.saturating_sub(2).max(1);
        if live.len() > cap {
            if cap == 1 {
                live.drain(..live.len() - 1);
            } else {
                let hidden = live.len() - (cap - 1);
                live.drain(..hidden);
                live.insert(0, to_ansi(&Line::from(Span::styled(format!("  … {hidden} more lines above"), theme::dim()))));
            }
        }
        if commit.is_empty() && live == self.last_live {
            return String::new();
        }
        let mut buf = String::new();
        if self.live_lines > 0 {
            // To the first line of the old tail, then clear to the end of the screen.
            buf.push_str(&format!("\x1b[{}F\x1b[J", self.live_lines));
        }
        for l in commit.iter().chain(live.iter()) {
            buf.push_str(l);
            buf.push_str("\r\n");
        }
        self.live_lines = live.len();
        self.last_live = live;
        buf
    }
}

/// A row can go to scrollback once nothing in it will change: not the last row
/// while an agent works, no running tool or streaming text, no open approval.
fn is_settled(row: &Row, items: &[ThreadItem], last: bool, working: bool) -> bool {
    if last && working {
        return false;
    }
    let moving = |it: &ThreadItem| matches!(it.status, Some(ItemStatus::Running | ItemStatus::Pending));
    match row {
        Row::Prose(i) | Row::Thought(i) | Row::Choice(i) => !moving(&items[*i]),
        Row::Tool { idx, .. } => !moving(&items[*idx]),
        Row::Fold(v) => !v.iter().any(|i| moving(&items[*i])),
    }
}

/// Lines for one row.  A pending approval gets a stream-specific card (it is
/// answered in the composer pane below).
fn render_row(v: &ThreadView, row: &Row, width: usize) -> Vec<Line<'static>> {
    if let Row::Choice(i) = row {
        let it = &v.data.items[*i];
        if it.status == Some(ItemStatus::Pending)
            && let ItemBody::Choice { prompt, options, .. } = &it.body
        {
            let who = it.agent.as_ref().map(|(n, _)| n.clone()).unwrap_or_else(|| "agent".into());
            let (glyph, verb, color) = match it.kind {
                ItemKind::Permission => ("!", "asks permission", theme::DANGER),
                _ => ("?", "asks", Color::Yellow),
            };
            let opts: Vec<String> = options.iter().enumerate().map(|(n, o)| format!("{} {}", n + 1, o.label)).collect();
            return vec![
                Line::from(vec![
                    Span::styled(format!("{glyph} "), Style::default().fg(color).bold()),
                    Span::styled(format!("{who} {verb}: "), theme::bold()),
                    Span::raw(crate::util::ellipsize(prompt, width.saturating_sub(who.len() + verb.len() + 6))),
                ]),
                Line::from(Span::styled(format!("    {} · answer below", opts.join("  ")), theme::dim())),
            ];
        }
    }
    let mut lines = row_lines(v, row, width.saturating_sub(1), false);
    // Same left margin as the TUI's cursor column.
    for l in &mut lines {
        l.spans.insert(0, Span::raw(" "));
    }
    lines
}

fn header_line(v: &ThreadView) -> Line<'static> {
    let mut spans = vec![Span::styled(v.data.title.clone(), theme::accent_bold())];
    for (name, color, _) in &v.data.agents {
        spans.push(Span::styled("  ·  ", theme::dim()));
        spans.push(Span::styled(name.clone(), Style::default().fg(theme::agent_color(*color)).bold()));
    }
    Line::from(spans)
}

/// A styled line as ANSI text (SGR per span, reset at the end).
pub fn to_ansi(line: &Line) -> String {
    let mut s = String::new();
    for span in &line.spans {
        let st = line.style.patch(span.style);
        let sgr = sgr(st);
        if sgr.is_empty() {
            s.push_str(&span.content);
        } else {
            s.push_str(&format!("\x1b[{sgr}m{}\x1b[0m", span.content));
        }
    }
    s
}

fn sgr(st: Style) -> String {
    let mut codes: Vec<String> = Vec::new();
    let m = st.add_modifier;
    for (flag, code) in [
        (Modifier::BOLD, "1"),
        (Modifier::DIM, "2"),
        (Modifier::ITALIC, "3"),
        (Modifier::UNDERLINED, "4"),
        (Modifier::REVERSED, "7"),
        (Modifier::CROSSED_OUT, "9"),
    ] {
        if m.contains(flag) {
            codes.push(code.into());
        }
    }
    if let Some(c) = st.fg.and_then(|c| color(c, false)) {
        codes.push(c);
    }
    if let Some(c) = st.bg.and_then(|c| color(c, true)) {
        codes.push(c);
    }
    codes.join(";")
}

fn color(c: Color, bg: bool) -> Option<String> {
    let base = |n: u8| Some((if bg { n + 10 } else { n }).to_string());
    match c {
        Color::Reset => None,
        Color::Black => base(30),
        Color::Red => base(31),
        Color::Green => base(32),
        Color::Yellow => base(33),
        Color::Blue => base(34),
        Color::Magenta => base(35),
        Color::Cyan => base(36),
        Color::Gray => base(37),
        Color::DarkGray => base(90),
        Color::LightRed => base(91),
        Color::LightGreen => base(92),
        Color::LightYellow => base(93),
        Color::LightBlue => base(94),
        Color::LightMagenta => base(95),
        Color::LightCyan => base(96),
        Color::White => base(97),
        Color::Indexed(n) => Some(format!("{};5;{n}", if bg { 48 } else { 38 })),
        Color::Rgb(r, g, b) => Some(format!("{};2;{r};{g};{b}", if bg { 48 } else { 38 })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thread::fixtures;

    fn plain(s: &str) -> String {
        // Strip SGR sequences for assertions.
        let mut out = String::new();
        let mut it = s.chars().peekable();
        while let Some(c) = it.next() {
            if c == '\x1b' {
                for d in it.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn ansi_styles_spans() {
        let l = Line::from(vec![Span::raw("a "), Span::styled("b", Style::default().fg(Color::Indexed(179)).bold())]);
        assert_eq!(to_ansi(&l), "a \x1b[1;38;5;179mb\x1b[0m");
    }

    #[test]
    fn settled_rows_commit_once_and_the_tail_redraws() {
        // Fixture: user, assistant, fold, pending permission (agent blocked, not working).
        let mut v = ThreadView::new("T1", fixtures::data());
        let mut s = Stream::default();
        let f1 = plain(&s.frame(&v, 80, 40));
        assert!(f1.contains("Fix the login bug") && f1.contains("5 tool calls"), "{f1}");
        assert!(f1.contains("Atlas asks permission: Run cargo test"), "pending card in the tail: {f1}");
        assert!(f1.contains("1 Allow once  2 Always allow  3 Reject · answer below"), "{f1}");
        assert_eq!(s.committed.len(), 3, "user, assistant, fold are settled");
        assert_eq!(s.live_lines, 2);
        // Nothing changed: nothing written.
        assert_eq!(s.frame(&v, 80, 40), "");
        // Answered: the old tail is erased and the card is committed in its resolved form.
        let mut d = fixtures::data();
        d.items[7].status = Some(ItemStatus::Allowed);
        v.set_data(d.clone());
        let raw = s.frame(&v, 80, 40);
        assert!(raw.starts_with("\x1b[2F\x1b[J"), "erases the 2-line tail first: {raw:?}");
        assert!(plain(&raw).contains("allowed"), "{raw}");
        assert!(!plain(&raw).contains("Fix the login bug"), "settled rows are never printed twice");
        assert_eq!(s.live_lines, 0);
        // While the agent works, the last row and a spinner stay live.
        d.agents[0].2 = crate::model::StatusState::Working;
        let mut it = fixtures::item("a2", ItemKind::AssistantMessage, ItemBody::Text("Running the".into()), 9_000);
        it.status = Some(ItemStatus::Running);
        d.items.push(it);
        v.set_data(d);
        let f3 = plain(&s.frame(&v, 80, 40));
        assert!(f3.contains("Running the") && f3.contains("Atlas is working…"), "{f3}");
        assert_eq!(s.live_lines, 3, "header, text, spinner");
        // A tail taller than the pane is cut from the top, keeping the newest line.
        let f4 = plain(&s.frame(&v, 80, 3));
        assert_eq!(s.live_lines, 1);
        assert!(f4.contains("Atlas is working…"), "{f4}");
    }
}
