use crate::color::ColorSet;
use crate::lexer::{LexStream, OwnedLexToken, lex_line, parser_token_word_refs};
use crate::model::token_definition::{
    ArtifactTokenShape, AstartesWarriorTokenShape, BuiltinTokenShape,
    ConstructArtifactScalingShape, ConstructTokenShape, CreatureTokenInlineRuleKind,
    CreatureTokenInlineRulePresentation, CreatureTokenRulesShape, CreatureTokenShape,
    EnchantmentTokenShape, LandTokenShape, ModifiedBuiltinTokenShape, ShapeshifterTokenShape, TokenCombatRestrictionShape,
    TokenDefinitionSpec, TokenKeywordShape, TokenPowerAsThoughGreaterShape, VehicleTokenShape,
};
use crate::target::SourceReferenceSurface;
use crate::types::{CardType, Subtype};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::{leaf, primitives};
use super::common;
use super::equipment;
use super::names;
use super::rules;

struct TokenDescriptionScope {
    outer: Vec<OwnedLexToken>,
    rule_source: Vec<OwnedLexToken>,
    name: Option<String>,
    complete_quotes: bool,
}

fn token_description_scope(tokens: &[OwnedLexToken]) -> TokenDescriptionScope {
    let projection = common::token_description_projection(tokens);
    let appositive = names::leading_appositive_token_parts(&projection.outer);
    let descriptor = appositive.as_ref().map_or(projection.outer.as_slice(), |(_, descriptor)| *descriptor);
    let start = descriptor.first().map(|token| token.span.start);
    let named = names::named_card_name_parts(descriptor);
    let outside_name = |token: &&OwnedLexToken| named.as_ref().is_none_or(|(_, span)|
        token.span.end <= span.start || token.span.start >= span.end);
    let outer = descriptor.iter().filter(|token| outside_name(token)).cloned().collect();
    let rule_source = tokens.iter().filter(|token| start.is_none_or(|start| token.span.start >= start))
        .filter(|token| outside_name(token)).cloned().collect();
    let name = named.map(|(name, _)| name).or_else(|| appositive.map(|(name, _)| name));
    TokenDescriptionScope { outer, rule_source, name, complete_quotes: projection.complete_quotes }
}

pub fn token_definition_outer_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    token_description_scope(tokens).outer
}

pub fn token_definition_rule_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    token_description_scope(tokens).rule_source
}

fn token_pt(words: &[&str]) -> Option<(i32, i32)> {
    for word in words {
        if let Ok(parsed) = leaf::parse_leaf_unsigned_pt_complete(word) {
            return Some(parsed);
        }
    }
    None
}

fn parse_token_definition_pt_token<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    let token = any.parse_next(input)?;
    let word = token
        .parser_word_pieces()
        .first()
        .ok_or_else(|| primitives::backtrack_err("token definition", "power/toughness"))?;
    leaf::parse_leaf_power_toughness_complete(word.text.as_str())
        .map_err(|_| primitives::backtrack_err("token definition", "power/toughness"))?;
    Ok(())
}

pub fn first_token_definition_pt_token(tokens: &[OwnedLexToken]) -> Option<usize> {
    primitives::find_prefix(tokens, || parse_token_definition_pt_token)
        .map(|(token_idx, _, _)| token_idx)
}

fn artifact_subtypes(words: &[&str]) -> Vec<Subtype> {
    let mut subtypes = Vec::new();
    for word in words {
        // Only artifact-family subtypes belong on an artifact token's type
        // line; a leading proper-noun token name ("Tamiyo's Notebook") must
        // not match same-named subtypes from other families.
        if let Ok(subtype) = leaf::parse_leaf_subtype_complete(word)
            && ironsmith_core::SubtypeFamily::Artifact
                .all_subtypes()
                .contains(&subtype)
            && !subtypes.contains(&subtype)
        {
            subtypes.push(subtype);
        }
    }
    subtypes
}

