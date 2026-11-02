//! `axel status STATE [MSG]` — for harnesses and scripts without hooks.

use anyhow::{Context, Result};
use axel_rex::osc;
use axel_store::{BlockedKind, SessionState, SessionUpdate, StatusReport};

use super::{Ctx, convert};

/// `axel status` and the MCP `set_status` tool.
pub fn run(state: &str, msg: Option<&str>, kind: Option<&str>) -> Result<()> {
    // `clear` is an OSC-only state (it removes the tab indicator) and records as idle.
    let osc_state = osc::State::parse(state)
        .with_context(|| format!("unknown state {state:?}; use idle|working|done|blocked|error|clear"))?;
    let kind: Option<BlockedKind> = kind
        .map(|k| k.parse().map_err(|_| anyhow::anyhow!("unknown kind {k:?}; use permission|question|auth")))
        .transpose()?;
    let app = std::env::var("AXEL_AGENT_NAME").ok();
    let _ = osc::emit_to_tty(&osc::Report {
        state: osc_state,
        id: None,
        kind: kind.map(convert::osc_kind),
        progress: None,
        app: app.as_deref(),
        title: None,
        msg,
    });

    if let (Ok(session_id), Ok(agent_pk)) = (std::env::var("AXEL_SESSION"), std::env::var("AXEL_AGENT")) {
        let ctx = Ctx::open()?;
        let Some(session) = ctx.store.get_session(&session_id)? else { return Ok(()) };
        let Ok(kp) = ctx.agent_signer(&agent_pk, Some(&session.thread_id)) else { return Ok(()) };
        ctx.store.record_status(
            &kp,
            &session_id,
            &StatusReport { state: convert::status_state(osc_state), kind, title: None, msg: msg.map(String::from), progress: None },
        )?;
        let ss = match osc_state {
            osc::State::Working => Some(SessionState::Running),
            osc::State::Blocked => Some(SessionState::Blocked),
            osc::State::Done | osc::State::Idle | osc::State::Clear => Some(SessionState::Idle),
            osc::State::Error => None,
        };
        if let Some(ss) = ss {
            ctx.store.update_session(&kp, &session_id, &SessionUpdate { state: Some(ss), ..Default::default() })?;
        }
    }
    Ok(())
}
