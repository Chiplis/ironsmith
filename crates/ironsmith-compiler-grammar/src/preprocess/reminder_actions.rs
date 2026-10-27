//! Complete, executable reminder descriptions mapped to existing typed actions.
//! A face-down body alone does not grant a turn-up permission: require the
//! matching permission sentence before recognizing manifest or cloak.

pub(super) fn canonicalize(text: &str) -> String {
    if text.contains(['"', '“', '”']) {
        return text.to_owned();
    }
    let lower = text.to_ascii_lowercase();
    let Some((permission_start, permission)) = [
        "turn it face up any time for its mana cost if it's a creature card",
        "turn it face up any time for its mana cost if it’s a creature card",
    ]
    .into_iter()
    .find_map(|permission| lower.find(permission).map(|start| (start, permission))) else {
        return text.to_owned();
    };
    let permission_end = permission_start + permission.len();
    if !lower[permission_end..].is_empty() && !lower[permission_end..].starts_with('.') {
        return text.to_owned();
    }
    let before = lower[..permission_start].trim_end();
    let Some(body) = before.strip_suffix('.') else {
        return text.to_owned();
    };
    for object in [
        "the top card of your library",
        "one of those cards",
        "one of them",
        "one",
    ] {
        let start_phrase = format!("put {object} onto the battlefield face down as a 2/2 creature");
        let Some(start) = body.rfind(&start_phrase) else {
            continue;
        };
        if start > 0 && !body[..start].ends_with([' ', ':']) {
            continue;
        }
        let tail = &body[start + start_phrase.len()..];
        let (keyword, tail) = if let Some(rest) = tail.strip_prefix(" with ward {2}") {
            ("cloak", rest)
        } else {
            ("manifest", tail)
        };
        let remainder = match tail {
            "" => "",
            " and the other into your graveyard" | " and put the other into your graveyard" => {
                " and put the other into your graveyard"
            }
            _ => continue,
        };
        // A bare "one" refers to the viewed group, not an arbitrary object.
        let object = if object == "one" {
            "one of them"
        } else {
            object
        };
        let mut prefix = text[..start].to_owned();
        if object.starts_with("one of") {
            if !lower[..start].contains("look at the top ") {
                continue;
            }
            if prefix.to_ascii_lowercase().ends_with(", then ") {
                prefix.truncate(prefix.len() - ", then ".len());
                prefix.push_str(". ");
            }
        }
        let suffix = &text[permission_end..];
        return format!(
            "{prefix}{keyword} {object}{remainder}{}",
            if suffix.is_empty() { "." } else { suffix }
        );
    }
    text.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    const PERMISSION: &str = "Turn it face up any time for its mana cost if it's a creature card.";

    #[test]
    fn reminder_actions_existing_descriptions_compile_without_keyword_labels() {
        for (card_type, text) in [
            (
                "Creature",
                "This creature can't be blocked except by creatures with flying or reach.",
            ),
            (
                "Creature",
                "This creature can't be blocked except by two or more creatures.",
            ),
            (
                "Instant",
                "Look at the top two cards of your library. Put any number of them on the bottom of your library and the rest on top in any order.",
            ),
            (
                "Instant",
                "Look at the top two cards of your library. Put any number of them into your graveyard and the rest on top of your library in any order.",
            ),
        ] {
            crate::parse_card_text(
                crate::CardDefinitionBuilder::new(ironsmith_core::ids::CardId(1), "Reminder Audit"),
                format!("Type: {card_type}\n{text}"),
            )
            .unwrap_or_else(|error| panic!("{text}: {error:?}"));
        }
    }

    #[test]
    fn reminder_actions_require_the_complete_definition() {
        let body = "Put the top card of your library onto the battlefield face down as a 2/2 creature with ward {2}.";
        assert_eq!(
            canonicalize(&format!("{body} {PERMISSION}")),
            "cloak the top card of your library."
        );
        assert_eq!(canonicalize(body), body);
        assert_eq!(
            canonicalize(&format!("{body} {}", PERMISSION.replace("it's", "it’s"))),
            "cloak the top card of your library."
        );
        for altered in [body.replace("2/2", "3/3"), body.replace("{2}", "{3}")] {
            let text = format!("{altered} {PERMISSION}");
            assert_eq!(canonicalize(&text), text);
        }
        let text = format!(
            "{body} {}",
            PERMISSION.replace("creature card", "artifact card")
        );
        assert_eq!(canonicalize(&text), text);
    }

    #[test]
    fn reminder_actions_preserve_view_count_and_remainder() {
        for object in ["one", "one of them", "one of those cards"] {
            let text = format!(
                "Look at the top four cards of your library, then put {object} onto the battlefield face down as a 2/2 creature with ward {{2}} and the other into your graveyard. {PERMISSION}"
            );
            assert!(
                canonicalize(&text)
                    .starts_with("Look at the top four cards of your library. cloak one of")
            );
            assert!(canonicalize(&text).ends_with("and put the other into your graveyard."));
        }
    }

    #[test]
    fn reminder_actions_compile_to_existing_typed_effects() {
        use crate::{CardDefinitionBuilder, parse_card_text};
        use ironsmith_core::ids::CardId;
        for (ward, keyword) in [(" with ward {2}", "cloak"), ("", "manifest")] {
            for object in ["one", "one of them", "one of those cards"] {
                let text = format!(
                    "Type: Creature — Bird\nFlying\nWhen this creature enters, look at the top two cards of your library, then put {object} onto the battlefield face down as a 2/2 creature{ward} and the other into your graveyard. {PERMISSION}\nPower/Toughness: 1/1"
                );
                let actual = parse_card_text(
                    CardDefinitionBuilder::new(CardId(1), "Reminder Test"),
                    &text,
                )
                .unwrap_or_else(|error| panic!("{text}: {error:?}"));
                let canonical = format!(
                    "Type: Creature — Bird\nFlying\nWhen this creature enters, look at the top two cards of your library. {keyword} one of them and put the other into your graveyard.\nPower/Toughness: 1/1"
                );
                let expected = parse_card_text(
                    CardDefinitionBuilder::new(CardId(1), "Reminder Test"),
                    canonical,
                )
                .unwrap();
                for definition in [actual, expected] {
                    let crate::ability::AbilityKind::Triggered(trigger) =
                        &definition.abilities[1].kind
                    else {
                        panic!("expected enter trigger")
                    };
                    let effects = trigger.effects.all_effects();
                    let look = effects
                        .iter()
                        .find_map(|e| e.downcast_ref::<crate::effects::LookAtTopCardsEffect>())
                        .unwrap();
                    assert_eq!(look.count, crate::effect::Value::Fixed(2));
                    let choose = effects
                        .iter()
                        .find_map(|e| e.downcast_ref::<crate::effects::ChooseObjectsEffect>())
                        .unwrap();
                    assert_eq!(choose.count, crate::effect::ChoiceCount::exactly(1));
                    assert_eq!(choose.filter.zone, Some(crate::zone::Zone::Library));
                    assert!(
                        choose
                            .filter
                            .tagged_constraints
                            .iter()
                            .any(|c| c.tag == look.tag)
                    );
                    let manifest = effects
                        .iter()
                        .find_map(|e| e.downcast_ref::<crate::effects::ManifestObjectsEffect>())
                        .unwrap();
                    assert_eq!(manifest.cloak, keyword == "cloak");
                    assert_eq!(
                        manifest.target,
                        crate::target::ChooseSpec::Tagged(choose.tag.clone())
                    );
                    let rest = effects
                        .iter()
                        .find_map(|e| e.downcast_ref::<crate::effects::TagMatchingObjectsEffect>())
                        .unwrap();
                    assert_eq!(rest.filter.zone, Some(crate::zone::Zone::Library));
                    assert!(
                        rest.filter
                            .tagged_constraints
                            .iter()
                            .any(|c| c.tag == choose.tag
                                && c.relation
                                    == crate::target::TaggedOpbjectRelation::IsNotTaggedObject)
                    );
                    assert!(
                        rest.filter
                            .tagged_constraints
                            .iter()
                            .any(|c| c.tag == look.tag)
                    );
                    let movement = effects
                        .iter()
                        .find_map(|e| e.downcast_ref::<crate::effects::MoveToZoneEffect>())
                        .unwrap();
                    assert_eq!(movement.zone, crate::zone::Zone::Graveyard);
                    assert_eq!(
                        movement.target,
                        crate::target::ChooseSpec::Tagged(rest.tag.clone())
                    );
                }
            }
        }
        for text in [
            "Type: Creature\nAttacking doesn't cause this creature to tap.\nPower/Toughness: 1/1"
                .to_owned(),
            format!(
                "Type: Instant\nPut the top card of your library onto the battlefield face down as a 2/2 creature with ward {{2}}. {PERMISSION}"
            ),
        ] {
            parse_card_text(CardDefinitionBuilder::new(CardId(1), "Reminder Test"), text).unwrap();
        }
    }
}

