//! Row and value types shared across crate boundaries (see docs/CONTRACTS.md).
//!
//! Pubkeys are base58 strings; timestamps are Unix seconds.

use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Error;

macro_rules! str_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident => $s:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($(#[$vmeta])* $variant),+ }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The snake_case name stored in the database and in envelopes.
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $s),+ }
            }
        }

        impl FromStr for $name {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self, Error> {
                match s {
                    $($s => Ok($name::$variant),)+
                    other => Err(Error::Corrupt(format!(concat!("unknown ", stringify!($name), " {:?}"), other))),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.pad(self.as_str())
            }
        }
    };
}

/// Lenient serde helpers for enum fields read back from the log: text this
/// build does not know maps to the enum's default (or `None`) instead of
/// rejecting the whole envelope.
pub(crate) mod lenient {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer};

    pub fn or_default<'de, D, T>(d: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr + Default,
    {
        let s = String::deserialize(d)?;
        Ok(s.parse().unwrap_or_default())
    }

    pub fn or_none<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
    {
        let s = Option::<String>::deserialize(d)?;
        Ok(s.and_then(|s| s.parse().ok()))
    }
}

str_enum! {
    /// Coarse thread state, set by a human or by policy.  Live state is derived
    /// from the thread's sessions instead.  `Done` is "settled": finished but
    /// still listed.  `Deleted` is a tombstone: the log is append-only, so a
    /// deleted thread stays in it but is hidden everywhere.
    ThreadState { Open => "open", Done => "done", Archived => "archived", Deleted => "deleted" }
}

str_enum! {
    /// Lifecycle of one agent instance on a thread.
    SessionState { Starting => "starting", Running => "running", Blocked => "blocked", Idle => "idle", Ended => "ended" }
}

str_enum! {
    /// How a thread works on a project: in its directory, in a dedicated
    /// worktree, or in a scratch directory with no project at all.
    WorkspaceKind { InPlace => "in_place", Worktree => "worktree", Scratch => "scratch" }
}

str_enum! {
    /// What a participant may do on a thread.  Owners add participants.
    ParticipantRole { Owner => "owner", Member => "member", Agent => "agent" }
}

