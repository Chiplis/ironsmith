use std::env;
use std::io::{self, IsTerminal, Read, Write};

use ironsmith::cards::{CardDefinition, CardRegistry};
use ironsmith::compiled_text::compiled_text_lines;
use ironsmith::semantic_compare::compare_semantics_scored;
use ironsmith_compiler::parse_trace as card_parse_trace;
use ironsmith_registry::CardRegistry as RegistryCardRegistry;
use ironsmith_tools::{
    CompilationSnapshot, ParseStatus, build_parse_input,
    compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_by_name, load_card_payloads_by_name,
    parse_card_definition_with_runtime_builder,
};

const DEFAULT_PROBE_NAME: &str = "Parser Probe";
const DEFAULT_SHOW_DEFINITION: bool = true;

fn text_includes_metadata(text: &str) -> bool {
    text.lines().map(str::trim).any(|line| {
        line.starts_with("Mana cost:")
            || line.starts_with("Type:")
            || line.starts_with("Power/Toughness:")
            || line.starts_with("Loyalty:")
            || line.starts_with("Defense:")
    })
}

fn read_input_text(text_arg: Option<String>) -> Result<Option<String>, String> {
    if let Some(text) = text_arg {
        return Ok(Some(text));
    }
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("failed to read stdin: {err}"))?;
    if input.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(input))
}

fn should_read_input_text(
    text_arg_present: bool,
    names_empty: bool,
    stdin_is_terminal: bool,
) -> bool {
    text_arg_present || names_empty || !stdin_is_terminal
}

fn metadata_lines_from_definition(definition: &CardDefinition) -> Vec<String> {
    let mut metadata_lines = Vec::new();

    if let Some(mana_cost) = definition
        .card
        .mana_cost
        .as_ref()
        .map(|cost| cost.to_oracle())
        .filter(|value| !value.trim().is_empty())
    {
        metadata_lines.push(format!("Mana cost: {}", mana_cost.trim()));
    }

    let mut type_line = definition
        .card
        .supertypes
        .iter()
        .map(|value| format!("{value:?}"))
        .chain(
            definition
                .card
                .card_types
                .iter()
                .map(|value| format!("{value:?}")),
        )
        .collect::<Vec<_>>()
        .join(" ");
    let subtypes = definition
        .card
        .subtypes
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>();
    if !subtypes.is_empty() {
        if !type_line.is_empty() {
            type_line.push_str(" — ");
        }
        type_line.push_str(&subtypes.join(" "));
    }
    if !type_line.trim().is_empty() {
        metadata_lines.push(format!("Type: {}", type_line.trim()));
    }

    if let Some(power_toughness) = definition.card.power_toughness {
        metadata_lines.push(format!(
            "Power/Toughness: {}/{}",
            power_toughness.power, power_toughness.toughness
        ));
    }

    if let Some(loyalty) = definition.card.loyalty {
        metadata_lines.push(format!("Loyalty: {loyalty}"));
    }

    if let Some(defense) = definition.card.defense {
        metadata_lines.push(format!("Defense: {defense}"));
    }

    metadata_lines
}

fn payload_from_definition(definition: &CardDefinition) -> ironsmith_tools::CardPayload {
    let raw_oracle_text = compiled_text_lines(definition).join("\n");
    let oracle_text = ironsmith_tools::postprocess_oracle_text(&raw_oracle_text);
    let metadata_lines = metadata_lines_from_definition(definition);
    let parse_input = build_parse_input(&metadata_lines, &raw_oracle_text);

    ironsmith_tools::CardPayload {
        name: definition.name().to_string(),
        parse_name: None,
        oracle_text,
        raw_oracle_text,
        metadata_lines,
        parse_input,
        other_face_name: definition.card.other_face_name.clone(),
        linked_face_layout: Some(definition.card.linked_face_layout),
    }
}

struct CompileJob {
    name: String,
    oracle_text: String,
    parse_input: String,
    authoritative_snapshot: Option<CompilationSnapshot>,
    compiled_definition: Option<CardDefinition>,
}

