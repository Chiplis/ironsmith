use std::env;
use std::io::{self, Read};

use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::compiled_text::{compiled_lines, oracle_like_lines};
use ironsmith::ids::CardId;

fn read_input_text(text_arg: Option<String>) -> Result<String, String> {
    if let Some(text) = text_arg {
        return Ok(text);
    }
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("failed to read stdin: {err}"))?;
    if input.trim().is_empty() {
        return Err("missing oracle text (pass --text or stdin)".to_string());
    }
    Ok(input)
}

fn main() -> Result<(), String> {
    let mut name = "Parser Probe".to_string();
    let mut text_arg: Option<String> = None;
    let mut stacktrace = false;
    let mut trace = false;
    let mut allow_unsupported = false;
    let mut detailed = false;
    let mut raw = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => {
                name = args
                    .next()
                    .ok_or_else(|| "--name requires a value".to_string())?;
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
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --name <value>, --text <value>, --trace, --allow-unsupported, --detailed, --raw, and/or --stacktrace"
                ));
            }
        }
    }

    if trace {
        unsafe {
            env::set_var("IRONSMITH_PARSER_TRACE", "1");
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

    let text = read_input_text(text_arg)?;
    let builder = CardDefinitionBuilder::new(CardId::new(), &name);
    let def = builder
        .parse_text(text)
        .map_err(|err| format!("parse failed: {err:?}"))?;

    println!("Name: {}", def.card.name);
    println!(
        "Type: {}",
        def.card
            .card_types
            .iter()
            .map(|t| format!("{t:?}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("Compiled abilities/effects");
    if raw {
        println!("- {:#?}", def);
        return Ok(());
    }
    let lines = if detailed {
        compiled_lines(&def)
    } else {
        oracle_like_lines(&def)
    };
    if lines.is_empty() {
        println!("- <none>");
    } else {
        for line in lines {
            println!("- {}", line.trim());
        }
    }

    Ok(())
}