fn creature_card_types(words: &[&str]) -> Vec<CardType> {
    let mut card_types = vec![CardType::Creature];
    if let Some(creature_idx) = common::first_word_offset(words, "creature") {
        let prefix = &words[..creature_idx];
        if common::word_present(prefix, "artifact") {
            card_types.insert(0, CardType::Artifact);
        }
        if common::word_present(prefix, "enchantment") {
            card_types.insert(0, CardType::Enchantment);
        }
        if common::word_present(prefix, "land") {
            card_types.insert(0, CardType::Land);
        }
    }
    card_types
}

fn creature_subtypes(words: &[&str]) -> Vec<Subtype> {
    let mut scan_end = words.len();
    let mut idx = 0usize;
    while idx < words.len() {
        if matches!(
            words[idx],
            "token"
                | "tokens"
                | "with"
                | "when"
                | "whenever"
                | "has"
                | "have"
                | "gains"
                | "gain"
                | "gets"
                | "get"
                | "named"
        ) {
            scan_end = idx;
            break;
        }
        idx += 1;
    }

    let mut subtypes = Vec::new();
    let scanned = &words[..scan_end];
    let mut idx = 0usize;
    while idx < scanned.len() {
        let word = &scanned[idx];
        idx += 1;
        // Normalized tokens split a hyphenated creature type into two words
        // ("Assembly-Worker"). Rejoin the pair only when the vocabulary names
        // the whole hyphenated subtype, so neither half is misread alone.
        if let Some(next) = scanned.get(idx)
            && let Some(subtype) =
                leaf::classify_token_definition_subtype(&format!("{word}-{next}"))
            && subtype.display_name().contains('-')
        {
            idx += 1;
            if !subtypes.contains(&subtype) {
                subtypes.push(subtype);
            }
            continue;
        }
        if leaf::parse_leaf_card_type_complete(word).is_ok() {
            continue;
        }
        if let Some(subtype) = crate::grammar::primitives::probe_shape(
            leaf::parse_leaf_subtype_flexible_complete(word),
        )
        .or_else(|| leaf::classify_token_definition_subtype(word))
            && !subtypes.contains(&subtype)
        {
            subtypes.push(subtype);
        }
    }
    subtypes
}

/// Word positions naming the colors of a "protection from [color]" keyword,
/// which are not colors of the token itself.
fn protection_color_word_positions(words: &[&str]) -> Vec<usize> {
    let mut positions = Vec::new();
    for start in 0..words.len() {
        if words[start] != "protection" || words.get(start + 1) != Some(&"from") {
            continue;
        }
        let mut idx = start + 2;
        while let Some(word) = words.get(idx) {
            match *word {
                "white" | "blue" | "black" | "red" | "green" | "and" | "or" | "from" => {
                    positions.push(idx);
                    idx += 1;
                }
                _ => break,
            }
        }
    }
    positions
}

fn protection_colors(words: &[&str]) -> Option<ColorSet> {
    let mut colors = ColorSet::new();
    for idx in protection_color_word_positions(words) {
        colors = colors.union(match words[idx] {
            "white" => ColorSet::WHITE,
            "blue" => ColorSet::BLUE,
            "black" => ColorSet::BLACK,
            "red" => ColorSet::RED,
            "green" => ColorSet::GREEN,
            _ => continue,
        });
    }
    (!colors.is_empty()).then_some(colors)
}

fn token_colors(words: &[&str]) -> ColorSet {
    token_color_words(words).0
}

fn token_color_words(words: &[&str]) -> (ColorSet, ironsmith_core::TokenWordRole) {
    // The descriptor ends before the token noun, explicit name or abilities.
    // Postnominal colors have their own exact suffix parser below.
    let end = words.iter().position(|word| matches!(*word, "token" | "tokens" | "named" | "with" | "when" | "whenever" | "has" | "gains"))
        .unwrap_or(words.len());
    let words = &words[..end];
    if common::phrase_present(words, &["all", "colors"])
        || common::phrase_present(words, &["all", "colours"])
    {
        return (ColorSet::WHITE
            .union(ColorSet::BLUE)
            .union(ColorSet::BLACK)
            .union(ColorSet::RED)
            .union(ColorSet::GREEN), ironsmith_core::TokenWordRole::RulesImplied);
    }
    let mut colors = ColorSet::new();
    for (word, color) in [
        ("white", ColorSet::WHITE),
        ("blue", ColorSet::BLUE),
        ("black", ColorSet::BLACK),
        ("red", ColorSet::RED),
        ("green", ColorSet::GREEN),
    ] {
        if common::word_present(words, word) {
            colors = colors.union(color);
        }
    }
    (colors, ironsmith_core::TokenWordRole::Authored)
}

