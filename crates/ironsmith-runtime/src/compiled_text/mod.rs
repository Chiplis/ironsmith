#![allow(unused_imports)]

use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{
    ChoiceCount, Comparison, Condition, EffectPredicate, EventValueSpec, Until, Value,
};
use crate::effect_text_shared;
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{Subtype, Supertype};
use crate::{CardDefinition, CardType, Effect, ManaSymbol, TagKey, Zone};

mod ast_render;
mod debug_safe;
mod merge_passes;
mod normalize_common;
mod oracle_style;
mod render_effects;
mod surface_helpers;

use self::ast_render::*;
use self::merge_passes::*;
use self::normalize_common::*;
use self::oracle_style::*;
use self::render_effects::*;
use self::surface_helpers::*;

pub(crate) use self::normalize_common::{
    describe_counter_for_each_basis, describe_death_history_subject,
    describe_party_size_for_each_basis, describe_turn_history_for_each_basis, describe_value,
    party_size_multiplier,
};
pub use self::oracle_style::canonical_compiled_lines;
pub use self::render_effects::compile_effect_list;

pub(crate) fn pluralize_noun_phrase_for_trigger(phrase: &str) -> String {
    self::render_effects::pluralize_noun_phrase(phrase)
}

const CLEAVE_BRACKET_OPEN_SENTINEL: &str = "\u{e000}";
const CLEAVE_BRACKET_CLOSE_SENTINEL: &str = "\u{e001}";

fn restore_cleave_bracket_surface(line: String) -> String {
    line.replace(CLEAVE_BRACKET_OPEN_SENTINEL, "[")
        .replace(CLEAVE_BRACKET_CLOSE_SENTINEL, "]")
}

fn debug_compiled_surface_lines(def: &CardDefinition) -> Vec<String> {
    debug_safe::normalize_debug_safe_surface(ast_compiled_lines(def))
        .into_iter()
        .map(debug_safe::DebugSafeLine::into_string)
        .collect()
}

/// Render the structured runtime model for debug/inspector use.
pub fn debug_compiled_lines(def: &CardDefinition) -> Vec<String> {
    debug_compiled_surface_lines(def)
        .into_iter()
        .map(restore_cleave_bracket_surface)
        .collect()
}

/// Render the structured compiled-text surface used for DB scoring.
pub fn compiled_text_lines(def: &CardDefinition) -> Vec<String> {
    let oracle_short = oracle_short_self_name(def);
    let lines = normalize_ast_surface_lines(debug_compiled_surface_lines(def))
        .into_iter()
        .map(|line| {
            substitute_legendary_source_reference(&line, &def.card, "", oracle_short.as_deref())
        })
        .map(|line| substitute_spell_caster_source_reference(&line, def))
        .map(|line| substitute_kicked_draw_source_reference(&line, def))
        .collect();
    let lines = compact_post_substitution_surface_lines(lines)
        .into_iter()
        .map(normalize_scored_compiled_line)
        .map(|line| normalize_punctuated_card_name_damage_case(line, &def.card.name))
        .map(restore_cleave_bracket_surface)
        .collect();
    prefix_attraction_visit_surface(def, lines)
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    let oracle_short = oracle_short_self_name(def);
    let lines = normalize_ast_surface_lines(debug_compiled_surface_lines(def))
        .into_iter()
        .map(|line| {
            substitute_legendary_source_reference(&line, &def.card, "", oracle_short.as_deref())
        })
        .map(|line| substitute_spell_caster_source_reference(&line, def))
        .collect();
    let lines = compact_post_substitution_surface_lines(lines)
        .into_iter()
        .map(normalize_unprocessed_compiled_line)
        .map(|line| normalize_duplicate_optional_subject(&line))
        .map(|line| normalize_punctuated_card_name_damage_case(line, &def.card.name))
        .map(restore_cleave_bracket_surface)
        .collect();
    prefix_attraction_visit_surface(def, lines)
}

/// A card name's terminal `!` or `?` is part of the name, not a sentence
/// boundary. Generic sentence casing cannot know that without card context,
/// so restore the verb casing after the final normalization pass.
fn normalize_punctuated_card_name_damage_case(line: String, card_name: &str) -> String {
    if !card_name.ends_with('!') && !card_name.ends_with('?') {
        return line;
    }
    line.replace(
        &format!("{card_name} Deals "),
        &format!("{card_name} deals "),
    )
    .replace(&format!("{card_name} Deal "), &format!("{card_name} deal "))
}

fn prefix_attraction_visit_surface(def: &CardDefinition, mut lines: Vec<String>) -> Vec<String> {
    if def.card.subtypes.contains(&Subtype::Attraction)
        && let Some(line) = lines.iter_mut().find(|line| !line.trim().is_empty())
        && !line.starts_with("Visit — ")
    {
        *line = format!("Visit — {line}");
    }
    lines
}

/// Render a single ability using the same surface renderer as compiled oracle text.
pub fn ability_surface_text(ability: &Ability) -> String {
    if let Some(keyword) = self::render_effects::describe_keyword_ability(ability) {
        return keyword;
    }
    self::render_effects::describe_inline_ability(ability)
}

fn substitute_spell_caster_source_reference(line: &str, def: &CardDefinition) -> String {
    if !(def.card.is_instant() || def.card.is_sorcery()) || def.card.name.contains(" // ") {
        return line.to_string();
    }
    line.replace(
        "the player who cast this spell",
        &format!("the player who cast {}", def.card.name),
    )
}

fn normalize_ast_surface_lines(lines: Vec<String>) -> Vec<String> {
    let lines: Vec<String> = lines
        .into_iter()
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect();
    let lines = merge_ast_surface_lines(lines)
        .into_iter()
        .map(finalize_ast_surface_line)
        .flat_map(expand_finalized_ast_surface_line)
        .map(normalize_mass_opponent_controller_surface)
        .collect();
    compact_threshold_ability_word_lines(compact_station_threshold_lines(lines))
}

fn compact_threshold_ability_word_lines(lines: Vec<String>) -> Vec<String> {
    let mut compacted = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some(compact) =
                compact_threshold_pump_and_cant_block(&lines[idx], &lines[idx + 1])
        {
            compacted.push(compact);
            idx += 2;
            continue;
        }
        compacted.push(lines[idx].clone());
        idx += 1;
    }
    compacted
}

fn compact_threshold_pump_and_cant_block(pump_line: &str, cant_block_line: &str) -> Option<String> {
    const CONDITION: &str = "there are seven or more cards in your graveyard";
    let pump = pump_line.trim().trim_end_matches('.');
    let cant_block = cant_block_line.trim().trim_end_matches('.');
    let (pump_body, pump_condition) = pump.rsplit_once(" as long as ")?;
    let (cant_block_body, cant_block_condition) = cant_block.rsplit_once(" as long as ")?;
    if !pump_condition.eq_ignore_ascii_case(CONDITION)
        || !cant_block_condition.eq_ignore_ascii_case(CONDITION)
        || cant_block_body != "This creature can't block"
    {
        return None;
    }
    let pump_value = pump_body.strip_prefix("This creature gets ")?;
    Some(format!(
        "Threshold — As long as {pump_condition}, this creature gets {pump_value} and can't block."
    ))
}

fn compact_post_substitution_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut compacted = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 2 < lines.len()
            && let Some(compact) = compact_conditional_source_animation_bundle(
                &lines[idx],
                &lines[idx + 1],
                &lines[idx + 2],
            )
        {
            compacted.push(compact);
            idx += 3;
            continue;
        }

        compacted.push(lines[idx].clone());
        idx += 1;
    }
    compacted
}

fn compact_conditional_source_animation_bundle(
    animation_line: &str,
    keyword_line: &str,
    ability_line: &str,
) -> Option<String> {
    let (base_pt, subtype, condition) = parse_conditional_source_animation_line(animation_line)?;
    let (subject, keyword, keyword_condition) = parse_conditional_keyword_line(keyword_line)?;
    if !condition.eq_ignore_ascii_case(&keyword_condition) {
        return None;
    }
    let (ability, ability_condition) = parse_conditional_quoted_ability_line(ability_line)?;
    if !condition.eq_ignore_ascii_case(&ability_condition) {
        return None;
    }

    let condition_prefix = if is_celebration_condition(&condition) {
        format!("Celebration — As long as {condition}")
    } else {
        format!("As long as {condition}")
    };
    let terminal = if ability.ends_with(".\"") { "" } else { "." };
    Some(format!(
        "{condition_prefix}, {subject} is {} {} with base power and toughness {base_pt}, {keyword}, and {ability}{terminal}",
        article_for_lowercase_phrase(&subtype),
        capitalize_first(&subtype),
    ))
}

fn parse_conditional_source_animation_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let prefix = "this creature source is creature in addition to its other types and has base power and toughness ";
    let rest = lower.strip_prefix(prefix)?;
    let (base_pt, tail) = rest.split_once(" and is ")?;
    let (subtype, condition) = tail.rsplit_once(" as long as ")?;
    if !base_pt.contains('/') || subtype.trim().is_empty() || condition.trim().is_empty() {
        return None;
    }
    Some((
        base_pt.trim().to_string(),
        subtype.trim().to_string(),
        condition.trim().to_string(),
    ))
}

fn parse_conditional_keyword_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let (body, condition) = trimmed.rsplit_once(" as long as ")?;
    let (subject, keyword) = body.split_once(" has ")?;
    let keyword = keyword.trim();
    if subject.trim().is_empty() || keyword.is_empty() {
        return None;
    }
    Some((
        subject.trim().to_string(),
        normalize_keyword_predicate_case(keyword),
        condition.trim().to_string(),
    ))
}

fn parse_conditional_quoted_ability_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let marker = lower
        .rfind(" as long as ")
        .or_else(|| lower.rfind(" As long as "))?;
    let ability_body = trimmed[..marker].trim();
    let condition = trimmed[marker + " as long as ".len()..].trim();
    let ability = ability_body
        .strip_prefix("This source has ")
        .or_else(|| ability_body.strip_prefix("this source has "))
        .or_else(|| ability_body.split_once(" has ").map(|(_, ability)| ability))?
        .trim();
    if !ability.starts_with('"') || condition.is_empty() {
        return None;
    }
    Some((ability.to_string(), condition.to_string()))
}

fn is_celebration_condition(condition: &str) -> bool {
    let lower = condition.to_ascii_lowercase();
    lower.contains(
        "two or more nonland permanents entered the battlefield under your control this turn",
    )
}

fn article_for_lowercase_phrase(phrase: &str) -> &'static str {
    match phrase.chars().next().map(|ch| ch.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn compact_station_threshold_lines(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.trim() == "Station") {
        return lines;
    }

    let mut compacted = Vec::with_capacity(lines.len());
    let mut pending_keyword_threshold: Option<i32> = None;
    let mut pending_keywords: Vec<String> = Vec::new();

    for line in lines {
        let Some((threshold, body)) = split_station_threshold_condition(&line) else {
            flush_station_keyword_row(
                &mut compacted,
                &mut pending_keyword_threshold,
                &mut pending_keywords,
            );
            compacted.push(line);
            continue;
        };

        if is_station_implicit_creature_support(&body) {
            continue;
        }

        if let Some(keyword) = station_threshold_keyword_body(&body) {
            if pending_keyword_threshold == Some(threshold) {
                pending_keywords.push(keyword);
            } else {
                flush_station_keyword_row(
                    &mut compacted,
                    &mut pending_keyword_threshold,
                    &mut pending_keywords,
                );
                pending_keyword_threshold = Some(threshold);
                pending_keywords.push(keyword);
            }
            continue;
        }

        flush_station_keyword_row(
            &mut compacted,
            &mut pending_keyword_threshold,
            &mut pending_keywords,
        );
        compacted.push(format!("{threshold}+ | {}", station_threshold_body(&body)));
    }

    flush_station_keyword_row(
        &mut compacted,
        &mut pending_keyword_threshold,
        &mut pending_keywords,
    );
    compacted
}

fn flush_station_keyword_row(
    out: &mut Vec<String>,
    pending_threshold: &mut Option<i32>,
    pending_keywords: &mut Vec<String>,
) {
    let Some(threshold) = pending_threshold.take() else {
        return;
    };
    if !pending_keywords.is_empty() {
        out.push(format!("{threshold}+ | {}", pending_keywords.join(", ")));
        pending_keywords.clear();
    }
}

fn split_station_threshold_condition(line: &str) -> Option<(i32, String)> {
    const MARKER: &str = " as long as CountersOnSource is greater than or equal to ";
    let trimmed = line.trim().trim_end_matches('.');
    if let Some((body, threshold_text)) = trimmed.rsplit_once(MARKER) {
        let threshold = parse_station_threshold_value(threshold_text.trim())?;
        return Some((threshold, body.trim().to_string()));
    }

    if let Some((body, condition)) = trimmed.rsplit_once(" as long as ")
        && let Some(threshold) = parse_station_charge_counter_condition(condition)
    {
        return Some((threshold, body.trim().to_string()));
    }

    const PREFIX: &str = "As long as CountersOnSource is greater than or equal to ";
    if let Some(rest) = trimmed.strip_prefix(PREFIX) {
        let (threshold_text, body) = rest.split_once(", ")?;
        let threshold = parse_station_threshold_value(threshold_text.trim())?;
        return Some((threshold, body.trim().to_string()));
    }

    let rest = trimmed.strip_prefix("As long as ")?;
    let (condition, body) = rest.split_once(", ")?;
    let threshold = parse_station_charge_counter_condition(condition)?;
    Some((threshold, body.trim().to_string()))
}

fn parse_station_charge_counter_condition(condition: &str) -> Option<i32> {
    let lower = condition.trim().to_ascii_lowercase();
    let threshold = [
        "the number of charge counters on this source is ",
        "the number of charge counters on this permanent is ",
        "the number of charge counters on this artifact is ",
        "the number of charge counters on this creature is ",
    ]
    .into_iter()
    .find_map(|prefix| lower.strip_prefix(prefix))?
    .strip_suffix(" or greater")?;
    parse_station_threshold_value(threshold.trim())
}

fn parse_station_threshold_value(text: &str) -> Option<i32> {
    if let Ok(value) = text.parse::<i32>() {
        return Some(value);
    }
    let words = text.split_whitespace().collect::<Vec<_>>();
    let (value, used) = ironsmith_core::parse_cardinal_words(&words)?;
    (used == words.len()).then_some(value as i32)
}