/// Bind an abbreviated "the last" counter reference only when this paragraph
/// explicitly establishes both the source and the counter being removed.
/// This is a reference expansion; the resulting removal trigger remains an
/// ordinary typed trigger and carries no named mechanic identity.
pub(super) fn bind_last_counter_reference(text: &str) -> String {
    if text.contains(['"', '“', '”']) { return text.to_owned(); }
    let Ok(tokens) = crate::lexer::lex_line(text, 0) else { return text.to_owned(); };
    let sentences = crate::lexer::split_lexed_sentences(&tokens);
    let Some(first) = sentences.first() else { return text.to_owned(); };
    let first_words = crate::lexer::token_word_refs(first);
    let Some(enters) = first_words.iter().position(|word| *word == "enters") else { return text.to_owned(); };
    if !crate::util::is_source_reference_words(&first_words[..enters])
        || first_words.get(enters + 1) != Some(&"with")
        || !first_words.ends_with(&["counters", "on", "it"])
    { return text.to_owned(); }
    let has_last_counter_trigger = sentences.iter().any(|sentence|
        crate::lexer::token_word_refs(sentence).starts_with(&["when", "the", "last", "is", "removed"]));
    let mut counter = None;
    let mut changed = false;
    let mut output = Vec::new();
    for sentence in sentences {
        let words = crate::lexer::token_word_refs(sentence);
        if let Some(remove) = words.iter().position(|word| *word == "remove")
            && words.get(remove + 1) == Some(&"a")
            && words.ends_with(&["counter", "from", "it"])
        {
            let descriptor = words[remove + 2..words.len() - 3].join(" ");
            if !descriptor.is_empty() { counter = Some(descriptor); }
        }
        let rendered = crate::lexer::render_token_slice(crate::util::trim_edge_punctuation_tokens(sentence));
        // CR 702.63a: the shorthand upkeep has an intervening-if. Retain
        // that gate even when no counters remain and the last-counter
        // sacrifice was countered. A custom countdown shares the gate.
        if has_last_counter_trigger
            && words.starts_with(&["at", "the", "beginning", "of", "your", "upkeep", "remove"])
            && let Some(counter) = &counter
            && let Some(comma) = rendered.find(',')
        {
            output.push(format!("{}, if this permanent has a {counter} counter on it,{}", &rendered[..comma], &rendered[comma + 1..]));
            changed = true;
            continue;
        }
        if words.starts_with(&["when", "the", "last", "is", "removed"])
            && let Some(counter) = &counter
            && let Some(comma) = sentence.iter().position(|token| token.is_comma())
            && crate::lexer::token_word_refs(&sentence[..comma]) == ["when", "the", "last", "is", "removed"]
        {
            let body = crate::lexer::render_token_slice(crate::util::trim_edge_punctuation_tokens(&sentence[comma + 1..]));
            output.push(format!("When the last {counter} counter is removed from this permanent, {body}"));
            changed = true;
        } else { output.push(rendered); }
    }
    if changed { format!("{}.", output.join(". ")) } else { text.to_owned() }
}