/// Parse the postnominal color surface used after a token noun, as in
/// "Sand Warrior creature tokens that are red, green, and white." Token
/// identity normally appears before `token(s)`, so this suffix is not part of
/// the definition slice passed to the ordinary token-shape parser.
pub fn parse_postnominal_token_colors_tokens(tokens: &[OwnedLexToken]) -> Option<ColorSet> {
    parse_postnominal_token_color_words_tokens(tokens).map(|(colors, _)| colors)
}

pub fn parse_postnominal_token_color_words_tokens(tokens: &[OwnedLexToken]) -> Option<(ColorSet, ironsmith_core::TokenWordRole)> {
    let words = parser_token_word_refs(tokens);
    let color_start = if crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[&["that", "are"], &["that", "is"]],
    ) {
        2
    } else if crate::word_primitives::first_is_any(&words, &["that's", "thats", "that’s"]) {
        1
    } else {
        return None;
    };
    if crate::word_primitives::parse_any_sequence_prefix(
        &words[color_start..],
        &[&["all", "colors"], &["all", "colours"]],
    ) {
        return Some((
            ColorSet::WHITE
                .union(ColorSet::BLUE)
                .union(ColorSet::BLACK)
                .union(ColorSet::RED)
                .union(ColorSet::GREEN), ironsmith_core::TokenWordRole::RulesImplied,
        ));
    }
    let mut colors = ColorSet::new();
    for word in &words[color_start..] {
        let color = match *word {
            "white" => ColorSet::WHITE,
            "blue" => ColorSet::BLUE,
            "black" => ColorSet::BLACK,
            "red" => ColorSet::RED,
            "green" => ColorSet::GREEN,
            "and" | "or" => continue,
            _ => break,
        };
        colors = colors.union(color);
    }
    (!colors.is_empty()).then_some((colors, ironsmith_core::TokenWordRole::Authored))
}

pub(super) fn token_keywords(words: &[&str]) -> Vec<TokenKeywordShape> {
    // A trailing "where X is ..." clause defines the count, not the token
    // ("create X Blood tokens, where X is the number of abilities from among
    // flying, ..."); its keyword words never belong to the token.
    let words = &words[..words.iter().position(|word| *word == "where").unwrap_or(words.len())];
    let mut keywords = Vec::new();
    for (word, keyword) in [
        ("flying", TokenKeywordShape::Flying),
        ("defender", TokenKeywordShape::Defender),
        ("prowess", TokenKeywordShape::Prowess),
        ("vigilance", TokenKeywordShape::Vigilance),
        ("trample", TokenKeywordShape::Trample),
        ("lifelink", TokenKeywordShape::Lifelink),
        ("deathtouch", TokenKeywordShape::Deathtouch),
        ("haste", TokenKeywordShape::Haste),
        ("menace", TokenKeywordShape::Menace),
        ("reach", TokenKeywordShape::Reach),
        ("hexproof", TokenKeywordShape::Hexproof),
        ("shroud", TokenKeywordShape::Shroud),
        ("indestructible", TokenKeywordShape::Indestructible),
        ("infect", TokenKeywordShape::Infect),
        ("flash", TokenKeywordShape::Flash),
        ("islandwalk", TokenKeywordShape::Islandwalk),
        ("mountainwalk", TokenKeywordShape::Mountainwalk),
        ("forestwalk", TokenKeywordShape::Forestwalk),
        ("swampwalk", TokenKeywordShape::Swampwalk),
        ("plainswalk", TokenKeywordShape::Plainswalk),
    ] {
        if common::word_present(words, word) {
            keywords.push(keyword);
        }
    }
    if common::phrase_present(words, &["first", "strike"]) {
        keywords.push(TokenKeywordShape::FirstStrike);
    }
    if let Some(colors) = protection_colors(words) {
        keywords.push(TokenKeywordShape::ProtectionFromColors(colors));
    }
    if common::phrase_present(words, &["double", "strike"]) {
        keywords.push(TokenKeywordShape::DoubleStrike);
    }
    if let Some(amount) = crate::word_primitives::parse_sequence_start(words, &["ward"])
        .and_then(|idx| words.get(idx + 1))
        .and_then(|word| crate::util::decimal_count(word))
    {
        keywords.push(TokenKeywordShape::WardGeneric(amount));
    }
    if let Some(amount) = crate::word_primitives::parse_sequence_start(words, &["firebending"])
        .and_then(|idx| words.get(idx + 1))
        .and_then(|word| crate::util::decimal_count(word))
    {
        keywords.push(TokenKeywordShape::Firebending(amount));
    }
    if let Some(amount) = crate::word_primitives::parse_sequence_start(words, &["devour"])
        .and_then(|idx| words.get(idx + 1))
        .and_then(|word| crate::util::decimal_count(word))
    {
        keywords.push(TokenKeywordShape::Devour(amount));
    }
    keywords
}

