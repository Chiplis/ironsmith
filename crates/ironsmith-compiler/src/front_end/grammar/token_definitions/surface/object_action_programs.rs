use super::*;

pub fn parse_token_definition_shape_tokens(
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
    // A named legendary token can put its name before the descriptive article
    // ("Mechtitan, a legendary ... Construct ... token"). Detect that name
    // before the generic Construct shortcut so the subtype cannot replace the
    // authored token name.
    let leading_name =
        names::leading_name_phrase(&words).or_else(|| names::leading_explicit_name(&words));
    let declared_name = named_card.as_deref().or(leading_name.as_deref());
    let named_non_construct =
        declared_name.is_some_and(|name| !name.eq_ignore_ascii_case("Construct"));
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
    let subtype_fallback = (!subtypes.is_empty()).then(|| {
        subtypes
            .iter()
            .map(|subtype| format!("{subtype:?}"))
            .collect::<Vec<_>>()
            .join(" ")
    });
    // Named legendary token syntax can put the name before the descriptive
    // article ("Zabu, a legendary ... token") rather than after `named`.
    // Reuse the same typed leading-name parse that chooses the token's runtime
    // name when validating self references inside its quoted rules.
    let (use_source_chosen_color, use_source_chosen_creature_type) =
        source_chosen_token_characteristics(&words);
    Some(TokenDefinitionSpec::Creature(CreatureTokenShape {
        name: names::creature_surface_name(&words, declared_name, subtype_fallback.as_deref()),
        card_types: creature_card_types(&words),
        subtypes,
        power_toughness: pt.unwrap_or((0, 0)),
        legendary: has("legendary"),
        colors: token_colors(&words),
        use_source_chosen_color,
        use_source_chosen_creature_type,
        keywords: token_keywords(&words),
        rules: creature_rules(tokens, &words, declared_name),
    }))
}