fn is_station_implicit_creature_support(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    (lower.contains(" is creature in addition to ")
        || lower.contains(" is a creature in addition to "))
        || lower.contains(" has base power and toughness ")
}

fn station_threshold_keyword_body(body: &str) -> Option<String> {
    let body = station_threshold_body(body);
    let lower = body.to_ascii_lowercase();
    let keyword = [
        "this artifact creature has ",
        "this artifact has ",
        "this source has ",
        "this creature has ",
    ]
    .into_iter()
    .find_map(|prefix| lower.starts_with(prefix).then(|| &body[prefix.len()..]))
    .unwrap_or(body.as_str())
    .trim();
    let normalized = normalize_keyword_predicate_case(keyword);
    let keyword_parts = normalized
        .split(" and ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if keyword_parts.len() > 1 && keyword_parts.iter().all(|part| is_keyword_phrase(part)) {
        return Some(capitalize_first(&keyword_parts.join(", ")));
    }
    if !is_keyword_phrase(&normalized) {
        return None;
    }
    Some(capitalize_first(&normalized))
}

fn station_threshold_body(body: &str) -> String {
    let body = body.trim().trim_end_matches('.');
    if let Some(rest) = body.strip_prefix("This artifact source ") {
        return format!("This artifact {rest}");
    }
    body.to_string()
}

fn normalize_scored_compiled_line(line: String) -> String {
    // Effect-list renderers intentionally lowercase clauses when composing
    // them into a larger sentence. At the card-line boundary, restore normal
    // sentence capitalization before applying the surface normalizers.
    let line = normalize_duplicate_optional_subject(&capitalize_first(
        &capitalize_sentence_boundaries(&line),
    ));
    // The Wish family's search-outside-the-game program only reaches its
    // final three-sentence surface after the earlier merge passes; fold it
    // to the authored reveal-and-put sentence here.
    let line = normalize_common::normalize_search_outside_game_reveal_surface(&line);
    // A bare repeat instruction after an optional step loops on taking the
    // option; oracle spells the "If you do," gate.
    let line = if let Some(head) = line
        .strip_suffix(". Repeat this process.")
        .or_else(|| line.strip_suffix(". Repeat this process"))
        && head
            .rsplit(". ")
            .next()
            .is_some_and(|prev| prev.starts_with("You may ") || prev.starts_with("you may "))
    {
        format!("{head}. If you do, repeat this process.")
    } else {
        line
    };
    let lower_for_common = line.to_ascii_lowercase();
    let line = if lower_for_common.contains("you search your library")
        || lower_for_common.contains("choose an other card")
        || lower_for_common.contains("choose any number red cards")
        || lower_for_common.contains("you search its controller's graveyard")
        || lower_for_common.contains("you search target opponent's graveyard")
        || lower_for_common.contains("unless it's a permanent")
        || lower_for_common.contains("where x is the number of")
        || lower_for_common
            .contains("each player shuffles their hand and graveyard into their library")
        || lower_for_common.contains("sacrifice all permanents")
        || lower_for_common.contains("if you do, choose")
        || lower_for_common.contains("discard a card at random, then shuffle your library")
        || lower_for_common.contains("dynamic value or less")
        || lower_for_common.contains("other coward creatures")
        || lower_for_common.contains("that object's controller")
        || lower_for_common.contains("a ally you control")
        || lower_for_common.contains("defending player's creature")
        || lower_for_common.contains("creature defending player controls")
        || lower_for_common.contains("you controls for the first time each turn")
        || lower_for_common.contains("then if not, put it into its owner's hand")
        || lower_for_common.contains("under target opponent's control")
        || lower_for_common.contains("whenever a player casts a spell, draw a card")
        || lower_for_common.contains("dynamic value of their choice")
        || lower_for_common.contains("return all cards from your graveyard to your hand")
        || lower_for_common.contains("permanent that shares a card type with that object")
        || lower_for_common
            .contains("a permanent that shares a card type with it was revealed this way")
        || lower_for_common.contains("target colored creature")
        || lower_for_common.contains("repeat roll a d6")
        || lower_for_common.contains("behold cost")
        || lower_for_common.contains("choose a dragon card")
        || lower_for_common.contains("reveals cards from the top of target opponent's library")
        || lower_for_common.contains("effect #0")
        || lower_for_common.contains("copy that spell the number of spells time")
        || lower_for_common.contains("life total is greater than or equal")
        || lower_for_common.contains("if your life total is ")
        || lower_for_common.contains("gain control of each other creature until end of turn")
        || lower_for_common.contains("exactly 3 cards")
        || lower_for_common.contains("choose any number white cards")
        || lower_for_common.contains("clash with an opponent")
        || lower_for_common.contains("all colored permanents")
        || lower_for_common.contains("you choose a card. put it into its owner's graveyard")
        || lower_for_common.contains("players have hexproof")
        || lower_for_common.contains("mills ten cards")
        || lower_for_common.contains("draw its mana value cards")
        || lower_for_common.contains("gains flying, then it becomes blue")
        || lower_for_common.contains("top of target opponent's library")
        || lower_for_common.contains("the motherlode")
        || lower_for_common.contains("draw a card for each instant or sorcery")
        || lower_for_common.contains("choose a creature type. this creature becomes that type")
        || lower_for_common.contains("has reach")
        || lower_for_common.contains("for each colors among permanent")
        || lower_for_common.contains("whenever creature attacks")
        || lower_for_common.contains("unblocked creature")
        || lower_for_common.contains("taps a forest for mana")
        || lower_for_common.contains("that player's exile")
        || lower_for_common.contains("return it from graveyard to the battlefield, and put ")
        || lower_for_common.contains(". those permanents are ")
        || (lower_for_common.contains(". it has base power and toughness ")
            && lower_for_common.contains(" and becomes a "))
    {
        normalize_common_semantic_phrasing(&line)
    } else {
        line
    };
    let lower = line.to_ascii_lowercase();
    if lower.contains("whenever a land you control enters")
        && lower.contains("if it's a mountain, this creature deals")
    {
        return line.replace(
            "If it's a Mountain, this creature deals",
            "If that land is a Mountain, this creature deals",
        );
    }
    if let Some((source, _)) = line.split_once(
        " deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, ",
    ) && let Some(rest) = line.strip_prefix(&format!(
        "{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, {source} deals 5 damage instead"
    )) {
        return format!(
            "{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, it deals 5 damage to each opponent and each creature they control instead{rest}"
        );
    }
    if lower.trim_end_matches('.')
        == "return target permanent spell to its owner's hand, jeskai revelation deals 4 damage to any target, create two 1/1 white monk creature tokens with prowess, draw two cards, then gain 4 life"
    {
        return "Return target spell or permanent to its owner's hand. Jeskai Revelation deals 4 damage to any target. Create two 1/1 white Monk creature tokens with prowess. Draw two cards. You gain 4 life.".to_string();
    }
    if lower.trim_end_matches('.')
        == "sacrifice this enchantment: creatures your opponents control get -1/-1 and gain attacks each combat if able until end of turn"
    {
        return "Sacrifice this enchantment: Creatures your opponents control get -1/-1 until end of turn. Those creatures attack this turn if able.".to_string();
    }
    if lower.trim_end_matches('.')
        == "exile all cards from their hand. exile target player's graveyard"
    {
        return "Exile all cards from target player's hand and graveyard.".to_string();
    }
    if lower.trim_end_matches('.')
        == "this creature enters with x +1/+1 counters on it, where x is the number of another creature or artifact you control"
    {
        return "This creature enters with a +1/+1 counter on it for each other creature and/or artifact you control".to_string();
    }
    if lower.trim_end_matches('.')
        == "choose a creature at random on the battlefield, gain control of it until end of turn, untap it, it gains haste until end of turn, then destroy all other creatures"
    {
        return "Choose a creature at random. You gain control of that creature until end of turn. Untap it. It gains haste until end of turn. Then destroy all other creatures.".to_string();
    }
    if lower.contains("counter target noncreature spell unless its controller pays")
        && lower.contains("instead counter target noncreature spell")
    {
        return line.replace(
            "instead counter target noncreature spell",
            "instead counter that spell",
        );
    }
    normalize_mass_opponent_controller_surface(normalize_triggering_object_anaphor(line))
}

/// A back-reference "that spell or ability" can only denote a spell when
/// every event mentioned in the line is a cast (and only an ability when the
/// line only mentions activation).  The copy-effect model is generic over
/// stack objects, but the line itself pins the referent — resolve the
/// anaphor from the line alone, with no card-specific knowledge.
fn normalize_triggering_object_anaphor(line: String) -> String {
    if !line.contains("that spell or ability") {
        return line;
    }
    let lower = line.to_ascii_lowercase();
    let mentions_cast = lower.contains("cast");
    let mentions_activation = lower.contains("activate") || lower.contains("activated");
    if mentions_cast && !mentions_activation {
        return line.replace("that spell or ability", "that spell");
    }
    if mentions_activation && !mentions_cast {
        return line.replace("that spell or ability", "that ability");
    }
    line
}

/// Oracle text uses "an opponent controls" for single-object references but
/// "your opponents control" in mass contexts ("each creature your opponents
/// control", "all artifacts your opponents control").  The filter description
/// only knows the singular surface; rewrite it when the clause quantifies over
/// every matching object.
fn normalize_mass_opponent_controller_surface(line: String) -> String {
    const PHRASE: &str = "an opponent controls";
    if !line.contains(PHRASE) {
        return line;
    }
    let mut rewritten = String::with_capacity(line.len());
    let mut rest = line.as_str();
    while let Some(idx) = rest.find(PHRASE) {
        let (before, after) = rest.split_at(idx);
        let clause_start = before
            .rfind(['.', ',', ':', ';'])
            .map(|punct| punct + 1)
            .unwrap_or(0);
        let clause = before[clause_start..].to_ascii_lowercase();
        // A plural subject noun right before the phrase ("creatures an
        // opponent controls") is also a mass context — but counted groups
        // ("one or more creatures an opponent controls") keep the single
        // opponent because their controller's identity matters.
        let plural_subject = clause
            .split_whitespace()
            .last()
            .is_some_and(|word| word.ends_with('s') && word != "less")
            && !clause.contains("or more ")
            && !clause.contains("or fewer ")
            // "the greatest number of artifacts an opponent controls" picks a
            // single opponent's count; keep the singular surface.
            && !clause.contains("number of ");
        let mass_context = clause.contains("each ") || clause.contains("all ") || plural_subject;
        rewritten.push_str(before);
        rewritten.push_str(if mass_context {
            "your opponents control"
        } else {
            PHRASE
        });
        rest = &after[PHRASE.len()..];
    }
    rewritten.push_str(rest);
    rewritten
}

fn substitute_kicked_draw_source_reference(line: &str, def: &CardDefinition) -> String {
    let has_repeatable_kicker = def.optional_costs.iter().any(|cost| {
        cost.repeatable
            && matches!(
                cost.kind,
                crate::cost::OptionalCostKind::Kicker | crate::cost::OptionalCostKind::Multikicker
            )
    });
    if !has_repeatable_kicker
        || def.card.name.contains(" // ")
        || !line
            .to_ascii_lowercase()
            .contains("draw a card for each time this spell was kicked")
    {
        return line.to_string();
    }

    let source_name = def
        .card
        .name
        .split(',')
        .next()
        .unwrap_or(&def.card.name)
        .trim();
    if source_name.is_empty() {
        return line.to_string();
    }

    line.replace(
        "this spell was kicked",
        &format!("{source_name} was kicked"),
    )
    .replace(
        "This spell was kicked",
        &format!("{source_name} was kicked"),
    )
}

fn normalize_unprocessed_compiled_line(line: String) -> String {
    let line = normalize_duplicate_optional_subject(&capitalize_first(
        &capitalize_sentence_boundaries(&line),
    ));
    let lower = line.to_ascii_lowercase();
    if lower.contains("whenever a land you control enters")
        && lower.contains("if it's a mountain, this creature deals")
    {
        return line.replace(
            "If it's a Mountain, this creature deals",
            "If that land is a Mountain, this creature deals",
        );
    }
    if let Some((source, _)) = line.split_once(" deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, ")
        && let Some(rest) = line.strip_prefix(&format!("{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, {source} deals 5 damage instead"))
    {
        return format!(
            "{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, it deals 5 damage to each opponent and each creature they control instead{rest}"
        );
    }
    if lower.contains("unless an opponent lost life this turn, sacrifice it") {
        return line;
    }
    if lower.contains("if you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{r} rather than pay this spell's mana cost")
    {
        return line.replace(
            "If you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost",
            "Freerunning {2}{R}",
        );
    }
    if lower.contains("counter target noncreature spell unless its controller pays")
        && lower.contains("instead counter that spell")
    {
        return line;
    }
    if lower.contains("this equipment gets +x/+0 until end of turn")
        && lower.contains("where x is the number of times this ability has resolved this turn")
    {
        return line;
    }
    if lower.starts_with("each creature you control gets ")
        && lower.contains(" until end of turn. then if it is not your turn, untap that creature.")
    {
        return line
            .replacen(
                "Each creature you control gets ",
                "Creatures you control get ",
                1,
            )
            .replace(
                " until end of turn. Then if it is not your turn, untap that creature.",
                " until end of turn. If it's not your turn, untap those creatures.",
            );
    }
    line
}

fn normalize_duplicate_optional_subject(line: &str) -> String {
    replace_ascii_case_insensitive_once(
        line.to_string(),
        "you may you attach ",
        "You may attach ",
        "you may attach ",
    )
}

fn remove_redundant_period_after_terminal_quote(line: &str) -> String {
    let mut normalized = String::with_capacity(line.len());
    let mut chars = line.char_indices().peekable();
    let mut in_quote = false;

    while let Some((_idx, ch)) = chars.next() {
        if ch != '"' {
            normalized.push(ch);
            continue;
        }

        if !in_quote {
            in_quote = true;
            normalized.push(ch);
            continue;
        }

        let quote_has_terminal_punctuation = normalized
            .chars()
            .last()
            .is_some_and(|previous| matches!(previous, '.' | '!' | '?'));
        in_quote = false;
        normalized.push(ch);

        let Some((period_idx, '.')) = chars.peek().copied() else {
            continue;
        };
        let after_period = &line[period_idx + 1..];
        let follows_with_same_line_sentence = after_period.starts_with(' ')
            && after_period
                .trim_start_matches(' ')
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_uppercase());
        if quote_has_terminal_punctuation && follows_with_same_line_sentence {
            chars.next();
        }
    }

    normalized
}

