use super::*;

pub fn parse_token_definition_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenDefinitionSpec> {
    // CR 111.10w/x and 111.11: these names denote a complete canonical
    // token definition, not a host-card special case or runtime name lookup.
    // A complete parser prevents modifiers/trailing text from being discarded.
    let canonical_tokens = if tokens.first().is_some_and(|token| token.is_word("tapped")) {
        &tokens[1..]
    } else {
        tokens
    };
    if let Some(shape) = super::super::rules::parse_canonical_named_token_shape(canonical_tokens) {
        return Some(TokenDefinitionSpec::Builtin(shape));
    }
    let scope = token_description_scope(tokens);
    let outer_words = parser_token_word_refs(&scope.outer);
    let descriptor_end = outer_words.iter().position(|word| matches!(*word,
        "token" | "tokens" | "with" | "named" | "when" | "whenever" | "has" | "gains"))
        .unwrap_or(outer_words.len());
    let words = &outer_words[..descriptor_end];
    let rule_words = parser_token_word_refs(&scope.rule_source);
    let has = |word| common::word_present(&words, word);
    let all = |expected: &[&str]| common::all_words_present(&words, expected);
    let pt = token_pt(&words);
    let named_card = &scope.name;
    let described_roles = |explicit_name: bool| {
        if !scope.complete_quotes { return None; }
        let mut roles = crate::model::token_definition::TokenDescriptionTextRoles::authored(
            if explicit_name { ironsmith_core::TokenNameTextRole::Explicit } else { ironsmith_core::TokenNameTextRole::SubtypeDerived });
        roles.colors = token_color_words(&words).1;
        Some(roles)
    };

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
    } else if all(&["eldrazi", "spawn"]) && !has("creature") {
        Some(BuiltinTokenShape::EldraziSpawn)
    } else if all(&["eldrazi", "scion"]) && !has("creature") {
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
    } else if all(&["virtuous", "role"]) {
        Some(BuiltinTokenShape::VirtuousRole)
    } else if has("blood") && !has("creature") {
        Some(BuiltinTokenShape::Blood)
    } else if has("powerstone") && !has("creature") {
        Some(BuiltinTokenShape::Powerstone)
    } else if has("heartwood") && !has("creature") {
        Some(BuiltinTokenShape::Heartwood)
    } else if has("vibranium") && !has("creature") {
        Some(BuiltinTokenShape::Vibranium)
    } else {
        None
    };
    if let Some(builtin) = builtin {
        let mut shape = ModifiedBuiltinTokenShape::new(builtin);
        shape.name = scope.name.clone();
        if words.iter().any(|word| matches!(*word,
            "white" | "blue" | "black" | "red" | "green" | "colorless"))
            || common::phrase_present(words, &["all", "colors"])
        {
            let (colors, role) = token_color_words(words);
            shape.colors = Some(colors);
            shape.color_words = role;
        }
        shape.power_toughness = pt;
        shape.supertypes = words.iter().filter_map(|word| leaf::parse_leaf_supertype_complete(word).ok()).collect();
        let inherited_types = builtin.card_types();
        shape.additional_card_types = words.iter().filter_map(|word| leaf::parse_leaf_card_type_complete(word).ok())
            .filter(|kind| !inherited_types.contains(kind)).collect();
        // The predefined-token noun denotes its complete rule definition.
        // Other subtype nouns in the descriptor are separate authored facts.
        let identifier: &[&str] = match builtin {
            BuiltinTokenShape::Treasure => &["treasure"], BuiltinTokenShape::Clue => &["clue"],
            BuiltinTokenShape::Map => &["map"], BuiltinTokenShape::Lander => &["lander"],
            BuiltinTokenShape::Junk => &["junk"], BuiltinTokenShape::Mutagen => &["mutagen"],
            BuiltinTokenShape::Gold => &["gold"], BuiltinTokenShape::Shard => &["shard"],
            BuiltinTokenShape::Walker => &["walker"], BuiltinTokenShape::Food => &["food"],
            BuiltinTokenShape::Blood => &["blood"], BuiltinTokenShape::Powerstone => &["powerstone"],
            BuiltinTokenShape::Heartwood => &["heartwood"], BuiltinTokenShape::Vibranium => &["vibranium"],
            BuiltinTokenShape::EldraziSpawn => &["eldrazi", "spawn"],
            BuiltinTokenShape::EldraziScion => &["eldrazi", "scion"],
            BuiltinTokenShape::WickedRole => &["wicked", "role"],
            BuiltinTokenShape::YoungHeroRole => &["young", "hero", "role"],
            BuiltinTokenShape::MonsterRole => &["monster", "role"],
            BuiltinTokenShape::SorcererRole => &["sorcerer", "role"],
            BuiltinTokenShape::RoyalRole => &["royal", "role"],
            BuiltinTokenShape::CursedRole => &["cursed", "role"],
            BuiltinTokenShape::VirtuousRole => &["virtuous", "role"],
            BuiltinTokenShape::Gingerbrute | BuiltinTokenShape::Mutavault
            | BuiltinTokenShape::SpellgorgerWeird | BuiltinTokenShape::Tarmogoyf => unreachable!("complete card-name token leaf owns these shapes"),
        };
        let additional_words: Vec<_> = words.iter().copied().filter(|word| !identifier.contains(word)).collect();
        shape.additional_subtypes = creature_subtypes(&additional_words);
        shape.keywords = token_keywords(&outer_words);
        shape.words_complete = scope.complete_quotes;
        return Some(if shape == ModifiedBuiltinTokenShape::new(builtin) {
            TokenDefinitionSpec::Builtin(builtin)
        } else { TokenDefinitionSpec::ModifiedBuiltin(shape) });
    }

    if all(&["vehicle", "artifact"]) && !has("creature") {
        return Some(TokenDefinitionSpec::Vehicle(VehicleTokenShape {
            name: names::vehicle_surface_name(&words, named_card.as_deref()),
            text_roles: described_roles(named_card.is_some()),
            power_toughness: pt,
            colorless: has("colorless"),
            colors: token_colors(words),
            legendary: has("legendary"),
            flying: common::word_present(&outer_words, "flying"),
            crew_amount: rules::parse_token_crew_shape_words(&outer_words).map(|shape| shape.amount),
        }));
    }

    if has("enchantment")
        && pt.is_none()
        && !["creature", "artifact", "land", "planeswalker", "battle"]
            .iter()
            .any(|kind| has(kind))
    {
        return Some(TokenDefinitionSpec::Enchantment(EnchantmentTokenShape {
            name: scope.name.clone().unwrap_or_else(|| "Enchantment".into()),
            text_roles: described_roles(named_card.is_some()),
            subtypes: words
                .iter()
                .take_while(|word| **word != "named")
                .filter_map(|word| leaf::parse_leaf_subtype_complete(word).ok())
                .filter(|subtype| {
                    ironsmith_core::SubtypeFamily::Enchantment
                        .all_subtypes()
                        .contains(subtype)
                })
                .collect(),
            legendary: has("legendary"),
            colors: token_colors(&words),
            token_rules: rules::parse_token_rules_surfaces_tokens(&scope.rule_source),
        }));
    }

    // "a [tapped] colorless land token [named <name>]": a land token whose
    // land types (if any) give it intrinsic mana abilities (CR 305.6). Every
    // descriptor word must be accounted for.
    if has("land")
        && pt.is_none()
        && !["creature", "artifact", "enchantment", "planeswalker", "battle"]
            .iter()
            .any(|kind| has(kind))
    {
        let descriptors = words.iter().take_while(|word| **word != "named");
        let mut subtypes = Vec::new();
        let mut complete = true;
        for word in descriptors {
            if matches!(*word, "land" | "colorless" | "legendary" | "tapped" | "a" | "an") {
                continue;
            }
            match leaf::parse_leaf_subtype_complete(word).ok().filter(|subtype| {
                ironsmith_core::SubtypeFamily::Land.all_subtypes().contains(subtype)
            }) {
                Some(subtype) => subtypes.push(subtype),
                None => complete = false,
            }
        }
        if complete {
            return Some(TokenDefinitionSpec::Land(LandTokenShape {
                name: scope.name.clone().unwrap_or_else(|| {
                    if subtypes.is_empty() {
                        "Land".into()
                    } else {
                        subtypes
                            .iter()
                            .map(|subtype| format!("{subtype:?}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                }),
                subtypes,
                legendary: has("legendary"),
            }));
        }
    }

    let equipment_subject =
        has("equipment") && common::phrase_present(&rule_words, &["equipped", "creature"]);
    if has("artifact") && pt.is_none() && (!has("creature") || equipment_subject) {
        let leaves_damage = common::all_words_present(&rule_words, &[
            "when",
            "token",
            "leaves",
            "battlefield",
            "deals",
            "damage",
            "target",
        ])
        .then(|| rules::damage_amount(&rule_words))
        .flatten();
        return Some(TokenDefinitionSpec::Artifact(ArtifactTokenShape {
            name: names::artifact_surface_name(&words, named_card.as_deref()),
            text_roles: described_roles(named_card.is_some()),
            subtypes: artifact_subtypes(&words),
            legendary: has("legendary"),
            colorless: has("colorless"),
            colors: token_colors(&words),
            equipment_rules: equipment::parse_equipment_rules_tokens(&scope.rule_source),
            token_rules: rules::parse_token_rules_surfaces_tokens(&scope.rule_source),
            leaves_damage_any_target: leaves_damage,
        }));
    }

    if has("angel") && pt.is_none() {
        return Some(TokenDefinitionSpec::Angel);
    }
    // A complete creature description owns its literal colors, subtypes,
    // name and abilities. Compact templates must not replace those facts or
    // introduce an ability (for example defender) absent from the instruction.
    if all(&["squirrel", "1/1", "green"]) && !has("creature") {
        return Some(TokenDefinitionSpec::Squirrel);
    }
    if all(&["dragon", "egg", "0/2"]) && !has("creature")
        && common::all_words_present(&rule_words, &[
            "when", "token", "dies", "create", "2/2", "flying", "r", "+1/+0",
        ])
    {
        return Some(TokenDefinitionSpec::DragonEgg);
    }
    if all(&["elephant", "3/3", "green"]) && !has("creature") {
        return Some(TokenDefinitionSpec::Elephant);
    }

    let construct_cda = common::all_words_present(&rule_words, &[
        "power",
        "toughness",
        "equal",
        "number",
        "artifacts",
        "you",
        "control",
    ]);
    let construct_plus = common::all_words_present(&rule_words, &["gets", "+1/+1", "for", "each", "artifact", "you", "control"]);
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
            changeling: common::word_present(&outer_words, "changeling") || common::phrase_exact(words, &["shapeshifter"]),
        }));
    }
    if all(&["astartes", "warrior", "2/2", "white"]) && !has("creature") {
        return Some(TokenDefinitionSpec::AstartesWarrior(
            AstartesWarriorTokenShape {
                vigilance: common::word_present(&outer_words, "vigilance"),
            },
        ));
    }
    if !has("creature") {
        return None;
    }

    let subtypes = creature_subtypes(&words);
    let subtype_fallback = if subtypes.is_empty() { None } else {
        subtypes.iter().map(|subtype| ironsmith_core::token_text::token_subtype_rules_word(*subtype))
            .collect::<Option<Vec<_>>>().map(|words| words.join(" "))
    };
    // Named legendary token syntax can put the name before the descriptive
    // article ("Zabu, a legendary ... token") rather than after `named`.
    // Reuse the same typed leading-name parse that chooses the token's runtime
    // name when validating self references inside its quoted rules.
    let (use_source_chosen_color, use_source_chosen_creature_type) =
        source_chosen_token_characteristics(&outer_words);
    Some(TokenDefinitionSpec::Creature(CreatureTokenShape {
        name: names::creature_surface_name(&words, declared_name, subtype_fallback.as_deref()),
        text_roles: described_roles(declared_name.is_some()),
        card_types: creature_card_types(&words),
        subtypes,
        power_toughness: pt.unwrap_or((0, 0)),
        legendary: has("legendary"),
        colors: token_colors(&words),
        use_source_chosen_color,
        use_source_chosen_creature_type,
        keywords: token_keywords(&outer_words),
        rules: creature_rules(&scope.rule_source, &rule_words, &outer_words, declared_name),
    }))
}