/// A definition header names the operation being explained; it does not
/// execute that named operation in addition to the definition's body.
/// Preserve cost and timing prose by admitting only a short label before
/// the comma and a complete instruction after it (validated downstream).
pub(super) fn strip_definition_header(text: &str) -> String {
    let Ok(tokens) = crate::lexer::lex_line(text, 0) else { return text.to_owned(); };
    if !tokens.first().is_some_and(|t| t.is_word("to")) { return text.to_owned(); }
    let Some(comma) = tokens.iter().position(|t| t.is_comma()) else { return text.to_owned(); };
    if !(2..=5).contains(&comma) || tokens[1..comma].iter().any(|t| t.as_word().is_none() && !t.slice.chars().all(|c| c.is_ascii_digit())) {
        return text.to_owned();
    }
    let label = crate::lexer::token_word_refs(&tokens[1..comma]);
    if label.iter().any(|word| matches!(*word, "cast" | "activate" | "pay" | "prevent" | "attack" | "block" | "do" | "you" | "your" | "this" | "that")) {
        return text.to_owned();
    }
    if !tokens.get(comma + 1).is_some_and(|t| t.is_any_word(&[
        "look", "choose", "reveal", "put", "draw", "discard", "exile", "create", "remove", "add", "return", "tap", "untap",
    ])) { return text.to_owned(); }
    // Work from the original span so quoted rules in the definition survive.
    text[tokens[comma].span.end..].trim_start().to_owned()
}

