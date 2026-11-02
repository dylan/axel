//! `AXEL.md`: YAML frontmatter (workspace, agents, skill directories) followed by a
//! markdown body that becomes the project context for agents.
//!
//! Ported from Axel v1 `config.rs` with one flat [`AgentProfile`] instead of the
//! per-harness enum, and without the tmux `layouts`/`grids` section (Rex owns layout).
//! Unknown top-level keys such as a legacy `layouts:` block are ignored.

use std::{
    collections::HashSet,
    convert::Infallible,
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeMap};

use crate::Error;

/// File name of the workspace manifest.
pub const AXEL_MD: &str = "AXEL.md";

// =============================================================================
// Harness
// =============================================================================

/// The program that runs an agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HarnessKind {
    Claude,
    Codex,
    Opencode,
    Custom,
}

impl HarnessKind {
    /// The harnesses Axel knows how to drive natively.
    pub const BUILTIN: [HarnessKind; 3] = [
        HarnessKind::Claude,
        HarnessKind::Codex,
        HarnessKind::Opencode,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            HarnessKind::Claude => "claude",
            HarnessKind::Codex => "codex",
            HarnessKind::Opencode => "opencode",
            HarnessKind::Custom => "custom",
        }
    }

    /// Map a harness name to a kind. Anything that is not a builtin is `Custom`.
    pub fn parse(name: &str) -> HarnessKind {
        match name.trim().to_ascii_lowercase().as_str() {
            "claude" => HarnessKind::Claude,
            "codex" => HarnessKind::Codex,
            "opencode" => HarnessKind::Opencode,
            _ => HarnessKind::Custom,
        }
    }

    pub fn is_builtin(self) -> bool {
        self != HarnessKind::Custom
    }

    /// Default executable for a builtin harness.
    pub fn program(self) -> Option<&'static str> {
        match self {
            HarnessKind::Custom => None,
            builtin => Some(builtin.as_str()),
        }
    }
}

impl fmt::Display for HarnessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HarnessKind {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HarnessKind::parse(s))
    }
}

/// How to run an agent: a builtin harness, optionally with an explicit executable,
/// or a custom command.
///
/// YAML accepts either a bare name (`harness: claude`) or an object
/// (`harness: {type: custom, command: "npm run dev", args: [...]}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Harness {
    pub kind: HarnessKind,
    /// Executable to run. `None` for a builtin means "use the harness's own binary";
    /// `None` for `Custom` means "run the user's shell".
    pub command: Option<String>,
    /// Arguments always passed to the executable, before the profile's `args`.
    pub args: Vec<String>,
}

impl Harness {
    pub fn builtin(kind: HarnessKind) -> Self {
        Harness {
            kind,
            command: None,
            args: Vec::new(),
        }
    }

    pub fn custom(command: impl Into<String>, args: Vec<String>) -> Self {
        Harness {
            kind: HarnessKind::Custom,
            command: Some(command.into()),
            args,
        }
    }

    /// Build from a bare harness name.
    ///
    /// Builtin names map to their kind. `custom` and `shell` give a `Custom` harness
    /// with no command (the user's shell); any other name is treated as the executable
    /// to run, so `harness: goose` runs `goose`.
    pub fn from_name(name: &str) -> Self {
        let kind = HarnessKind::parse(name);
        let command = match kind {
            HarnessKind::Custom => match name.trim() {
                "" | "custom" | "shell" => None,
                other => Some(other.to_string()),
            },
            _ => None,
        };
        Harness {
            kind,
            command,
            args: Vec::new(),
        }
    }
}