fn double_quoted_rule_bodies(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    common::token_description_projection(tokens).quoted_rules
}

fn inline_rule_self_surface(
    rule_tokens: &[OwnedLexToken],
    named_token: Option<&str>,
) -> Option<SourceReferenceSurface> {
    let words = parser_token_word_refs(rule_tokens);
    let subject_words = match words.first().copied() {
        Some("when" | "whenever") => &words[1..],
        _ => words.as_slice(),
    };
    if crate::word_primitives::parse_sequence_prefix(subject_words, &["this", "token"]) {
        return Some(SourceReferenceSurface::ThisPermanentType(
            "this token".to_string(),
        ));
    }
    if crate::word_primitives::parse_sequence_prefix(subject_words, &["this", "creature"]) {
        return Some(SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string(),
        ));
    }
    let named_token = named_token?;
    let name_tokens = crate::lexer::synthetic_phrase_tokens(named_token);
    let name_words = parser_token_word_refs(&name_tokens);
    crate::word_primitives::parse_sequence_prefix(subject_words, &name_words)
        .then(|| SourceReferenceSurface::FullName(named_token.to_string()))
}

pub fn authored_inline_rule_presentations(
    source_tokens: &[OwnedLexToken],
    named_token: Option<&str>,
) -> Vec<CreatureTokenInlineRulePresentation> {
    let mut presentations = Vec::new();
    for rule_tokens in double_quoted_rule_bodies(source_tokens) {
        let words = parser_token_word_refs(rule_tokens);
        // Reuse the ordinary specialized-rule recognizer on this single
        // quoted ability. It sees no quote delimiters, so this recursion
        // terminates immediately while keeping presentation classification
        // aligned with the executable fields.
        let rules = creature_rules(rule_tokens, &words, &words, named_token);
        let mut kinds = Vec::new();
        if rules.combat_restriction.is_some() {
            let position = crate::slice_primitives::select_position(&words, |word| {
                matches!(*word, "cant" | "attacks")
            })
            .unwrap_or(usize::MAX);
            kinds.push((position, CreatureTokenInlineRuleKind::CombatRestriction));
        }
        if rules.leaves_return_named_to_hand.is_some() {
            let position = crate::slice_primitives::select_position(&words, |word| *word == "when")
                .unwrap_or(usize::MAX);
            kinds.push((
                position,
                CreatureTokenInlineRuleKind::LeavesReturnNamedToHand,
            ));
        }
        kinds.sort_by_key(|(position, _)| *position);
        let self_surface = inline_rule_self_surface(rule_tokens, named_token);
        presentations.extend(kinds.into_iter().map(|(_, kind)| {
            CreatureTokenInlineRulePresentation {
                kind,
                self_surface: self_surface.clone(),
            }
        }));
    }
    presentations
}

