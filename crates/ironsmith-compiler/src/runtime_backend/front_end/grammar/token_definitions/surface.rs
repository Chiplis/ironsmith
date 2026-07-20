use crate::color::ColorSet;
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, lex_line, parser_token_word_refs,
};
use crate::runtime_backend::token_definition::{
    ArtifactTokenShape, AstartesWarriorTokenShape, BuiltinTokenShape,
    ConstructArtifactScalingShape, ConstructTokenShape, CreatureTokenRulesShape,
    CreatureTokenShape, ShapeshifterTokenShape, TokenCombatRestrictionShape, TokenDefinitionSpec,
    TokenKeywordShape, TokenPowerAsThoughGreaterShape, VehicleTokenShape,
};
use crate::types::{CardType, Subtype};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::{leaf, primitives};
use super::common;
use super::equipment;
use super::names;
use super::rules;

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

pub(crate) fn first_token_definition_pt_token(tokens: &[OwnedLexToken]) -> Option<usize> {
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
            && !subtypes.iter().any(|candidate| *candidate == subtype)
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
    }
    card_types
}

fn creature_subtypes(words: &[&str]) -> Vec<Subtype> {
    let mut scan_end = words.len();
    let mut idx = 0usize;
    while idx < words.len() {
        if matches!(
            words[idx],
            "with" | "when" | "whenever" | "has" | "have" | "gains" | "gain" | "gets" | "get"
        ) {
            scan_end = idx;
            break;
        }
        idx += 1;
    }

    let mut subtypes = Vec::new();
    for word in &words[..scan_end] {
        if leaf::parse_leaf_card_type_complete(word).is_ok() {
            continue;
        }
        if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word)
            && !subtypes.iter().any(|candidate| *candidate == subtype)
        {
            subtypes.push(subtype);
        }
    }
    subtypes
}

fn token_colors(words: &[&str]) -> ColorSet {
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
    colors
}

pub(super) fn token_keywords(words: &[&str]) -> Vec<TokenKeywordShape> {
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
    ] {
        if common::word_present(words, word) {
            keywords.push(keyword);
        }
    }
    keywords
}

pub(super) fn creature_rules(
    source_tokens: &[OwnedLexToken],
    words: &[&str],
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
            crate::runtime_backend::token_definition::TokenEmbeddedRuleShape::DealsDamageToPlayerPutCounters {
                counter_type: crate::object::CounterType::Poison,
                ..
            }
        )
    });

    CreatureTokenRulesShape {
        token_rules,
        cumulative_upkeep_mana_symbols: rules::cumulative_upkeep_mana_symbols(words),
        tap_mana_ability: rules::parse_token_tap_mana_ability_tokens(source_tokens),
        saddle_crew_power_bonus: power_bonus,
        banding: common::word_present(words, "banding"),
        hexproof: common::word_present(words, "hexproof"),
        indestructible: common::word_present(words, "indestructible"),
        copies_exiled_triggered_abilities: all(&["all", "triggered", "abilities"])
            && common::word_present(words, "exiled")
            && common::word_present(words, "cards"),
        toxic_amount: rules::toxic_amount(words),
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
        first_strike: all(&["first", "strike"]),
        double_strike: all(&["double", "strike"]),
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
        changeling: common::word_present(words, "changeling"),
        graveyard_anthem_card_name,
        landfall_pump,
    }
}

pub(crate) fn parse_token_definition_shape_text(source_text: &str) -> Option<TokenDefinitionSpec> {
    let trimmed = source_text.trim();
    let tokens = lex_line(trimmed, 0).ok()?;
    parse_token_definition_shape_tokens(&tokens)
}