#[cfg(test)]
mod definition_binding_tests {
    use super::*;
    #[test]
    fn generic_headers_preserve_the_instruction_and_reject_cost_purposes() {
        for name in ["surveil", "foresight", "deep sight"] {
            let body = "look at the top two cards of your library, then put any number of them into your graveyard and the rest on top in any order.";
            assert_eq!(strip_definition_header(&format!("To {name} 2, {body}")), body);
        }
        for text in ["To cast this spell, exile a card.", "To pay {1}, tap a creature.", "To prevent damage, discard a card.", "To forecast, something unsupported happens."] {
            assert_eq!(strip_definition_header(text), text);
        }
    }
    #[test]
    fn last_counter_reference_is_local_and_requires_explicit_source() {
        let text = "This enchantment enters with twelve time counters on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, sacrifice it.";
        let bound = bind_last_counter_reference(text);
        assert!(bound.contains("if this permanent has a time counter on it"));
        assert!(bound.contains("When the last time counter is removed from this permanent"));
        let unknown = "When the last is removed, sacrifice it.";
        assert_eq!(bind_last_counter_reference(unknown), unknown);
        let fading = "This enchantment enters with seven fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.";
        assert_eq!(bind_last_counter_reference(fading), fading);
    }
}

/// The complete Vehicle-animation reminder abbreviates CR 702.122a's
/// "other untapped" payment. Expand that shorthand without a Crew identity.
/// The attachment sentence restates the global creature-attachment rule.
pub(super) fn expand_vehicle_payment(text: &str) -> String {
    if text.contains(['"', '“', '”']) { return text.to_owned(); }
    let lower = text.trim().to_ascii_lowercase();
    let Some((payment, body)) = lower.split_once(':') else { return text.to_owned(); };
    let Some(amount) = payment.trim().strip_prefix("tap any number of creatures you control with total power ")
        .and_then(|s| s.strip_suffix(" or more").or_else(|| s.strip_suffix(" or greater"))) else { return text.to_owned(); };
    if crate::util::parse_number_word_u32(amount).is_none() { return text.to_owned(); }
    let body = body.trim().trim_end_matches('.');
    if !matches!(body,
        "this vehicle becomes an artifact creature until end of turn"
        | "this vehicle becomes an artifact creature until end of turn. creatures can't be attached to other permanents"
        | "this vehicle becomes an artifact creature until end of turn. creatures can’t be attached to other permanents") {
        return text.to_owned();
    }
    format!("Tap any number of other untapped creatures you control with total power {amount} or more: This Vehicle becomes an artifact creature until end of turn.")
}