pub(super) fn creature_rules(
    source_tokens: &[OwnedLexToken],
    words: &[&str],
    intrinsic_words: &[&str],
    named_card: Option<&str>,
) -> CreatureTokenRulesShape {
    let all = |expected: &[&str]| common::all_words_present(words, expected);
    let damage = rules::damage_amount(words);
    let sacrifice_return_pattern = all(&[
        "sacrifice",
        "this",
        "token",
        "return",
        "named",
        "graveyard",
        "battlefield",
    ]) && !common::word_present(words, "beginning");
    let upkeep_return_pattern =
        common::phrase_present(words, &["at", "the", "beginning", "of", "your"])
            && all(&[
                "upkeep",
                "sacrifice",
                "this",
                "token",
                "return",
                "named",
                "graveyard",
                "battlefield",
            ]);
    let dies_create = all(&[
        "when", "token", "dies", "create", "2/2", "red", "dragon", "flying", "r", "+1/+0",
    ]);
    let dies_damage = (all(&["when", "token", "dies", "deals", "damage", "target"])
        || all(&[
            "when", "this", "token", "dies", "it", "deals", "damage", "target",
        ]))
    .then_some(damage)
    .flatten();
    let leaves_damage = all(&[
        "when",
        "token",
        "leaves",
        "battlefield",
        "deals",
        "damage",
        "you",
        "each",
        "creature",
        "control",
    ])
    .then_some(damage)
    .flatten();
    let becomes_tapped_damage = all(&[
        "whenever", "token", "becomes", "tapped", "deals", "damage", "target", "player",
    ])
    .then_some(damage)
    .flatten();
    let referenced_card_name = names::referenced_card_name(source_tokens);
    let leaves_return_named = all(&[
        "when",
        "leaves",
        "battlefield",
        "return",
        "named",
        "graveyard",
        "hand",
    ])
    .then(|| referenced_card_name.clone())
    .flatten();
    let cant_attack_or_block = all(&["cant", "attack", "or", "block"]);
    let qualified_cant_be_blocked = common::phrase_present(words, &["cant", "be", "blocked", "by"])
        || common::phrase_present(words, &["cant", "be", "blocked", "except", "by"])
        || common::phrase_present(words, &["cant", "block", "or", "be", "blocked", "by"])
        || common::phrase_present(
            words,
            &["cant", "block", "or", "be", "blocked", "except", "by"],
        );
    let coordinated_cant_block_and_be_blocked =
        common::phrase_present(words, &["cant", "block", "or", "be", "blocked"]);
    let combat_restriction = if cant_attack_or_block && common::word_present(words, "alone") {
        Some(TokenCombatRestrictionShape::CantAttackOrBlockAlone)
    } else if cant_attack_or_block {
        Some(TokenCombatRestrictionShape::CantAttackOrBlock)
    // A qualified restriction ("can't be blocked by ...") is not the
    // unconditional unblockable ability. The quoted-rule parser lowers the
    // qualifier structurally; adding Unblockable here would both duplicate
    // that rule and make the token impossible to block by otherwise-legal
    // creatures.
    } else if all(&["cant", "be", "blocked"]) && !qualified_cant_be_blocked {
        Some(TokenCombatRestrictionShape::Unblockable)
    } else if all(&["cant", "block"]) && !coordinated_cant_block_and_be_blocked {
        Some(TokenCombatRestrictionShape::CantBlock)
    } else if common::phrase_present(words, &["attacks", "each", "combat", "if", "able"]) {
        Some(TokenCombatRestrictionShape::MustAttack)
    } else {
        None
    };
    let graveyard_anthem_pattern =
        all(&[
            "this", "token", "gets", "+1/+1", "for", "each", "card", "named",
        ]) && common::any_word_present(words, &["graveyard", "graveyards"]);
    let graveyard_anthem_card_name = graveyard_anthem_pattern
        .then(|| {
            names::graveyard_anthem_card_name(words).or_else(|| named_card.map(str::to_string))
        })
        .flatten();
    let landfall_pump = all(&[
        "whenever", "land", "control", "enters", "this", "token", "gets", "+1/+0",
    ]) && (common::phrase_present(words, &["until", "end", "of", "turn"])
        || common::phrase_present(words, &["until", "the", "end", "of", "turn"]));
    let power_bonus = all(&["saddles", "mounts", "crews", "vehicles", "power", "greater"])
        .then(|| rules::parse_token_power_as_though_greater_shape_words(words))
        .flatten()
        .map(|TokenPowerAsThoughGreaterShape { amount }| amount);
    let token_rules = rules::parse_token_rules_surfaces_for_named_token(source_tokens, named_card);
    let has_typed_poison_damage_rule = token_rules.embedded_rules.iter().any(|rule| {
        matches!(
            rule,
            crate::model::token_definition::TokenEmbeddedRuleShape::DealsDamageToPlayerPutCounters {
                counter_type: crate::object::CounterType::Poison,
                ..
            }
        )
    });

    CreatureTokenRulesShape {
        token_rules,
        authored_inline_rules: authored_inline_rule_presentations(source_tokens, named_card),
        cumulative_upkeep_mana_symbols: rules::cumulative_upkeep_mana_symbols(intrinsic_words),
        tap_mana_ability: rules::parse_token_tap_mana_ability_tokens(source_tokens),
        saddle_crew_power_bonus: power_bonus,
        banding: common::word_present(intrinsic_words, "banding"),
        hexproof: common::word_present(intrinsic_words, "hexproof"),
        indestructible: common::word_present(intrinsic_words, "indestructible"),
        copies_exiled_triggered_abilities: all(&["all", "triggered", "abilities"])
            && common::word_present(words, "exiled")
            && common::word_present(words, "cards"),
        toxic_amount: rules::toxic_amount(intrinsic_words),
        sacrifice_return: sacrifice_return_pattern
            .then(|| rules::sacrifice_return_shape(words, named_card))
            .flatten(),
        upkeep_return_name: upkeep_return_pattern
            .then(|| named_card.map(str::to_string))
            .flatten(),
        upkeep_return_grants_haste: common::word_present(words, "haste"),
        dies_create_firebreathing_dragon: dies_create,
        dies_damage_any_target: dies_damage,
        dies_minus_one_target_creature: all(&[
            "when", "token", "dies", "target", "creature", "gets", "-1/-1",
        ]),
        leaves_damage_you_and_creatures: leaves_damage,
        bands_with_wolves: all(&["bands", "other", "creatures", "named", "wolves"]),
        red_pump: all(&["r", "this", "creature", "gets", "+1/+0"]) && !dies_create,
        white_tap_target_creature: all(&["w", "t", "tap", "target", "creature"]),
        combat_damage_poison: !has_typed_poison_damage_rule
            && all(&["deals", "combat", "damage", "player", "poison", "counter"]),
        noncreature_spell_each_opponent_damage:
            rules::parse_inline_noncreature_spell_damage_tokens(source_tokens)
                .map(|shape| shape.amount),
        becomes_tapped_damage_player: becomes_tapped_damage,
        combat_damage_gain_artifact: all(&[
            "whenever", "token", "deals", "combat", "damage", "player", "gain", "control",
            "artifact",
        ]),
        leaves_return_named_to_hand: leaves_return_named,
        pest_dies_gain_life: common::word_present(words, "pest")
            && all(&["when", "token", "dies", "gain", "1", "life"]),
        first_strike: common::all_words_present(intrinsic_words, &["first", "strike"]),
        double_strike: common::all_words_present(intrinsic_words, &["double", "strike"]),
        mercenary_pump: common::word_present(words, "mercenary")
            && all(&["creature", "1/1", "red"]),
        combat_restriction,
        can_block_only_flying: all(&["can", "block", "only", "creatures", "flying"]),
        counter_noncreature_unless_pays: all(&[
            "counter",
            "noncreature",
            "spell",
            "sacrifice",
            "token",
            "unless",
            "controller",
            "pays",
            "1",
        ]),
        changeling: common::word_present(intrinsic_words, "changeling"),
        graveyard_anthem_card_name,
        landfall_pump,
    }
}

/// Test-only: the shape a token definition's text parses to. Production
/// callers hold the sentence's tokens and use `parse_token_definition_shape_tokens`.
#[cfg(test)]
pub fn parse_token_definition_shape_text(source_text: &str) -> Option<TokenDefinitionSpec> {
    let trimmed = source_text.trim();
    let tokens = crate::util::lex_fragment(trimmed, 0)?;
    parse_token_definition_shape_tokens(&tokens)
}

#[cfg(test)]
#[path = "surface_inline_tests.rs"]
mod tests;

#[path = "surface/choice.rs"]
mod choice_programs;
pub use choice_programs::source_chosen_token_characteristics;
#[path = "surface/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::parse_token_definition_shape_tokens;