fn compile_definition_for_job(job: &CompileJob) -> Result<CardDefinition, String> {
    if let Some(definition) = job.compiled_definition.as_ref() {
        return Ok(definition.clone());
    }
    let _scope = card_parse_trace::scope("display definition");
    parse_card_definition_with_runtime_builder(&job.name, job.parse_input.clone(), false)
        .map_err(|err| format!("parse failed for {}: {err:?}", job.name))
}

fn compile_job_for_name(
    cards_path: &str,
    name: &str,
    input_text: Option<&str>,
) -> Result<CompileJob, String> {
    fn compile_from_registry_name(name: &str) -> Option<CardDefinition> {
        RegistryCardRegistry::try_compile_card(name)
            .ok()
            .or_else(|| CardRegistry::try_compile_card(name).ok())
            .or_else(|| {
                let (front_face, _) = name.split_once("//")?;
                let front_face = front_face.trim();
                RegistryCardRegistry::try_compile_card(front_face)
                    .ok()
                    .or_else(|| CardRegistry::try_compile_card(front_face).ok())
            })
    }

    let card_input = load_card_by_name(cards_path, name).map_err(|err| err.to_string())?;
    // Prefer the payload-backed cards.json entry when available so inspection reflects the
    // active parser/lowering path. Fall back to registry-backed definitions only when the card
    // is not present in the source snapshot and the caller did not supply ad hoc text.
    if input_text.is_none()
        && card_input.is_none()
        && let Some(definition) = compile_from_registry_name(name)
    {
        let payload = payload_from_definition(&definition);
        return Ok(CompileJob {
            name: definition.name().to_string(),
            oracle_text: payload.oracle_text.clone(),
            parse_input: payload.parse_input.clone(),
            authoritative_snapshot: None,
            compiled_definition: Some(definition),
        });
    }

    match (input_text, card_input) {
        (Some(text), Some(card)) if !text_includes_metadata(text) => {
            let parse_input = build_parse_input(&card.metadata_lines, text);
            let oracle_text = text.trim().to_string();
            Ok(CompileJob {
                name: card.name,
                oracle_text,
                parse_input,
                authoritative_snapshot: None,
                compiled_definition: None,
            })
        }
        (Some(text), Some(card)) => Ok(CompileJob {
            name: card.name,
            oracle_text: card.oracle_text,
            parse_input: text.to_string(),
            authoritative_snapshot: None,
            compiled_definition: None,
        }),
        (Some(text), None) => Ok(CompileJob {
            name: name.to_string(),
            oracle_text: text.to_string(),
            parse_input: text.to_string(),
            authoritative_snapshot: None,
            compiled_definition: None,
        }),
        (None, Some(card)) => {
            let name = card.name.clone();
            let oracle_text = card.oracle_text.clone();
            let parse_input = card.parse_input.clone();
            let authoritative_snapshot = {
                let _scope = card_parse_trace::scope("authoritative snapshot");
                compile_authoritative_snapshot_from_payload(&card)
            };
            let compiled_definition = {
                let _scope = card_parse_trace::scope("display definition");
                compile_definition_from_payload(&card).ok()
            };
            Ok(CompileJob {
                name,
                oracle_text,
                parse_input,
                authoritative_snapshot: Some(authoritative_snapshot),
                compiled_definition,
            })
        }
        (None, None) => Err(format!("unknown card name: {name}")),
    }
}

fn compile_jobs_for_name(
    cards_path: &str,
    name: &str,
    input_text: Option<&str>,
) -> Result<Vec<CompileJob>, String> {
    if input_text.is_none() {
        let payloads =
            load_card_payloads_by_name(cards_path, name).map_err(|err| err.to_string())?;
        if !payloads.is_empty() {
            return payloads
                .into_iter()
                .map(|card| {
                    let name = card.name.clone();
                    let oracle_text = card.oracle_text.clone();
                    let parse_input = card.parse_input.clone();
                    let authoritative_snapshot = {
                        let _scope = card_parse_trace::scope("authoritative snapshot");
                        compile_authoritative_snapshot_from_payload(&card)
                    };
                    let compiled_definition = {
                        let _scope = card_parse_trace::scope("display definition");
                        compile_definition_from_payload(&card).ok()
                    };
                    Ok(CompileJob {
                        name,
                        oracle_text,
                        parse_input,
                        authoritative_snapshot: Some(authoritative_snapshot),
                        compiled_definition,
                    })
                })
                .collect();
        }
    }

    compile_job_for_name(cards_path, name, input_text).map(|job| vec![job])
}

