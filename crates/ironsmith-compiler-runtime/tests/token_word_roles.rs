//! Independent described-token word-role contracts. All are authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::effects::CreateTokenEffect;
use ironsmith_core::{Color, ColorSet, Subtype, TextChange, TokenNameTextRole, TokenWordRole};

fn routes(body: &str) -> [CardDefinition; 2] {
    let text = format!("Mana cost: {{U}}\nType: Sorcery\n{body}");
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition("Token-role witness", &text, false));
    assert!(!loss.is_lossy());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact("Token-role witness", &text, false));
    assert!(!loss.is_lossy());
    let (artifact, _) = artifact.unwrap();
    let artifact = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    artifact.validate().unwrap();
    let result = [direct.unwrap(), ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()];
    for definition in &result { assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition)); }
    result
}
fn instruction(definition: &CardDefinition) -> ironsmith::effect::Effect {
    fn find(effect: &ironsmith::effect::Effect, found: &mut Vec<ironsmith::effect::Effect>) {
        if effect.downcast_ref::<CreateTokenEffect>().is_some() { found.push(effect.clone()); }
        effect.visit_child_effects(&mut |child| find(child, found));
    }
    let mut found = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { find(effect, &mut found); }
    assert_eq!(found.len(), 1);
    found.pop().unwrap()
}

fn frozen_name_routes(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, &text, false));
    assert!(!loss.is_lossy(), "{name}");
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact(name, &text, false));
    assert!(!loss.is_lossy(), "{name}");
    let (artifact, _) = compiled.unwrap();
    let artifact = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    artifact.validate().unwrap();
    [direct.unwrap(), ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()]
}

#[test]
fn exact_frozen_name_holds_have_derived_blueprints_and_created_object_names() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::effect::{Effect, OutcomeValue, Value};
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::{GameState, PlayerId, Zone};
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/token_name_roles.json.fixture")).unwrap();
    assert_eq!(rows.len(), 7);
    for row in rows {
        let (expected, subtypes, pt, colors, artifact, ability_count) = match row["name"].as_str().unwrap() {
            "Adipose Offspring" => ("Alien Token", vec![Subtype::Alien], (2, 2), ColorSet::WHITE, false, 0),
            "Baloth Cage Trap" => ("Beast Token", vec![Subtype::Beast], (4, 4), ColorSet::GREEN, false, 0),
            "Cobra Trap" => ("Snake Token", vec![Subtype::Snake], (1, 1), ColorSet::GREEN, false, 0),
            "Oni-Cult Anvil" => ("Construct Token", vec![Subtype::Construct], (1, 1), ColorSet::default(), true, 0),
            "Camellia, the Seedmiser" => ("Squirrel Token", vec![Subtype::Squirrel], (1, 1), ColorSet::GREEN, false, 0),
            "Belisarius Cawl" => ("Astartes Warrior Token", vec![Subtype::Astartes, Subtype::Warrior], (2, 2), ColorSet::WHITE, false, 1),
            "Brood Birthing" => ("Eldrazi Spawn Token", vec![Subtype::Eldrazi, Subtype::Spawn], (0, 1), ColorSet::default(), false, 1),
            _ => unreachable!(),
        };
        for definition in frozen_name_routes(&row) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let mut tokens = Vec::new();
            fn collect(effect: &Effect, tokens: &mut Vec<CreateTokenEffect>) {
                if let Some(token) = effect.downcast_ref::<CreateTokenEffect>() { tokens.push(token.clone()); }
                effect.visit_child_effects(&mut |child| collect(child, tokens));
            }
            if let Some(program) = &definition.spell_effect {
                for effect in program.all_effects() { collect(effect, &mut tokens); }
            }
            for ability in &definition.abilities {
                let program = match &ability.kind {
                    AbilityKind::Activated(ability) => &ability.effects,
                    AbilityKind::Triggered(ability) => &ability.effects,
                    AbilityKind::Static(_) => continue,
                };
                for effect in program.all_effects() { collect(effect, &mut tokens); }
            }
            assert!(!tokens.is_empty(), "{}", row["name"]);
            for mut token in tokens {
                assert_eq!(token.token.card.name, expected);
                assert_eq!(token.token.card.subtypes, subtypes);
                assert_eq!(token.token.card.power_toughness, Some(ironsmith::card::PowerToughness::fixed(pt.0, pt.1)));
                assert_eq!(token.token.card.color_indicator.unwrap_or_default(), colors);
                assert_eq!(token.token.card.card_types, if artifact {
                    vec![ironsmith::CardType::Artifact, ironsmith::CardType::Creature]
                } else { vec![ironsmith::CardType::Creature] });
                assert!(token.token.card.supertypes.is_empty());
                assert!(token.token.card.mana_cost.is_none());
                assert!(token.token.spell_effect.is_none());
                assert!(token.token.aura_attach_filter.is_none());
                assert_eq!(token.token.abilities.len(), ability_count);
                if row["name"] == "Belisarius Cawl" {
                    assert!(matches!(&token.token.abilities[0].kind, AbilityKind::Static(ability)
                        if ability.id() == ironsmith::static_abilities::StaticAbilityId::Vigilance));
                }
                if row["name"] == "Brood Birthing" {
                    let AbilityKind::Activated(mana) = &token.token.abilities[0].kind else { panic!("authored Spawn mana ability"); };
                    assert!(mana.is_mana_ability());
                    assert_eq!(mana.mana_cost.costs().len(), 1);
                    assert!(matches!(mana.mana_cost.costs()[0].compiled_model(), Some(ironsmith_core::Cost::SacrificeSelf)));
                    assert_eq!(mana.mana_output, Some(vec![ironsmith::mana::ManaSymbol::Colorless]));
                }
                assert_eq!(token.text_roles.as_ref().unwrap().name, TokenNameTextRole::SubtypeDerived);
                // Exercise the native installation of this exact full-body
                // blueprint independently of each card's payment/count owner.
                token.count = Value::Fixed(1);
                let native = Effect::new(token);
                let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(native).unwrap();
                let effect = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let player = PlayerId::from_index(0);
                let source = game.create_object_from_definition(&definition, player, Zone::Battlefield);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                let result = execute_effect(&mut game, &effect, &mut EffectContext::new(source, player, &mut dm)).unwrap();
                let OutcomeValue::Objects(created) = result.value else { panic!("created token result"); };
                assert_eq!(created.len(), 1);
                assert_eq!(game.object(created[0]).unwrap().name.as_ref(), expected);
                assert_eq!(game.object(created[0]).unwrap().subtypes.to_vec(), subtypes);
            }
        }
    }
}

