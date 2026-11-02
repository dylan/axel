//! The composer pane of a split thread: prompt, queue, pending approval and
//! mode footer, with the timeline streaming natively in the pane above
//! ([`crate::stream`]).  Same state machine as the full thread view
//! ([`ThreadView`] with `compose_only`), so sending, queueing, steering,
//! history and Shift+Tab modes behave identically.

use std::io::stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind};
use crossterm::execute;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::Deps;
use crate::glue;
use crate::thread::{Row, ThreadAction, ThreadView};
use crate::util::now_secs;
use crate::view::thread_view::{composer_height, queue_height, render_composer, render_queue, row_lines};

const POLL: Duration = Duration::from_millis(60);
const REFRESH: Duration = Duration::from_millis(300);
const RETRY_FIT: Duration = Duration::from_millis(400);

/// Run the composer for `thread_id` until Esc (empty composer) or Ctrl+C.
///
/// `fit(rows_wanted, rows_now)` is called at start, on resize, and whenever
/// the pane needs a different height (an approval card appears, the queue
/// grows), so the host can move the split border; Rex sizes splits by ratio.
pub fn run_compose(deps: Deps, thread_id: &str, fit: &dyn Fn(u16, u16)) -> Result<()> {
    let owner_pk = glue::owner_pubkey(deps.keystore)?;
    let data = glue::load_thread(deps.store, &owner_pk, thread_id, now_secs())?;
    let mut v = ThreadView::new(thread_id, data);
    v.compose_only = true;

    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableBracketedPaste);
    let result = (|| -> Result<()> {
        let mut last = Instant::now();
        // Fit the pane when the wanted height changes, or when the pane is
        // short or more than a row tall.  A resize can land a row short (Rex
        // splits by ratio and rounds), so each retry asks for one row more; a
        // single spare row is left alone so the split doesn't flip-flop.
        let mut fitted_for: Option<u16> = None;
        let mut retries = 0u16;
        let mut fitted_at = Instant::now().checked_sub(RETRY_FIT).unwrap_or_else(Instant::now);
        loop {
            let size = terminal.size()?;
            let want = desired_height(&v, size.width);
            if fitted_for != Some(want) {
                retries = 0;
            }
            let off = size.height < want || size.height > want + 1;
            if (fitted_for != Some(want) || off) && retries < 4 && fitted_at.elapsed() >= RETRY_FIT {
                let extra = if size.height < want { retries } else { 0 };
                fit(want + extra, size.height);
                retries = if fitted_for == Some(want) { retries + 1 } else { 1 };
                fitted_for = Some(want);
                fitted_at = Instant::now();
            }
            terminal.draw(|f| render(f, &v))?;
            if event::poll(POLL)? {
                match event::read()? {
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        match v.handle_compose_key(key) {
                            ThreadAction::None | ThreadAction::OpenLog(_) => {}
                            ThreadAction::Quit | ThreadAction::Back => return Ok(()),
                            ThreadAction::Send { session_id, cmd } => {
                                let reply = (deps.send)(&session_id, cmd).map_err(|e| format!("{e:#}"));
                                v.on_reply(reply);
                                last = Instant::now() - REFRESH;
                            }
                            ThreadAction::Summon { thread_id, agent, text, attachments } => {
                                let reply = (deps.summon)(&thread_id, &agent, &text, &attachments).map_err(|e| format!("{e:#}"));
                                v.on_reply(reply.map(|_| crate::thread::SendReply::Ok));
                                last = Instant::now() - REFRESH;
                            }
                        }
                    }
                    Event::Paste(text) => v.handle_paste(&text),
                    _ => {}
                }
            }
            if last.elapsed() >= REFRESH {
                if let Ok(d) = glue::load_thread(deps.store, &owner_pk, thread_id, now_secs()) {
                    v.set_data(d);
                }
                last = Instant::now();
            }
        }
    })();
    let _ = execute!(stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}

/// Rows the pane wants: composer, queue and (when pending) the approval card.
pub fn desired_height(v: &ThreadView, width: u16) -> u16 {
    // `composer_height` picks its text rows from the area height; ask as a tall pane.
    let composer = composer_height(v, Rect::new(0, 0, width, 40));
    let card = v.pending_choice().map_or(0, |i| row_lines(v, &Row::Choice(i), width.saturating_sub(1) as usize, true).len() as u16);
    composer + queue_height(v) + card
}

