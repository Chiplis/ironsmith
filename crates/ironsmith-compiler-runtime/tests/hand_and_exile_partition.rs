//! End-to-end coverage for the revealed-library hand and exile partition.
use ironsmith::effects::MoveToZoneEffect;
use ironsmith::zone::Zone;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn eye_of_yawgmoth_exiles_the_rest() {
    let rows = common::rows(include_str!("../../../fixtures/hand_and_exile_partition.json.fixture"));
    for definition in common::definitions(common::row(&rows, "Eye of Yawgmoth")) {
        let effects = common::all_effects(&definition);
        let zones = effects
            .iter()
            .filter_map(|effect| effect.downcast_ref::<MoveToZoneEffect>())
            .map(|moved| moved.zone)
            .collect::<Vec<_>>();
        assert!(zones.contains(&Zone::Hand), "{zones:?}");
        assert!(zones.contains(&Zone::Exile), "{zones:?}");
    }
}

#[test]
fn eye_power_count_partition_moves_one_revealed_card_to_hand_and_only_the_rest_to_exile() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::card::CardBuilder;
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::effects::{execute_effect, EffectContext, LookAtTopCardsEffect};
    use ironsmith::ids::{CardId, ObjectId};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::types::CardType;
    use ironsmith::PowerToughness;
    use ironsmith::{PlayerId, Zone};

    struct PickOne(ObjectId);
    impl DecisionMaker for PickOne {
        fn decide_objects(&mut self, _: &ironsmith::GameState, choice: &SelectObjectsContext) -> Vec<ObjectId> {
            choice.candidates.iter().filter(|candidate| candidate.legal && candidate.id == self.0)
                .map(|candidate| candidate.id).collect()
        }
    }

    let rows = common::rows(include_str!("../../../fixtures/hand_and_exile_partition.json.fixture"));
    let definition = common::definitions(common::row(&rows, "Eye of Yawgmoth"))
        .into_iter().next().expect("compiled ability");
    let program = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => Some(&activated.effects),
        _ => None,
    }).expect("activated resolution");
    let effects = program.all_effects();
    assert_eq!(effects.len(), 5);
    let look = effects[0].downcast_ref::<LookAtTopCardsEffect>().unwrap();
    let ironsmith::effect::Value::PowerOf(power_source) = look.count.unhinted() else {
        panic!("count must use sacrificed creature's power: {:?}", look.count);
    };
    let ironsmith::target::ChooseSpec::Tagged(sacrifice_tag) = power_source.unhinted() else {
        panic!("count must refer to the cost's sacrificed creature");
    };

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardBuilder::new(CardId::new(), "Sacrificed creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3)).build();
    let sacrifice = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let sacrifice_snapshot = ObjectSnapshot::from_object_id(&game, sacrifice).unwrap();
    game.move_object(sacrifice, Zone::Graveyard, ironsmith::events::EventCause::effect());
    let mut original = Vec::new();
    for index in 0..3 {
        let card = CardBuilder::new(CardId::new(), format!("Revealed {index}"))
            .card_types(vec![CardType::Artifact]).build();
        original.push(game.create_object_from_card(&card, alice, Zone::Library));
    }
    let chosen_stable = game.object(original[1]).unwrap().stable_id;
    let unrelated_card = CardBuilder::new(CardId::new(), "Unrelated exile")
        .card_types(vec![CardType::Artifact]).build();
    let unrelated = game.create_object_from_card(&unrelated_card, alice, Zone::Exile);
    let mut chooser = PickOne(original[1]);
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut chooser);
    ctx.set_tagged_objects(sacrifice_tag.clone(), vec![sacrifice_snapshot]);
    for effect in effects {
        execute_effect(&mut game, effect, &mut ctx).expect("reveal and partition resolves");
    }
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    let in_hand = *game.player(alice).unwrap().hand.first().unwrap();
    assert_eq!(game.object(in_hand).unwrap().stable_id, chosen_stable);
    assert_eq!(game.exile.len(), 3, "two revealed remainder cards plus unrelated exile");
    assert!(game.exile.contains(&unrelated));
    assert_eq!(game.player(alice).unwrap().library.len(), 0);
}

#[test]
fn audit_detects_wrong_partition_destination_and_complement() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::effects::TagMatchingObjectsEffect;
    use ironsmith::Effect;
    use ironsmith_semantic_compare::{compare_card_semantics_scored, report_embedding_config};

    let rows = common::rows(include_str!("../../../fixtures/hand_and_exile_partition.json.fixture"));
    let name = "Eye of Yawgmoth";
    let row = common::row(&rows, name);
    let oracle = row["oracle_text"].as_str().unwrap();
    for original in common::definitions(row) {
        assert_eq!(ironsmith_text::compiled_text_lines(&original), [oracle]);
        let audit = ironsmith_text::semantic_audit_lines(&original);
        let (_, _, score, _, mismatch) = compare_card_semantics_scored(name, oracle, &audit, report_embedding_config());
        assert!(!mismatch && score == 1.0, "{score}: {audit:?}");

        for wrong_destination in [true, false] {
            let mut mutated = original.clone();
            let ability = mutated.abilities.iter_mut().find_map(|ability| match &mut ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            }).expect("activated ability");
            let effects = &mut ability.effects.segments[0].default_effects;
            if wrong_destination {
                let mut moved = effects[4].downcast_ref::<MoveToZoneEffect>().unwrap().clone();
                moved.zone = Zone::Graveyard;
                effects[4] = Effect::new(moved);
            } else {
                let mut captured = effects[2].downcast_ref::<TagMatchingObjectsEffect>().unwrap().clone();
                captured.filter.tagged_constraints.retain(|constraint| {
                    constraint.relation != ironsmith::filter::TaggedOpbjectRelation::IsNotTaggedObject
                });
                effects[2] = Effect::new(captured);
            }
            let audit = ironsmith_text::semantic_audit_lines(&mutated);
            let (_, _, changed_score, _, mismatch) = compare_card_semantics_scored(name, oracle, &audit, report_embedding_config());
            if wrong_destination {
                assert!(audit.join(" ").contains("the rest into your graveyard"), "{audit:?}");
            } else {
                assert!(!audit.join(" ").contains("exile the rest"), "{audit:?}");
            }
            assert!(mismatch && changed_score < score, "{changed_score}: {audit:?}");
        }
    }
}

#[test]
fn revealed_partition_preserves_library_bottom_order() {
    for order in ["in a random order", "in any order"] {
        let oracle = format!("Reveal the top four cards of your library. Put one into your hand and the rest on the bottom of your library {order}.");
        let row = serde_json::json!({
            "name": "Partition Order Probe", "text": format!("Type: Sorcery\n{oracle}"),
        });
        for definition in common::definitions(&row) {
            assert_eq!(ironsmith_text::semantic_audit_lines(&definition), [oracle.as_str()]);
        }
    }
}