#[test]
fn explicit_and_derived_same_spelling_names_remain_distinct_through_compiler_native_and_wire_paths() {
    for (body, role, initial, changed_name) in [
        ("Create a 2/2 red Elf creature token.", TokenNameTextRole::SubtypeDerived, "Elf Token", "Human Token"),
        ("Create a 2/2 red Elf creature token named Elf.", TokenNameTextRole::Explicit, "Elf", "Elf"),
    ] {
        for definition in routes(body) {
            let effect = instruction(&definition);
            let original = effect.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(original.text_roles.as_ref().unwrap().name, role);
            assert_eq!(original.token.card.name, initial);
            assert_eq!(original.token.card.subtypes, vec![Subtype::Elf]);
            let rewritten = effect.with_text_change(TextChange::creature_type(Subtype::Elf, Subtype::Human).unwrap()).unwrap();
            let encoded = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(rewritten).unwrap();
            let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(encoded).unwrap();
            let changed = restored.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(changed.token.card.name, changed_name);
            assert_eq!(changed.token.card.subtypes, vec![Subtype::Human]);
            assert_eq!(changed.text_roles, original.text_roles);
            assert_eq!(original.token.card.name, initial);
            if role == TokenNameTextRole::Explicit { assert!(definition.canonical_text.contains("named Elf")); }
        }
    }
}

#[test]
fn appositive_name_words_do_not_supply_token_colors_or_subtypes() {
    for definition in routes("Create Red Elf, a legendary 2/2 blue Human creature token.") {
        let effect = instruction(&definition);
        let original = effect.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(original.token.card.name, "Red Elf");
        assert_eq!(original.token.card.subtypes, vec![Subtype::Human]);
        assert_eq!(original.token.card.color_indicator, Some(ColorSet::BLUE));
        let changed = effect.with_text_change(TextChange::creature_type(Subtype::Elf, Subtype::Zombie).unwrap()).unwrap()
            .with_text_change(TextChange::color(Color::Red, Color::Green).unwrap()).unwrap();
        let changed = changed.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(changed.token.card.name, "Red Elf");
        assert_eq!(changed.token.card.subtypes, vec![Subtype::Human]);
        assert_eq!(changed.token.card.color_indicator, Some(ColorSet::BLUE));
    }
}