impl Default for Harness {
    fn default() -> Self {
        Harness::builtin(HarnessKind::Claude)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum HarnessRaw {
    Name(String),
    Detailed {
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        command: Option<String>,
        #[serde(default, deserialize_with = "one_or_many")]
        args: Vec<String>,
    },
}

impl<'de> Deserialize<'de> for Harness {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match HarnessRaw::deserialize(deserializer)? {
            HarnessRaw::Name(name) => Ok(Harness::from_name(&name)),
            HarnessRaw::Detailed {
                kind,
                command,
                args,
            } => {
                let mut harness = Harness::from_name(&kind);
                if command.is_some() {
                    harness.command = command;
                }
                harness.args = args;
                Ok(harness)
            }
        }
    }
}

impl Serialize for Harness {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.kind.is_builtin() && self.command.is_none() && self.args.is_empty() {
            return serializer.serialize_str(self.kind.as_str());
        }
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", self.kind.as_str())?;
        if let Some(command) = &self.command {
            map.serialize_entry("command", command)?;
        }
        if !self.args.is_empty() {
            map.serialize_entry("args", &self.args)?;
        }
        map.end()
    }
}

// =============================================================================
// API + agent profile
// =============================================================================

/// Model API an agent's harness should talk to. Secrets are referenced by the *name*
/// of an environment variable; the key itself is never stored here.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Wire protocol, e.g. `openai` or `anthropic`. Defaults to `openai` when unset.
    #[serde(default, alias = "format", skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// Provider label such as `openrouter` or `local`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Base URL of the endpoint. Configuration, not a secret.
    #[serde(
        default,
        alias = "url",
        alias = "endpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub base_url: Option<String>,
    /// Name of the environment variable holding the API key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
}

impl ApiConfig {
    pub const DEFAULT_PROTOCOL: &'static str = "openai";

    pub fn protocol_or_default(&self) -> &str {
        self.protocol.as_deref().unwrap_or(Self::DEFAULT_PROTOCOL)
    }
}

/// A reusable agent definition: identity + harness + model + skills + launch options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AgentProfile {
    /// Stable identifier used from the CLI and the TUI. Defaults to `name`, then to
    /// the harness name.
    pub id: String,
    /// Human-readable name; falls back to `id` for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub harness: Harness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Skill names to install; `"*"` means every skill found.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    /// Default first prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub disallowed_tools: Vec<String>,
    /// Extra CLI arguments appended after the generated ones.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Base58-encoded 32-byte Ed25519 public key (an `axel-identity` `PublicKey`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<ApiConfig>,
    /// Working directory override, relative to the workspace or absolute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl AgentProfile {
    pub fn new(id: impl Into<String>, harness: Harness) -> Self {
        AgentProfile {
            id: id.into(),
            harness,
            ..Default::default()
        }
    }

    /// `name` when set, otherwise `id`.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// `true` when the profile asks for every available skill.
    pub fn wants_all_skills(&self) -> bool {
        self.skills.iter().any(|s| s == "*")
    }
}

#[derive(Deserialize)]
struct AgentProfileRaw {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    harness: Option<Harness>,
    /// Agent-level `command:` is accepted as a fallback for a custom harness.
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    skills: Vec<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    allowed_tools: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    disallowed_tools: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    args: Vec<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    pubkey: Option<String>,
    #[serde(default)]
    api: Option<ApiConfig>,
    #[serde(default)]
    path: Option<String>,
}

impl<'de> Deserialize<'de> for AgentProfile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = AgentProfileRaw::deserialize(deserializer)?;
        let label = raw
            .id
            .clone()
            .or_else(|| raw.name.clone())
            .unwrap_or_else(|| "<unnamed>".to_string());

        let mut harness = raw.harness.ok_or_else(|| {
            de::Error::custom(format!("agent `{label}` requires a `harness` field"))
        })?;
        if harness.command.is_none() && raw.command.is_some() {
            harness.command = raw.command;
        }

        let id = raw
            .id
            .or(raw.name.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| match harness.kind.program() {
                Some(program) => program.to_string(),
                None => harness
                    .command
                    .as_deref()
                    .and_then(|c| c.split_whitespace().next())
                    .map(|c| {
                        Path::new(c)
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_else(|| c.to_string())
                    })
                    .unwrap_or_else(|| "custom".to_string()),
            });

        if let Some(pubkey) = raw.pubkey.as_deref()
            && !is_base58_pubkey(pubkey)
        {
            return Err(de::Error::custom(format!(
                "agent `{id}`: pubkey must be a base58-encoded 32-byte Ed25519 public key"
            )));
        }

        Ok(AgentProfile {
            id,
            name: raw.name,
            harness,
            model: raw.model,
            skills: raw.skills,
            prompt: raw.prompt,
            allowed_tools: raw.allowed_tools,
            disallowed_tools: raw.disallowed_tools,
            args: raw.args,
            color: raw.color,
            pubkey: raw.pubkey,
            api: raw.api,
            path: raw.path,
        })
    }
}

