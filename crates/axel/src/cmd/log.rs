//! `axel log SESSION [--follow]` — the raw harness log, one line per frame.

use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::time::Duration;

use anyhow::{Context, Result};

use super::session::log_path;

pub fn run(session: &str, follow: bool) -> Result<()> {
    let path = log_path(session);
    let mut file = std::fs::File::open(&path).with_context(|| format!("no log at {}", path.display()))?;
    let mut pos = 0u64;
    loop {
        file.seek(SeekFrom::Start(pos))?;
        let mut reader = BufReader::new(&file);
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            if line.ends_with('\n') {
                pos += line.len() as u64;
                print_frame(&line);
                line.clear();
            } else {
                break;
            }
        }
        if !follow {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

fn print_frame(line: &str) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
        print!("{line}");
        return;
    };
    let dir = v.get("dir").and_then(|d| d.as_str()).unwrap_or("?");
    let arrow = if dir == "out" { "\x1b[36m→\x1b[0m" } else { "\x1b[35m←\x1b[0m" };
    let frame = v.get("frame").cloned().unwrap_or_default();
    let method = frame.get("method").and_then(|m| m.as_str()).map(String::from).unwrap_or_else(|| {
        if frame.get("result").is_some() { "result".into() } else if frame.get("error").is_some() { "error".into() } else { "frame".into() }
    });
    let body = serde_json::to_string(&frame).unwrap_or_default();
    let body: String = body.chars().take(400).collect();
    println!("{arrow} \x1b[1m{method}\x1b[0m \x1b[2m{body}\x1b[0m");
}
