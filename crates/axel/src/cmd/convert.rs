//! Conversions between the store's enums and the enums of the crates it must
//! not depend on (`axel_rex::osc`, `axel_config`).  Both sides are foreign to
//! this binary, so the orphan rules leave plain functions rather than `From`.

use axel_config::HarnessKind;
use axel_rex::osc;
use axel_store::{BlockedKind, Harness, StatusState};

/// The store state a program status maps to; `clear` reads as idle.
pub fn status_state(s: osc::State) -> StatusState {
    match s {
        osc::State::Idle | osc::State::Clear => StatusState::Idle,
        osc::State::Working => StatusState::Working,
        osc::State::Done => StatusState::Done,
        osc::State::Blocked => StatusState::Blocked,
        osc::State::Error => StatusState::Error,
    }
}

pub fn blocked_kind(k: osc::Kind) -> BlockedKind {
    match k {
        osc::Kind::Permission => BlockedKind::Permission,
        osc::Kind::Question => BlockedKind::Question,
        osc::Kind::Auth => BlockedKind::Auth,
    }
}

pub fn osc_kind(k: BlockedKind) -> osc::Kind {
    match k {
        BlockedKind::Permission => osc::Kind::Permission,
        BlockedKind::Question => osc::Kind::Question,
        BlockedKind::Auth => osc::Kind::Auth,
    }
}

/// The config-side kind of a stored harness; any custom command is `Custom`.
pub fn harness_kind(h: &Harness) -> HarnessKind {
    match h {
        Harness::Claude => HarnessKind::Claude,
        Harness::Codex => HarnessKind::Codex,
        Harness::Opencode => HarnessKind::Opencode,
        Harness::Custom(_) => HarnessKind::Custom,
    }
}