/// Accept `key: value` as well as `key: [value, ...]`.
fn one_or_many<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Option::<OneOrMany>::deserialize(deserializer)? {
        None => Vec::new(),
        Some(OneOrMany::One(s)) => vec![s],
        Some(OneOrMany::Many(v)) => v,
    })
}

/// Validate the canonical base58 representation of a 32-byte key (what Solana calls an
/// address). Ported from Axel v1.
pub fn is_base58_pubkey(value: &str) -> bool {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if value.is_empty() || value.len() > 44 {
        return false;
    }

    let mut bytes = vec![0_u8];
    for character in value.bytes() {
        let Some(digit) = ALPHABET
            .iter()
            .position(|candidate| *candidate == character)
        else {
            return false;
        };

        let mut carry = digit as u32;
        for byte in bytes.iter_mut().rev() {
            carry += (*byte as u32) * 58;
            *byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
    }

    let leading_zeroes = value.bytes().take_while(|byte| *byte == b'1').count();
    let significant = bytes.iter().skip_while(|byte| **byte == 0).count();
    leading_zeroes + significant == 32
}

// =============================================================================
// Skill sources + workspace config
// =============================================================================

/// A directory to search for skills. Written as `- path: ./skills` (or a bare string).
/// Earlier entries win over later ones for duplicate skill names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkillSource {
    pub path: PathBuf,
}

impl SkillSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        SkillSource { path: path.into() }
    }

    /// Expand a leading `~` and resolve a relative path against `base`.
    pub fn resolve(&self, base: Option<&Path>) -> PathBuf {
        let expanded = expand_tilde(&self.path);
        match base {
            Some(base) if expanded.is_relative() => base.join(expanded),
            _ => expanded,
        }
    }
}

impl<'de> Deserialize<'de> for SkillSource {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Path(String),
            Object { path: String },
        }
        let path = match Raw::deserialize(deserializer)? {
            Raw::Path(p) | Raw::Object { path: p } => p,
        };
        Ok(SkillSource::new(path))
    }
}

/// Expand a leading `~` or `~/` to the home directory. Other paths are returned as-is.
pub fn expand_tilde(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if text == "~" {
        return dirs::home_dir().unwrap_or_else(|| path.to_path_buf());
    }
    match (text.strip_prefix("~/"), dirs::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => path.to_path_buf(),
    }
}

/// The parsed `AXEL.md`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceConfig {
    /// `workspace:` (alias `name:`) from the frontmatter.
    pub workspace: Option<String>,
    pub agents: Vec<AgentProfile>,
    /// Skill directories in priority order. After [`WorkspaceConfig::load`] relative
    /// paths are already resolved against the manifest's directory.
    pub skills: Vec<SkillSource>,
    /// Markdown after the frontmatter (the project context), trimmed.
    pub body: String,
    /// Where the manifest was read from, when it was read from disk.
    pub path: Option<PathBuf>,
}

#[derive(Deserialize, Default)]
struct Frontmatter {
    #[serde(default, alias = "name")]
    workspace: Option<String>,
    #[serde(default)]
    agents: Vec<AgentProfile>,
    #[serde(default)]
    skills: Vec<SkillSource>,
}

