use std::collections::HashMap;
use std::io::{self, Read};

use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::cards::builders::CardTextError;
use ironsmith::ids::CardId;

#[derive(Debug, Clone)]
struct FailedCard {
    len: usize,
    name: String,
    text: String,
    card_type: String,
}

fn classify_card_type(type_line: &str) -> String {
    let left = type_line.split('—').next().unwrap_or(type_line);
    let mut types = Vec::new();
    for raw in left.split_whitespace() {
        let token = raw.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
        if token.is_empty() {
            continue;
        }
        match token {
            "Land" | "Creature" | "Planeswalker" | "Battle" | "Artifact" | "Enchantment"
            | "Instant" | "Sorcery" | "Kindred" => {
                if !types.contains(&token) {
                    types.push(token);
                }
            }
            _ => {}
        }
    }

    if types.is_empty() {
        "Unknown".to_string()
    } else {
        types.join(" ")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pattern: Option<String> = None;
    let mut allow_unsupported = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--pattern"
            && let Some(value) = args.next()
        {
            pattern = Some(value);
        } else if arg == "--allow-unsupported" {
            allow_unsupported = true;
        }
    }

    if allow_unsupported {
        unsafe {
            std::env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", "1");
        }
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let mut total = 0u32;
    let mut ok = 0u32;
    let mut failed = 0u32;
    let mut parse_error_counts: HashMap<String, u32> = HashMap::new();
    let mut unsupported_counts: HashMap<String, u32> = HashMap::new();
    let mut pattern_examples: Vec<String> = Vec::new();
    let mut failed_cards: Vec<FailedCard> = Vec::new();
    let mut failed_by_type: HashMap<String, Vec<usize>> = HashMap::new();

    for block in input.split("\n---\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let mut lines = block.lines();
        let name_line = lines.next().unwrap_or_default();
        let name = name_line.strip_prefix("Name: ").unwrap_or(name_line).trim();
        if name.is_empty() {
            continue;
        }

        let mut type_line: Option<String> = None;
        let mut text_lines: Vec<String> = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("type:") {
                let value = trimmed[trimmed.len() - rest.len()..].trim();
                type_line = Some(value.to_string());
            } else if let Some(rest) = lower.strip_prefix("type line:") {
                let value = trimmed[trimmed.len() - rest.len()..].trim();
                type_line = Some(value.to_string());
            }
            text_lines.push(line.to_string());
        }
        let text = text_lines.join("\n");
        total += 1;

        let builder = CardDefinitionBuilder::new(CardId::new(), name);
        match builder.parse_text(text.as_str()) {
            Ok(_) => ok += 1,
            Err(CardTextError::ParseError(message)) => {
                failed += 1;
                if let Some(ref pattern) = pattern
                    && pattern_examples.len() < 10
                    && message.contains(pattern)
                {
                    pattern_examples.push(name.to_string());
                }
                *parse_error_counts.entry(message).or_insert(0) += 1;
                let card_type = type_line
                    .as_deref()
                    .map(classify_card_type)
                    .unwrap_or_else(|| "Unknown".to_string());
                failed_cards.push(FailedCard {
                    len: text.len(),
                    name: name.to_string(),
                    text,
                    card_type: card_type.clone(),
                });
                let idx = failed_cards.len() - 1;
                failed_by_type.entry(card_type).or_default().push(idx);
            }
            Err(CardTextError::InvariantViolation(message)) => {
                failed += 1;
                let labeled = format!("invariant violation: {message}");
                *parse_error_counts.entry(labeled).or_insert(0) += 1;
                let card_type = type_line
                    .as_deref()
                    .map(classify_card_type)
                    .unwrap_or_else(|| "Unknown".to_string());
                failed_cards.push(FailedCard {
                    len: text.len(),
                    name: name.to_string(),
                    text,
                    card_type: card_type.clone(),
                });
                let idx = failed_cards.len() - 1;
                failed_by_type.entry(card_type).or_default().push(idx);
            }
            Err(CardTextError::UnsupportedLine(message)) => {
                failed += 1;
                if let Some(ref pattern) = pattern
                    && pattern_examples.len() < 10
                    && message.contains(pattern)
                {
                    pattern_examples.push(name.to_string());
                }
                *unsupported_counts.entry(message).or_insert(0) += 1;
                let card_type = type_line
                    .as_deref()
                    .map(classify_card_type)
                    .unwrap_or_else(|| "Unknown".to_string());
                failed_cards.push(FailedCard {
                    len: text.len(),
                    name: name.to_string(),
                    text,
                    card_type: card_type.clone(),
                });
                let idx = failed_cards.len() - 1;
                failed_by_type.entry(card_type).or_default().push(idx);
            }
        }
    }

    println!("Total: {total} Ok: {ok} Failed: {failed}");
    if !failed_cards.is_empty() {
        let mut shortest_indices: Vec<usize> = (0..failed_cards.len()).collect();
        shortest_indices.sort_by(|a_idx, b_idx| {
            let a = &failed_cards[*a_idx];
            let b = &failed_cards[*b_idx];
            a.len.cmp(&b.len).then_with(|| a.name.cmp(&b.name))
        });
        println!("Shortest 20 failed cards (by oracle text length):");
        for idx in shortest_indices.into_iter().take(20) {
            let card = &failed_cards[idx];
            println!("- {} [{}] ({} chars)", card.name, card.card_type, card.len);
            println!("{}", card.text);
            println!();
        }
    }
    if !failed_by_type.is_empty() {
        let mut type_groups: Vec<(String, Vec<usize>)> = failed_by_type.into_iter().collect();
        type_groups.sort_by(|(a_type, a_idx), (b_type, b_idx)| {
            b_idx
                .len()
                .cmp(&a_idx.len())
                .then_with(|| a_type.cmp(b_type))
        });
        println!("Failed cards by type (shortest 10 per type):");
        for (card_type, mut indices) in type_groups {
            indices.sort_by(|a_idx, b_idx| {
                let a = &failed_cards[*a_idx];
                let b = &failed_cards[*b_idx];
                a.len.cmp(&b.len).then_with(|| a.name.cmp(&b.name))
            });
            println!("- {} ({} failed)", card_type, indices.len());
            for idx in indices.into_iter().take(10) {
                let card = &failed_cards[idx];
                println!("  - {} ({} chars)", card.name, card.len);
            }
        }
    }
    if !parse_error_counts.is_empty() {
        let mut parse_errors: Vec<(String, u32)> = parse_error_counts.into_iter().collect();
        parse_errors.sort_by(|(a_msg, a_count), (b_msg, b_count)| {
            b_count.cmp(a_count).then_with(|| a_msg.cmp(b_msg))
        });
        println!("ParseErrors by count:");
        for (message, count) in parse_errors {
            println!("- {}x: {}", count, message);
        }
    }
    if !unsupported_counts.is_empty() {
        let mut unsupported: Vec<(String, u32)> = unsupported_counts.into_iter().collect();
        unsupported.sort_by(|(a_msg, a_count), (b_msg, b_count)| {
            b_count.cmp(a_count).then_with(|| a_msg.cmp(b_msg))
        });
        println!("Unsupported lines by count:");
        for (message, count) in unsupported {
            println!("- {}x: {}", count, message);
        }
    }
    if let Some(pattern) = pattern {
        if pattern_examples.is_empty() {
            println!("No examples found for pattern: {}", pattern);
        } else {
            println!("Examples for pattern '{}':", pattern);
            for name in pattern_examples {
                println!("- {}", name);
            }
        }
    }

    Ok(())
}
