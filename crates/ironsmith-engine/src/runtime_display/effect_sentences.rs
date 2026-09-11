//! Prompt wording for runtime effects, recovered from the source's card text.
//!
//! Gameplay builds do not link the canonical renderer in `ironsmith-text`, so a
//! runtime effect carries no authored wording of its own and the structural
//! `Debug` rendering is the only thing left to print. Game objects do carry
//! their compiled card text, though, and every effect the engine prompts for
//! was compiled out of one of that text's sentences. Matching an effect back to
//! the sentences it came from keeps prompts in the card's own words.
//!
//! Matching is deliberately structural rather than card-specific: an effect's
//! executor type names (`CopySpellEffect` -> "copy", "spell") are scored
//! against the words of each printed sentence, so a new card needs no entry
//! here to get a readable prompt.

use crate::effect::Effect;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;

/// Depth and breadth caps for the executor-tree walk that collects keywords.
/// Prompts are rare, but effect trees can nest arbitrarily.
const MAX_EFFECT_DEPTH: usize = 6;
const MAX_EFFECT_NODES: usize = 64;

/// The shortest prefix two words must share to count as the same word, so that
/// "copy" matches "copies" and "target" matches "targets".
const MIN_STEM_LEN: usize = 4;

/// Structural words that every effect tree carries and no card text contains.
const STRUCTURAL_WORDS: &[&str] = &[
    "effect",
    "with",
    "id",
    "tag",
    "tagged",
    "sequence",
    "list",
    "each",
    "may",
    "optional",
    "outcome",
    "only",
    "inner",
    "wrapper",
    "conditional",
    "composite",
    "apply",
    "new",
    "value",
    "fixed",
    "ref",
];