impl WorkspaceConfig {
    /// Read `dir/AXEL.md`. `Ok(None)` when the file does not exist.
    pub fn load(dir: &Path) -> Result<Option<WorkspaceConfig>, Error> {
        let path = dir.join(AXEL_MD);
        if !path.is_file() {
            return Ok(None);
        }
        Self::load_file(&path).map(Some)
    }

    /// Read a manifest at an explicit path, resolving relative skill sources against
    /// its directory.
    pub fn load_file(path: &Path) -> Result<WorkspaceConfig, Error> {
        let text = fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        let mut config = Self::parse(&text).map_err(|e| e.at(path))?;
        config.path = Some(path.to_path_buf());
        let base = path.parent().filter(|p| !p.as_os_str().is_empty());
        for source in &mut config.skills {
            if source.path.is_relative() && !source.path.starts_with("~") {
                source.path = source.resolve(base);
            }
        }
        Ok(config)
    }

    /// Parse manifest text. Frontmatter is optional: text without a leading `---` line
    /// is all body, which makes a plain markdown `AXEL.md` a valid, agent-less manifest.
    pub fn parse(text: &str) -> Result<WorkspaceConfig, Error> {
        let (yaml, body) = split_frontmatter(text)?;
        let frontmatter: Frontmatter = match yaml {
            Some(yaml) if !yaml.trim().is_empty() => serde_norway::from_str(yaml)?,
            _ => Frontmatter::default(),
        };

        let mut seen = HashSet::new();
        for agent in &frontmatter.agents {
            if !seen.insert(agent.id.as_str()) {
                return Err(Error::Invalid(format!("duplicate agent id `{}`", agent.id)));
            }
        }

        Ok(WorkspaceConfig {
            workspace: frontmatter.workspace.filter(|w| !w.trim().is_empty()),
            agents: frontmatter.agents,
            skills: frontmatter.skills,
            body: body.trim().to_string(),
            path: None,
        })
    }

    /// Directory containing the manifest, when known.
    pub fn dir(&self) -> Option<&Path> {
        self.path.as_deref().and_then(Path::parent)
    }

    /// Workspace name: `workspace:` if set, otherwise the manifest's directory name.
    pub fn name(&self) -> Option<String> {
        self.workspace.clone().or_else(|| {
            self.dir()
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned())
        })
    }

    /// Find an agent by `id`, then by `name` (case-insensitive).
    pub fn agent(&self, id_or_name: &str) -> Option<&AgentProfile> {
        self.agents.iter().find(|a| a.id == id_or_name).or_else(|| {
            self.agents
                .iter()
                .find(|a| a.display_name().eq_ignore_ascii_case(id_or_name))
        })
    }

    /// Skill directories that exist on disk, resolved and in priority order.
    pub fn skills_dirs(&self) -> Vec<PathBuf> {
        self.skills
            .iter()
            .map(|s| s.resolve(self.dir()))
            .filter(|p| p.is_dir())
            .collect()
    }

    /// A fresh `AXEL.md` for a workspace. Round-trips through [`WorkspaceConfig::parse`].
    pub fn template(workspace_name: &str) -> String {
        format!(
            r#"---
workspace: {workspace}

# =============================================================================
# Skill directories
# =============================================================================
# Search paths for skill files (first match wins for duplicate names).
# A skill is `<name>.md` or `<name>/SKILL.md` with optional YAML frontmatter.
# Supports: ./relative, ~/home, /absolute paths.

skills:
  - path: ./skills
  - path: ~/.config/axel/skills

# =============================================================================
# Agents
# =============================================================================
# An agent combines identity + harness + model API + model.
# The stable `id` is what you reference from the CLI; `name` can change freely.

agents:
  # Claude Code - Anthropic
  - id: claude
    name: Claude
    harness: claude
    color: gray
    skills:
      - "*"                    # Load all skills, or list specific: ["skill1", "skill2"]
    # model: sonnet            # Model: sonnet, opus, haiku
    # prompt: "Your task..."   # Default first prompt
    # allowed_tools: []        # Restrict to specific tools
    # disallowed_tools: []     # Block specific tools
    # args: []                 # Additional CLI arguments
    # pubkey: 11111111111111111111111111111111 # base58 Ed25519 public key
    # api:
    #   protocol: openai        # OpenAI-compatible by default
    #   provider: openrouter
    #   base_url: https://openrouter.ai/api/v1
    #   api_key_env: OPENROUTER_API_KEY   # name of the env var, never the key itself

  # Codex - OpenAI
  - id: codex
    name: Codex
    harness: codex
    color: green
    skills:
      - "*"
    # model: gpt-5-codex       # Model to use
    # prompt: "Your task..."
    # args: []

  # OpenCode - open source
  # - id: opencode
  #   name: OpenCode
  #   harness: opencode
  #   color: blue
  #   skills: ["*"]

  # Custom harness: any executable, with optional arguments
  # - id: goose
  #   name: Goose
  #   harness:
  #     type: custom
  #     command: goose
  #     args: ["session"]
  #   color: red
---

# {workspace}

<!-- Project context for agents. Axel links this file as CLAUDE.md / AGENTS.md. -->

## Overview

<!-- Brief description of what this project does -->

## Getting Started

<!-- How to set up and run the project -->

## Architecture

<!-- High-level architecture overview -->

## Key Files

<!-- Important files and directories -->
"#,
            workspace = workspace_name,
        )
    }
}