str_enum! {
    /// How a session is run: the native CLI in a Rex pane, or driven by Axel
    /// over ACP with the conversation owned by Axel.
    #[derive(Default)]
    SessionRuntime { #[default] Pane => "pane", Managed => "managed" }
}

str_enum! {
    /// What a conversation item is.
    ItemKind {
        UserMessage => "user_message",
        AssistantMessage => "assistant_message",
        Thought => "thought",
        ToolCall => "tool_call",
        Permission => "permission",
        Question => "question",
        Answer => "answer",
        Error => "error",
        TurnEnd => "turn_end",
    }
}

str_enum! {
    /// Where a conversation item stands.
    ItemStatus {
        Pending => "pending",
        Running => "running",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
        Allowed => "allowed",
        Denied => "denied",
        Answered => "answered",
    }
}

str_enum! {
    /// How much an agent may do unattended.  `autopilot` (the default) never
    /// asks; `edits` auto-accepts file edits; `supervised` asks for everything;
    /// `plan` is read-only.
    #[derive(Default)]
    RuntimeMode { #[default] Autopilot => "autopilot", Edits => "edits", Supervised => "supervised", Plan => "plan" }
}

str_enum! {
    /// What a session says it is doing (the OSC 7501 states minus `clear`).
    #[derive(Default)]
    StatusState { #[default] Idle => "idle", Working => "working", Done => "done", Blocked => "blocked", Error => "error" }
}

impl StatusState {
    /// Rank for "most urgent": blocked > error > working > done > idle.
    pub fn urgency(self) -> u8 {
        match self {
            StatusState::Blocked => 5,
            StatusState::Error => 4,
            StatusState::Working => 3,
            StatusState::Done => 2,
            StatusState::Idle => 1,
        }
    }
}

str_enum! {
    /// What a `blocked` session is waiting for.
    BlockedKind { Permission => "permission", Question => "question", Auth => "auth" }
}

str_enum! {
    /// How a child thread relates to its parent.
    Relationship { Fork => "fork", Subagent => "subagent", Delegation => "delegation" }
}

str_enum! {
    /// The agent colours the UI knows how to draw, in the new-agent form's cycle order.
    AgentColor { Purple => "purple", Yellow => "yellow", Red => "red", Green => "green", Blue => "blue", Gray => "gray", Orange => "orange" }
}

/// The CLI an agent runs in: one Axel drives natively, or any other command.
///
/// Stored and serialised as plain text: the builtin name, or the custom
/// command itself.  Parsing never fails, so there is no `ALL`; see
/// [`Harness::BUILTIN`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Harness {
    #[default]
    Claude,
    Codex,
    Opencode,
    /// Any other program, by command.
    Custom(String),
}

impl Harness {
    /// The harnesses Axel knows how to drive natively, in display order.
    pub const BUILTIN: [Harness; 3] = [Harness::Claude, Harness::Codex, Harness::Opencode];

    /// The builtin name, or the custom command.
    pub fn as_str(&self) -> &str {
        match self {
            Harness::Claude => "claude",
            Harness::Codex => "codex",
            Harness::Opencode => "opencode",
            Harness::Custom(cmd) => cmd,
        }
    }

    /// A builtin by name (case-insensitive), anything else as [`Harness::Custom`].
    pub fn parse(s: &str) -> Harness {
        let t = s.trim();
        match t.to_ascii_lowercase().as_str() {
            "claude" => Harness::Claude,
            "codex" => Harness::Codex,
            "opencode" => Harness::Opencode,
            _ => Harness::Custom(t.to_string()),
        }
    }

    pub fn is_builtin(&self) -> bool {
        !matches!(self, Harness::Custom(_))
    }
}

impl FromStr for Harness {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Harness::parse(s))
    }
}

impl std::fmt::Display for Harness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(self.as_str())
    }
}

impl Serialize for Harness {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Harness {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Harness::parse(&String::deserialize(d)?))
    }
}

/// Outcome of [`crate::Store::append`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appended {
    /// False when the envelope was already in the log.
    pub inserted: bool,
    /// True when the envelope is reflected in the projections.
    pub projected: bool,
    /// Why it is not projected, when it is not.
    pub rejected_reason: Option<String>,
}

/// A machine running Axel.  Exactly one row per database is flagged local.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Environment {
    pub id: String,
    pub label: String,
    pub rex_host_id: Option<String>,
    pub is_local: bool,
    pub created_at: i64,
    /// For a remote host: how to reach it (`ssh` destination, e.g. `mini`).
    #[serde(default)]
    pub ssh_target: Option<String>,
    /// For a remote host: its Axel owner key, which agents delegate to.
    #[serde(default)]
    pub host_pubkey: Option<String>,
}

/// A directory on an environment, usually a git root.  Local-only.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub environment_id: String,
    pub path: PathBuf,
    pub name: String,
    /// Identifies the repository across environments (e.g. a normalised remote URL).
    pub repo_key: Option<String>,
    pub has_axel_md: bool,
    pub last_opened_at: i64,
}

/// A named identity plus its harness profile.  Local-only.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub pubkey: String,
    pub name: String,
    pub harness: Harness,
    pub model: Option<String>,
    pub skills: Vec<String>,
    pub prompt: Option<String>,
    pub color: Option<AgentColor>,
    pub runtime_mode: RuntimeMode,
    pub auth_tag: Option<Vec<String>>,
    pub created_at: i64,
    pub last_seen_at: Option<i64>,
    pub archived_at: Option<i64>,
}

