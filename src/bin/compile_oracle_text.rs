use std::env;
use std::io::{self, Read};

use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::compiled_text::{compiled_lines, oracle_like_lines};
use ironsmith::ids::CardId;
use ironsmith_tools::{
    CardStatusDb, build_parse_input, compile_snapshot_from_payload, default_db_path,
    load_card_by_name,
};

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

fn store_snapshot_if_requested(
    should_write_db: bool,
    db_payload: Option<&ironsmith_tools::CardPayload>,
    db_path: &str,
) -> Result<(), String> {
    if !should_write_db {
        return Ok(());
    }
    let Some(payload) = db_payload else {
        return Ok(());
    };

    let db = CardStatusDb::open(db_path).map_err(|err| err.to_string())?;
    let snapshot = compile_snapshot_from_payload(payload);
    let inserted = db
        .insert_snapshot_if_changed(&snapshot)
        .map_err(|err| err.to_string())?;
    eprintln!(
        "[INFO] {} card status snapshot in {}",
        if inserted {
            "stored"
        } else {
            "skipped unchanged"
        },
        db_path
    );
    Ok(())
}

fn snapshot_payload_for_db(
    card: Option<&ironsmith_tools::CardPayload>,
    oracle_text: &str,
    parse_input: &str,
) -> Option<ironsmith_tools::CardPayload> {
    let card = card?;
    let oracle_matches = card.oracle_text.trim() == oracle_text.trim();
    let parse_input_matches = card.parse_input.trim() == parse_input.trim();
    (oracle_matches && parse_input_matches).then(|| ironsmith_tools::CardPayload {
        name: card.name.clone(),
        oracle_text: card.oracle_text.clone(),
        metadata_lines: card.metadata_lines.clone(),
        parse_input: card.parse_input.clone(),
    })
}

fn main() -> Result<(), String> {
    let mut name = "Parser Probe".to_string();
    let mut cards_path = "cards.json".to_string();
    let mut text_arg: Option<String> = None;
    let mut stacktrace = false;
    let mut trace = false;
    let mut allow_unsupported = false;
    let mut detailed = false;
    let mut raw = false;
    let mut show_definition = false;
    let mut db_path = default_db_path().display().to_string();
    let mut no_db = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => {
                name = args
                    .next()
                    .ok_or_else(|| "--name requires a value".to_string())?;
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
            "--db-path" => {
                db_path = args
                    .next()
                    .ok_or_else(|| "--db-path requires a value".to_string())?;
            }
            "--no-db" => {
                no_db = true;
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --name <value>, --cards <path>, --text <value>, --trace, --allow-unsupported, --detailed, --raw, --show-definition, --db-path <path>, --no-db, and/or --stacktrace"
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

    let input_text = read_input_text(text_arg)?;
    let card_input = if name != "Parser Probe" {
        load_card_by_name(&cards_path, &name).map_err(|err| err.to_string())?
    } else {
        None
    };

    let (name, oracle_text, parse_input, db_payload) = match (input_text, card_input) {
        (Some(text), Some(card)) if !text_includes_metadata(&text) => {
            let parse_input = build_parse_input(&card.metadata_lines, &text);
            let oracle_text = text.trim().to_string();
            let db_payload = snapshot_payload_for_db(Some(&card), &oracle_text, &parse_input);
            (card.name, oracle_text, parse_input, db_payload)
        }
        (Some(text), Some(card)) => {
            let db_payload = snapshot_payload_for_db(Some(&card), &card.oracle_text, &text);
            (card.name, card.oracle_text, text, db_payload)
        }
        (Some(text), None) => (name, text.clone(), text, None),
        (None, Some(card)) => {
            let name = card.name.clone();
            let oracle_text = card.oracle_text.clone();
            let parse_input = build_parse_input(&card.metadata_lines, &card.oracle_text);
            let db_payload = snapshot_payload_for_db(Some(&card), &oracle_text, &parse_input);
            (name, oracle_text, parse_input, db_payload)
        }
        (None, None) => {
            return Err(
                "missing oracle text (pass --text or stdin) and no matching card found via --name/--cards"
                    .to_string(),
            )
        }
    };
    let should_write_db = !no_db && db_payload.is_some();

    let builder = CardDefinitionBuilder::new(CardId::new(), &name);
    let def = builder.parse_text(parse_input.clone()).map_err(|err| {
        let _ = store_snapshot_if_requested(should_write_db, db_payload.as_ref(), &db_path);
        format!("parse failed: {err:?}")
    })?;

    println!("Name: {}", def.card.name);
    if detailed {
        println!("Oracle text:");
        println!("{}", oracle_text.trim());
        println!("Parse input:");
        println!("{}", parse_input.trim());
    }
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
    if show_definition {
        println!("Compiled card definition:");
        println!("{:#?}", def);
    }

    store_snapshot_if_requested(should_write_db, db_payload.as_ref(), &db_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith_tools::CardPayload;

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
    fn snapshot_payload_for_db_accepts_canonical_stdin_parse_block() {
        let payload = CardPayload {
            name: "House Cartographer".to_string(),
            oracle_text: "Survival — At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.".to_string(),
            metadata_lines: vec![
                "Mana cost: {1}{G}".to_string(),
                "Type: Creature — Human Scout Survivor".to_string(),
                "Power/Toughness: 2/2".to_string(),
            ],
            parse_input: "Mana cost: {1}{G}\nType: Creature — Human Scout Survivor\nPower/Toughness: 2/2\nSurvival — At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.".to_string(),
        };

        let matched =
            snapshot_payload_for_db(Some(&payload), &payload.oracle_text, &payload.parse_input);

        assert!(
            matched.is_some(),
            "canonical parse block should store a snapshot"
        );
    }

    #[test]
    fn snapshot_payload_for_db_rejects_modified_override_text() {
        let payload = CardPayload {
            name: "House Cartographer".to_string(),
            oracle_text: "Survival — At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.".to_string(),
            metadata_lines: vec![
                "Mana cost: {1}{G}".to_string(),
                "Type: Creature — Human Scout Survivor".to_string(),
                "Power/Toughness: 2/2".to_string(),
            ],
            parse_input: "Mana cost: {1}{G}\nType: Creature — Human Scout Survivor\nPower/Toughness: 2/2\nSurvival — At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.".to_string(),
        };

        let matched =
            snapshot_payload_for_db(Some(&payload), "Modified text", &payload.parse_input);

        assert!(
            matched.is_none(),
            "non-canonical override text should not store a snapshot"
        );
    }
}
