//! `axel doctor`

use anyhow::Result;

use super::Ctx;

pub fn run() -> Result<()> {
    println!("axel {}", env!("CARGO_PKG_VERSION"));
    println!("db:        {}", axel_store::Store::default_path().display());
    match axel_rex::detect() {
        Some(env) => {
            println!("rex:       session {}  block {}", env.session, env.block);
            let rex = axel_rex::Rex::new(env);
            match rex.host_id() {
                Ok(h) => println!("rex host:  {h}"),
                Err(e) => println!("rex host:  unavailable ({e})"),
            }
        }
        None => println!("rex:       not detected (REX_SESSION unset); launcher runs inline"),
    }
    let ctx = Ctx::open()?;
    let owner = ctx.owner()?;
    println!("owner:     {}", owner.public());
    let env = ctx.store.local_environment()?;
    println!("env:       {} ({})", env.label, env.id);
    let agents = ctx.store.list_agents()?;
    println!("agents:    {}", agents.len());
    for a in &agents {
        let key = ctx.keystore.agent(&a.pubkey.parse()?)?.is_some();
        println!("  {:<16} {:<9} key:{}", a.name, a.harness, if key { "ok" } else { "MISSING" });
    }
    println!("pending:   {}", ctx.store.pending_requests()?.len());
    let open = ctx.store.list_threads(axel_store::ThreadFilter { states: Some(vec![axel_store::ThreadState::Open]), ..Default::default() })?.len();
    let live = ctx.store.statuses()?.len();
    println!("threads:   {open} open, {live} live sessions");
    for h in ["claude", "codex", "opencode", "rex", "git"] {
        let found = which(h);
        println!("{:<10} {}", format!("{h}:"), found.map(|p| p.display().to_string()).unwrap_or_else(|| "not found".into()));
    }
    Ok(())
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(bin)).find(|p| p.is_file())
}