pub(crate) fn parse_token_definition_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenDefinitionSpec> {
    let words = parser_token_word_refs(tokens);
    let has = |word| common::word_present(&words, word);
    let all = |expected: &[&str]| common::all_words_present(&words, expected);
    let pt = token_pt(&words);
    let named_card = names::named_card_name(tokens).or_else(|| names::leading_comma_name(tokens));

    let builtin = if has("treasure") && !has("creature") {
        Some(BuiltinTokenShape::Treasure)
    } else if has("clue") && !has("creature") {
        Some(BuiltinTokenShape::Clue)
    } else if has("map") && !has("creature") {
        Some(BuiltinTokenShape::Map)
    } else if has("lander") && !has("creature") {
        Some(BuiltinTokenShape::Lander)
    } else if has("junk") && !has("creature") {
        Some(BuiltinTokenShape::Junk)
    } else if has("mutagen") && !has("creature") {
        Some(BuiltinTokenShape::Mutagen)
    } else if has("gold") && !has("creature") {
        Some(BuiltinTokenShape::Gold)
    } else if has("shard") && !has("creature") {
        Some(BuiltinTokenShape::Shard)
    } else if has("walker") && !has("planeswalker") {
        Some(BuiltinTokenShape::Walker)
    } else if all(&["eldrazi", "spawn"]) {
        Some(BuiltinTokenShape::EldraziSpawn)
    } else if all(&["eldrazi", "scion"]) {
        Some(BuiltinTokenShape::EldraziScion)
    } else if has("food") && !has("creature") {
        Some(BuiltinTokenShape::Food)
    } else if all(&["wicked", "role"]) {
        Some(BuiltinTokenShape::WickedRole)
    } else if all(&["young", "hero", "role"]) {
        Some(BuiltinTokenShape::YoungHeroRole)
    } else if all(&["monster", "role"]) {
        Some(BuiltinTokenShape::MonsterRole)
    } else if all(&["sorcerer", "role"]) {
        Some(BuiltinTokenShape::SorcererRole)
    } else if all(&["royal", "role"]) {
        Some(BuiltinTokenShape::RoyalRole)
    } else if all(&["cursed", "role"]) {
        Some(BuiltinTokenShape::CursedRole)
    } else if has("blood") && !has("creature") {
        Some(BuiltinTokenShape::Blood)
    } else if has("powerstone") && !has("creature") {
        Some(BuiltinTokenShape::Powerstone)
    } else {
        None
    };
    if let Some(builtin) = builtin {
        return Some(TokenDefinitionSpec::Builtin(builtin));
    }

    if all(&["vehicle", "artifact"]) && !has("creature") {
        return Some(TokenDefinitionSpec::Vehicle(VehicleTokenShape {
            name: names::vehicle_surface_name(&words, named_card.as_deref()),
            power_toughness: pt,
            colorless: has("colorless"),
            flying: has("flying"),
            crew_amount: rules::parse_token_crew_shape_words(&words).map(|shape| shape.amount),
        }));
    }

    let equipment_subject =
        has("equipment") && common::phrase_present(&words, &["equipped", "creature"]);
    if has("artifact") && pt.is_none() && (!has("creature") || equipment_subject) {
        let leaves_damage = all(&[
            "when",
            "token",
            "leaves",
            "battlefield",
            "deals",
            "damage",
            "target",
        ])
        .then(|| rules::damage_amount(&words))
        .flatten();
        return Some(TokenDefinitionSpec::Artifact(ArtifactTokenShape {
            name: names::artifact_surface_name(&words, named_card.as_deref()),
            subtypes: artifact_subtypes(&words),
            legendary: has("legendary"),
            colorless: has("colorless"),
            colors: token_colors(&words),
            equipment_rules: equipment::parse_equipment_rules_tokens(tokens),
            token_rules: rules::parse_token_rules_surfaces_tokens(tokens),
            leaves_damage_any_target: leaves_damage,
        }));
    }

    if has("angel") && pt.is_none() {
        return Some(TokenDefinitionSpec::Angel);
    }
    if all(&["wall", "0/4", "artifact", "creature"]) {
        return Some(TokenDefinitionSpec::Wall);
    }
    if all(&["squirrel", "1/1", "green"]) {
        return Some(TokenDefinitionSpec::Squirrel);
    }
    if all(&["dragon", "egg", "0/2"])
        && all(&[
            "when", "token", "dies", "create", "2/2", "flying", "r", "+1/+0",
        ])
    {
        return Some(TokenDefinitionSpec::DragonEgg);
    }
    if all(&["elephant", "3/3", "green"]) {
        return Some(TokenDefinitionSpec::Elephant);
    }

    let construct_cda = all(&[
        "power",
        "toughness",
        "equal",
        "number",
        "artifacts",
        "you",
        "control",
    ]);
    let construct_plus = all(&["gets", "+1/+1", "for", "each", "artifact", "you", "control"]);
    let named_non_construct = named_card
        .as_ref()
        .is_some_and(|name| !name.eq_ignore_ascii_case("Construct"));
    if has("construct")
        && !named_non_construct
        && (pt.is_none() || construct_cda || construct_plus || all(&["construct", "0/0"]))
    {
        let artifact_scaling = if construct_plus {
            Some(ConstructArtifactScalingShape::GetsPlusOnePerArtifact)
        } else if construct_cda {
            Some(ConstructArtifactScalingShape::CharacteristicDefining)
        } else {
            None
        };
        return Some(TokenDefinitionSpec::Construct(ConstructTokenShape {
            power_toughness: pt.unwrap_or((0, 0)),
            artifact_scaling,
        }));
    }

    if has("shapeshifter") && !has("creature") {
        return Some(TokenDefinitionSpec::Shapeshifter(ShapeshifterTokenShape {
            changeling: has("changeling") || common::phrase_exact(&words, &["shapeshifter"]),
        }));
    }
    if all(&["astartes", "warrior", "2/2", "white"]) {
        return Some(TokenDefinitionSpec::AstartesWarrior(
            AstartesWarriorTokenShape {
                vigilance: has("vigilance"),
            },
        ));
    }
    if !has("creature") {
        return None;
    }

    let subtypes = creature_subtypes(&words);
    let subtype_fallback = subtypes.first().map(|subtype| format!("{subtype:?}"));
    // Named legendary token syntax can put the name before the descriptive
    // article ("Zabu, a legendary ... token") rather than after `named`.
    // Reuse the same typed leading-name parse that chooses the token's runtime
    // name when validating self references inside its quoted rules.
    let leading_name =
        names::leading_name_phrase(&words).or_else(|| names::leading_explicit_name(&words));
    let declared_name = named_card.as_deref().or(leading_name.as_deref());
    Some(TokenDefinitionSpec::Creature(CreatureTokenShape {
        name: names::creature_surface_name(&words, declared_name, subtype_fallback.as_deref()),
        card_types: creature_card_types(&words),
        subtypes,
        power_toughness: pt.unwrap_or((0, 0)),
        legendary: has("legendary"),
        colors: token_colors(&words),
        keywords: token_keywords(&words),
        rules: creature_rules(tokens, &words, declared_name),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_shape_preserves_vehicle_crew_and_named_creature_facts() {
        let vehicle = parse_token_definition_shape_text(
            "3/3 colorless artifact Vehicle token named Airship with flying and crew 2",
        )
        .unwrap();
        assert!(matches!(
            vehicle,
            TokenDefinitionSpec::Vehicle(VehicleTokenShape {
                name,
                power_toughness: Some((3, 3)),
                colorless: true,
                flying: true,
                crew_amount: Some(2),
            }) if name == "Airship"
        ));

        let creature = parse_token_definition_shape_text(
            "0/0 colorless Construct artifact creature token named Twin that's attacking.",
        )
        .unwrap();
        assert!(matches!(
            creature,
            TokenDefinitionSpec::Creature(CreatureTokenShape { name, .. }) if name == "Twin"
        ));
    }

    #[test]
    fn token_shape_preserves_multitype_creature_metadata() {
        let shape = parse_token_definition_shape_text(
            "2/2 black Zombie Employee artifact creature token with flying",
        )
        .unwrap();
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };
        assert_eq!(
            creature.card_types,
            vec![CardType::Artifact, CardType::Creature]
        );
        assert_eq!(creature.subtypes, vec![Subtype::Zombie, Subtype::Employee]);
        assert_eq!(creature.colors, ColorSet::BLACK);
        assert_eq!(creature.keywords, vec![TokenKeywordShape::Flying]);
    }

    #[test]
    fn leading_artifact_token_name_preserves_apostrophe_and_subtype() {
        let shape = parse_token_definition_shape_text(
            "Tamiyo's Notebook, a legendary colorless Book artifact token with \"{T}: Draw a card.\"",
        )
        .expect("leading named artifact token should parse");
        let TokenDefinitionSpec::Artifact(artifact) = shape else {
            panic!("expected artifact token shape");
        };
        assert_eq!(artifact.name, "Tamiyo's Notebook");
        assert_eq!(artifact.subtypes, vec![Subtype::Book]);
        assert!(artifact.legendary);
    }

    #[test]
    fn appositive_artifact_token_name_preserves_internal_comma_and_color() {
        let shape = parse_token_definition_shape_text(
            "Icingdeath, Frost Tongue, a legendary white Equipment artifact token",
        )
        .expect("appositive named artifact token should parse");
        let TokenDefinitionSpec::Artifact(artifact) = shape else {
            panic!("expected artifact token shape");
        };
        assert_eq!(artifact.name, "Icingdeath, Frost Tongue");
        assert_eq!(artifact.subtypes, vec![Subtype::Equipment]);
        assert_eq!(artifact.colors, ColorSet::WHITE);
        assert!(artifact.legendary);
    }

    #[test]
    fn appositive_creature_token_name_can_start_with_the_and_contain_subtypes() {
        let shape = parse_token_definition_shape_text(
            "The Tiger God, a legendary 4/4 green Cat God creature token",
        )
        .expect("article-prefixed appositive named creature token should parse");
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };
        assert_eq!(creature.name, "The Tiger God");
        assert_eq!(creature.subtypes, vec![Subtype::God, Subtype::Cat]);
        assert_eq!(creature.power_toughness, (4, 4));
        assert_eq!(creature.colors, ColorSet::GREEN);
        assert!(creature.legendary);
    }

    #[test]
    fn token_shape_accepts_hyphenated_creature_subtype() {
        let shape = parse_token_definition_shape_text(
            "a 2/2 colorless Assembly-Worker artifact creature token",
        )
        .expect("hyphenated creature subtype token should parse");
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };
        assert_eq!(creature.power_toughness, (2, 2));
        assert!(creature.card_types.contains(&CardType::Artifact));
        assert!(creature.card_types.contains(&CardType::Creature));

        assert!(matches!(
            parse_token_definition_shape_text("2/2 colorless Assembly-Worker artifact creature"),
            Some(TokenDefinitionSpec::Creature(_))
        ));
    }

    #[test]
    fn construct_artifact_scaling_requires_explicit_rules_text() {
        let dynamic =
            parse_token_definition_shape_text("X/X colorless Construct artifact creature token")
                .expect("dynamic Construct token should parse");
        assert!(matches!(
            dynamic,
            TokenDefinitionSpec::Construct(ConstructTokenShape {
                power_toughness: (0, 0),
                artifact_scaling: None,
            })
        ));

        let explicit = parse_token_definition_shape_text(
            "colorless Construct artifact creature token with \"This token's power and toughness are each equal to the number of artifacts you control.\"",
        )
        .expect("explicit artifact-scaling Construct should parse");
        assert!(matches!(
            explicit,
            TokenDefinitionSpec::Construct(ConstructTokenShape {
                artifact_scaling: Some(ConstructArtifactScalingShape::CharacteristicDefining),
                ..
            })
        ));

        let explicit_plus = parse_token_definition_shape_text(
            "0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
        )
        .expect("explicit artifact-pump Construct should parse");
        assert!(matches!(
            explicit_plus,
            TokenDefinitionSpec::Construct(ConstructTokenShape {
                artifact_scaling: Some(ConstructArtifactScalingShape::GetsPlusOnePerArtifact),
                ..
            })
        ));
    }

    #[test]
    fn creature_token_shape_keeps_embedded_dies_creation_rule() {
        let shape = parse_token_definition_shape_text(
            "1/1 green Boar creature token with \"When this token dies, create a Food token.\"",
        )
        .unwrap();
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };
        assert_eq!(
            creature.rules.token_rules.embedded_rules,
            vec![crate::runtime_backend::token_definition::TokenEmbeddedRuleShape::DiesCreateBuiltinToken {
                token: BuiltinTokenShape::Food,
                count: 1,
            }]
        );
    }

    #[test]
    fn qualified_blocking_rule_is_typed_without_unconditional_fallbacks() {
        let shape = parse_token_definition_shape_text(
            "a 1/1 colorless Spirit creature token with \"This token can't block or be blocked by non-Spirit creatures.\"",
        )
        .expect("qualified Spirit blocking token should parse");
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };

        assert_eq!(creature.rules.combat_restriction, None);
        assert_eq!(
            creature.rules.token_rules.embedded_rules,
            vec![
                crate::runtime_backend::token_definition::TokenEmbeddedRuleShape::CantBlockOrBeBlockedByNonSubtypeCreatures {
                    subtype: Subtype::Spirit,
                }
            ]
        );

        for (text, expected) in [
            (
                "a 1/1 creature token with \"This token can't block.\"",
                TokenCombatRestrictionShape::CantBlock,
            ),
            (
                "a 1/1 creature token with \"This token can't be blocked.\"",
                TokenCombatRestrictionShape::Unblockable,
            ),
        ] {
            let shape = parse_token_definition_shape_text(text)
                .expect("ordinary unconditional blocking rule should parse");
            let TokenDefinitionSpec::Creature(creature) = shape else {
                panic!("expected creature token shape");
            };
            assert_eq!(creature.rules.combat_restriction, Some(expected));
        }
    }

    #[test]
    fn leading_named_token_shape_binds_quoted_rule_self_reference() {
        let shape = parse_token_definition_shape_text(
            "Zabu, a legendary 2/2 green Cat creature token with \"Landfall — Whenever a land you control enters, put a +1/+1 counter on Zabu.\"",
        )
        .unwrap();
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };
        assert_eq!(creature.name, "Zabu");
        assert_eq!(
            creature.rules.token_rules.embedded_rules,
            vec![crate::runtime_backend::token_definition::TokenEmbeddedRuleShape::LandEntersPutCountersOnSelf {
                counter_type: crate::object::CounterType::PlusOnePlusOne,
                count: 1,
            }]
        );
    }

    #[test]
    fn creature_token_shape_distinguishes_referenced_card_name_from_token_name() {
        let mut tokens = lex_line(
            "Jumblebones, a legendary 2/1 black Skeleton creature with \"Jumblebones can't block\" and \"When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.\"",
            0,
        )
        .unwrap();
        assert!(tokens.last().is_some_and(OwnedLexToken::is_quote));
        tokens.pop();

        let shape = parse_token_definition_shape_tokens(&tokens).unwrap();
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };
        assert_eq!(
            creature.rules.leaves_return_named_to_hand.as_deref(),
            Some("Ozox, the Clattering King")
        );
    }
}