#[test]
fn all_colors_and_enumerated_color_words_have_different_text_semantics() {
    for (body, expected_role, red_survives) in [
        ("Create a 2/2 Elf creature token that's all colors.", TokenWordRole::RulesImplied, true),
        ("Create a 2/2 white, blue, black, red, and green Elf creature token.", TokenWordRole::Authored, false),
    ] {
        for definition in routes(body) {
            let effect = instruction(&definition);
            assert_eq!(effect.downcast_ref::<CreateTokenEffect>().unwrap().text_roles.as_ref().unwrap().colors, expected_role);
            let changed = effect.with_text_change(TextChange::color(Color::Red, Color::Blue).unwrap()).unwrap();
            assert_eq!(changed.downcast_ref::<CreateTokenEffect>().unwrap().token.card.color_indicator.unwrap().contains(Color::Red), red_survives);
            assert_eq!(definition.canonical_text.contains("all colors"), red_survives);
        }
    }
}

#[test]
fn quoted_ability_characteristics_and_names_stay_inside_the_ability_owner() {
    for definition in routes("Create a colorless artifact token with \"{T}: Target Equipment gains indestructible until end of turn.\"") {
        let effect = instruction(&definition);
        let token = &effect.downcast_ref::<CreateTokenEffect>().unwrap().token;
        assert!(token.card.subtypes.is_empty());
        assert_eq!(token.card.name, "Token");
        assert!(token.abilities.iter().any(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))),
            "excluding quoted words from the description does not discard the quoted ability");
    }
    for definition in routes("Create a 2/2 blue Human creature token with \"Creatures named Elf get +1/+1.\"") {
        let effect = instruction(&definition);
        let original = effect.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(original.token.card.name, "Human Token");
        assert_eq!(original.token.card.subtypes, vec![Subtype::Human]);
        assert!(!original.token.abilities.is_empty());
        let changed = effect.with_text_change(TextChange::creature_type(Subtype::Elf, Subtype::Zombie).unwrap()).unwrap();
        let changed = changed.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(changed.token.card.name, "Human Token");
        assert_eq!(changed.token.card.subtypes, vec![Subtype::Human]);
    }
}

#[test]
fn described_vehicle_colors_and_legendary_characteristic_survive_lowering_and_rewriting() {
    for definition in routes("Create a legendary 3/3 blue Vehicle artifact token with flying.") {
        let effect = instruction(&definition);
        let original = effect.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(original.token.card.name, "Vehicle Token");
        assert_eq!(original.token.card.color_indicator, Some(ColorSet::BLUE));
        assert!(original.token.card.supertypes.contains(&ironsmith::Supertype::Legendary));
        assert_eq!(original.token.card.subtypes, vec![Subtype::Vehicle]);
        let changed = effect.with_text_change(TextChange::color(Color::Blue, Color::Red).unwrap()).unwrap();
        let changed = changed.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(changed.token.card.color_indicator, Some(ColorSet::RED));
        assert_eq!(changed.token.card.name, "Vehicle Token");
        assert_eq!(changed.token.card.power_toughness, original.token.card.power_toughness);
        assert_eq!(changed.token.card.supertypes, original.token.card.supertypes);
    }
}

#[test]
fn later_reminder_merging_keeps_names_and_filtered_or_activated_keyword_grants_scoped() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::static_abilities::StaticAbilityId;
    for (body, name) in [
        ("Create a 2/2 blue Human creature token named Hexproof.", "Hexproof"),
        ("Create a 2/2 blue Human creature token with \"{T}: Target creature gains indestructible until end of turn.\"", "Human Token"),
        ("Create a 2/2 blue Human creature token with \"Elves you control have hexproof.\"", "Human Token"),
    ] {
        for definition in routes(body) {
            let effect = instruction(&definition);
            let token = &effect.downcast_ref::<CreateTokenEffect>().unwrap().token;
            assert_eq!(token.card.name, name);
            assert!(!token.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability)
                if matches!(ability.id(), StaticAbilityId::Hexproof | StaticAbilityId::Indestructible))));
            if name == "Human Token" { assert!(!token.abilities.is_empty(), "the actual grant remains executable"); }
        }
    }
    for definition in routes("Create a 2/2 blue Human creature token with \"Hexproof.\"") {
        let effect = instruction(&definition);
        assert!(effect.downcast_ref::<CreateTokenEffect>().unwrap().token.abilities.iter().any(|ability|
            matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Hexproof)));
    }
}