/// Input for [`crate::Store::insert_agent`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewAgent {
    pub pubkey: String,
    pub name: String,
    pub harness: Harness,
    pub model: Option<String>,
    pub skills: Vec<String>,
    pub prompt: Option<String>,
    pub color: Option<AgentColor>,
    pub runtime_mode: RuntimeMode,
    pub auth_tag: Option<Vec<String>>,
}

/// A thread: an intent with a title, projected from its event log.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Thread {
    /// The id of the `thread.create` envelope.
    pub id: String,
    pub title: String,
    pub intent: String,
    pub state: ThreadState,
    /// Signer of the root envelope; the first owner.
    pub created_by: String,
    pub parent_thread_id: Option<String>,
    /// Set when `parent_thread_id` is (and its text is a relationship this build knows).
    pub relationship: Option<Relationship>,
    pub created_at: i64,
    pub updated_at: i64,
    pub done_at: Option<i64>,
}

/// How a thread works on one project.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadWorkspace {
    pub kind: WorkspaceKind,
    pub branch: Option<String>,
    pub worktree_path: Option<PathBuf>,
}

/// A project attached to a thread, as other replicas see it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadProject {
    pub thread_id: String,
    pub repo_key: Option<String>,
    pub path_hint: PathBuf,
    pub workspace: ThreadWorkspace,
    /// The matching `projects.id` on this replica (by `repo_key`, else by path), if any.
    pub local_project_id: Option<String>,
    pub added_at: i64,
}

/// Someone allowed to act on a thread.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Participant {
    pub thread_id: String,
    pub pubkey: String,
    pub role: ParticipantRole,
    pub added_by: String,
    pub added_at: i64,
}

/// One agent instance on a thread: agent × environment × harness process.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    /// ULID minted by the replica that started it.
    pub id: String,
    pub thread_id: String,
    pub agent_pubkey: String,
    pub environment_id: String,
    pub environment_label: String,
    pub project_repo_key: Option<String>,
    pub harness: Harness,
    pub runtime_mode: RuntimeMode,
    /// Claude `session_id` / Codex thread id, for `--resume`.
    pub harness_session_id: Option<String>,
    pub transcript_path: Option<String>,
    /// Local-only: how far into the transcript the usage reader has consumed.
    pub transcript_offset: i64,
    pub resumed_from: Option<String>,
    pub runtime: SessionRuntime,
    /// Local-only: the `axel session host` control socket, while it runs.
    pub socket_path: Option<PathBuf>,
    pub rex_session_id: Option<String>,
    pub rex_window_id: Option<String>,
    pub rex_block_id: Option<String>,
    pub state: SessionState,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub ended_reason: Option<String>,
}

/// Input for [`crate::Store::start_session`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewSession {
    pub thread_id: String,
    pub agent_pubkey: String,
    pub environment_id: String,
    pub environment_label: String,
    pub project_repo_key: Option<String>,
    pub harness: Harness,
    pub runtime_mode: RuntimeMode,
    pub resumed_from: Option<String>,
    pub runtime: SessionRuntime,
}

/// One entry of a managed session's conversation.  Local-only: never an envelope.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationItem {
    pub id: String,
    pub session_id: String,
    pub thread_id: String,
    /// Position within the session, from 1.
    pub seq: i64,
    pub turn: i64,
    /// Unix milliseconds.
    pub at: i64,
    pub kind: ItemKind,
    pub status: Option<ItemStatus>,
    /// ACP `toolCallId` / permission request id, for later updates.
    pub ref_id: Option<String>,
    pub content: serde_json::Value,
}

/// Input for [`crate::Store::push_item`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewItem {
    pub session_id: String,
    pub thread_id: String,
    pub turn: i64,
    pub kind: ItemKind,
    pub status: Option<ItemStatus>,
    pub ref_id: Option<String>,
    pub content: serde_json::Value,
}

