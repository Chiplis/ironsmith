//! UNVALIDATED implementation-first coverage: open-ended ("one or more
//! creatures") and per-slot ("a Dinosaur, a Merfolk, a Pirate, and a Vampire")
//! Craft materials (CR 702.167a). Source-authored, deliberately unrun.
use ironsmith::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
use ironsmith::cards::CardDefinition;
use ironsmith::effect::ChoiceCount;
use ironsmith::effects::{ChooseObjectsEffect, EmitKeywordActionEffect, ExileEffect};
use ironsmith::events::KeywordActionKind;
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::{CardType, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/craft_material_slots.json.fixture")).unwrap()
}

fn definitions(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

fn craft(definition: &CardDefinition) -> &ActivatedAbility {
    let crafts: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) if activated.mana_cost.costs().iter().any(|cost|
            cost.effect_ref().and_then(|effect| effect.downcast_ref::<EmitKeywordActionEffect>())
                .is_some_and(|emit| emit.action == KeywordActionKind::Craft)) => {
            assert!(ability.functional_zones.contains(&Zone::Battlefield));
            Some(activated)
        }
        _ => None,
    }).collect();
    assert_eq!(crafts.len(), 1, "{}", definition.card.name);
    crafts[0]
}

/// Each material slot as (filter material with mechanic scope stripped, count).
fn material_slots(activated: &ActivatedAbility) -> Vec<(ObjectFilter, ChoiceCount)> {
    let costs = activated.mana_cost.costs();
    let mut slots = Vec::new();
    for pair in costs.windows(2) {
        let Some(choose) = pair[0].effect_ref().and_then(|effect| effect.downcast_ref::<ChooseObjectsEffect>()) else { continue };
        let Some(exile) = pair[1].effect_ref().and_then(|effect| effect.downcast_ref::<ExileEffect>()) else { continue };
        if !matches!(&exile.spec, ChooseSpec::Tagged(tag) if tag == &choose.tag) { continue; }
        assert_eq!(choose.chooser, PlayerFilter::You);
        assert_eq!(choose.filter.any_of.len(), 2);
        let battlefield = choose.filter.any_of.iter().find(|branch| branch.zone == Some(Zone::Battlefield)).unwrap();
        let graveyard = choose.filter.any_of.iter().find(|branch| branch.zone == Some(Zone::Graveyard)).unwrap();
        assert_eq!(battlefield.controller, Some(PlayerFilter::You));
        assert_eq!(graveyard.owner, Some(PlayerFilter::You));
        assert!(battlefield.other && graveyard.other, "the crafting source is never its own material");
        let mut material = battlefield.clone();
        material.zone = None; material.controller = None; material.other = false;
        slots.push((material, choose.count));
    }
    // The source itself is exiled as part of the same payment.
    assert!(costs.iter().any(|cost| cost.effect_ref().and_then(|effect| effect.downcast_ref::<ExileEffect>())
        .is_some_and(|exile| matches!(exile.spec, ChooseSpec::Source))));
    slots
}

#[test]
fn open_ended_and_slot_materials_compile_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 4);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let (id, expected, rendered): (&str, Vec<(ObjectFilter, ChoiceCount)>, &str) = match name {
            "Altar of the Wretched" => ("c551f007-9740-4a2a-8ef5-7be1a7afe14b",
                vec![(ObjectFilter::default().with_type(CardType::Creature), ChoiceCount::at_least(1))],
                "Craft with one or more creatures {2}{B}{B}"),
            "Paleontologist's Pick-Axe" => ("9b0c6b77-69df-4330-86a7-8dc4666abd38",
                vec![(ObjectFilter::default().with_type(CardType::Creature), ChoiceCount::at_least(1))],
                "Craft with one or more creatures {5}"),
            "Saheeli's Lattice" => ("ef8df503-6160-4e40-a689-49e97fd56cd0",
                vec![(ObjectFilter::default().with_subtype(Subtype::Dinosaur), ChoiceCount::at_least(1))],
                "Craft with one or more Dinosaurs {4}{R}"),
            "Throne of the Grim Captain" => ("a7c96442-706b-4fd8-9b38-052872dafe58",
                [Subtype::Dinosaur, Subtype::Merfolk, Subtype::Pirate, Subtype::Vampire].into_iter()
                    .map(|subtype| (ObjectFilter::default().with_subtype(subtype), ChoiceCount::exactly(1)))
                    .collect(),
                "Craft with a Dinosaur, a Merfolk, a Pirate, and a Vampire {4}"),
            other => panic!("unexpected cohort member {other}"),
        };
        assert_eq!(row["oracle_id"], id);
        for definition in definitions(row) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition), "{name}");
            let activated = craft(&definition);
            assert_eq!(activated.timing, ActivationTiming::SorcerySpeed, "craft is sorcery-speed");
            assert!(activated.choices.is_empty());
            assert_eq!(material_slots(activated), expected, "{name}");
            let lines = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
            assert!(lines.contains(rendered), "{name}: {lines}");
        }
    }
}

#[test]
fn unsupported_compound_materials_still_fail_closed() {
    for clause in [
        "four or more creatures with different names",
        "two artifacts and two creatures",
        "zero or more creatures",
    ] {
        let text = format!("Mana cost: {{2}}\nType: Artifact\nCraft with {clause} {{4}}");
        assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Craft probe", &text, false).is_err(), "{clause}");
    }
}

mod slot_matching {
    use ironsmith::ability::AbilityKind;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
    use ironsmith_compiler_runtime::compile_to_runtime_definition;

    const A: PlayerId = PlayerId(0);

    fn game() -> GameState {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game
    }

    fn material(game: &mut GameState, subtypes: &str) -> ObjectId {
        let text = format!("Type: Creature — {subtypes}\nPower/Toughness: 1/1");
        let definition = compile_to_runtime_definition("Craft material", &text, false).unwrap();
        game.create_object_from_definition(&definition, A, Zone::Graveyard)
    }

    fn craft_action(game: &mut GameState) -> (ObjectId, LegalAction) {
        let rows = super::fixtures();
        let row = rows.iter().find(|row| row["name"] == "Throne of the Grim Captain").unwrap();
        let definition = compile_to_runtime_definition(
            row["name"].as_str().unwrap(), row["text"].as_str().unwrap(), false).unwrap();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ability_index = definition.abilities.iter().position(|ability| matches!(&ability.kind,
            AbilityKind::Activated(activated) if activated.mana_cost.costs().len() > 4)).unwrap();
        for symbol in [ManaSymbol::Colorless, ManaSymbol::Black] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, 10);
        }
        (source, LegalAction::ActivateAbility { source, ability_index })
    }

    /// CR 702.167a: one Dinosaur Pirate can fill the Dinosaur slot or the
    /// Pirate slot, never both.
    #[test]
    fn one_object_cannot_fill_two_slots() {
        let mut game = game();
        material(&mut game, "Dinosaur Pirate");
        material(&mut game, "Merfolk");
        material(&mut game, "Vampire");
        let (_, action) = craft_action(&mut game);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
    }

    #[test]
    fn four_distinct_materials_make_craft_available() {
        let mut game = game();
        material(&mut game, "Dinosaur Pirate");
        material(&mut game, "Pirate");
        material(&mut game, "Merfolk");
        material(&mut game, "Vampire");
        let (_, action) = craft_action(&mut game);
        assert!(compute_legal_actions(&game, A).unwrap().contains(&action));
    }
}
