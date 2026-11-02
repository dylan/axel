# Import provenance

| Destination | Source | Changes |
|---|---|---|
| `axel-identity/src/attest.rs` | `Buzz/crates/buzz-sdk/src/nip_oa.rs` | Schnorr→Ed25519, domain `axel:agent-auth:v1:`, base58 keys |
| `axel-inbox/src/mentions.rs` | `Buzz/crates/buzz-sdk/src/mentions.rs` | npub→base58, own profile type |
| `axel-inbox/src/threading.rs` | `Buzz/crates/buzz-core/src/nip10.rs` | own envelope type |
| `axel-inbox/src/envelope.rs` | shape from `Buzz/crates/buzz-core/src/verification.rs` | Ed25519 `verify_strict` |
| `axel-config/src/{config,skills}.rs` | `axel/cli/crates/core/src/config.rs` | flat `AgentProfile`, no legacy panes/grids |
| `axel-config/src/drivers/*` | `axel/cli/crates/core/src/drivers/*` | skill install only; no OTEL/tmux hooks |
| `axel-config/src/git.rs` | `axel/cli/crates/core/src/git.rs` | trimmed: no isolated-worktree squash flow; new branches fork from HEAD |
| `axel/src/cmd/acp.rs` | `axel/cli/crates/cli/src/commands/acp.rs` | frame parsing hooked to inbox |