/// The clause a "may" prompt should offer, phrased to follow "You may ".
///
/// Falls back to `"perform the effect"` rather than to any structural
/// rendering: a prompt is read by a player, never by the rules engine.
pub fn optional_effect_prompt(
    game: &GameState,
    source: ObjectId,
    source_snapshot: Option<&ObjectSnapshot>,
    ability_index: Option<usize>,
    effects: &[Effect],
) -> String {
    let clause = matched_sentences(game, source, source_snapshot, ability_index, effects, true)
        .unwrap_or_else(|| "perform the effect".to_string());
    // The other optional prompts (`describe_move`, the same-name search prompt)
    // read as standalone capitalized phrases, so this one does too.
    capitalize_first(&clause)
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Whether a string is a Rust `Debug` rendering rather than card wording.
///
/// The engine keeps structural `Debug` fallbacks for effects, abilities and
/// conditions, and any of them can end up in a string bound for a player. Two
/// shapes give a derived `Debug` away and never occur in card text: a struct
/// literal (`Ident { field: `) and a tuple/newtype head with no space before
/// its parenthesis (`TagKey(`). Reminder text always puts a space before "(",
/// and mana symbols never carry a field separator.
pub fn looks_like_compiled_structure(text: &str) -> bool {
    let bytes = text.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'{' => {
                let mut at = index + 1;
                while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
                    at += 1;
                }
                let start = at;
                while bytes
                    .get(at)
                    .is_some_and(|ch| ch.is_ascii_lowercase() || *ch == b'_' || ch.is_ascii_digit())
                {
                    at += 1;
                }
                if at > start && bytes.get(at) == Some(&b':') && bytes.get(at + 1) != Some(&b':') {
                    return true;
                }
            }
            b'(' if index > 0 => {
                let mut at = index;
                while at > 0
                    && bytes[at - 1]
                        .is_ascii_alphanumeric()
                {
                    at -= 1;
                }
                let head = &bytes[at..index];
                if head.len() >= 2
                    && head[0].is_ascii_uppercase()
                    && head[1..].iter().any(u8::is_ascii_lowercase)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// The printed sentence(s) these effects were compiled from, for labels that
/// describe an ability rather than offer a choice about it.
pub fn effect_summary_text(
    game: &GameState,
    source: ObjectId,
    source_snapshot: Option<&ObjectSnapshot>,
    ability_index: Option<usize>,
    effects: &[Effect],
) -> Option<String> {
    matched_sentences(game, source, source_snapshot, ability_index, effects, false)
}

/// Find the printed sentences covering `effects`.
///
/// With `optional`, only sentences that actually say "may" are eligible and the
/// leading "you may" is stripped, so the result reads as the offer itself.
fn matched_sentences(
    game: &GameState,
    source: ObjectId,
    source_snapshot: Option<&ObjectSnapshot>,
    ability_index: Option<usize>,
    effects: &[Effect],
    optional: bool,
) -> Option<String> {
    if effects.is_empty() {
        return None;
    }
    let per_effect: Vec<Vec<String>> = effects
        .iter()
        .map(|effect| effect_keywords(std::slice::from_ref(effect)))
        .collect();

    let mut best: Option<(usize, String)> = None;
    let mut offers = 0usize;
    let mut sole_offer = None;
    for line in candidate_lines(game, source, source_snapshot, ability_index) {
        let sentences = split_sentences(&line);
        for (index, sentence) in sentences.iter().enumerate() {
            let opening = if optional {
                match may_clause(sentence) {
                    Some(clause) => clause,
                    None => continue,
                }
            } else {
                sentence.trim()
            };
            if optional {
                offers += 1;
                sole_offer = Some(opening.to_string());
            }
            let score = per_effect
                .iter()
                .filter(|keywords| sentence_matches(opening, keywords))
                .count();
            if score == 0 || best.as_ref().is_some_and(|(seen, _)| *seen >= score) {
                continue;
            }
            best =
                Some((score, extend_with_followups(opening, &sentences[index + 1..], &per_effect)));
        }
    }

    // An effect built entirely out of structural wrappers offers no words to
    // score with. When the source prints exactly one offer, that offer is still
    // unambiguously this one.
    best.map(|(_, text)| text).or(if offers == 1 { sole_offer } else { None })
}

/// Append the sentences that spell out the rest of the same instruction.
///
/// A following sentence only joins when it matches an effect the opening
/// sentence did not, which is what separates "The copy targets Ivy." (part of
/// the optional copy) from a mandatory instruction printed after it.
fn extend_with_followups(
    opening: &str,
    following: &[String],
    per_effect: &[Vec<String>],
) -> String {
    let mut claimed: Vec<bool> = per_effect
        .iter()
        .map(|keywords| sentence_matches(opening, keywords))
        .collect();
    let mut text = opening.to_string();
    for sentence in following {
        let Some(index) = per_effect
            .iter()
            .enumerate()
            .position(|(index, keywords)| !claimed[index] && sentence_matches(sentence, keywords))
        else {
            break;
        };
        claimed[index] = true;
        if !text.ends_with('.') {
            text.push('.');
        }
        text.push(' ');
        text.push_str(sentence.trim());
    }
    text.trim_end_matches(['.', ' ']).to_string()
}

/// The printed text lines that could have produced this effect.
///
/// A known ability index narrows the search to its own line, but only when the
/// text box still has one line per ability; granted or removed abilities break
/// that correspondence, and then every line stays in play.
fn candidate_lines(
    game: &GameState,
    source: ObjectId,
    source_snapshot: Option<&ObjectSnapshot>,
    ability_index: Option<usize>,
) -> Vec<String> {
    let object = game.object(source);
    let text = object
        .map(|object| object.compiled_card_text.as_ref())
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            source_snapshot
                .map(|snapshot| snapshot.compiled_card_text.clone())
                .filter(|text| !text.trim().is_empty())
        })
        .unwrap_or_default();

    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();

    let ability_count = object
        .map(|object| object.abilities.len())
        .or_else(|| source_snapshot.map(|snapshot| snapshot.abilities.len()));
    if let (Some(index), Some(count)) = (ability_index, ability_count)
        && count == lines.len()
        && let Some(line) = lines.get(index)
    {
        return vec![line.clone()];
    }
    lines
}

/// Split one printed line into sentences.
fn split_sentences(line: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') && chars.peek().is_none_or(|next| next.is_whitespace()) {
            let sentence = current.trim().to_string();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            current.clear();
        }
    }
    let sentence = current.trim().to_string();
    if !sentence.is_empty() {
        sentences.push(sentence);
    }
    sentences
}