#[test]
fn quoted_spawn_mana_followups_keep_each_authored_rule_without_name_based_skips() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::mana::ManaSymbol;
    for description in [
        "a 0/1 colorless Eldrazi Spawn creature token",
        "a 2/2 blue Human creature token named Eldrazi Spawn",
    ] {
        for definition in routes(&format!("Create {description}. It has \"Sacrifice this token: Add {{C}}.\" and \"{{T}}: Add {{G}}.\"")) {
            let effect = instruction(&definition);
            let token = effect.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(token.token.abilities.len(), 2);
            let outputs: Vec<_> = token.token.abilities.iter().map(|ability| {
                let AbilityKind::Activated(mana) = &ability.kind else { panic!("quoted mana ability"); };
                assert!(mana.is_mana_ability());
                mana.mana_output.clone().unwrap()
            }).collect();
            assert_eq!(outputs, vec![vec![ManaSymbol::Colorless], vec![ManaSymbol::Green]]);
            assert_eq!(token.text_roles.as_ref().unwrap().abilities, vec![TokenWordRole::Authored; 2]);
        }
    }
}

#[test]
fn predefined_profiles_preserve_implied_words_and_card_names() {
    for (noun, expected_name, colors, subtypes) in [
        ("Heartwood", "Heartwood Token", ColorSet::RED.union(ColorSet::GREEN), vec![Subtype::Heartwood]),
        ("Vibranium", "Vibranium Token", ColorSet::default(), vec![Subtype::Vibranium]),
        ("Walker", "Walker", ColorSet::BLACK, vec![Subtype::Zombie]),
        ("Spellgorger Weird", "Spellgorger Weird", ColorSet::RED, vec![Subtype::Weird]),
    ] {
        for definition in routes(&format!("Create a {noun} token.")) {
            let effect = instruction(&definition);
            let original = effect.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(original.token.card.name, expected_name);
            assert_eq!(original.token.card.subtypes, subtypes);
            let roles = original.text_roles.as_ref().unwrap();
            assert_eq!(roles.colors, TokenWordRole::RulesImplied);
            assert_eq!(roles.subtypes, TokenWordRole::RulesImplied);
            assert!(roles.abilities.iter().all(|role| *role == TokenWordRole::RulesImplied));
            let changed = effect.with_text_change(TextChange::color(Color::Red, Color::Blue).unwrap()).unwrap()
                .with_text_change(TextChange::creature_type(Subtype::Zombie, Subtype::Elf).unwrap()).unwrap()
                .with_text_change(TextChange::creature_type(Subtype::Weird, Subtype::Human).unwrap()).unwrap();
            let encoded = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(changed).unwrap();
            let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(encoded).unwrap();
            let changed = restored.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(changed.token.card.name, expected_name);
            assert_eq!(changed.token.card.subtypes, subtypes);
            assert_eq!(changed.token.card.colors(), colors);
            assert_eq!(changed.token.card.mana_cost, original.token.card.mana_cost);
            assert_eq!(changed.token.abilities, original.token.abilities);
            assert_eq!(changed.text_roles, original.text_roles);
        }
    }
}