/// Bottom-anchored: [approval card] · [queue] · composer.
pub fn render(frame: &mut Frame, v: &ThreadView) {
    let area = frame.area();
    let composer_h = composer_height(v, area).min(area.height);
    let queue_h = queue_height(v).min(area.height.saturating_sub(composer_h));
    let card: Vec<Line> = match v.pending_choice() {
        Some(i) => row_lines(v, &Row::Choice(i), area.width.saturating_sub(1) as usize, true),
        None => Vec::new(),
    };
    let card_h = (card.len() as u16).min(area.height.saturating_sub(composer_h + queue_h));
    let [_, card_area, queue_area, composer_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(card_h),
        Constraint::Length(queue_h),
        Constraint::Length(composer_h),
    ])
    .areas(area);
    if card_h > 0 {
        // Keep the options (the end of the card) when the pane is short.
        let skip = card.len().saturating_sub(card_h as usize);
        let shown: Vec<Line> = card.into_iter().skip(skip).collect();
        frame.render_widget(Paragraph::new(shown), Rect { x: card_area.x + 1, width: card_area.width.saturating_sub(1), ..card_area });
    }
    if queue_h > 0 {
        render_queue(frame, v, queue_area);
    }
    render_composer(frame, v, composer_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thread::fixtures;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn text(term: &Terminal<TestBackend>) -> String {
        let buf = term.backend().buffer();
        (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn a_pane_shorter_than_the_composer_still_shows_the_input() {
        let mut d = fixtures::data();
        d.items[7].status = Some(crate::model::ItemStatus::Allowed);
        let mut v = ThreadView::new("T1", d);
        v.compose_only = true;
        for rows in 1..=3u16 {
            let mut term = Terminal::new(TestBackend::new(90, rows)).unwrap();
            term.draw(|f| render(f, &v)).unwrap();
            let t = text(&term);
            assert!(t.contains("message the thread"), "{rows} rows: {t}");
        }
        let mut term = Terminal::new(TestBackend::new(90, 3)).unwrap();
        term.draw(|f| render(f, &v)).unwrap();
        let t = text(&term);
        assert!(t.lines().next().unwrap().starts_with('─') && t.contains("autopilot"), "rule · input · footer: {t}");
    }

    #[test]
    fn approval_is_answered_from_the_composer_pane() {
        let mut v = ThreadView::new("T1", fixtures::data());
        v.compose_only = true;
        let mut term = Terminal::new(TestBackend::new(70, 12)).unwrap();
        term.draw(|f| render(f, &v)).unwrap();
        let t = text(&term);
        assert!(t.contains("Atlas asks permission: Run cargo test") && t.contains("▸ Allow once"), "{t}");
        assert!(t.contains("1-9") && t.contains("pick"), "{t}");
        assert!(!t.contains("PgUp") && !t.contains("timeline keys"), "no timeline here: {t}");
        // ↓ chooses, a digit answers directly.
        assert_eq!(v.handle_compose_key(key(KeyCode::Down)), ThreadAction::None);
        assert_eq!(v.choice_index("p1"), 1);
        match v.handle_compose_key(key(KeyCode::Char('3'))) {
            ThreadAction::Send { cmd: crate::thread::SendCommand::Answer { option_id, .. }, .. } => assert_eq!(option_id, "reject"),
            other => panic!("expected answer, got {other:?}"),
        }
        v.on_reply(Ok(crate::thread::SendReply::Ok));
        // Typing still declines and redirects; Tab never lands on a timeline.
        v.handle_compose_key(key(KeyCode::Tab));
        assert_eq!(v.focus, crate::thread::Focus::Composer);
        v.handle_compose_key(key(KeyCode::Char('n')));
        assert_eq!(v.composer.text(), "n");
        // Empty composer, no approval: ↑ is history.
        let mut d = fixtures::data();
        d.items[7].status = Some(crate::model::ItemStatus::Allowed);
        let mut v = ThreadView::new("T1", d);
        v.compose_only = true;
        v.handle_compose_key(key(KeyCode::Up));
        assert_eq!(v.composer.text(), "Fix the login bug");
    }
}