fn write_compiled_job<W: Write>(
    out: &mut W,
    job: &CompileJob,
    detailed: bool,
    raw: bool,
    show_definition: bool,
) -> Result<(), String> {
    macro_rules! outln {
        ($($arg:tt)*) => {
            writeln!(out, $($arg)*)
                .map_err(|err| format!("failed to write compile output: {err}"))?
        };
    }

    if let Some(snapshot) = job.authoritative_snapshot.as_ref()
        && snapshot.parse_status == ParseStatus::ParseFailed
    {
        return Err(format!(
            "parse failed for {}: {}",
            job.name,
            snapshot
                .parse_error
                .as_deref()
                .unwrap_or("unknown authoritative parse failure"),
        ));
    }

    let display_def = compile_definition_for_job(job)?;

    outln!("Name: {}", display_def.card.name);
    if detailed {
        outln!("Parse input:");
        outln!("{}", job.parse_input.trim());
    }
    outln!(
        "Type: {}",
        display_def
            .card
            .card_types
            .iter()
            .map(|t| format!("{t:?}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    outln!("Original oracle text:");
    outln!("{}", job.oracle_text.trim());
    outln!("Compiled abilities/effects");
    if raw {
        outln!("- {:#?}", display_def);
    } else {
        let lines = compiled_text_lines(&display_def);
        if lines.is_empty() {
            outln!("- <none>");
        } else {
            for line in lines {
                outln!("- {}", line.trim());
            }
        }
    }
    if show_definition {
        outln!("Compiled card definition:");
        outln!("{:#?}", &display_def);
    }
    Ok(())
}

fn write_compare_text_job<W: Write>(out: &mut W, job: &CompileJob) -> Result<(), String> {
    macro_rules! outln {
        ($($arg:tt)*) => {
            writeln!(out, $($arg)*)
                .map_err(|err| format!("failed to write compile output: {err}"))?
        };
    }

    if let Some(snapshot) = job.authoritative_snapshot.as_ref()
        && snapshot.parse_status == ParseStatus::ParseFailed
    {
        return Err(format!(
            "parse failed for {}: {}",
            job.name,
            snapshot
                .parse_error
                .as_deref()
                .unwrap_or("unknown authoritative parse failure"),
        ));
    }

    let display_def = compile_definition_for_job(job)?;
    let compiled_lines = compiled_text_lines(&display_def);
    let (similarity, _, _, _, semantic_mismatch) =
        compare_semantics_scored(&job.oracle_text, &compiled_lines, None);

    outln!("Name: {}", display_def.card.name);
    outln!("Similarity: {:.4}", similarity);
    outln!("Semantic mismatch: {}", semantic_mismatch);
    outln!("Original oracle text:");
    outln!("{}", job.oracle_text.trim());
    outln!("Compiled oracle text:");
    if compiled_lines.is_empty() {
        outln!("<none>");
    } else {
        outln!("{}", compiled_lines.join("\n").trim());
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let mut names: Vec<String> = Vec::new();
    let mut cards_path = default_cards_path().display().to_string();
    let mut text_arg: Option<String> = None;
    let mut stacktrace = false;
    let mut trace = false;
    let mut allow_unsupported = false;
    let mut detailed = false;
    let mut raw = false;
    let mut show_definition = DEFAULT_SHOW_DEFINITION;
    let mut compare_text = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => {
                names.push(
                    args.next()
                        .ok_or_else(|| "--name requires a value".to_string())?,
                );
            }
            "--names" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--names requires a value".to_string())?;
                let contents = std::fs::read_to_string(&path)
                    .map_err(|err| format!("failed to read --names file {path}: {err}"))?;
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    names.push(trimmed.to_string());
                }
            }
            "--cards" => {
                cards_path = args
                    .next()
                    .ok_or_else(|| "--cards requires a value".to_string())?;
            }
            "--text" => {
                text_arg = Some(
                    args.next()
                        .ok_or_else(|| "--text requires a value".to_string())?,
                );
            }
            "--stacktrace" => {
                stacktrace = true;
            }
            "--trace" => {
                trace = true;
            }
            "--allow-unsupported" => {
                allow_unsupported = true;
            }
            "--detailed" => {
                detailed = true;
            }
            "--raw" => {
                raw = true;
            }
            "--show-definition" => {
                show_definition = true;
            }
            "--compare-text" => {
                compare_text = true;
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --name <value>, --names <path>, --cards <path>, --text <value>, --trace, --allow-unsupported, --detailed, --raw, --show-definition, --compare-text, and/or --stacktrace"
                ));
            }
        }
    }

    if stacktrace {
        unsafe {
            env::set_var("IRONSMITH_PARSER_STACKTRACE", "1");
        }
    }

    if allow_unsupported {
        unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", "1");
        }
    }

    let input_text = if should_read_input_text(
        text_arg.is_some(),
        names.is_empty(),
        io::stdin().is_terminal(),
    ) {
        read_input_text(text_arg)?
    } else {
        None
    };
    if input_text.is_some() && names.len() > 1 {
        return Err(
            "pass --text/stdin with at most one --name; batch mode only supports card lookups"
                .to_string(),
        );
    }

    if names.is_empty() && input_text.is_none() {
        return Err(
            "missing oracle text (pass --text or stdin) and no matching card found via --name/--cards"
                .to_string(),
        );
    }

    if names.is_empty() {
        names.push(DEFAULT_PROBE_NAME.to_string());
    }

    let mut stdout = io::stdout().lock();
    let mut output_idx = 0;
    for name in names.iter() {
        let compile_one = || -> Result<Vec<Vec<u8>>, String> {
            card_parse_trace::event(format!("Trace: {name}"));
            compile_jobs_for_name(&cards_path, name, input_text.as_deref())?
                .iter()
                .map(|job| {
                    let mut output = Vec::new();
                    if compare_text {
                        write_compare_text_job(&mut output, job)?;
                    } else {
                        write_compiled_job(&mut output, job, detailed, raw, show_definition)?;
                    }
                    Ok(output)
                })
                .collect()
        };

        let outputs = if trace {
            let (result, report) = card_parse_trace::capture(compile_one);
            if !report.is_empty() {
                eprint!("{}", report.render());
            }
            result?
        } else {
            compile_one()?
        };

        for output in outputs {
            if output_idx > 0 {
                writeln!(stdout).map_err(|err| format!("failed to write compile output: {err}"))?;
            }
            stdout
                .write_all(&output)
                .map_err(|err| format!("failed to write compile output: {err}"))?;
            output_idx += 1;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_parse_input_appends_oracle_text_after_metadata() {
        let parse_input = build_parse_input(
            &[
                "Mana cost: {U}{U}".to_string(),
                "Type: Creature — Merfolk Wizard".to_string(),
                "Power/Toughness: 1/3".to_string(),
            ],
            "When this creature enters, draw a card.",
        );

        assert_eq!(
            parse_input,
            "Mana cost: {U}{U}\nType: Creature — Merfolk Wizard\nPower/Toughness: 1/3\nWhen this creature enters, draw a card."
        );
    }

    #[test]
    fn metadata_detection_ignores_plain_oracle_lines() {
        assert!(!text_includes_metadata(
            "When Thassa's Oracle enters the battlefield, look at the top X cards of your library."
        ));
        assert!(text_includes_metadata(
            "Type: Creature — Merfolk Wizard\nWhen this creature enters, draw a card."
        ));
    }

    #[test]
    fn should_read_input_text_only_when_needed() {
        assert!(should_read_input_text(true, false, true));
        assert!(should_read_input_text(false, true, true));
        assert!(should_read_input_text(false, false, false));
        assert!(!should_read_input_text(false, false, true));
    }

    #[test]
    fn show_definition_defaults_on() {
        assert!(DEFAULT_SHOW_DEFINITION);
    }

    #[test]
    fn compile_job_for_name_builds_batch_lookup_job() {
        let cards_path = format!("{}/../../cards.json", env!("CARGO_MANIFEST_DIR"));
        let job = compile_job_for_name(&cards_path, "House Cartographer", None)
            .expect("House Cartographer should exist");

        assert_eq!(job.name, "House Cartographer");
        assert!(job.parse_input.contains("Type: Creature"));
        assert!(job.authoritative_snapshot.is_some());
        assert!(job.compiled_definition.is_some());
    }
}
