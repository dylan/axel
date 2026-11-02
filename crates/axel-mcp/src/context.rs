//! Who a tool call acts for.
//!
//! Each tool receives a [`CallScope`] — the store and keystore it needs.
//! [`AxelContext`] turns a request into a scope; [`LocalContext`] opens the
//! default database and keychain, the same path the CLI and TUI take.

use std::sync::Arc;

use axel_identity::Keystore;
use rmcp::schemars;
use axel_store::Store;

/// Resolves the caller of a tool, and carries out the actions that need the
/// CLI's machinery (delivery into live sessions, summoning).  The `axel`
/// binary implements those with the same code as `axel send` and
/// `axel status`, so a tool call and a command behave identically.
pub trait AxelContext: Send + Sync {
    /// Everything a tool call needs. Errors when the environment is not
    /// usable (missing database, broken keychain, etc.).
    fn scope(&self) -> Result<CallScope, rmcp::ErrorData>;

    /// The agent this server acts for (`$AXEL_AGENT` of the session that
    /// started it), or `None` when the owner runs it.
    fn caller_agent(&self) -> Option<String> {
        std::env::var("AXEL_AGENT").ok().filter(|s| !s.trim().is_empty())
    }

    /// Send `text` to the agents named in `to`, as `axel send` does.
    fn send(&self, _to: &[String], _text: &str) -> Result<SendReport, String> {
        Err("sending is not available in this server".into())
    }

    /// Report this session's status, as `axel status` does.
    fn set_status(&self, _state: StatusState, _msg: Option<&str>, _kind: Option<BlockedKind>) -> Result<(), String> {
        Err("status is not available in this server".into())
    }

    /// The newest messages addressed to the caller (agent or owner).
    fn inbox(&self, _limit: usize) -> Result<Vec<InboxMessage>, String> {
        Err("the inbox is not available in this server".into())
    }
}

/// What `send` did: the signed message, then one outcome per recipient.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SendReport {
    /// Id of the signed message (it is in every recipient's inbox).
    pub message_id: String,
    /// The thread it was sent on, if any.
    pub thread: Option<String>,
    pub deliveries: Vec<Delivery>,
}

/// One recipient's outcome.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Delivery {
    pub to: String,
    #[serde(flatten)]
    pub outcome: Outcome,
}

/// How a message reached (or didn't reach) one recipient.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// Became the next prompt of its live session.
    Delivered,
    /// Its session is mid-turn; the message waits in its queue.
    Queued,
    /// It wasn't on the thread; a new session started with the message.
    Summoned { session: String },
    /// It runs in a terminal pane; the message waits in its inbox.
    InPane,
    /// No thread to deliver on; the message waits in its inbox.
    InboxOnly,
    /// It lives on the owner's machine and this is a remote host: the
    /// message goes there with the next sync, and is delivered then.
    AwaitingSync,
    /// Delivery failed; the message is still in its inbox.
    Failed { error: String },
}

impl Outcome {
    /// One line for people.
    pub fn describe(&self) -> String {
        match self {
            Outcome::Delivered => "delivered to its session".into(),
            Outcome::Queued => "busy; queued for its next turn".into(),
            Outcome::Summoned { session } => format!("summoned onto the thread (session {})", &session[..12.min(session.len())]),
            Outcome::InPane => "runs in a pane; left in its inbox".into(),
            Outcome::InboxOnly => "not on a thread; left in its inbox".into(),
            Outcome::AwaitingSync => "lives on the owner's machine; delivered when it syncs (within seconds while it's awake)".into(),
            Outcome::Failed { error } => format!("not delivered: {error}"),
        }
    }
}

/// The states a session can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusState {
    Working,
    Blocked,
    Done,
    Idle,
    Error,
}

impl StatusState {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusState::Working => "working",
            StatusState::Blocked => "blocked",
            StatusState::Done => "done",
            StatusState::Idle => "idle",
            StatusState::Error => "error",
        }
    }
}

/// What a `blocked` session waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockedKind {
    Permission,
    Question,
    Auth,
}

impl BlockedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BlockedKind::Permission => "permission",
            BlockedKind::Question => "question",
            BlockedKind::Auth => "auth",
        }
    }
}

/// One inbox message.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InboxMessage {
    pub id: String,
    /// Sender's name ("owner" for the human), else a pubkey prefix.
    pub from: String,
    /// Unix seconds.
    pub at: u64,
    pub thread: Option<String>,
    pub text: String,
}

/// One call's view of the world.
pub struct CallScope {
    /// The event log and projections.
    pub store: Store,
    /// Agent keys and the owner key.
    pub keystore: Keystore,
}

/// `axel mcp` on the user's machine: opens the default database and
/// keychain, exactly like `Ctx::open()` in the CLI.
pub struct LocalContext;

impl AxelContext for LocalContext {
    fn scope(&self) -> Result<CallScope, rmcp::ErrorData> {
        let store = Store::open_default().map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to open store: {e}"), None)
        })?;
        let keystore = Keystore::open().map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to open keystore: {e}"), None)
        })?;
        Ok(CallScope { store, keystore })
    }
}

/// Shared context held by the MCP server. Created once, used for every call.
pub(crate) struct SharedContext {
    inner: Arc<dyn AxelContext>,
}

impl SharedContext {
    pub fn new(ctx: Arc<dyn AxelContext>) -> Self {
        Self { inner: ctx }
    }

    pub fn scope(&self) -> Result<CallScope, rmcp::ErrorData> {
        self.inner.scope()
    }

    pub fn get(&self) -> &dyn AxelContext {
        self.inner.as_ref()
    }
}