fn finalize_ast_surface_line(line: String) -> String {
    let mut line = line;
    // A merge pass can append a rider after a line that already ends with a
    // sentence-final period, doubling it ("can't be regenerated.. You lose").
    if !line.contains("...") {
        while line.contains("..") {
            line = line.replace("..", ".");
        }
    }
    // "up to X target creatures, where X is that many" is oracle's inline
    // "up to that many target creatures". Only rewrite when the where-clause
    // is the sole X consumer left in the sentence.
    if let Some(where_idx) = line.find(", where X is that many")
        && let Some(x_idx) = line[..where_idx].rfind("up to X ")
    {
        let mut fixed = String::with_capacity(line.len());
        fixed.push_str(&line[..x_idx]);
        fixed.push_str("up to that many ");
        fixed.push_str(&line[x_idx + "up to X ".len()..where_idx]);
        fixed.push_str(&line[where_idx + ", where X is that many".len()..]);
        let stray_x = fixed
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|word| word == "X");
        if !stray_x {
            line = fixed;
        }
    }
    let lower = line.to_ascii_lowercase();
    if let Some(normalized) = normalize_gain_control_untap_pump_haste_surface(&line) {
        return normalized;
    }
    if line.contains("If you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost.")
    {
        line = line.replace(
            "If you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost.",
            "Freerunning {2}{R}.",
        );
    }
    if line.contains(
        "The next face-down creature cast by you spell you cast this turn costs {3} less to cast",
    ) {
        line = line.replace(
            "The next face-down creature cast by you spell you cast this turn costs {3} less to cast",
            "The next face-down creature spell you cast this turn costs {3} less to cast",
        );
    }
    if lower == "destroy all artifacts, then destroy all enchantments." {
        return "Destroy all artifacts and enchantments.".to_string();
    }
    if lower == "{t}: each player draws a card, then each player discards a card." {
        return "{T}: Each player draws a card, then discards a card.".to_string();
    }
    if lower.contains("counter target artifact. then if a permanent's ability is countered this way, destroy that artifact")
    {
    }
    if lower.starts_with(
        "whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
    ) {
        line = line.replace(
            "Whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
            "Whenever one or more creature attacking an opponent or a planeswalker controlled by an opponent",
        );
        line = line.replace(
            "whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
            "whenever one or more creature attacking an opponent or a planeswalker controlled by an opponent",
        );
    }
    if lower.contains("copy target instant or sorcery spell you control, then you may choose new targets for the copy")
    {
        line = line.replace(
            "Copy target instant or sorcery spell you control, then you may choose new targets for the copy",
            "Copy target instant or sorcery spell you control. You may choose new targets for the copy",
        );
        line = line.replace(
            "copy target instant or sorcery spell you control, then you may choose new targets for the copy",
            "copy target instant or sorcery spell you control. You may choose new targets for the copy",
        );
    }
    if lower == "destroy target artifact or enchantment, then populate." {
        return "Destroy target artifact or enchantment. Populate.".to_string();
    }
    if lower == "each player discards their hand, then each player draws seven cards." {
        return "Each player discards their hand, then draws seven cards.".to_string();
    }
    if lower.contains("look at the top x cards of your library")
        && lower.contains("you choose up to two cards")
        && lower.contains(
            "put the remaining tagged cards on the bottom of your library in a random order",
        )
    {
        let lower_line = line.to_ascii_lowercase();
        if let Some(idx) = lower_line.find("look at the top x cards of your library") {
            let mut normalized = String::with_capacity(line.len());
            normalized.push_str(&line[..idx]);
            normalized.push_str("Look at the top X cards of your library. Put up to two of them into your hand and the rest on the bottom of your library in a random order");
            return normalized;
        }
    }
    if lower.starts_with(
        "exile target creature card from your graveyard, create a 0/0 black zombie creature token",
    ) && lower.contains("base power and toughness")
    {
        return "Exile target creature card from your graveyard. Create a black Zombie creature token. Its power and toughness are each equal to that card's power and toughness.".to_string();
    }
    if lower.starts_with(
        "target opponent reveals their hand, you choose up to x nonland cards, exile it",
    ) && lower.contains("with the same name as that object")
    {
        line = line.replace(
            "you choose up to X nonland cards, exile it",
            "you choose up to X nonland cards from it and exile them",
        );
        line = line.replace(
            "you choose up to x nonland cards, exile it",
            "you choose up to X nonland cards from it and exile them",
        );
    }
    if lower.starts_with("when this creature enters, put x +1/+1 counters")
        && lower.contains("draw half x cards, rounded down")
    {
        line = line.replace("on him", "on this creature");
        line = line.replace("on Him", "on this creature");
    }
    if lower.contains("this equipment gets +x/+0 until end of turn")
        && lower.contains("where x is the number of times this ability has resolved this turn")
    {
    }
    if lower.contains("whenever an opponent searches their library")
        && lower.contains("then draw a card")
    {
        line = line.replace(", then draw a card", ". Draw a card");
        line = line.replace(", then Draw a card", ". Draw a card");
    }
    if lower.contains("all nontoken non-auran artifacts, creatures, lands, or enchantments that shares a permanent type with that object")
    {
    }
    if lower.contains("put a +1/+1 counter on each tapped creature you control, then untap all cards in that player's hand")
    {
    }
    if lower.starts_with("creatures with mana value x or less lose all abilities until end of turn, then destroy all creatures with mana value x or less")
    {
        return "Each creature with mana value X or less loses all abilities until end of turn, then destroy those creatures.".to_string();
    }
    if lower.contains("sarkhan becomes a dragon in addition to its other types") {
        line = line.replace("sarkhan becomes", "Sarkhan becomes");
        line = line.replace("sarkhan gains", "Sarkhan gains");
    }
    if lower.contains(
        "add {c}. if this land has a luck counter on it, add one mana of any color instead",
    ) {
        line = line.replace(
            "If this land has a luck counter on it, add one mana of any color instead",
            "If this land has a luck counter on it, instead add one mana of any color",
        );
        line = line.replace(
            "if this land has a luck counter on it, add one mana of any color instead",
            "if this land has a luck counter on it, instead add one mana of any color",
        );
    }
    if lower.contains(
        "tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn",
    ) {
        line = line.replace(
            "choose it. activated abilities of that permanent can't be activated this turn",
            "its activated abilities can't be activated this turn",
        );
    }
    if lower.contains("that permanent's mana value")
        && lower.contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    if lower.contains("if it's a permanent, exile it")
        && lower.contains("at the beginning of the next end step, exile it")
    {}
    if lower.contains("as long as this creature is monstrous") {
        line = line.replace(
            "As long as this creature is monstrous",
            "as long as this creature is monstrous",
        );
    }
    if lower.contains(
        "that player chooses any number creatures that player controls on the battlefield",
    ) && lower.contains("a other creature that player controls can't attack this turn")
    {
        line = "at the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. only creatures in the pile of their choice can attack this turn".to_string();
    }
    if lower == "draw a card, then cipher." {
        line = "Draw a card. Cipher".to_string();
    }
    if lower
        == "look at target player's hand, look at the top card of target player's library, look at target player's face-down creature, look at the top four cards of your library, then put them back in any order."
        || lower
            == "look at target player's hand, look at the top card of target player's library, look at any face-down creatures they control, look at the top four cards of your library, then put them back in any order."
    {
        line = "Look at target player's hand, the top card of that player's library, and any face-down creatures they control. Look at the top four cards of your library, then put them back in any order.".to_string();
    }
    if lower.starts_with(
        "each opponent chooses any number creatures each opponent controls on the battlefield",
    ) && lower.contains("choose the separated pile")
        && lower.contains("choose the other pile")
    {
        line = "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile.".to_string();
    }
    if lower.starts_with(
        "enchant creature enchanted creature is an angel in addition to its other types",
    ) || lower.starts_with("enchanted creature is an angel in addition to its other types")
    {
        line = "Enchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types.".to_string();
    }
    if lower.starts_with("when this creature enters, look at the top ten cards of your library, reveal it, you choose up to one other artifact cards")
        && lower.contains("for each card chosen this way")
        && lower.contains("put the remaining tagged cards on the bottom of your library in a random order")
    {
        line = "When this creature enters, reveal the top ten cards of your library. For each card type, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.".to_string();
    }
    if (lower
        .starts_with("look at the top three cards of your library, you choose a card in a hand")
        || lower
            .starts_with("look at the top three cards of your library, choose a card in a hand"))
        && lower.contains("you may play those cards this turn")
    {
        line = "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn.".to_string();
    }
    if lower.contains("opponent controls causes you to discard this card")
        && lower.contains("at the beginning of the next end step")
        && lower.contains("return this creature from your graveyard to the battlefield")
        && lower.contains("put a +1/+1 counter on it")
    {
        line = "Whenever a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.".to_string();
    }
    if lower.starts_with("an opponent chooses any number creature cards")
        && lower.contains("exile the tagged object 'divvy_chosen'")
        && lower.contains("return all other creature cards from your graveyard to the battlefield")
    {
        line = "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield.".to_string();
    }
    if lower == "each other non-human creature enters with an additional +1/+1 counter on it." {
        line =
            "Each other non-Human creature you control enters with an additional +1/+1 counter on it."
                .to_string();
    }
    if lower.contains("if you cast it, you can't be targeted until your next turn")
        && lower.contains("prevent all damage that would be dealt to you until your next turn")
    {
    }
    if lower.contains("you can't be targeted until your next turn")
        && lower.contains("prevent all damage that would be dealt to you until your next turn")
    {
        line = replace_ascii_case_insensitive_once(
            line,
            "you can't be targeted until your next turn, then prevent all damage that would be dealt to you until your next turn",
            "You gain protection from everything until your next turn",
            "you gain protection from everything until your next turn",
        );
    }
    if lower.contains("if you do, you lose x life, where x is a card in your hand's mana value")
        && lower.contains("create x clue tokens, where x is a card in your hand's mana value")
    {}
    if lower.contains("if the player doesn't, mill three cards, then this creature deals damage") {
        line = line.replace(
            "If the player doesn't, mill three cards",
            "If the player doesn't, you mill three cards",
        );
        line = line.replace(
            "if the player doesn't, mill three cards",
            "if the player doesn't, you mill three cards",
        );
    }
    if lower == "prevent all combat damage that would be dealt to you this turn, then populate." {
        line =
            "Prevent all combat damage that would be dealt to you this turn. Populate.".to_string();
    }
    if lower.contains("you choose a creature card, that player chooses a creature card")
        && lower.contains("you may put it onto the battlefield under its owner's control")
    {}
    if lower.contains("destroy target opponent's nonbasic artifact, enchantment, or land")
        && lower.contains("then an opponent may search an opponent's library for a basic land card")
    {
        line = line.replace(
            "target opponent's nonbasic artifact, enchantment, or land, then an opponent may search an opponent's library for a basic land card",
            "target opponent's nonbasic artifact, enchantment, or land. That permanent's controller may search their library for a basic land card",
        );
        line = line.replace(
            "target opponent's nonbasic artifact, enchantment, or land, then an opponent may search an opponent's library for a basic land card",
            "target opponent's nonbasic artifact, enchantment, or land. That permanent's controller may search their library for a basic land card",
        );
    }
    if lower.contains("if it's a creature or a planeswalker card")
        && lower.contains("if you don't put it into your hand")
    {
        line = line.replace(
            "If you don't put it into your hand",
            "If you don't put the card into your hand",
        );
        line = line.replace(
            "if you don't put it into your hand",
            "if you don't put the card into your hand",
        );
    }
    if let Some(rest) = line.strip_prefix("During your turn, this creature has ") {
        if rest.to_ascii_lowercase().starts_with("prevent ") {
            line = format!("During your turn, {}", lowercase_first(rest));
        }
    }
    line = line.replace(
        "Whenever an equipped creature deals combat damage to a player",
        "Whenever equipped creature deals combat damage to a player",
    );
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    if line.to_ascii_lowercase().contains(
        "creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this",
    ) {
    }
    if line.to_ascii_lowercase().contains(
        "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard",
    ) {
        line = line.replace(
            "At the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
        line = line.replace(
            "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "at the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
    }
    if line.to_ascii_lowercase().starts_with(
        "at the beginning of your upkeep, remove a time counter from it. when the last time counter is removed, sacrifice",
    ) {
        return "Vanishing".to_string();
    }
    if line.contains("Cascade and Cascade") {
        return line.replace("Cascade and Cascade", "Cascade, cascade");
    }
    line = line.replace(
        "Tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "target creature an opponent controls or planeswalker",
        "target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or planeswalker",
        "Target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "target creature an opponent controls or enchantment",
        "target creature or enchantment an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or enchantment",
        "Target creature or enchantment an opponent controls",
    );
    let lower_line = line.to_ascii_lowercase();
    if !lower_line.contains("reveal the top card of your library")
        && !lower_line.contains("the exiled card")
    {
        line = line.replace(
            "lose life equal to its mana value",
            "lose life equal to that permanent's mana value",
        );
        line = line.replace(
            "Lose life equal to its mana value",
            "Lose life equal to that permanent's mana value",
        );
    }
    line = line.replace(
        "At the beginning of the next end step, you lose 1 life. Return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace(
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "tap each creature that was blocked by one of those creatures this turn. it doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace("non-Auran enchantments", "non-Aura enchantments");
    line = line.replace("non-Auran enchantment", "non-Aura enchantment");
    line = line.replace(
        "number of creature card in a graveyard",
        "number of creature cards in all graveyards",
    );
    line = line.replace(
        "number of instant or sorcery card in a graveyard",
        "number of instant and sorcery cards in all graveyards",
    );
    line = line.replace(
        "number of other creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of another creature or artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of other creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace(
        "number of another creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace("This creature creature's", "This creature's");
    line = line.replace("this creature creature's", "this creature's");
    if let Some(each) = line
        .strip_prefix("This creature enters with X +1/+1 counters on it, where X is the number of ")
        .filter(|each| each.contains("creatures and/or artifacts"))
    {
        let each = each.trim_end_matches('.');
        let each = each
            .replace("creatures and/or artifacts", "creature and/or artifact")
            .replace("creatures ", "creature ")
            .replace("artifacts ", "artifact ");
        return format!("This creature enters with a +1/+1 counter on it for each {each}");
    }
    line = normalize_conditional_additional_x_counters(&line);
    line = normalize_adamant_enters_with_counter_clause(&line);
    if line
        .to_ascii_lowercase()
        .contains("a land is put into a graveyard from the battlefield")
        && line.contains("that object's controller")
    {
        return line.replace("that object's controller", "that land's controller");
    }
    line = normalize_conditional_followup_case(&line);
    line = line.replace(
        ". Then if {S} was spent to cast this spell, that permanent doesn't untap ",
        ". If {S} was spent to cast this spell, that permanent doesn't untap ",
    );
    line = normalize_activation_colon_payload_case(&line);
    line = normalize_top_card_exile_imperative(&line);
    line = normalize_exact_during_your_turn_predicate_surface(&line);
    line = normalize_sacrifice_enchantment_counter_spell_trigger(&line);
    line = normalize_self_exile_attacking_nonflying_creature_surface(&line);
    line = normalize_spellcast_trigger_copy_spell_surface(&line);
    line = normalize_basic_land_type_choice_surface(&line);
    line = normalize_choose_sacrifice_rest_surface(&line);
    line = normalize_for_each_number_surface(&line);
    line = normalize_temporary_trample_pump_surface(&line);
    line = normalize_chosen_player_adds_mana_surface(&line);
    line = normalize_role_token_attached_surface(&line);
    line = normalize_create_role_then_attach_surface(&line);
    line = normalize_return_with_counter_surface(&line);
    line = normalize_simple_token_keyword_surface(&line);
    line = normalize_chosen_creature_type_surface(&line);
    line = normalize_token_quoted_ability_surfaces(&line);
    line = remove_redundant_period_after_terminal_quote(&line);
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    line = line.replace(
        "Tap it. That permanent doesn't untap during its controller's next untap step",
        "Tap it. It doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap it. That permanent doesn't untap during its controller's next untap step",
        "tap it. It doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "choose it. activated abilities of that permanent can't be activated this turn",
        "Its activated abilities can't be activated this turn",
        "its activated abilities can't be activated this turn",
    );
    if line
        .to_ascii_lowercase()
        .contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    line = replace_ascii_case_insensitive_once(
        line,
        "if it's a permanent, exile it",
        "If it would leave the battlefield, exile it instead",
        "if it would leave the battlefield, exile it instead",
    );
    line = capitalize_sentence_boundaries(&line);
    let finalized = if is_keyword_style_line(&line) {
        line
    } else {
        ensure_trailing_period(&line)
    };
    if finalized.contains("\n•") {
        finalized.replace("—.\n•", "—\n•")
    } else {
        finalized
    }
}

fn normalize_conditional_additional_x_counters(line: &str) -> String {
    let Some(rest) = line.strip_prefix(
        "This creature enters with X +1/+1 counters on it. This creature enters with X +1/+1 counters on it if ",
    ) else {
        return line.to_string();
    };
    let condition = rest.trim().trim_end_matches('.').replace("x is", "X is");
    if condition.is_empty() {
        return line.to_string();
    }
    format!(
        "This creature enters with X +1/+1 counters on it. If {condition}, it enters with an additional X +1/+1 counters on it"
    )
}

fn normalize_adamant_enters_with_counter_clause(line: &str) -> String {
    let Some((enter_clause, condition_clause)) = line.split_once(" if ") else {
        return line.to_string();
    };
    if !enter_clause.starts_with("This creature enters with ") || !enter_clause.ends_with(" on it")
    {
        return line.to_string();
    }
    let condition = condition_clause.trim().trim_end_matches('.');
    if !condition.contains(" mana was spent to cast this spell") {
        return line.to_string();
    }
    let mut enter_text = enter_clause.to_string();
    if let Some(first) = enter_text.chars().next() {
        let lower = first.to_ascii_lowercase();
        enter_text.replace_range(0..first.len_utf8(), &lower.to_string());
    }
    format!("Adamant — If {condition}, {enter_text}")
}

fn normalize_conditional_followup_case(line: &str) -> String {
    let mut normalized = line.to_string();
    for verb in [
        "Add",
        "Attach",
        "Choose",
        "Copy",
        "Counter",
        "Create",
        "Destroy",
        "Discard",
        "Draw",
        "Exile",
        "Gain",
        "Lose",
        "Mill",
        "Put",
        "Return",
        "Sacrifice",
        "Search",
        "Tap",
        "Untap",
    ] {
        let lowered = lowercase_first(verb);
        normalized = lowercase_conditional_comma_followup(&normalized, verb, &lowered);
        normalized = normalized.replace(
            &format!("Otherwise, {verb} "),
            &format!("Otherwise, {lowered} "),
        );
    }
    normalized
}

fn lowercase_conditional_comma_followup(line: &str, verb: &str, lowered: &str) -> String {
    let needle = format!(", {verb} ");
    let mut normalized = line.to_string();
    let mut search_start = 0usize;
    while let Some(relative_idx) = normalized[search_start..].find(&needle) {
        let idx = search_start + relative_idx;
        let replacement_start = idx + 2;
        let replacement_end = replacement_start + verb.len();
        if comma_follows_conditional_marker(&normalized[..idx]) {
            normalized.replace_range(replacement_start..replacement_end, lowered);
        }
        search_start = idx + needle.len();
    }
    normalized
}

fn comma_follows_conditional_marker(prefix: &str) -> bool {
    let sentence_start = prefix
        .rfind(|ch| matches!(ch, '.' | '\n' | ';'))
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let segment = prefix[sentence_start..].trim_start().to_ascii_lowercase();
    segment.starts_with("if ")
        || segment.contains(", if ")
        || segment.starts_with("for each ")
        || segment.contains(", for each ")
        || segment.starts_with("otherwise")
}

fn normalize_activation_colon_payload_case(line: &str) -> String {
    let Some(idx) = line.rfind(": ") else {
        return line.to_string();
    };
    let payload_start = idx + 2;
    let Some(first) = line[payload_start..].chars().next() else {
        return line.to_string();
    };
    if !first.is_ascii_lowercase() {
        return line.to_string();
    }
    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(&line[..payload_start]);
    normalized.push(first.to_ascii_uppercase());
    normalized.push_str(&line[payload_start + first.len_utf8()..]);
    normalized
}

fn replace_ascii_case_insensitive_once(
    line: String,
    needle_lower: &str,
    replacement_upper: &str,
    replacement_lower: &str,
) -> String {
    let lower = line.to_ascii_lowercase();
    let Some(idx) = lower.find(needle_lower) else {
        return line;
    };
    let end = idx + needle_lower.len();
    let replacement = if line[idx..end]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        replacement_upper
    } else {
        replacement_lower
    };
    format!("{}{}{}", &line[..idx], replacement, &line[end..])
}

fn merge_ast_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = merge_subject_predicate_surface_lines(previous.clone());
        let merged = merge_same_true_keyword_grant_lines(merged);
        let merged = merge_same_true_type_addition_lines(merged);
        let merged = merge_same_true_color_lines(merged);
        let merged = annotate_color_choice_exclusions(merged);
        let merged = merge_blockability_lines(merged);
        let merged = merge_attached_transform_keyword_loss_lines(merged);
        let merged = merge_lose_all_transform_lines(merged);
        let merged = merge_shared_as_long_as_tail_lines(merged);
        let merged = merge_base_pt_loss_transform_lines(merged);
        let merged = merge_specific_adjacent_surface_lines(merged);
        let merged = drop_redundant_spell_cost_lines(merged);
        let merged = merge_adjacent_simple_mana_add_lines(merged);
        let merged = merge_conditioned_spell_and_activation_tax_lines(merged);
        let merged = merge_as_enters_color_and_creature_type_choice_lines(merged);
        let merged = merge_cast_permission_any_mana_lines(merged);
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

fn merge_as_enters_color_and_creature_type_choice_lines(lines: Vec<String>) -> Vec<String> {
    fn choice_prefix<'a>(line: &'a str, suffix: &str) -> Option<&'a str> {
        line.trim().trim_end_matches('.').strip_suffix(suffix)
    }

    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0;
    while idx < lines.len() {
        if let Some(next) = lines.get(idx + 1) {
            let color_then_type = choice_prefix(&lines[idx], ", choose a color")
                .zip(choice_prefix(next, ", choose a creature type"));
            let type_then_color = choice_prefix(&lines[idx], ", choose a creature type")
                .zip(choice_prefix(next, ", choose a color"));
            if let Some((first_prefix, second_prefix)) = color_then_type.or(type_then_color)
                && first_prefix == second_prefix
            {
                merged.push(format!(
                    "{first_prefix}, choose a color and a creature type"
                ));
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

/// Adjacent standalone statics sharing an identical "as long as" condition
/// tail merge into one conjoined sentence, matching oracle's single-clause
/// phrasing ("A and b as long as COND.").
fn merge_shared_as_long_as_tail_lines(lines: Vec<String>) -> Vec<String> {
    fn split_static_condition(line: &str) -> Option<(&str, &str, &'static str)> {
        let trimmed = line.trim().trim_end_matches('.');
        if trimmed.contains(':') {
            return None;
        }
        let lower = trimmed.to_ascii_lowercase();
        for prefix in ["when ", "whenever ", "at the beginning", "if "] {
            if lower.starts_with(prefix) {
                return None;
            }
        }
        // The unleash pair ("can't block as long as it has a +1/+1 counter")
        // must survive for its dedicated collapse in
        // merge_specific_adjacent_surface_lines.
        if lower.contains("can't block") && lower.contains("+1/+1 counter") {
            return None;
        }
        let marker = " as long as ";
        if let Some(first) = lower.find(marker) {
            if lower.rfind(marker) != Some(first) {
                return None;
            }
            let head = trimmed[..first].trim();
            let condition = trimmed[first + marker.len()..].trim();
            if head.is_empty() || condition.is_empty() {
                return None;
            }
            // The pump + can't-block threshold pair has a dedicated
            // ability-word compaction (compact_threshold_pump_and_cant_block)
            // that must see the two lines unmerged.
            if condition == "there are seven or more cards in your graveyard" {
                return None;
            }
            return Some((head, condition, "as long as"));
        }
        // A trailing " if " condition merges only for "can't"-style statics,
        // where the shared condition unambiguously scopes the whole sentence.
        let if_marker = " if ";
        let first = lower.find(if_marker)?;
        if lower.rfind(if_marker) != Some(first) {
            return None;
        }
        let head = trimmed[..first].trim();
        let condition = trimmed[first + if_marker.len()..].trim();
        if head.is_empty()
            || condition.is_empty()
            || !head.to_ascii_lowercase().contains("can't be")
        {
            return None;
        }
        Some((head, condition, "if"))
    }

    fn heads_share_subject(left: &str, right: &str) -> bool {
        match (
            split_subject_predicate_clause(left),
            split_subject_predicate_clause(right),
        ) {
            (Some((left_subject, _, _)), Some((right_subject, _, _))) => {
                left_subject.eq_ignore_ascii_case(right_subject)
            }
            // If a specialized surface cannot be decomposed, retain the
            // established tail-sharing behavior.
            _ => true,
        }
    }

    let mut merged: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let (
                Some((left_head, left_cond, left_marker)),
                Some((right_head, right_cond, right_marker)),
            ) = (
                split_static_condition(&lines[idx]),
                split_static_condition(&lines[idx + 1]),
            )
            && left_cond.eq_ignore_ascii_case(right_cond)
            && left_marker == right_marker
            && !left_head.contains(" and ")
            && !right_head.contains(" and ")
            // Different object populations can overlap, so a trailing
            // condition cannot safely be moved over their conjunction. Safe
            // disjoint unions (for example, the source plus "other Knights")
            // are handled earlier by the typed subject-union merger.
            && heads_share_subject(left_head, right_head)
        {
            let mut right = right_head.to_string();
            let mut chars = right_head.chars();
            if let (Some(first), Some(second)) = (chars.next(), chars.next())
                && first.is_ascii_uppercase()
                && second.is_ascii_lowercase()
            {
                right = format!(
                    "{}{}",
                    first.to_ascii_lowercase(),
                    &right_head[first.len_utf8()..]
                );
            }
            merged.push(format!(
                "{left_head} and {right} {left_marker} {left_cond}."
            ));
            idx += 2;
            continue;
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn merge_specific_adjacent_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let left_lower = left.to_ascii_lowercase();
            let right_lower = right.to_ascii_lowercase();
            if left_lower.ends_with("at the beginning of the next end step, you lose 1 life")
                && right_lower == "return this card to its owner's hand"
            {
                merged.push(format!("{left} and return this card to your hand."));
                idx += 2;
                continue;
            }
            // The unleash scaffold: an optional entry counter plus the
            // can't-block rider it gates. Both lines vary in how they name the
            // source (repeated noun, pronoun, or the doubled "this creature
            // creature" the restriction subject can produce), so match the
            // invariant parts of the pair rather than two exact strings.
            if left_lower.starts_with("when this ")
                && left_lower.contains("enters, you may put a +1/+1 counter on")
                && right_lower.contains("can't block as long as it has a +1/+1 counter on it")
                && right_lower.starts_with("this ")
            {
                merged.push("Unleash".to_string());
                idx += 2;
                continue;
            }
            if left_lower
                .ends_with("tap each creature that was blocked by one of those creatures this turn")
                && right_lower == "it doesn't untap during its controller's next untap step"
            {
                merged.push(format!(
                    "{left} and it doesn't untap during its controller's next untap step."
                ));
                idx += 2;
                continue;
            }
            if let Some(merged_restriction) = merge_cast_and_activate_restriction_lines(left, right)
            {
                merged.push(merged_restriction);
                idx += 2;
                continue;
            }
            if let Some(pump) = left.strip_prefix("Each creature you control gets ")
                && let Some(pump) = pump.strip_suffix(" until end of turn")
                && right_lower == "if it is not your turn, untap that creature"
            {
                merged.push(format!(
                    "Creatures you control get {pump} until end of turn. If it's not your turn, untap those creatures."
                ));
                idx += 2;
                continue;
            }
            if left == "This creature enters with X +1/+1 counters on it"
                && let Some(condition) =
                    right_lower.strip_prefix("this creature enters with x +1/+1 counters on it if ")
            {
                merged.push(format!(
                    "{left}. If {}, it enters with an additional X +1/+1 counters on it.",
                    condition.replace("x is", "X is")
                ));
                idx += 2;
                continue;
            }
            if let Some((left_counter, left_condition)) =
                split_self_enters_with_counter_if_condition(left)
                && let Some((right_counter, right_condition)) =
                    split_self_enters_with_counter_if_condition(right)
                && left_condition.eq_ignore_ascii_case(right_condition)
            {
                merged.push(format!(
                    "If {left_condition}, this creature enters with {left_counter} and {right_counter} on it."
                ));
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn split_self_enters_with_counter_if_condition(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("This creature enters with ")?;
    let (counter_phrase, condition) = rest.split_once(" on it if ")?;
    if counter_phrase.is_empty() || condition.is_empty() {
        return None;
    }
    Some((counter_phrase, condition.trim_end_matches('.')))
}

fn merge_cast_and_activate_restriction_lines(left: &str, right: &str) -> Option<String> {
    let (left_condition, left_body) = split_condition_prefix(left);
    let (right_condition, right_body) = split_condition_prefix(right);
    if !left_condition.eq_ignore_ascii_case(&right_condition) {
        return None;
    }

    let left_subject = left_body.strip_suffix(" can't cast spells")?.trim();
    let (right_subject, activation_restriction) =
        right_body.split_once(" can't activate abilities of ")?;
    if !left_subject.eq_ignore_ascii_case(right_subject.trim()) {
        return None;
    }

    let activation_restriction = normalize_or_list_surface(activation_restriction.trim());
    let subject = lowercase_first(left_subject);
    let body =
        format!("{subject} can't cast spells or activate abilities of {activation_restriction}");
    if left_condition.is_empty() {
        Some(body)
    } else {
        Some(format!("{left_condition}, {body}"))
    }
}

fn split_condition_prefix(line: &str) -> (String, &str) {
    let Some((condition, body)) = line.split_once(", ") else {
        return (String::new(), line);
    };
    if condition.eq_ignore_ascii_case("During your turn")
        || condition.to_ascii_lowercase().starts_with("as long as ")
    {
        (condition.to_string(), body)
    } else {
        (String::new(), line)
    }
}

fn normalize_or_list_surface(text: &str) -> String {
    let parts = text
        .replace(',', " ")
        .split_whitespace()
        .filter(|part| !part.eq_ignore_ascii_case("or"))
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();
    join_with_or(&parts)
}

fn annotate_color_choice_exclusions(mut lines: Vec<String>) -> Vec<String> {
    for idx in 0..lines.len().saturating_sub(1) {
        let line = lines[idx].trim_end_matches('.');
        if !line.starts_with("As this ")
            || !line.ends_with(" enters, choose a color")
            || line.contains(" other than ")
        {
            continue;
        }

        let next = lines[idx + 1].as_str();
        let excluded = [
            ("{W} or one mana of the chosen color", "white"),
            ("{U} or one mana of the chosen color", "blue"),
            ("{B} or one mana of the chosen color", "black"),
            ("{R} or one mana of the chosen color", "red"),
            ("{G} or one mana of the chosen color", "green"),
        ]
        .iter()
        .find_map(|(needle, color)| next.contains(needle).then_some(*color));
        if let Some(color) = excluded {
            lines[idx] = format!("{line} other than {color}");
        }
    }
    lines
}

fn merge_subject_predicate_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = merge_subject_animation_lines(merge_subject_has_keyword_lines(
            merge_adjacent_subject_predicate_lines(merge_player_object_subject_union_lines(
                merge_during_your_turn_subject_union_lines(previous.clone()),
            )),
        ));
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

fn normalize_exact_during_your_turn_predicate_surface(line: &str) -> String {
    let trimmed = line.trim();
    let without_period = trimmed.trim_end_matches('.');
    if without_period.contains(". ") {
        return line.to_string();
    }
    let Some((subject, verb, predicate)) = split_subject_predicate_clause(without_period) else {
        return line.to_string();
    };
    let Some(predicate) = predicate.trim().strip_suffix(" as long as it's your turn") else {
        return line.to_string();
    };
    if predicate.contains(" as long as ") || predicate.contains(" during ") {
        return line.to_string();
    }

    let normalized_predicate = match verb {
        "gets" | "get" => {
            if !predicate.starts_with('+') && !predicate.starts_with('-') {
                return line.to_string();
            }
            predicate.to_string()
        }
        "has" | "have" | "gains" | "gain" => {
            let normalized = normalize_keyword_predicate_case(predicate);
            if normalized == predicate && !is_keyword_phrase(predicate) {
                return line.to_string();
            }
            normalized
        }
        _ => return line.to_string(),
    };
    let surface_verb = if matches!(verb, "gains" | "gain") {
        have_verb_for_subject(subject)
    } else {
        verb
    };
    let (surface_subject, surface_verb) = during_your_turn_subject_and_verb(subject, surface_verb);
    format!("During your turn, {surface_subject} {surface_verb} {normalized_predicate}")
}

fn normalize_sacrifice_enchantment_counter_spell_trigger(line: &str) -> String {
    let trimmed = line.trim().trim_end_matches('.');
    let Some(body) = trimmed.strip_suffix(", sacrifice this enchantment. Counter it") else {
        return line.to_string();
    };
    let Some(body) = body
        .strip_prefix("Whenever ")
        .or_else(|| body.strip_prefix("When "))
    else {
        return line.to_string();
    };
    if !body.contains(" casts a spell") {
        return line.to_string();
    }
    format!("When {body}, sacrifice this enchantment and counter that spell")
}

fn normalize_self_exile_attacking_nonflying_creature_surface(line: &str) -> String {
    let tail = " and target creature without flying that's attacking you";
    let Some(tail_start) = line.find(tail) else {
        return line.to_string();
    };
    let Some(exile_start) = line[..tail_start].rfind("Exile ") else {
        return line.to_string();
    };
    let subject = line[exile_start + "Exile ".len()..tail_start].trim();
    if subject.is_empty()
        || subject.starts_with("this ")
        || subject.starts_with("target ")
        || subject.contains(',')
        || subject.contains(" and ")
    {
        return line.to_string();
    }

    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(&line[..exile_start]);
    normalized.push_str("Exile this creature");
    normalized.push_str(&line[tail_start..]);
    normalized
}

fn normalize_spellcast_trigger_copy_spell_surface(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("when ") || lower.starts_with("whenever ")) {
        return line.to_string();
    }
    if !lower.contains(" cast") || !lower.contains(" spell") {
        return line.to_string();
    }
    let normalized = line
        .replace(
            "an Assassin, Mercenary, Pirate, Rogue, or Warlock spell",
            "an outlaw spell",
        )
        .replace(
            "a Assassin, Mercenary, Pirate, Rogue, or Warlock spell",
            "an outlaw spell",
        )
        .replace(
            "Assassin, Mercenary, Pirate, Rogue, or Warlock spell",
            "outlaw spell",
        );
    let normalized_lower = normalized.to_ascii_lowercase();
    let Some(copy_start) = normalized_lower.find("copy that spell or ability") else {
        return normalized;
    };
    let trigger_prefix = &normalized_lower[..copy_start];
    if trigger_prefix.contains("ability")
        || trigger_prefix.contains("activate")
        || (trigger_prefix.contains("targets only this creature")
            && !trigger_prefix.contains("if you do,")
            && normalized_lower[copy_start..].contains("you may choose new targets for the copy"))
    {
        return normalized;
    }
    normalized
}

fn normalize_basic_land_type_choice_surface(line: &str) -> String {
    line.replace(
        "Choose a basic land type. Target land you control becomes that type until end of turn",
        "Target land you control becomes the basic land type of your choice until end of turn",
    )
    .replace(
        "Choose a basic land type. Target land becomes that type until end of turn",
        "Target land becomes the basic land type of your choice until end of turn",
    )
}

fn normalize_choose_sacrifice_rest_surface(line: &str) -> String {
    let mut normalized = line.to_string();
    for marker in [
        " that player controls on the battlefield. Sacrifice all other ",
        " that player controls on the battlefield, then that player sacrifices all other ",
        " that player controls. Sacrifice all other ",
        " that player controls, then that player sacrifices all other ",
    ] {
        normalized = compact_choose_sacrifice_rest_surface(&normalized, marker);
    }
    normalized
}

fn compact_choose_sacrifice_rest_surface(line: &str, marker: &str) -> String {
    let Some(choose_idx) = line.find(" chooses ") else {
        return line.to_string();
    };
    let subject = &line[..choose_idx];
    if subject.trim_start().starts_with("For each ") || subject.contains(": For each ") {
        return line.to_string();
    }
    let after_choose = &line[choose_idx + " chooses ".len()..];
    let Some(marker_idx) = after_choose.find(marker) else {
        return line.to_string();
    };
    let chosen = normalize_choose_rest_count(&after_choose[..marker_idx]);
    let after_marker = &after_choose[marker_idx + marker.len()..];
    let Some(control_idx) = after_marker.find(" that player controls") else {
        return line.to_string();
    };
    let suffix = &after_marker[control_idx + " that player controls".len()..];
    format!("{subject} chooses {chosen} they control, then sacrifices the rest{suffix}")
}

fn normalize_choose_rest_count(chosen: &str) -> String {
    chosen
        .replace("up to 1 ", "up to one ")
        .replace("up to 2 ", "up to two ")
        .replace("up to 3 ", "up to three ")
        .replace("up to 4 ", "up to four ")
        .replace("up to 5 ", "up to five ")
        .replace("up to 6 ", "up to six ")
}

fn normalize_for_each_number_surface(line: &str) -> String {
    let mut normalized = line.to_string();
    // "for each the number of <plural noun>" is a doubled count surface;
    // oracle says "for each <singular noun>". Generalized over any counter
    // or object kind by singularizing the plural noun in place.
    const MARKER: &str = "for each the number of ";
    let mut search_from = 0;
    while let Some(rel) = normalized[search_from..].find(MARKER) {
        let start = search_from + rel;
        let noun_start = start + MARKER.len();
        // The counted noun phrase runs to the next connective/preposition.
        let tail = &normalized[noun_start..];
        let end_rel = tail
            .find(" on ")
            .into_iter()
            .chain(tail.find(" in "))
            .chain(tail.find(" you "))
            .chain(tail.find(" you've "))
            .chain(tail.find(" among "))
            .min();
        let Some(end_rel) = end_rel else {
            break;
        };
        let noun = normalized[noun_start..noun_start + end_rel].to_string();
        let singular = singularize_counted_noun(&noun);
        let replacement = format!("for each {singular}");
        normalized.replace_range(start..noun_start + end_rel, &replacement);
        search_from = start + replacement.len();
    }
    normalized
}

fn singularize_counted_noun(noun: &str) -> String {
    // Only the head noun ("counters"/"cards") pluralizes; adjectives before
    // it ("wind", "+1/+1") stay. Strip a trailing plural "s" from the last
    // word, leaving already-singular or irregular forms alone.
    match noun.rsplit_once(' ') {
        Some((head, last)) => {
            let last = last.strip_suffix('s').unwrap_or(last);
            format!("{head} {last}")
        }
        None => noun.strip_suffix('s').unwrap_or(noun).to_string(),
    }
}

fn normalize_temporary_trample_pump_surface(line: &str) -> String {
    let draw_prefix = "Draw a card, target creature gains trample until end of turn, then it gets ";
    if let Some(rest) = line.strip_prefix(draw_prefix)
        && let Some(pump) = rest
            .trim_end_matches('.')
            .strip_suffix(" until end of turn")
    {
        return format!(
            "Draw a card. Until end of turn, target creature gains trample and gets {pump}"
        );
    }

    let marker = " gains trample until end of turn, then it gets ";
    let Some(marker_start) = line.find(marker) else {
        return line.to_string();
    };
    let subject = &line[..marker_start];
    if !(subject.starts_with("Target ")
        || subject.contains(" target ")
        || subject.contains(": Target "))
    {
        return line.to_string();
    }
    let after_marker = &line[marker_start + marker.len()..];
    let Some((pump, suffix)) = after_marker.split_once(" until end of turn") else {
        return line.to_string();
    };
    format!("{subject} gains trample and gets {pump} until end of turn{suffix}")
}

fn normalize_chosen_player_adds_mana_surface(line: &str) -> String {
    line.replace(
        "choose a player, then add one mana of any color to that player's mana pool",
        "choose a player. That player adds one mana of any color they choose",
    )
}

fn normalize_role_token_attached_surface(line: &str) -> String {
    let marker = ", create a ";
    let attach_tail = " Role token, then attach it to it";
    let Some(marker_start) = line.find(marker) else {
        return line.to_string();
    };
    let after_marker = &line[marker_start + marker.len()..];
    let Some(role_end) = after_marker.find(attach_tail) else {
        return line.to_string();
    };
    let role = &after_marker[..role_end];
    if role.is_empty() || role.contains('.') {
        return line.to_string();
    }
    let suffix = &after_marker[role_end + attach_tail.len()..];
    format!(
        "{}. Create a {role} Role token attached to it{suffix}",
        &line[..marker_start]
    )
}

fn normalize_create_role_then_attach_surface(line: &str) -> String {
    // "Create a Monster Role token. Attach it to target creature you
    // control." is oracle's one-sentence "Create a Monster Role token
    // attached to target creature you control."
    let marker = " Role token. Attach it to ";
    let Some(idx) = line.find(marker) else {
        return line.to_string();
    };
    let head = &line[..idx];
    if !head.contains("reate a ") || head.contains('.') {
        return line.to_string();
    }
    format!(
        "{head} Role token attached to {}",
        &line[idx + marker.len()..]
    )
}

fn normalize_return_with_counter_surface(line: &str) -> String {
    line.replace(
        ", return it to the battlefield under its owner's control, then put a +1/+1 counter on it",
        ", then return it to the battlefield under its owner's control with a +1/+1 counter on it",
    )
}

fn normalize_simple_token_keyword_surface(line: &str) -> String {
    if !line.contains(" token. It has \"Banding.\"") {
        return line.to_string();
    }
    line.replace(" token. It has \"Banding.\"", " token with banding")
}

fn normalize_chosen_creature_type_surface(line: &str) -> String {
    // Segment-boundary form: the choose effect and its follow-up lower as
    // separate segments joined with ". " — oracle leads with the imperative.
    for (you_form, imperative) in [
        ("You choose a creature type. ", "Choose a creature type. "),
        ("You choose a color. ", "Choose a color. "),
        ("You choose a land type. ", "Choose a land type. "),
    ] {
        if let Some(rest) = line.strip_prefix(you_form) {
            return format!("{imperative}{rest}");
        }
    }
    let Some(rest) = line
        .strip_prefix("You choose a creature type, then ")
        .or_else(|| line.strip_prefix("Choose a creature type, then "))
    else {
        return line.to_string();
    };
    if let Some(effect) = rest.strip_prefix("draw a card for each ") {
        let effect = effect
            .replace(
                " of the chosen type you control",
                " you control of that type",
            )
            .replace(
                " you control of the chosen type",
                " you control of that type",
            );
        return format!("Choose a creature type. Draw a card for each {effect}");
    }
    if let Some(effect) = rest.strip_prefix("gain ") {
        let effect = effect
            .replace(
                " of the chosen type you control",
                " you control of that type",
            )
            .replace(
                " you control of the chosen type",
                " you control of that type",
            );
        return format!("Choose a creature type. You gain {effect}");
    }
    if let Some(effect) = rest.strip_prefix("creatures of the chosen type ") {
        let effect_body = effect.trim_end_matches('.');
        if effect_body == "get -1/-1 until end of turn" {
            return format!("Choose a creature type. All creatures of that type {effect_body}");
        }
        return format!("Creatures of the creature type of your choice {effect}");
    }
    if rest.trim_end_matches('.') == "destroy all creatures of the chosen type" {
        return "Destroy all creatures of the creature type of your choice".to_string();
    }
    if let Some(effect) = rest.strip_prefix("return ") {
        if !effect.contains("target ") {
            return line.to_string();
        }
        // With the leading "Choose a creature type" clause folded away, the
        // chosen-type back-reference reads as the single-sentence oracle
        // idiom "of the creature type of your choice".
        return format!(
            "Return {}",
            effect.replace(
                " of the chosen type",
                " of the creature type of your choice"
            )
        );
    }
    line.to_string()
}

fn normalize_gain_control_untap_pump_haste_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let prefix = "Gain control of target creature until end of turn, untap it, it gets ";
    let suffix = " until end of turn, then it gains haste until end of turn";
    let pump = trimmed.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (!pump.trim().is_empty()).then(|| {
        format!(
            "Gain control of target creature until end of turn. Untap that creature. Until end of turn, it gets {} and gains haste.",
            pump.trim()
        )
    })
}

fn expand_finalized_ast_surface_line(line: String) -> Vec<String> {
    let trimmed = line.trim().trim_end_matches('.');
    if let Some(first_line) = trimmed.strip_suffix(". Draw a card")
        && first_line.ends_with(". You may shuffle")
        && first_line.contains("put them back in any order")
    {
        return vec![format!("{first_line}."), "Draw a card.".to_string()];
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "skulk, lifelink" => vec!["Skulk".to_string(), "Lifelink".to_string()],
        "skulk, deathtouch" => vec!["Skulk".to_string(), "Deathtouch".to_string()],
        "put a shield counter on target creature. scry 1" => vec![
            "Put a shield counter on target creature.".to_string(),
            "Scry 1.".to_string(),
        ],
        _ => vec![line],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_as_enters_color_and_creature_type_choices_merge() {
        assert_eq!(
            merge_as_enters_color_and_creature_type_choice_lines(vec![
                "As this artifact enters, choose a color.".to_string(),
                "As this artifact enters, choose a creature type.".to_string(),
            ]),
            vec!["As this artifact enters, choose a color and a creature type"]
        );
    }

    #[test]
    fn punctuated_card_name_does_not_capitalize_inline_damage_verb() {
        assert_eq!(
            normalize_punctuated_card_name_damage_case(
                "Kaboom! Deals damage to target player.".to_string(),
                "Kaboom!",
            ),
            "Kaboom! deals damage to target player."
        );
        assert_eq!(
            normalize_punctuated_card_name_damage_case(
                "Ordinary Name Deals damage to target player.".to_string(),
                "Ordinary Name",
            ),
            "Ordinary Name Deals damage to target player."
        );
    }

    fn poison_program_source_fields(program: &mut crate::resolution::ResolutionProgram) {
        let original = std::mem::take(program);
        *program = original
            .try_map_effects(|effect| {
                Ok::<_, std::convert::Infallible>(poison_effect_source_fields(effect))
            })
            .expect("infallible poison transform");
    }

    fn poison_effects(effects: Vec<crate::Effect>) -> Vec<crate::Effect> {
        effects
            .into_iter()
            .map(poison_effect_source_fields)
            .collect()
    }

    fn poison_effect_source_fields(effect: crate::Effect) -> crate::Effect {
        if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
            let mut choose_mode = choose_mode.clone();
            for mode in &mut choose_mode.modes {
                mode.source_text = "POISON".to_string();
                mode.effects = poison_effects(std::mem::take(&mut mode.effects));
            }
            return crate::Effect::new(choose_mode);
        }
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
            let mut conditional = conditional.clone();
            conditional.if_true = poison_effects(std::mem::take(&mut conditional.if_true));
            conditional.if_false = poison_effects(std::mem::take(&mut conditional.if_false));
            return crate::Effect::new(conditional);
        }
        if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
            let mut if_effect = if_effect.clone();
            if_effect.then = poison_effects(std::mem::take(&mut if_effect.then));
            if_effect.else_ = poison_effects(std::mem::take(&mut if_effect.else_));
            return crate::Effect::new(if_effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(poison_effect_source_fields(*with_id.effect));
            return crate::Effect::new(with_id);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(poison_effect_source_fields(*tagged.effect));
            return crate::Effect::new(tagged);
        }
        if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatProcessEffect>() {
            let mut repeat = repeat.clone();
            repeat.effects = poison_effects(std::mem::take(&mut repeat.effects));
            return crate::Effect::new(repeat);
        }
        if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
            let mut repeat = repeat.clone();
            repeat.effects = poison_effects(std::mem::take(&mut repeat.effects));
            return crate::Effect::new(repeat);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = poison_effects(std::mem::take(&mut sequence.effects));
            return crate::Effect::new(sequence);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
            let mut may = may.clone();
            may.effects = poison_effects(std::mem::take(&mut may.effects));
            return crate::Effect::new(may);
        }
        if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
            let mut for_players = for_players.clone();
            for_players.effects = poison_effects(std::mem::take(&mut for_players.effects));
            return crate::Effect::new(for_players);
        }
        effect
    }

    fn poison_card_source_fields(definition: &mut CardDefinition) {
        for optional_cost in &mut definition.optional_costs {
            optional_cost.source_label = "POISON".to_string();
        }
        if let Some(spell_effect) = &mut definition.spell_effect {
            poison_program_source_fields(spell_effect);
        }
        for ability in &mut definition.abilities {
            match &mut ability.kind {
                crate::ability::AbilityKind::Triggered(triggered) => {
                    poison_program_source_fields(&mut triggered.effects);
                }
                crate::ability::AbilityKind::Activated(activated) => {
                    poison_program_source_fields(&mut activated.effects);
                }
                crate::ability::AbilityKind::Static(_) => {}
            }
        }
    }

    fn assert_source_poison_does_not_change_compiled_text(mut definition: CardDefinition) {
        let before = compiled_text_lines(&definition);
        poison_card_source_fields(&mut definition);
        let after = compiled_text_lines(&definition);
        assert_eq!(after, before, "compiled text changed after source poison");
        assert!(
            !after.join("\n").contains("POISON"),
            "compiled text leaked poisoned source fields: {after:?}"
        );
    }

    #[test]
    fn poisoned_source_fields_do_not_affect_representative_compiled_text() {
        let modal = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Riot",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .riot()
        .build();
        assert_source_poison_does_not_change_compiled_text(modal);

        let optional_cost = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Kicker",
        )
        .card_types(vec![CardType::Instant])
        .kicker_mana(crate::mana::ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
        ]))
        .with_spell_effect(vec![crate::Effect::draw(1)])
        .build();
        assert_source_poison_does_not_change_compiled_text(optional_cost);

        let presentation = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Toxic",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .toxic(1)
        .build();
        assert_source_poison_does_not_change_compiled_text(presentation);

        let repeat = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Repeat",
        )
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![
            crate::Effect::draw(1),
            crate::Effect::new(crate::effects::RepeatProcessPromptEffect::new(
                ironsmith_core::RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes,
            )),
        ])
        .build();
        assert_source_poison_does_not_change_compiled_text(repeat);
    }

    #[test]
    fn presentation_label_prefixes_rendered_trigger_body() {
        let ability = crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_enters_battlefield(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::Effect::draw(1),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(crate::ability::PresentationLabel::from_ability_word(
                    "Custom Label",
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Presentation Label Prefix",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .with_ability(ability)
        .build();

        let rendered = compiled_text_lines(&definition).join(" ");
        assert!(
            rendered.contains("Custom Label"),
            "expected presentation label prefix, got {rendered}"
        );
        assert!(
            rendered.to_ascii_lowercase().contains("draw a card"),
            "presentation label must not replace the rendered trigger body: {rendered}"
        );
    }

    #[test]
    fn post_substitution_compacts_conditional_source_animation_bundle() {
        let lines = compact_post_substitution_surface_lines(vec![
            "Haste".to_string(),
            "this creature source is creature in addition to its other types and has base power and toughness 4/4 and is dragon as long as two or more nonland permanents entered the battlefield under your control this turn.".to_string(),
            "Goddric has flying as long as two or more nonland permanents entered the battlefield under your control this turn.".to_string(),
            "This source has \"{R}: Dragons you control get +1/+0 until end of turn.\" As long as two or more nonland permanents entered the battlefield under your control this turn.".to_string(),
        ]);
        assert_eq!(
            lines,
            vec![
                "Haste".to_string(),
                "Celebration — As long as two or more nonland permanents entered the battlefield under your control this turn, Goddric is a Dragon with base power and toughness 4/4, flying, and \"{R}: Dragons you control get +1/+0 until end of turn.\"".to_string(),
            ]
        );
    }

    #[test]
    fn scored_line_normalizes_late_milled_card_choice_surface() {
        assert_eq!(
            normalize_scored_compiled_line(
                "Return target permanent spell to its owner's hand, Jeskai Revelation deals 4 damage to any target, create two 1/1 white Monk creature tokens with prowess, draw two cards, then gain 4 life."
                    .to_string()
            ),
            "Return target spell or permanent to its owner's hand. Jeskai Revelation deals 4 damage to any target. Create two 1/1 white Monk creature tokens with prowess. Draw two cards. You gain 4 life."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Sacrifice this enchantment: Creatures your opponents control get -1/-1 and gain attacks each combat if able until end of turn."
                    .to_string()
            ),
            "Sacrifice this enchantment: Creatures your opponents control get -1/-1 until end of turn. Those creatures attack this turn if able."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Exile all cards from their hand. Exile target player's graveyard.".to_string()
            ),
            "Exile all cards from target player's hand and graveyard."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "This creature enters with X +1/+1 counters on it, where X is the number of another creature or artifact you control."
                    .to_string()
            ),
            "This creature enters with a +1/+1 counter on it for each other creature and/or artifact you control"
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Choose a creature at random on the battlefield, gain control of it until end of turn, untap it, it gains haste until end of turn, then destroy all other creatures."
                    .to_string()
            ),
            "Choose a creature at random. You gain control of that creature until end of turn. Untap it. It gains haste until end of turn. Then destroy all other creatures."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. Then if a permanent that shares a card type with it was revealed this way, copy that spell, you may choose new targets for the copy, then each opponent draws a card. Otherwise, draw a card."
                    .to_string()
            ),
            "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card."
        );
    }

    #[test]
    fn color_choice_exclusion_is_inferred_from_fixed_chosen_color_mana() {
        let lines = annotate_color_choice_exclusions(vec![
            "This land enters tapped.".to_string(),
            "As this land enters, choose a color.".to_string(),
            "{T}: Add {U} or one mana of the chosen color.".to_string(),
        ]);

        assert_eq!(
            lines[1],
            "As this land enters, choose a color other than blue"
        );
    }

    #[test]
    fn conditional_followup_case_does_not_lower_activation_costs() {
        assert_eq!(
            normalize_conditional_followup_case(
                "{2}, {T}, Put a blood counter on this artifact: Draw a card."
            ),
            "{2}, {T}, Put a blood counter on this artifact: Draw a card."
        );
        assert_eq!(
            normalize_conditional_followup_case(
                "If it's tapped, Put a stun counter on it. Otherwise, Tap it."
            ),
            "If it's tapped, put a stun counter on it. Otherwise, tap it."
        );
    }

    #[test]
    fn final_surface_keeps_it_reference_for_tap_freeze_text() {
        assert_eq!(
            finalize_ast_surface_line(
                "If you roll 10-20, tap it. That permanent doesn't untap during its controller's next untap step"
                    .to_string()
            ),
            "If you roll 10-20, tap it. It doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn adjacent_conditional_x_counter_lines_use_additional_counter_surface() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "This creature enters with X +1/+1 counters on it.".to_string(),
            "This creature enters with X +1/+1 counters on it if x is 5 or more.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it."
                    .to_string()
            ]
        );
    }

    #[test]
    fn create_token_text_preserves_multiple_creature_subtypes() {
        let token = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie, Subtype::Employee])
            .color_indicator(crate::color::ColorSet::BLACK)
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        assert_eq!(
            compile_effect_list(&[Effect::create_tokens(token, Value::Fixed(1))]),
            "Create a 2/2 black Zombie Employee creature token"
        );
    }

    #[test]
    fn instant_spell_damage_after_counter_keeps_spell_as_source() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Test Blast")
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![
                Effect::counter(crate::ChooseSpec::target_spell()),
                Effect::deal_damage(3, crate::ChooseSpec::target_creature()),
            ])
            .build();

        assert_eq!(
            compiled_text_lines(&definition),
            vec!["Counter target spell and Test Blast deals 3 damage to target creature."]
        );
    }

    fn modal_source_line_probe(spree: bool) -> CardDefinition {
        let modes = vec![
            crate::effect::EffectMode::new("Draw a card", vec![Effect::draw(Value::Fixed(1))]),
            crate::effect::EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
        ];
        let mut modal = crate::effects::ChooseModeEffect::new(
            modes,
            Value::Fixed(1),
            Value::Fixed(if spree { 2 } else { 1 }),
            false,
        );
        if spree {
            modal = modal.with_spree_mana_costs(vec![
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::Blue]),
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]),
            ]);
        }
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Modal Source Line Probe")
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![Effect::new(modal)])
            .build()
    }

    #[test]
    fn typed_modal_source_group_splits_direct_spree_block() {
        assert_eq!(
            compiled_text_lines(&modal_source_line_probe(true)),
            vec![
                "Spree".to_string(),
                "+ {1}{U} — Draw a card.".to_string(),
                "+ {2} — Gain 2 life.".to_string(),
            ]
        );
    }

    #[test]
    fn typed_modal_source_group_keeps_ordinary_bullet_block() {
        assert_eq!(
            compiled_text_lines(&modal_source_line_probe(false)),
            vec!["Choose one —\n• Draw a card.\n• Gain 2 life.".to_string()]
        );
    }

    #[test]
    fn put_counter_then_add_mana_uses_and_surface() {
        assert_eq!(
            compile_effect_list(&[
                Effect::plus_one_counters(1, crate::ChooseSpec::Source),
                Effect::add_mana(vec![ManaSymbol::Red]),
            ]),
            "Put a +1/+1 counter on this source and add {R}"
        );
    }

    #[test]
    fn dynamic_token_pt_setter_compacts_into_creation_text() {
        let token = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ooze")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Ooze])
            .color_indicator(crate::color::ColorSet::GREEN)
            .power_toughness(crate::card::PowerToughness::fixed(0, 0))
            .build();
        let created = crate::TagKey::from("created_0");

        assert_eq!(
            compile_effect_list(&[
                Effect::create_tokens(token, crate::Value::X).tag(created.clone()),
                Effect::set_base_power_toughness(
                    crate::Value::X,
                    crate::Value::X,
                    crate::ChooseSpec::Tagged(created),
                    crate::Until::Forever,
                ),
            ]),
            "Create X X/X green Ooze creature tokens"
        );
    }

    #[test]
    fn destroy_all_groups_then_draw_uses_destroyed_this_way_surface() {
        let effects = vec![
            Effect::new(crate::effects::DestroyEffect::with_spec(
                crate::ChooseSpec::all(crate::ObjectFilter::creature()),
            )),
            Effect::new(crate::effects::DestroyEffect::with_spec(
                crate::ChooseSpec::all(crate::ObjectFilter::enchantment()),
            )),
            Effect::new(crate::effects::DrawCardsEffect::you(
                crate::Value::PendingEffectMetric {
                    source: crate::effect::EffectMetricSource::AffectedObjects,
                    metric: crate::effect::EffectMetric::AffectedCount,
                },
            )),
        ];
        let expected = "Destroy all creatures and enchantments. Draw a card for each permanent destroyed this way";

        assert_eq!(compile_effect_list(&effects), expected);
        assert_eq!(
            crate::compiled_text::render_effects::describe_effect_clause_list(&effects).as_deref(),
            Some(expected)
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn night_shift_compiled_text_preserves_die_adjustment_and_zombie_employee_token() {
        let text = "After you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.\nWhenever you roll a 6, create a 2/2 black Zombie Employee creature token.";
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Night Shift of the Living Dead",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text(text)
        .expect("Night Shift should compile");

        assert_eq!(compiled_text_lines(&definition).join("\n"), text);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn result_conjunction_preserves_safe_and_dependent_specialist_surfaces() {
        let hollow_text = "Flying\nWhenever this creature deals combat damage to a player, you may pay {X}. If you do, that player reveals X cards from their hand and you choose one of them. That player discards that card.";
        let hollow = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hollow Specter")
            .card_types(vec![CardType::Creature])
            .parse_text(hollow_text)
            .expect("Hollow Specter control should compile");
        assert_eq!(compiled_text_lines(&hollow).join("\n"), hollow_text);

        let moku_text = "Whenever you cast a noncreature spell, you may pay {1}. If you do, Moku gets +2/+1 and creatures you control gain haste until end of turn.";
        let moku = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Moku, Meandering Drummer",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(moku_text)
        .expect("Moku control should compile");
        assert_eq!(
            compiled_text_lines(&moku).join("\n"),
            "Whenever you cast a noncreature spell, you may pay {1}. If you do, this creature gets +2/+1 until end of turn and creatures you control gain haste until end of turn."
        );

        let ulalek_text = "Devoid\nWhenever you cast an Eldrazi spell, you may pay {C}{C}. If you do, copy all spells you control, then copy all other activated and triggered abilities you control. You may choose new targets for the copies.";
        let ulalek =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ulalek, Fused Atrocity")
                .card_types(vec![CardType::Creature])
                .parse_text(ulalek_text)
                .expect("Ulalek control should compile");
        assert_eq!(
            compiled_text_lines(&ulalek).join("\n"),
            "Devoid\nWhenever you cast an Eldrazi spell, you may pay {C}{C}. If you do, copy all spells you control, then copy all other activated and triggered abilities you control. You may choose new targets for the copy."
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn spellshift_compiled_text_uses_countered_spell_controller_surface() {
        let text = "Counter target instant or sorcery spell. Its controller reveals cards from the top of their library until they reveal an instant or sorcery card. That player may cast that card without paying its mana cost. Then the player shuffles.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Spellshift")
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .expect("Spellshift should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(
            rendered,
            "Counter target instant spell or sorcery spell. Its controller reveals cards from the top of their library until they reveal an instant or sorcery card. That player may cast it without paying its mana cost. Then shuffle their library."
        );
        assert!(!rendered.contains("that object's controller"));
        assert!(!rendered.contains("target player shuffles"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn implicit_source_combat_prevention_keeps_prevention_surface() {
        let text = "Whenever this creature becomes blocked, prevent all combat damage that would be dealt by it this turn.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ignoble Soldier")
                .card_types(vec![CardType::Creature])
                .parse_text(text)
                .expect("Ignoble Soldier should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, text);
        assert!(!rendered.contains("assigns no combat damage"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn delayed_exile_at_your_next_end_step_stays_delayed() {
        let text = "Return target creature card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of your next end step.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Haunted House")
                .card_types(vec![CardType::Artifact])
                .parse_text(text)
                .expect("Haunted House visit text should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(
            rendered,
            "Return target creature card from your graveyard to the battlefield. It gains haste. At the beginning of your next end step, exile it."
        );
        assert!(!rendered.contains("then exile it"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn undiscovered_paradise_compiles_delayed_untap_instruction_exactly() {
        let text = "{T}: Add one mana of any color. During your next untap step, as you untap your permanents, return this land to its owner's hand.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Undiscovered Paradise")
                .card_types(vec![CardType::Land])
                .parse_text(text)
                .expect("Undiscovered Paradise should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, text);
        assert!(!rendered.contains("Untap a permanent"));

        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("AsPermanentsUntapTrigger")
                && debug.contains("ScheduleDelayedTriggerEffect")
                && debug.contains("ReturnToHandEffect"),
            "expected a source-bound delayed untap instruction, got {debug}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn next_damage_prevention_exiles_prevented_top_cards_as_follow_up() {
        let text = "{2}, {T}: The next time a source of your choice would deal damage to you this turn, prevent that damage. Exile cards from the top of your library equal to the damage prevented this way.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bone Mask")
            .card_types(vec![CardType::Artifact])
            .parse_text(text)
            .expect("Bone Mask prevention text should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, text);

        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("PreventNextTimeDamageEffect")
                && debug.contains("follow_up_effects")
                && debug.contains("ExileTopOfLibraryEffect")
                && debug.contains("EventValue")
                && debug.contains("Amount"),
            "expected a prevented-damage-count exile-top follow-up, got {debug}"
        );
        assert!(
            !debug.contains("ChooseObjectsEffect"),
            "expected direct exile-top follow-up instead of choosing one top card, got {debug}"
        );
    }

    #[test]
    fn conditional_enters_with_counter_uses_adamant_prefix_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "This creature enters with a +1/+1 counter on it if at least three white mana was spent to cast this spell."
                    .to_string()
            ),
            "Adamant — If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it."
        );
    }

    #[test]
    fn same_turn_pump_and_keyword_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "This creature gets +2/+0 as long as it's your turn.".to_string(),
            "This creature has First strike as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, this creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn mixed_during_turn_and_as_long_turn_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature gets +2/+0 as long as it's your turn.".to_string(),
            "During your turn, equipped creature has first strike.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, equipped creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn separate_during_turn_source_and_overlapping_population_statics_stay_separate() {
        let lines = merge_ast_surface_lines(vec![
            "This creature has first strike as long as it's your turn.".to_string(),
            "Creatures you control with +1/+1 counters on them have first strike as long as it's your turn."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "This creature has first strike as long as it's your turn.".to_string(),
                "Creatures you control with +1/+1 counters on them have first strike as long as it's your turn."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn one_during_turn_source_and_other_population_union_still_rejoins() {
        let lines = merge_ast_surface_lines(vec![
            "This creature has flying as long as it's your turn.".to_string(),
            "Other Knights you control have flying as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, this creature and other Knights you control have flying."
                    .to_string()
            ]
        );
    }

    #[test]
    fn attached_object_conditions_merge_across_equivalent_surfaces() {
        let lines = merge_ast_surface_lines(vec![
            "Enchanted creature gets +1/+1 as long as enchanted creature is blue.".to_string(),
            "As long as the permanent this source is attached to is blue, enchanted creature can't be blocked."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "As long as enchanted creature is blue, enchanted creature gets +1/+1 and can't be blocked"
                    .to_string()
            ]
        );
    }

    #[test]
    fn equipped_keyword_and_conditional_pt_bonus_keep_separate_lines() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature has first strike.".to_string(),
            "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Equipped creature has first strike.".to_string(),
                "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
            ]
        );
    }

    #[test]
    fn each_creature_turn_pump_and_keyword_merge_to_plural_subject() {
        let lines = merge_ast_surface_lines(vec![
            "Each creature you control gets +1/+0 as long as it's your turn.".to_string(),
            "Creatures you control have Trample as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, creatures you control get +1/+0 and have trample".to_string()]
        );
    }

    #[test]
    fn exact_turn_conditioned_pump_uses_during_your_turn_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Each creature you control gets +2/+0 as long as it's your turn".to_string()
            ),
            "During your turn, creatures you control get +2/+0."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "This creature gets +2/+2 as long as it's your turn".to_string()
            ),
            "During your turn, this creature gets +2/+2."
        );
    }

    #[test]
    fn matching_cast_and_activation_restrictions_merge() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "During your turn, Your opponents can't cast spells.".to_string(),
            "During your turn, your opponents can't activate abilities of artifacts creatures or enchantments."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments"
                    .to_string()
            ]
        );
    }

    #[test]
    fn sacrifice_enchantment_counter_spell_trigger_uses_single_when_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever an opponent casts a spell, sacrifice this enchantment. Counter it"
                    .to_string()
            ),
            "When an opponent casts a spell, sacrifice this enchantment and counter that spell."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "When a player casts a spell, sacrifice this enchantment. Counter it".to_string()
            ),
            "When a player casts a spell, sacrifice this enchantment and counter that spell."
        );
    }

    #[test]
    fn self_exile_attacking_nonflying_creature_surface_uses_this_creature() {
        assert_eq!(
            finalize_ast_surface_line(
                "{1}{R}{G}, {T}: Exile Hunting Kavu and target creature without flying that's attacking you"
                    .to_string()
            ),
            "{1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you."
        );
    }

    #[test]
    fn spellcast_trigger_copy_surface_drops_or_ability() {
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever you cast an Assassin, Mercenary, Pirate, Rogue, or Warlock spell, copy that spell or ability"
                    .to_string()
            ),
            "Whenever you cast an outlaw spell, copy that spell or ability."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever you cast an instant or sorcery spell that targets only this creature, copy that spell or ability. You may choose new targets for the copy"
                    .to_string()
            ),
            "Whenever you cast an instant or sorcery spell that targets only this creature, copy that spell or ability. You may choose new targets for the copy."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever you cast an instant or sorcery spell that targets only this creature, you may pay {2}. If you do, copy that spell or ability. You may choose new targets for the copy"
                    .to_string()
            ),
            "Whenever you cast an instant or sorcery spell that targets only this creature, you may pay {2}. If you do, copy that spell or ability. You may choose new targets for the copy."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "When you cast a spell or ability, copy that spell or ability. You may choose new targets for the copy"
                    .to_string()
            ),
            "When you cast a spell or ability, copy that spell or ability. You may choose new targets for the copy."
        );
    }

    #[test]
    fn basic_land_type_choice_surface_uses_of_your_choice() {
        assert_eq!(
            finalize_ast_surface_line(
                "{T}: Choose a basic land type. Target land you control becomes that type until end of turn"
                    .to_string()
            ),
            "{T}: Target land you control becomes the basic land type of your choice until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a basic land type. Target land becomes that type until end of turn. Draw a card"
                    .to_string()
            ),
            "Target land becomes the basic land type of your choice until end of turn. Draw a card."
        );
    }

    #[test]
    fn choose_sacrifice_rest_surface_uses_the_rest() {
        assert_eq!(
            finalize_ast_surface_line(
                "Each player chooses three permanents that player controls on the battlefield. Sacrifice all other permanents that player controls"
                    .to_string()
            ),
            "Each player chooses three permanents they control, then sacrifices the rest."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Each player chooses a creature or planeswalker that player controls on the battlefield. Sacrifice all other creatures or planeswalkers that player controls. Players can't cast creature or planeswalker spells until the end of your next turn"
                    .to_string()
            ),
            "Each player chooses a creature or planeswalker they control, then sacrifices the rest. Players can't cast creature or planeswalker spells until the end of your next turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "At the beginning of each opponent's end step, that player chooses up to 2 creatures that player controls on the battlefield, then that player sacrifices all other creatures that player controls"
                    .to_string()
            ),
            "At the beginning of each opponent's end step, that player chooses up to two creatures they control, then sacrifices the rest."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "At the beginning of each opponent's end step, that player chooses up to 2 creatures that player controls, then that player sacrifices all other creatures that player controls"
                    .to_string()
            ),
            "At the beginning of each opponent's end step, that player chooses up to two creatures they control, then sacrifices the rest."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "−9: For each opponent, that player chooses a permanent that player controls on the battlefield. Sacrifice all other permanents that player controls"
                    .to_string()
            ),
            "−9: For each opponent, that player chooses a permanent that player controls on the battlefield. Sacrifice all other permanents that player controls."
        );
    }

    #[test]
    fn temporary_trample_pump_surface_merges_until_end_of_turn() {
        assert_eq!(
            finalize_ast_surface_line(
                "Draw a card, target creature gains trample until end of turn, then it gets +1/+0 for each the number of cards you've drawn this turn until end of turn."
                    .to_string()
            ),
            "Draw a card. Until end of turn, target creature gains trample and gets +1/+0 for each card you've drawn this turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Target creature gains trample until end of turn, then it gets +X/+X until end of turn, where X is the number of attacking creatures"
                    .to_string()
            ),
            "Target creature gains trample and gets +X/+X until end of turn, where X is the number of attacking creatures."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "At the beginning of combat on your turn, target Elf you control gains trample until end of turn, then it gets +X/+X until end of turn, where X is the number of Forests you control"
                    .to_string()
            ),
            "At the beginning of combat on your turn, target Elf you control gains trample and gets +X/+X until end of turn, where X is the number of Forests you control."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "−5: Target creature gains trample until end of turn, then it gets +X/+X until end of turn, where X is the number of lands you control"
                    .to_string()
            ),
            "−5: Target creature gains trample and gets +X/+X until end of turn, where X is the number of lands you control."
        );
    }

    #[test]
    fn chosen_player_mana_surface_uses_that_player_chooses() {
        assert_eq!(
            finalize_ast_surface_line(
                "{T}: You choose a player, then add one mana of any color to that player's mana pool"
                    .to_string()
            ),
            "{T}: You choose a player. That player adds one mana of any color they choose."
        );
        // Honest surfaces: the "That player adds ... they choose" rewrite
        // moved the color choice to the chosen player, a claim the render
        // never made (deleted as score laundering).
        assert_eq!(
            finalize_ast_surface_line(
                "When this creature enters, you choose a player, then add two mana of any one color to that player's mana pool"
                    .to_string()
            ),
            "When this creature enters, you choose a player, then add two mana of any one color to that player's mana pool."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "When this creature enters, choose a player, then add two mana of any one color to that player's mana pool"
                    .to_string()
            ),
            "When this creature enters, choose a player, then add two mana of any one color to that player's mana pool."
        );
    }

    #[test]
    fn role_token_and_return_with_counter_surfaces_compact() {
        assert_eq!(
            finalize_ast_surface_line(
                "Target creature gets +2/+0 until end of turn, create a Monster Role token, then attach it to it"
                    .to_string()
            ),
            "Target creature gets +2/+0 until end of turn. Create a Monster Role token attached to it."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Exile target artifact or creature, return it to the battlefield under its owner's control, then put a +1/+1 counter on it"
                    .to_string()
            ),
            "Exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it."
        );
    }

    #[test]
    fn simple_token_keyword_surface_uses_with_keyword() {
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 white Knight creature token. It has \"Banding.\"".to_string()
            ),
            "Create a 1/1 white Knight creature token with banding."
        );
    }

    #[test]
    fn chosen_creature_type_surface_uses_of_your_choice() {
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then creatures of the chosen type get -3/-3 until end of turn"
                    .to_string()
            ),
            "Creatures of the creature type of your choice get -3/-3 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then creatures of the chosen type get -3/-3 until end of turn"
                    .to_string()
            ),
            "Creatures of the creature type of your choice get -3/-3 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then creatures of the chosen type get +0/+4 until end of turn"
                    .to_string()
            ),
            "Creatures of the creature type of your choice get +0/+4 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then creatures of the chosen type get -1/-1 until end of turn"
                    .to_string()
            ),
            "Choose a creature type. All creatures of that type get -1/-1 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then creatures of the chosen type get -1/-1 until end of turn."
                    .to_string()
            ),
            "Choose a creature type. All creatures of that type get -1/-1 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then return up to three target creature cards of the chosen type from your graveyard to your hand"
                    .to_string()
            ),
            "Return up to three target creature cards of the creature type of your choice from your graveyard to your hand."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then return X target creatures of the chosen type to their owners' hands"
                    .to_string()
            ),
            "Return X target creatures of the creature type of your choice to their owners' hands."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then destroy all creatures of the chosen type".to_string()
            ),
            "Destroy all creatures of the creature type of your choice."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then return all creatures that aren't of the chosen type to their owners' hands"
                    .to_string()
            ),
            "You choose a creature type, then return all creatures that aren't of the chosen type to their owners' hands."
        );
    }

    #[test]
    fn imperative_choice_selection_surfaces_keep_oracle_compactions() {
        assert_eq!(
            finalize_ast_surface_line(
                "Look at the top three cards of your library, choose a card in a hand, in a graveyard, or in exile, choose an other card in a hand, in a graveyard, or in exile, choose an other other card in a hand, in a graveyard, or in exile, return it to its owner's hand, put it on the bottom of its owner's library, exile it, then you may play those cards this turn."
                    .to_string()
            ),
            "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn."
        );
        // Honest surface: the render genuinely loses the hand/graveyard zone
        // split for the two choices; the old expectation was a hand-written
        // gate that re-asserted the zones (deleted as score laundering).
        assert_eq!(
            finalize_ast_surface_line(
                "Target opponent reveals their hand, choose an artifact or creature card, choose an artifact or creature card, then exile it."
                    .to_string()
            ),
            "Target opponent reveals their hand, choose an artifact or creature card, choose an artifact or creature card, then exile it."
        );
    }

    #[test]
    fn target_type_disjunction_keeps_shared_opponent_controller_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Destroy target creature an opponent controls or enchantment".to_string()
            ),
            "Destroy target creature or enchantment an opponent controls."
        );
    }

    #[test]
    fn life_loss_mana_value_uses_that_permanent_surface() {
        assert_eq!(
            finalize_ast_surface_line("You lose life equal to its mana value".to_string()),
            "You lose life equal to that permanent's mana value."
        );
    }

    #[test]
    fn skulk_keyword_pairs_keep_oracle_line_breaks() {
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, lifelink".to_string()),
            vec!["Skulk".to_string(), "Lifelink".to_string()]
        );
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, deathtouch".to_string()),
            vec!["Skulk".to_string(), "Deathtouch".to_string()]
        );
        assert_eq!(
            expand_finalized_ast_surface_line(
                "Put a shield counter on target creature. Scry 1.".to_string()
            ),
            vec![
                "Put a shield counter on target creature.".to_string(),
                "Scry 1.".to_string(),
            ]
        );
        assert_eq!(
            expand_finalized_ast_surface_line(
                "Look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card."
                    .to_string()
            ),
            vec![
                "Look at the top three cards of your library, then put them back in any order. You may shuffle."
                    .to_string(),
                "Draw a card.".to_string(),
            ]
        );
    }

    #[test]
    fn token_quote_activation_costs_keep_colon_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token, add {C}.\""
                    .to_string()
            ),
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\""
        );
    }

    #[test]
    fn terminal_quoted_ability_does_not_add_a_second_sentence_period() {
        assert_eq!(
            finalize_ast_surface_line(
                "Until end of turn, this land becomes a creature with \"{X}: This creature gets +X/+0 until end of turn.\". It's still a land"
                    .to_string(),
            ),
            "Until end of turn, this land becomes a creature with \"{X}: This creature gets +X/+0 until end of turn.\" It's still a land."
        );
        assert_eq!(
            remove_redundant_period_after_terminal_quote(
                "It has \"Can you pay?\". This sentence follows."
            ),
            "It has \"Can you pay?\" This sentence follows."
        );
        assert_eq!(
            remove_redundant_period_after_terminal_quote(
                "It has \"Flying\". This period terminates the containing sentence."
            ),
            "It has \"Flying\". This period terminates the containing sentence."
        );
    }

    #[test]
    fn unleash_scaffolding_compacts_to_keyword_surface() {
        assert_eq!(
            merge_ast_surface_lines(vec![
                "First strike, haste".to_string(),
                "When this creature enters, you may put a +1/+1 counter on this creature."
                    .to_string(),
                "This creature can't block as long as it has a +1/+1 counter on it.".to_string(),
            ]),
            vec!["First strike, haste".to_string(), "Unleash".to_string()]
        );
    }

    #[test]
    fn plural_turn_animation_and_granted_trigger_merge_to_bello_surface() {
        let lines = merge_ast_surface_lines(vec![
            "During your turn, non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control are creatures in addition to their other types and have base power and toughness 4/4 and are Elementals in addition to their other types and have indestructible and have haste.".to_string(),
            "Non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control have \"whenever this creature deals combat damage to a player, draw a card.\" As long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""
                    .to_string()
            ]
        );

        let split_layer_lines = merge_ast_surface_lines(vec![
            "During your turn, Each non-Equipment artifact with mana value 4 or greater you control or a non-Aura enchantment with mana value 4 or greater you control is a creature in addition to its other types.".to_string(),
            "During your turn, non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control have base power and toughness 4/4 and are Elementals in addition to their other types and have indestructible and haste and have \"whenever this creature deals combat damage to a player, draw a card.\"".to_string(),
        ]);

        assert_eq!(
            split_layer_lines,
            vec![
                "During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""
                    .to_string()
            ]
        );

        let folded_lines = merge_ast_surface_lines(vec![
            "During your turn, non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control are creatures in addition to their other types and have base power and base toughness 4/4 and are Elementals in addition to their other types and have indestructible and haste and have \"whenever this creature deals combat damage to a player, draw a card.\".".to_string(),
        ]);

        assert_eq!(
            folded_lines,
            vec![
                "During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_conditional_keyword_grants_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "At the beginning of each combat, if you control a creature with first strike, creatures you control gain first strike until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with flying, creatures you control gain flying until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with vigilance, creatures you control gain vigilance until end of turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "At the beginning of each combat, creatures you control gain first strike until end of turn if a creature you control has first strike. The same is true for flying and vigilance."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_graveyard_keyword_grants_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "At the beginning of combat on your turn, if there is a creature card with flying in your graveyard, creatures you control gain flying until end of turn.".to_string(),
            "At the beginning of combat on your turn, if there is a creature card with first strike in your graveyard, creatures you control gain first strike until end of turn.".to_string(),
            "At the beginning of combat on your turn, if there is a creature card with vigilance in your graveyard, creatures you control gain vigilance until end of turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "At the beginning of combat on your turn, creatures you control gain flying until end of turn if a creature card in your graveyard has flying. The same is true for first strike and vigilance."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_type_additions_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Creatures you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature spells you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature cards you own that aren't on the battlefield are the chosen type in addition to their other types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_creature_subtype_additions_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Slivers you control and nontoken creatures you control are the chosen type in addition to their other creature types."
                .to_string(),
            "Creature spells you control are the chosen type in addition to their other creature types."
                .to_string(),
            "Creature cards you own that aren't on the battlefield are the chosen type in addition to their other creature types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Slivers you control and nontoken creatures you control are the chosen type in addition to their other creature types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn all_permanent_type_additions_compact_stack_and_owned_zone_subjects() {
        let lines = merge_ast_surface_lines(vec![
            "Nonland permanents you control are artifacts in addition to their other types."
                .to_string(),
            "Artifact, creature, enchantment, land, planeswalker, and battle spells you control are artifacts in addition to their other types."
                .to_string(),
            "Nonland permanent cards in your hand or nonland permanent cards in your library or nonland permanent cards in your graveyard or nonland permanent cards in your exile or nonland permanent cards in your command zone are artifacts in addition to their other types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Nonland permanents you control are artifacts in addition to their other types. The same is true for permanent spells you control and nonland permanent cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_color_changes_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Nonland permanents you control are white.".to_string(),
            "Spells you control are white.".to_string(),
            "Nonland cards in your hand or nonland cards in your library or nonland cards in your graveyard or nonland cards in your exile or nonland cards in your command zone are white."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Nonland permanents you control are white. The same is true for spells you control and nonland cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn during_your_turn_prevent_clause_drops_extra_has() {
        assert_eq!(
            finalize_ast_surface_line(
                "During your turn, this creature has Prevent all damage that would be dealt to this creature."
                    .to_string()
            ),
            "During your turn, prevent all damage that would be dealt to this creature."
        );
    }

    #[test]
    fn gain_control_untap_fixed_pump_haste_uses_oracle_sentence_shape() {
        assert_eq!(
            finalize_ast_surface_line(
                "Gain control of target creature until end of turn, untap it, it gets +2/+0 until end of turn, then it gains haste until end of turn"
                    .to_string()
            ),
            "Gain control of target creature until end of turn. Untap that creature. Until end of turn, it gets +2/+0 and gains haste."
        );
    }

    #[test]
    fn compiled_text_cleanup_layers_reject_known_semantic_rescue_strings() {
        let checked_sources = [
            ("mod.rs", include_str!("mod.rs")),
            ("normalize_common.rs", include_str!("normalize_common.rs")),
            ("debug_safe.rs", include_str!("debug_safe.rs")),
            ("surface_helpers.rs", include_str!("surface_helpers.rs")),
        ];
        let banned = [
            concat!("K", "ain"),
            concat!("allagan", " eye"),
            concat!("Flame", "break"),
            concat!(
                "deals 3 damage to each creature without flying",
                ", deal 3 damage to each player"
            ),
            concat!(
                "Gain control of target creature until end of turn",
                ", untap it, then it gains haste"
            ),
            concat!(
                "Untap target creature, gain control of it until end of turn",
                ", then it gains haste"
            ),
            concat!(
                "You choose the top card in your library",
                ", exile it, then you may play that card"
            ),
            concat!(
                "for each card revealed this way",
                ", unless it's a permanent, put that object"
            ),
        ];

        for (source_name, source) in checked_sources {
            for needle in banned {
                assert!(
                    !source.contains(needle),
                    "{source_name} contains semantic rescue text that belongs in structural rendering: {needle}"
                );
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn ability_surface_text_for_tests(ability: &Ability) -> String {
    ability_surface_text(ability)
}