/// The part of a sentence a player is being offered, or `None` when the
/// sentence is not an offer at all.
fn may_clause(sentence: &str) -> Option<&str> {
    let lowered = sentence.to_ascii_lowercase();
    let at = find_word(&lowered, "may")?;
    let clause = sentence[at + "may".len()..]
        .trim_start()
        .trim_end_matches(['.', '!', '?'])
        .trim();
    (!clause.is_empty()).then_some(clause)
}

/// Byte offset of `needle` in `haystack` at word boundaries on both sides.
fn find_word(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(offset) = haystack.get(from..)?.find(needle) {
        let at = from + offset;
        let after = at + needle.len();
        let opens = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        let closes = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
        if opens && closes {
            return Some(at);
        }
        from = after;
    }
    None
}

/// Whether a sentence uses any of the words an effect's structure implies.
fn sentence_matches(sentence: &str, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return false;
    }
    let words = words_of(sentence);
    keywords
        .iter()
        .any(|keyword| words.iter().any(|word| same_word(keyword, word)))
}

fn words_of(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn same_word(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let shared = left.len().min(right.len());
    shared >= MIN_STEM_LEN && left[..shared] == right[..shared]
}

/// The vocabulary an effect tree implies, from its executor type names.
fn effect_keywords(effects: &[Effect]) -> Vec<String> {
    let mut keywords = Vec::new();
    let mut budget = MAX_EFFECT_NODES;
    collect_keywords(effects, 0, &mut budget, &mut keywords);
    keywords.sort();
    keywords.dedup();
    keywords.retain(|word| word.len() > 1 && !STRUCTURAL_WORDS.contains(&word.as_str()));
    keywords
}

fn collect_keywords(
    effects: &[Effect],
    depth: usize,
    budget: &mut usize,
    keywords: &mut Vec<String>,
) {
    if depth > MAX_EFFECT_DEPTH {
        return;
    }
    for effect in effects {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        push_split_identifier(&executor_type_name(effect), keywords);
        effect.visit_child_effects(&mut |child| {
            collect_keywords(std::slice::from_ref(child), depth + 1, budget, keywords);
        });
    }
}

/// The executor's type name, read off the head of its derived `Debug` output.
fn executor_type_name(effect: &Effect) -> String {
    let rendered = format!("{effect:?}");
    let inner = rendered.strip_prefix("Effect(").unwrap_or(rendered.as_str());
    inner
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect()
}

/// Split `CopySpellEffect` into "copy", "spell", "effect".
fn push_split_identifier(identifier: &str, out: &mut Vec<String>) {
    let mut word = String::new();
    for ch in identifier.chars() {
        if (ch.is_ascii_uppercase() || ch == '_') && !word.is_empty() {
            out.push(std::mem::take(&mut word));
        }
        if ch == '_' {
            continue;
        }
        word.push(ch.to_ascii_lowercase());
    }
    if !word.is_empty() {
        out.push(word);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{CardDefinition, CardDefinitionBuilder};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn source_with_text(game: &mut GameState, text: &str) -> ObjectId {
        let mut definition: CardDefinition =
            CardDefinitionBuilder::new(CardId::new(), "Printed Source")
                .card_types(vec![CardType::Creature])
                .build();
        definition.canonical_text = text.to_string();
        game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Battlefield)
    }

    fn prompt(text: &str, effects: &[Effect]) -> String {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = source_with_text(&mut game, text);
        optional_effect_prompt(&game, source, None, None, effects)
    }

    const IVY: &str = "Flying\nWhenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.";

    #[test]
    fn optional_prompt_quotes_the_printed_may_clause() {
        let text = "Whenever a creature you control dies, you may draw a card.";

        assert_eq!(prompt(text, &[Effect::draw(1)]), "Draw a card");
    }

    #[test]
    fn optional_prompt_keeps_the_follow_up_sentence_the_offer_covers() {
        let text = "At the beginning of your upkeep, you may draw a card. Gain 2 life.";

        assert_eq!(
            prompt(text, &[Effect::draw(1), Effect::gain_life(2)]),
            "Draw a card. Gain 2 life"
        );
    }

    #[test]
    fn optional_prompt_stops_before_instructions_the_offer_does_not_cover() {
        let text = "At the beginning of your upkeep, you may draw a card. Gain 2 life.";

        assert_eq!(prompt(text, &[Effect::draw(1)]), "Draw a card");
    }

    #[test]
    fn optional_prompt_picks_the_may_clause_matching_the_offered_effect() {
        let text = "You may draw a card.\nYou may gain 2 life.";

        assert_eq!(prompt(text, &[Effect::gain_life(2)]), "Gain 2 life");
    }

    #[test]
    fn optional_prompt_never_exposes_the_compiled_structure() {
        for text in [IVY, "", "Flying"] {
            let rendered = prompt(text, &[Effect::draw(1), Effect::gain_life(2)]);
            assert!(
                !rendered.contains("Effect(") && !rendered.contains('{'),
                "prompt leaked compiled structure for {text:?}: {rendered}"
            );
        }
    }

    #[test]
    fn optional_prompt_falls_back_to_plain_wording_without_printed_text() {
        assert_eq!(prompt("", &[Effect::draw(1)]), "Perform the effect");
        assert_eq!(prompt("Flying", &[Effect::draw(1)]), "Perform the effect");
    }

    #[test]
    fn summary_text_uses_the_printed_sentence_for_a_mandatory_ability() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = source_with_text(&mut game, "When this creature enters, draw a card.");

        assert_eq!(
            effect_summary_text(&game, source, None, None, &[Effect::draw(1)]).as_deref(),
            Some("When this creature enters, draw a card")
        );
    }

    #[test]
    fn ivy_shaped_copy_offer_reads_as_the_card_prints_it() {
        // The shape Ivy compiles to: a tagged, id-bearing copy of the
        // triggering spell, followed by the retarget that fixes the copy.
        let copy = Effect::with_id(0, Effect::copy_spell(crate::target::ChooseSpec::spell()))
            .tag("__copied_stack_object__");

        assert_eq!(prompt(IVY, &[copy]), "Copy that spell");
    }

    #[test]
    fn compiled_structure_is_recognized_but_card_text_is_not() {
        for structural in [
            r#"Effect(WithIdEffect { id: EffectId(0), effect: Effect(CopySpellEffect { copier: You }) })"#,
            r#"TagKey("__copied_stack_object__")"#,
            "Ability { kind: Static(StaticAbility(Flying, StaticAbilityInstanceId(1))) }",
            "MayEffect { effects: [], fallback: Decline }",
        ] {
            assert!(
                looks_like_compiled_structure(structural),
                "missed compiled structure: {structural}"
            );
        }

        for printed in [
            "Flying",
            "Whenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.",
            "{T}: Add {G}.",
            "Flying (This creature can't be blocked except by creatures with flying or reach.)",
            "Choose one \u{2014} \u{2022} Draw a card. \u{2022} Gain 2 life.",
            "Kicker {2}{B} (You may pay an additional {2}{B} as you cast this spell.)",
            "Sacrifice a creature: Draw a card. Activate only as a sorcery.",
            "Copy that spell. You may choose new targets for the copy.",
        ] {
            assert!(
                !looks_like_compiled_structure(printed),
                "card text misread as compiled structure: {printed}"
            );
        }
    }

    #[test]
    fn keywords_come_from_executor_type_names() {
        let keywords = effect_keywords(&[Effect::draw(1)]);

        assert!(keywords.contains(&"draw".to_string()), "{keywords:?}");
        assert!(keywords.contains(&"cards".to_string()), "{keywords:?}");
        assert!(!keywords.contains(&"effect".to_string()), "{keywords:?}");
    }

    #[test]
    fn sentences_split_on_terminators_only_at_word_boundaries() {
        assert_eq!(
            split_sentences("Draw a card. Then discard a card."),
            vec!["Draw a card.", "Then discard a card."]
        );
    }

    #[test]
    fn may_clause_requires_the_word_itself() {
        assert_eq!(may_clause("You may draw a card."), Some("draw a card"));
        assert_eq!(may_clause("Mayhem costs less."), None);
    }
}