/// Split text into `(frontmatter yaml, body)`.
///
/// Frontmatter is present when the first non-blank line is exactly `---`; it ends at
/// the next `---` line. Without an opening `---` the whole text is the body. An opening
/// `---` without a closing one is [`Error::UnterminatedFrontmatter`].
pub fn split_frontmatter(text: &str) -> Result<(Option<&str>, &str), Error> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text).trim_start();
    let mut offset = 0;
    let mut yaml_start = None;

    while offset < text.len() {
        let rest = &text[offset..];
        let line_len = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
        let line = rest[..line_len].trim_end();
        let next = offset + line_len;

        match yaml_start {
            None => {
                if line != "---" {
                    return Ok((None, text));
                }
                yaml_start = Some(next);
            }
            Some(start) if line == "---" => {
                return Ok((Some(&text[start..offset]), &text[next..]));
            }
            Some(_) => {}
        }
        offset = next;
    }

    match yaml_start {
        None => Ok((None, text)),
        Some(_) => Err(Error::UnterminatedFrontmatter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_harness_form() {
        let config = WorkspaceConfig::parse(
            r#"---
workspace: demo
agents:
  - id: reviewer
    name: Atlas
    harness: claude
    model: opus
    skills: ["code-review", "docs"]
    allowed_tools: [Read, Grep]
    disallowed_tools: Bash
    color: purple
---
# Demo
"#,
        )
        .unwrap();

        assert_eq!(config.workspace.as_deref(), Some("demo"));
        assert_eq!(config.body, "# Demo");
        assert_eq!(config.agents.len(), 1);
        let agent = &config.agents[0];
        assert_eq!(agent.id, "reviewer");
        assert_eq!(agent.display_name(), "Atlas");
        assert_eq!(agent.harness, Harness::builtin(HarnessKind::Claude));
        assert_eq!(agent.model.as_deref(), Some("opus"));
        assert_eq!(agent.skills, ["code-review", "docs"]);
        assert_eq!(agent.allowed_tools, ["Read", "Grep"]);
        assert_eq!(agent.disallowed_tools, ["Bash"]);
        assert_eq!(agent.color.as_deref(), Some("purple"));
        assert!(agent.api.is_none());
    }

    #[test]
    fn object_harness_form_and_api() {
        let config = WorkspaceConfig::parse(
            r#"---
name: demo
agents:
  - id: backend
    harness:
      type: custom
      command: "npm run dev"
      args: ["--port", "3000"]
    path: ./backend
  - id: fast
    harness:
      type: codex
      args: [--full-auto]
    model: openai/gpt-5
    pubkey: 11111111111111111111111111111111
    api:
      provider: openrouter
      base_url: https://openrouter.ai/api/v1
      api_key_env: OPENROUTER_API_KEY
---
"#,
        )
        .unwrap();

        assert_eq!(config.workspace.as_deref(), Some("demo"));
        let backend = &config.agents[0];
        assert_eq!(backend.harness.kind, HarnessKind::Custom);
        assert_eq!(backend.harness.command.as_deref(), Some("npm run dev"));
        assert_eq!(backend.harness.args, ["--port", "3000"]);
        assert_eq!(backend.path.as_deref(), Some("./backend"));

        let fast = &config.agents[1];
        assert_eq!(fast.harness.kind, HarnessKind::Codex);
        assert!(fast.harness.command.is_none());
        assert_eq!(fast.harness.args, ["--full-auto"]);
        assert_eq!(
            fast.pubkey.as_deref(),
            Some("11111111111111111111111111111111")
        );
        let api = fast.api.as_ref().unwrap();
        assert_eq!(api.provider.as_deref(), Some("openrouter"));
        assert_eq!(api.protocol_or_default(), "openai");
        assert_eq!(api.api_key_env.as_deref(), Some("OPENROUTER_API_KEY"));
    }

    #[test]
    fn bare_unknown_harness_name_is_a_custom_command() {
        let config =
            WorkspaceConfig::parse("---\nagents:\n  - harness: goose\n  - harness: shell\n---\n")
                .unwrap();
        assert_eq!(config.agents[0].id, "goose");
        assert_eq!(config.agents[0].harness, Harness::custom("goose", vec![]));
        assert_eq!(config.agents[1].id, "custom");
        assert_eq!(config.agents[1].harness.kind, HarnessKind::Custom);
        assert!(config.agents[1].harness.command.is_none());
    }

    #[test]
    fn id_defaults_to_name_then_harness() {
        let config = WorkspaceConfig::parse(
            "---\nagents:\n  - name: Atlas\n    harness: claude\n  - harness: codex\n---\n",
        )
        .unwrap();
        assert_eq!(config.agents[0].id, "Atlas");
        assert_eq!(config.agents[1].id, "codex");
        assert_eq!(config.agent("atlas").map(|a| a.id.as_str()), Some("Atlas"));
    }

    #[test]
    fn agent_requires_harness() {
        let err =
            WorkspaceConfig::parse("---\nagents:\n  - id: x\n    model: opus\n---\n").unwrap_err();
        assert!(err.to_string().contains("requires a `harness`"), "{err}");
    }

    #[test]
    fn rejects_bad_pubkey_and_duplicate_ids() {
        let err = WorkspaceConfig::parse(
            "---\nagents:\n  - id: x\n    harness: claude\n    pubkey: nope\n---\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("base58"), "{err}");

        let err = WorkspaceConfig::parse(
            "---\nagents:\n  - id: x\n    harness: claude\n  - id: x\n    harness: codex\n---\n",
        )
        .unwrap_err();
        assert!(matches!(err, Error::Invalid(_)), "{err}");
    }

    #[test]
    fn missing_frontmatter_is_all_body() {
        let config = WorkspaceConfig::parse("# Just notes\n\nNo config here.\n").unwrap();
        assert!(config.workspace.is_none());
        assert!(config.agents.is_empty());
        assert_eq!(config.body, "# Just notes\n\nNo config here.");

        let config = WorkspaceConfig::parse("").unwrap();
        assert_eq!(config, WorkspaceConfig::default());
    }

    #[test]
    fn unterminated_frontmatter_is_an_error() {
        let err = WorkspaceConfig::parse("---\nworkspace: x\n# oops").unwrap_err();
        assert!(matches!(err, Error::UnterminatedFrontmatter));
    }

    #[test]
    fn empty_frontmatter_and_legacy_keys_are_tolerated() {
        let config = WorkspaceConfig::parse("---\n---\nbody").unwrap();
        assert_eq!(config.body, "body");

        let config = WorkspaceConfig::parse(
            "---\nworkspace: old\nlayouts:\n  grids:\n    default:\n      type: tmux\nagents:\n  - id: a\n    harness: claude\n---\n",
        )
        .unwrap();
        assert_eq!(config.agents.len(), 1);
    }

    #[test]
    fn template_roundtrips() {
        let text = WorkspaceConfig::template("myproj");
        let config = WorkspaceConfig::parse(&text).unwrap();
        assert_eq!(config.workspace.as_deref(), Some("myproj"));
        assert_eq!(config.skills.len(), 2);
        assert_eq!(config.skills[0].path, PathBuf::from("./skills"));
        assert_eq!(
            config.skills[1].path,
            PathBuf::from("~/.config/axel/skills")
        );
        let ids: Vec<&str> = config.agents.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["claude", "codex"]);
        assert!(config.agents.iter().all(AgentProfile::wants_all_skills));
        assert_eq!(config.agents[0].harness.kind, HarnessKind::Claude);
        assert_eq!(config.agents[1].harness.kind, HarnessKind::Codex);
        assert!(config.body.starts_with("# myproj"));
        assert!(config.body.contains("## Key Files"));
    }

    #[test]
    fn load_resolves_relative_skill_sources() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(AXEL_MD),
            "---\nskills:\n  - path: ./skills\n  - ~/.config/axel/skills\n  - /abs\n---\n",
        )
        .unwrap();
        std::fs::create_dir(dir.path().join("skills")).unwrap();

        let config = WorkspaceConfig::load(dir.path()).unwrap().unwrap();
        assert_eq!(
            config.path.as_deref(),
            Some(dir.path().join(AXEL_MD).as_path())
        );
        assert_eq!(config.skills[0].path, dir.path().join("skills"));
        assert_eq!(
            config.skills[1].path,
            PathBuf::from("~/.config/axel/skills")
        );
        assert_eq!(config.skills[2].path, PathBuf::from("/abs"));
        assert_eq!(config.skills_dirs(), vec![dir.path().join("skills")]);
        assert_eq!(
            config.name().as_deref(),
            dir.path().file_name().and_then(|n| n.to_str())
        );

        assert!(
            WorkspaceConfig::load(&dir.path().join("nope"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn harness_serializes_compactly() {
        let yaml = serde_norway::to_string(&Harness::builtin(HarnessKind::Codex)).unwrap();
        assert_eq!(yaml.trim(), "codex");
        let yaml =
            serde_norway::to_string(&Harness::custom("npm run dev", vec!["--x".into()])).unwrap();
        let back: Harness = serde_norway::from_str(&yaml).unwrap();
        assert_eq!(back, Harness::custom("npm run dev", vec!["--x".into()]));
    }

    #[test]
    fn split_frontmatter_edge_cases() {
        assert_eq!(
            split_frontmatter("---\na: 1\n---").unwrap(),
            (Some("a: 1\n"), "")
        );
        assert_eq!(
            split_frontmatter("---\r\na: 1\r\n---\r\nbody").unwrap(),
            (Some("a: 1\r\n"), "body")
        );
        assert_eq!(
            split_frontmatter("\n\n---\n---\nx").unwrap(),
            (Some(""), "x")
        );
        assert_eq!(
            split_frontmatter("--- not\nx").unwrap(),
            (None, "--- not\nx")
        );
    }

    #[test]
    fn base58_pubkey_validation() {
        assert!(is_base58_pubkey("11111111111111111111111111111111"));
        assert!(is_base58_pubkey(
            "So11111111111111111111111111111111111111112"
        ));
        assert!(!is_base58_pubkey("not-a-key"));
        assert!(!is_base58_pubkey(""));
    }
}
