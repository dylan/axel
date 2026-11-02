//! `axel mcp` — the MCP server on stdio.  Inside a session (`$AXEL_AGENT`
//! set) it acts as that agent: `send` and `set_status` run the same code as
//! `axel send` and `axel status`, under the same policy.

use std::sync::Arc;

use anyhow::Result;
use axel_mcp::{AxelContext, BlockedKind, CallScope, InboxMessage, LocalContext, SendReport, StatusState};

use super::Ctx;

struct CliContext;

impl AxelContext for CliContext {
    fn scope(&self) -> Result<CallScope, axel_mcp::ErrorData> {
        LocalContext.scope()
    }

    fn send(&self, to: &[String], text: &str) -> Result<SendReport, String> {
        let go = || -> Result<SendReport> {
            let ctx = Ctx::open()?;
            let (recipients, hosts) = super::inbox::resolve_names(&ctx, to)?;
            super::inbox::send_message(&ctx, &recipients, &hosts, text)
        };
        go().map_err(|e| format!("{e:#}"))
    }

    fn set_status(&self, state: StatusState, msg: Option<&str>, kind: Option<BlockedKind>) -> Result<(), String> {
        super::status::run(state.as_str(), msg, kind.map(BlockedKind::as_str)).map_err(|e| format!("{e:#}"))
    }

    fn inbox(&self, limit: usize) -> Result<Vec<InboxMessage>, String> {
        let go = || -> Result<Vec<InboxMessage>> { super::inbox::inbox_messages(&Ctx::open()?, limit) };
        go().map_err(|e| format!("{e:#}"))
    }
}

pub fn run() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(axel_mcp::run_server_with(Arc::new(CliContext))).map_err(|e| anyhow::anyhow!("{e}"))
}
