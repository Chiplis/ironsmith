use std::env;
use std::fs;
use std::path::Path;

use ironsmith_audit::fixtures::{cheating_transcript, fair_transcript};
use ironsmith_audit::protocol::requirements_for_command;
use ironsmith_audit::{AuditTranscript, verify_transcript};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("verify") => {
            let path = args
                .next()
                .ok_or_else(|| "usage: ironsmith-audit verify <transcript.json>".to_string())?;
            let raw = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read {path}: {err}"))?;
            let transcript: AuditTranscript = serde_json::from_str(&raw)
                .map_err(|err| format!("invalid transcript json {path}: {err}"))?;
            let report = verify_transcript(&transcript).map_err(|err| err.to_string())?;
            println!(
                "valid: {}\nverified_actions: {}\nfinal_state_hash: {}",
                report.valid,
                report.verified_actions,
                report.final_state_hash.unwrap_or_else(|| "<none>".to_string())
            );
            Ok(())
        }
        Some("fixture") => {
            let kind = args
                .next()
                .ok_or_else(|| "usage: ironsmith-audit fixture <fair|cheat> <out.json>".to_string())?;
            let path = args
                .next()
                .ok_or_else(|| "usage: ironsmith-audit fixture <fair|cheat> <out.json>".to_string())?;
            let transcript = match kind.as_str() {
                "fair" => fair_transcript()?,
                "cheat" => cheating_transcript()?,
                _ => return Err("fixture kind must be 'fair' or 'cheat'".to_string()),
            };
            write_json(&path, &transcript)
        }
        Some("explain") => {
            let path = args
                .next()
                .ok_or_else(|| "usage: ironsmith-audit explain <transcript.json>".to_string())?;
            let raw = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read {path}: {err}"))?;
            let transcript: AuditTranscript = serde_json::from_str(&raw)
                .map_err(|err| format!("invalid transcript json {path}: {err}"))?;
            println!("match_id: {}", transcript.match_id);
            println!("players: {}", transcript.players.len());
            println!("deck_ceremonies: {}", transcript.deck_ceremonies.len());
            for ceremony in &transcript.deck_ceremonies {
                println!(
                    "deck {} owner {}: {} shuffle steps, {} committed slots",
                    ceremony.deck_id,
                    ceremony.owner,
                    ceremony.steps.len(),
                    ceremony.slot_commitments.len()
                );
            }
            for action in &transcript.actions {
                println!("seq {} actor {} command {:?}", action.seq, action.actor, action.command);
                for requirement in requirements_for_command(&action.command) {
                    println!("  requires {:?}", requirement);
                }
            }
            Ok(())
        }
        _ => Err(
            "usage:\n  ironsmith-audit verify <transcript.json>\n  ironsmith-audit fixture <fair|cheat> <out.json>\n  ironsmith-audit explain <transcript.json>"
                .to_string(),
        ),
    }
}

fn write_json(path: &str, transcript: &AuditTranscript) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(transcript)
        .map_err(|err| format!("failed to encode transcript: {err}"))?;
    fs::write(path, format!("{json}\n")).map_err(|err| format!("failed to write {path}: {err}"))
}
