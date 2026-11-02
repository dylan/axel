//! axel-config: `AXEL.md` frontmatter + body, agent profiles, skill discovery, harness
//! skill drivers, launch specs and git helpers. See `docs/DESIGN.md` and
//! `docs/CONTRACTS.md`.
//!
//! ```
//! use axel_config::{WorkspaceConfig, HarnessKind, launch};
//!
//! let text = WorkspaceConfig::template("demo");
//! let config = WorkspaceConfig::parse(&text).unwrap();
//! let claude = config.agent("claude").unwrap();
//! assert_eq!(claude.harness.kind, HarnessKind::Claude);
//!
//! let spec = launch::build(claude, Some("Add a README"));
//! assert_eq!(spec.program, "claude");
//! assert_eq!(spec.args.last().map(String::as_str), Some("Add a README"));
//! ```

mod config;
pub mod drivers;
mod error;
pub mod git;
pub mod launch;
mod skills;

pub use config::{
    AXEL_MD, AgentProfile, ApiConfig, Harness, HarnessKind, SkillSource, WorkspaceConfig,
    expand_tilde, is_base58_pubkey, split_frontmatter,
};
pub use error::Error;
pub use indexmap::IndexMap;
pub use skills::{
    SKILL_FILE, Skill, default_global_skills_dir, find_all_skills, missing_skills, resolve_skills,
};