#[test]
fn explicit_predefined_modifiers_keep_separate_color_name_and_ability_roles() {
    use ironsmith::ability::{AbilityKind, ProtectionFrom};
    for definition in routes("Create a legendary blue Heartwood token named Red with protection from red.") {
        let effect = instruction(&definition);
        let original = effect.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(original.token.card.name, "Red");
        assert_eq!(original.token.card.colors(), ColorSet::BLUE);
        assert!(original.token.card.supertypes.contains(&ironsmith::Supertype::Legendary));
        let roles = original.text_roles.as_ref().unwrap();
        assert_eq!(roles.name, TokenNameTextRole::Explicit);
        assert_eq!(roles.colors, TokenWordRole::Authored);
        assert_eq!(roles.abilities, vec![TokenWordRole::RulesImplied, TokenWordRole::Authored]);
        let changed = effect.with_text_change(TextChange::color(Color::Red, Color::Green).unwrap()).unwrap()
            .with_text_change(TextChange::color(Color::Blue, Color::Black).unwrap()).unwrap();
        let changed = changed.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(changed.token.card.name, "Red");
        assert_eq!(changed.token.card.colors(), ColorSet::BLACK);
        assert_eq!(changed.token.abilities[0], original.token.abilities[0]);
        let AbilityKind::Static(protection) = &changed.token.abilities[1].kind else { panic!("authored protection"); };
        assert_eq!(protection.protection_from(), Some(&ProtectionFrom::Color(ColorSet::GREEN)));
    }
    for definition in routes("Create a Food token that's green.") {
        let effect = instruction(&definition);
        let token = effect.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(token.token.card.colors(), ColorSet::GREEN);
        assert_eq!(token.text_roles.as_ref().unwrap().colors, TokenWordRole::Authored);
    }
}

#[test]
fn inherited_and_authored_equal_predefined_abilities_keep_two_occurrences() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::static_abilities::StaticAbilityId;
    for definition in routes("Create a Vibranium token with \"Indestructible.\"") {
        let effect = instruction(&definition);
        let token = effect.downcast_ref::<CreateTokenEffect>().unwrap();
        let occurrences: Vec<_> = token.token.abilities.iter().enumerate().filter_map(|(index, ability)|
            matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Indestructible)
                .then_some(index)).collect();
        assert_eq!(occurrences.len(), 2);
        let roles = token.text_roles.as_ref().unwrap();
        assert_eq!(roles.abilities[occurrences[0]], TokenWordRole::RulesImplied);
        assert_eq!(roles.abilities[occurrences[1]], TokenWordRole::Authored);
    }
}

#[test]
fn predefined_tokens_keep_quoted_rules_without_an_embedded_rule_owner() {
    use ironsmith::ability::AbilityKind;
    for description in ["a Heartwood token", "a blue Heartwood token named Witness"] {
        for definition in routes(&format!(
            "Create {description} with \"When this token dies, create a Treasure token.\""
        )) {
            let effect = instruction(&definition);
            let token = effect.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(token.token.abilities.len(), 2);
            assert!(matches!(&token.token.abilities[0].kind, AbilityKind::Activated(_)));
            assert!(matches!(&token.token.abilities[1].kind, AbilityKind::Triggered(_)));
            assert_eq!(token.text_roles.as_ref().unwrap().abilities,
                vec![TokenWordRole::RulesImplied, TokenWordRole::Authored]);
            let encoded = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(effect).unwrap();
            let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(encoded).unwrap();
            let restored = restored.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(restored.token.abilities.len(), 2);
            assert!(matches!(&restored.token.abilities[1].kind, AbilityKind::Triggered(_)));
        }
    }
}

#[test]
fn predefined_role_names_are_fixed_and_added_subtype_proof_stays_explicitly_incomplete() {
    for (noun, name) in [("Wicked Role", "Wicked"), ("Royal Role", "Royal"), ("Sorcerer Role", "Sorcerer")] {
        for definition in routes(&format!("Create a {noun} token.")) {
            let effect = instruction(&definition);
            let token = effect.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(token.token.card.name, name);
            assert_eq!(token.text_roles.as_ref().unwrap().name, TokenNameTextRole::Explicit);
            assert_eq!(token.token.card.subtypes, vec![Subtype::Aura, Subtype::Role]);
            assert!(token.token.aura_attach_filter.is_some());
        }
    }
    for definition in routes("Create a Forest land Food token.") {
        let effect = instruction(&definition);
        let original = effect.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(original.token.card.name, "Food Forest Token");
        assert_eq!(original.token.card.subtypes, vec![Subtype::Food, Subtype::Forest]);
        assert_eq!(original.text_roles.as_ref().unwrap().subtypes, TokenWordRole::Unrecorded);
        assert!(effect.with_text_change(TextChange::basic_land_type(Subtype::Forest, Subtype::Island).unwrap()).is_err());
        assert_eq!(original.token.card.name, "Food Forest Token");
    }
}