#[cfg(test)]
mod vehicle_definition_tests {
    use super::*;
    #[test]
    fn complete_vehicle_shorthand_expands_only_the_defined_payment() {
        let body = "This Vehicle becomes an artifact creature until end of turn.";
        let input = format!("Tap any number of creatures you control with total power 2 or more: {body} Creatures can't be attached to other permanents.");
        assert_eq!(expand_vehicle_payment(&input), format!("Tap any number of other untapped creatures you control with total power 2 or more: {body}"));
        for unrelated in [
            input.replace("artifact creature until end of turn", "artifact creature"),
            input.replace("Creatures can't be attached", "Creatures can be attached"),
            input.replace("This Vehicle", "Target Vehicle"),
            input.replace("you control", "an opponent controls"),
        ] {
            assert_eq!(expand_vehicle_payment(&unrelated), unrelated);
        }
    }
}

/// Printed explanatory sentences for global payment rules are documentation,
/// not additional abilities. Only consume a complete known rule after an instruction.
pub(super) fn strip_global_payment_explanation(text: &str) -> String {
    if text.contains(['"', '“', '”']) { return text.to_owned(); }
    let lower = text.to_ascii_lowercase();
    for rule in ["{s} can be paid with one mana from a snow source.", "copies become tokens."] {
        if rule == "copies become tokens." && !lower.contains("copy it") { continue; }
        if let Some(prefix) = lower.strip_suffix(rule) {
            let prefix = prefix.trim_end();
            if prefix.ends_with('.') && !prefix.is_empty() {
                return text[..prefix.len()].to_owned();
            }
        }
    }
    text.to_owned()
}

/// Put a postposed entry trigger before its condition so document grouping
/// keeps the entry replacement and triggered instruction as separate abilities.
pub(super) fn front_postposed_entry_trigger(text: &str) -> String {
    if text.contains(['"', '“', '”']) { return text.to_owned(); }
    let Ok(tokens) = crate::lexer::lex_line(text, 0) else { return text.to_owned(); };
    let sentences = crate::lexer::split_lexed_sentences(&tokens);
    if sentences.len() != 2 { return text.to_owned(); }
    let first = crate::lexer::token_word_refs(sentences[0]);
    let Some(enters) = first.iter().position(|word| *word == "enters") else { return text.to_owned(); };
    if !crate::util::is_source_reference_words(&first[..enters]) { return text.to_owned(); }
    let mut changed = false;
    let mut result = Vec::new();
    for sentence in sentences {
        let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
        let words = crate::lexer::token_word_refs(sentence);
        if words.first() == Some(&"if") && words.ends_with(&["when", "it", "enters"])
            && let Some(comma) = sentence.iter().position(|token| token.is_comma())
            && let Some(when) = sentence.iter().rposition(|token| token.is_word("when"))
            && when > comma + 1
        {
            let condition = crate::lexer::render_token_slice(&sentence[1..comma]);
            let effect = crate::lexer::render_token_slice(&sentence[comma + 1..when]);
            result.push(format!("When this permanent enters, if {condition}, {effect}"));
            changed = true;
        } else { result.push(crate::lexer::render_token_slice(sentence)); }
    }
    if changed { format!("{}.", result.join(". ")) } else { text.to_owned() }
}

