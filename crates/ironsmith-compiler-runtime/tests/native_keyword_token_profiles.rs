//! Native/direct/artifact keyword-token contracts. Authored, not executed.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::effects::CreateTokenEffect;
use ironsmith::{CardType, Color, ColorSet, PlayerId, Subtype, Zone};
use ironsmith_core::{TextChange, TokenNameTextRole, TokenTextRoles};

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = artifact.unwrap();
    let artifact = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    artifact.validate().unwrap();
    [direct.unwrap(), ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()]
}

fn creates(definition: &CardDefinition) -> Vec<Effect> {
    fn collect(effect: &Effect, found: &mut Vec<Effect>) {
        if effect.downcast_ref::<CreateTokenEffect>().is_some() { found.push(effect.clone()); }
        effect.visit_child_effects(&mut |child| collect(child, found));
    }
    let mut found = Vec::new();
    for ability in &definition.abilities {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() { collect(effect, &mut found); }
        }
    }
    found
}

#[test]
fn six_keyword_profiles_survive_direct_artifact_native_rewrite_and_encoding() {
    for (keyword, equipment, name, colors, subtypes, count, flying) in [
        ("Afterlife 2", false, "Spirit Token", ColorSet::WHITE.union(ColorSet::BLACK), vec![Subtype::Spirit], 2, true),
        ("Fabricate 2", false, "Servo Token", ColorSet::default(), vec![Subtype::Servo], 2, false),
        ("For Mirrodin!", true, "Rebel Token", ColorSet::RED, vec![Subtype::Rebel], 1, false),
        ("Job select", true, "Hero Token", ColorSet::default(), vec![Subtype::Hero], 1, false),
        ("Living weapon", true, "Phyrexian Germ Token", ColorSet::BLACK, vec![Subtype::Phyrexian, Subtype::Germ], 1, false),
        ("Mobilize 2", false, "Warrior Token", ColorSet::RED, vec![Subtype::Warrior], 2, false),
    ] {
        let header = if equipment { "Type: Artifact — Equipment" }
            else { "Type: Creature — Human\nPower/Toughness: 2/2" };
        for definition in definitions("Keyword source", &format!("{header}\n{keyword}")) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let effects = creates(&definition);
            assert_eq!(effects.len(), 1);
            let effect = &effects[0];
            let native = effect.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(native.token.card.name, name);
            assert_eq!(native.token.card.colors(), colors);
            assert_eq!(native.token.card.subtypes, subtypes);
            assert_eq!(native.count, ironsmith::effect::Value::Fixed(count));
            assert_eq!(native.token.abilities.len(), usize::from(flying));
            assert_eq!(native.text_roles, Some(TokenTextRoles::rules_implied(TokenNameTextRole::SubtypeDerived, native.token.abilities.len())));
            let changed = effect.with_text_change(TextChange::color(Color::Red, Color::Blue).unwrap()).unwrap()
                .with_text_change(TextChange::creature_type(subtypes[0], Subtype::Elf).unwrap()).unwrap();
            let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(changed).unwrap();
            let changed = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
            let changed = changed.downcast_ref::<CreateTokenEffect>().unwrap();
            assert_eq!(changed.token.card.name, name);
            assert_eq!(changed.token.card.colors(), colors);
            assert_eq!(changed.token.card.subtypes, subtypes);
            assert_eq!(changed.token.abilities.len(), native.token.abilities.len());
            assert_eq!(changed.text_roles, native.text_roles);
            assert_eq!(changed.enters_tapped, keyword == "Mobilize 2");
            assert_eq!(changed.enters_attacking, keyword == "Mobilize 2");
            assert_eq!(changed.sacrifice_at_next_end_step, keyword == "Mobilize 2");
            assert!(ironsmith_text::compiled_text_lines(&definition).iter().any(|line|
                line.trim_end_matches('.').eq_ignore_ascii_case(keyword)), "{keyword}");
        }
    }
}

#[test]
fn ordinary_descriptions_keep_authored_words_and_do_not_become_keyword_labels() {
    for definition in definitions("Literal spirit creator", "Type: Creature — Human\nPower/Toughness: 2/2\nWhen this creature dies, create two 1/1 white and black Spirit creature tokens with flying.") {
        let effect = creates(&definition).pop().unwrap();
        let changed = effect.with_text_change(TextChange::color(Color::White, Color::Green).unwrap()).unwrap()
            .with_text_change(TextChange::creature_type(Subtype::Spirit, Subtype::Elf).unwrap()).unwrap();
        let changed = changed.downcast_ref::<CreateTokenEffect>().unwrap();
        assert_eq!(changed.token.card.name, "Elf Token");
        assert_eq!(changed.token.card.colors(), ColorSet::GREEN.union(ColorSet::BLACK));
        assert!(!ironsmith_text::compiled_text_lines(&definition).iter().any(|line| line.starts_with("Afterlife")));
    }
}

#[test]
fn every_frozen_job_select_body_creates_and_attaches_the_correctly_named_hero() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/native_keyword_token_profiles.json.fixture")).unwrap();
    assert_eq!(rows.len(), 18);
    let rows: Vec<_> = rows.iter().filter(|row| row["oracle_text"].as_str().unwrap().contains("Job select")).collect();
    assert_eq!(rows.len(), 16);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
        for definition in definitions(name, &text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
            let player = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&definition, player, Zone::Battlefield);
            let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::ZoneChangeEvent::with_cause(source, Zone::Stack, Zone::Battlefield,
                    ironsmith::events::cause::EventCause::from_game_rule(), None), Default::default());
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 1, "{name}");
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let heroes: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id))
                .filter(|object| object.kind == ironsmith::object::ObjectKind::Token && object.subtypes.contains(&Subtype::Hero)).collect();
            assert_eq!(heroes.len(), 1, "{name}");
            let hero = heroes[0];
            assert_eq!(hero.name.as_ref(), "Hero Token");
            assert_eq!(hero.card_types.as_slice(), &[CardType::Creature]);
            assert_eq!(hero.base_power, Some(ironsmith::card::PtValue::Fixed(1)));
            assert_eq!(hero.base_toughness, Some(ironsmith::card::PtValue::Fixed(1)));
            assert_eq!(game.object(source).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(hero.id)));
        }
    }
}