/// What a completed turn looks like to other participants; the JSON content
/// of a kind-40 `turn` envelope.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnSummary {
    pub ordinal: i64,
    #[serde(default)]
    pub user_text: String,
    /// Head of the assistant's prose, at most 2000 characters.
    #[serde(default)]
    pub assistant_text: String,
    #[serde(default)]
    pub tool_calls: i64,
    #[serde(default)]
    pub files_edited: i64,
    #[serde(default)]
    pub duration_ms: i64,
    #[serde(default)]
    pub stop_reason: String,
}

/// A projected `turn` envelope.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Turn {
    /// The envelope id.
    pub id: String,
    pub thread_id: String,
    pub session_id: String,
    pub from_pubkey: String,
    pub created_at: u64,
    pub summary: TurnSummary,
}

/// Input for [`crate::Store::update_session`].  Every `None` leaves the field alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionUpdate {
    pub state: Option<SessionState>,
    pub harness_session_id: Option<String>,
    pub transcript_path: Option<String>,
    /// `(rex_session_id, rex_window_id, rex_block_id)`; sets all three.
    pub rex: Option<(Option<String>, Option<String>, Option<String>)>,
    /// The mode was switched mid-session (Shift+Tab in the thread view).
    pub runtime_mode: Option<RuntimeMode>,
}

/// Input for [`crate::Store::record_status`]; also the JSON content of a kind-20 envelope.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusReport {
    /// Unknown text in an old envelope reads as [`StatusState::Idle`].
    #[serde(default, deserialize_with = "lenient::or_default")]
    pub state: StatusState,
    /// Unknown text in an old envelope reads as `None`.
    #[serde(
        default,
        deserialize_with = "lenient::or_none",
        skip_serializing_if = "Option::is_none"
    )]
    pub kind: Option<BlockedKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
}

/// Live status of a session.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Status {
    pub session_id: String,
    pub thread_id: String,
    pub state: StatusState,
    pub kind: Option<BlockedKind>,
    pub title: Option<String>,
    pub msg: Option<String>,
    pub progress: Option<u8>,
    pub updated_at: i64,
}

impl Status {
    /// Rank for "most urgent": blocked > error > working > done > idle.
    pub fn urgency(&self) -> u8 {
        self.state.urgency()
    }
}

/// Accumulated token usage of one session (or, from
/// [`crate::Store::thread_usage`], of a whole thread with `session_id = ""`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Usage {
    pub session_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub turns: i64,
    pub cost_usd_est: f64,
    pub updated_at: i64,
}

/// What one [`crate::Store::add_usage`] call adds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageDelta {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub turns: i64,
    pub cost_usd_est: f64,
}

/// One usage envelope, kept for 24 hours.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageSample {
    pub session_id: String,
    pub at: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Everything the dashboard shows about one thread.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ThreadSummary {
    pub thread: Thread,
    pub projects: Vec<ThreadProject>,
    pub participants: Vec<Participant>,
    pub sessions_total: i64,
    /// Sessions not yet ended, newest first.
    pub sessions_live: Vec<Session>,
    /// Distinct agent pubkeys across all sessions, sorted.
    pub agents: Vec<String>,
    /// Distinct environment labels across all sessions, sorted.
    pub environments: Vec<String>,
    /// Sum over all sessions (`session_id = ""`).
    pub usage: Usage,
    /// Most urgent status among live sessions: blocked > error > working > done > idle.
    pub live: Option<Status>,
}

/// Filter for [`crate::Store::list_threads`].  Every `None` means "any".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadFilter {
    pub states: Option<Vec<ThreadState>>,
    /// Threads with at least one session by this agent.
    pub agent_pubkey: Option<String>,
    /// Threads this pubkey participates in.
    pub participant: Option<String>,
    /// Threads with this repository attached.
    pub repo_key: Option<String>,
    pub limit: Option<usize>,
}

/// Filter for [`crate::Store::list_sessions`].  Every `None` means "any".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionFilter {
    pub thread_id: Option<String>,
    pub agent_pubkey: Option<String>,
    pub environment_id: Option<String>,
    pub states: Option<Vec<SessionState>>,
    pub limit: Option<usize>,
}