/// Restore the counter kind omitted by an adjacent numeric type threshold.
/// The tapped object's characteristic remains an ordinary cost reference, so
/// effects that specifically modify Station do not modify a custom ability.
pub(super) fn expand_counter_threshold_activation(text: &str) -> Option<Vec<String>> {
    if text.contains(['"', '“', '”']) { return None; }
    let tokens = crate::lexer::lex_line(&text.to_ascii_lowercase(), 0).ok()?;
    let sentences = crate::lexer::split_lexed_sentences(&tokens);
    let [activation, timing, threshold] = sentences.as_slice() else { return None; };
    let colon = activation.iter().position(|token| token.kind == crate::lexer::TokenKind::Colon)?;
    let cost_words = crate::lexer::token_word_refs(&activation[..colon]);
    if !matches!(cost_words.as_slice(), ["tap", "another", "creature", "you", "control"] | ["tap", "another", "untapped", "creature", "you", "control"]) { return None; }
    let effect = &activation[colon + 1..];
    let view = crate::lexer::TokenWordView::new(effect);
    let words = view.word_refs();
    if words.first() != Some(&"put") { return None; }
    let counters = words.iter().position(|word| *word == "counters")?;
    if words.get(counters + 1..counters + 6)? != ["equal", "to", "its", "power", "on"]
        || !crate::util::is_source_reference_words(&words[counters + 6..]) { return None; }
    crate::grammar::filters::parse_counter_type_words(&words[1..=counters])?;
    let counter = crate::lexer::render_token_slice(&effect[view.token_start_indices()[1]..view.token_start_indices()[counters]]);
    let subject = crate::lexer::render_token_slice(crate::util::trim_edge_punctuation_tokens(&effect[view.token_start_indices()[counters + 6]..]));
    let timing_words = crate::lexer::token_word_refs(timing);
    if !timing_words.ends_with(&["only", "as", "a", "sorcery"]) { return None; }
    let threshold = crate::util::trim_edge_punctuation_tokens(threshold);
    let threshold_words = crate::lexer::token_word_refs(threshold);
    let ["it's" | "its", "an", "artifact", "creature", "at", amount] = threshold_words.as_slice() else { return None; };
    let amount = amount.parse::<u32>().ok()?;
    if !crate::lexer::render_token_slice(threshold).trim_end().ends_with('+') { return None; }
    Some(vec![
        format!("{}: Put X {counter} counters on {subject}, where X is the power of the creature tapped this way. Activate only as a sorcery.", crate::lexer::render_token_slice(&activation[..colon])),
        format!("As long as this permanent has {amount} or more {counter} counters on it, this permanent is an artifact creature in addition to its other types."),
    ])
}

/// A semicolon separates two activations sharing one printed price. Expand
/// the complete reminder before ordinary activation parsing, preserving both
/// the targetless detach action and its attached-only restriction.
pub(super) fn expand_attachment_pair(text: &str) -> Option<Vec<String>> {
    if text.contains(['"', '“', '”']) { return None; }
    let tokens = crate::lexer::lex_line(&text.to_ascii_lowercase(), 0).ok()?;
    let sentences = crate::lexer::split_lexed_sentences(&tokens);
    let [activation, timing, attached] = sentences.as_slice() else { return None; };
    let colon = activation.iter().position(|token| token.kind == crate::lexer::TokenKind::Colon)?;
    let body = crate::lexer::token_word_refs(&activation[colon + 1..]);
    if body != ["attach", "to", "target", "creature", "you", "control", "or", "unattach", "from", "a", "creature"] { return None; }
    if !activation.iter().any(|token| token.kind == crate::lexer::TokenKind::Semicolon) { return None; }
    if !crate::lexer::token_word_refs(timing).ends_with(&["only", "as", "a", "sorcery"]) { return None; }
    if !matches!(crate::lexer::token_word_refs(attached).as_slice(), ["while", "attached", "this", "isn't" | "isnt", "a", "creature"]) { return None; }
    let cost = crate::lexer::render_token_slice(&activation[..colon]);
    Some(vec![
        format!("{cost}: Attach this permanent to another target creature you control. Activate only as a sorcery."),
        format!("{cost}: Unattach this permanent. Activate only as a sorcery. Activate only if this Equipment is attached to a creature."),
        "This permanent can be attached while it's a creature.".into(),
        "While attached, this isn't a creature.".into(),
    ])
}
