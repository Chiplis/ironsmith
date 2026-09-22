use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, ViewCardsContext};
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::{AbilityKind, CardType, GameState, ObjectId, PlayerId, Zone};

struct Choices {
    accept: bool,
    expected: Vec<ObjectId>,
    seen: Vec<ObjectId>,
    selections: usize,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.accept
    }
    fn view_cards(
        &mut self,
        _: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        assert_eq!(viewer, PlayerId::from_index(0));
        assert!(!ctx.public);
        self.seen.extend_from_slice(cards);
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(ctx.player, PlayerId::from_index(0));
        assert_eq!((ctx.min, ctx.max), (1, Some(1)));
        let mut actual: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        actual.sort();
        let mut expected = self.expected.clone();
        expected.sort();
        assert_eq!(
            actual, expected,
            "only nonland cards in the targeted opponent's hand are eligible"
        );
        self.selections += 1;
        self.expected.iter().copied().take(1).collect()
    }
}

#[test]
fn bat_exiles_only_from_targeted_hand_and_returns_to_that_hand() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deep-Cavern Bat",
    )
    .unwrap()
    .remove(0);
    let definition = ironsmith_tools::compile_definition_from_payload(&payload).unwrap();
    exercise_bat(&definition);
}

#[test]
fn compiled_artifact_roundtrip_exiles_only_from_targeted_hand_and_returns_to_that_hand() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deep-Cavern Bat",
    )
    .unwrap()
    .remove(0);
    let compiled = ironsmith_compiler::CompilerFacade::new()
        .compile_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(
                ironsmith::ids::CardId::new(),
                &payload.name,
            ),
            payload.parse_input,
            ironsmith_compiler::CompilePolicy {
                allow_unsupported: false,
            },
        )
        .unwrap();
    let wire = serde_json::from_value(serde_json::to_value(&compiled.definition).unwrap()).unwrap();
    let definition = ironsmith::artifact_materializer::materialize_definition(wire).unwrap();
    exercise_bat(&definition);
}

fn exercise_bat(definition: &ironsmith::cards::CardDefinition) {
    let trigger = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert_eq!(trigger.choices.len(), 1, "only the opponent is targeted");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for accept in [false, true] {
        for left_before in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let mut expected = Vec::new();
            let mut hand = Vec::new();
            for owner in [alice, bob, carol] {
                for zone in [Zone::Hand, Zone::Battlefield, Zone::Graveyard] {
                    for kind in [CardType::Land, CardType::Instant, CardType::Sorcery] {
                        let card =
                            CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Candidate")
                                .card_types(vec![kind])
                                .build();
                        let id = game.create_object_from_definition(&card, owner, zone);
                        if owner == carol && zone == Zone::Hand {
                            hand.push(id);
                            if kind != CardType::Land {
                                expected.push(id);
                            }
                        }
                    }
                }
            }
            let chosen_stable = game.object(expected[0]).unwrap().stable_id;
            let source_snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(
                game.object(source).unwrap(),
                &game,
            );
            let mut dm = Choices {
                accept,
                expected,
                seen: vec![],
                selections: 0,
            };
            if left_before {
                game.move_object_by_effect(source, Zone::Graveyard);
            }
            let mut ctx = EffectContext::new(source, alice, &mut dm);
            ctx.source_snapshot = Some(source_snapshot);
            ctx.targets = vec![ResolvedTarget::Player(carol)];
            for effect in trigger.effects.flattened_default_effects() {
                execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            drop(ctx);
            assert!(
                dm.seen.starts_with(&hand),
                "the entire targeted hand is shown first"
            );
            assert!(
                dm.seen.iter().all(|id| hand.contains(id)),
                "only the targeted hand is viewed"
            );
            assert_eq!(dm.selections, usize::from(accept && !left_before));
            let chosen = game.find_object_by_stable_id(chosen_stable).unwrap();
            assert_eq!(
                game.object(chosen).unwrap().zone,
                if accept && !left_before {
                    Zone::Exile
                } else {
                    Zone::Hand
                }
            );
            if !left_before {
                game.move_object_by_effect(source, Zone::Graveyard);
                ironsmith::game_loop::drain_pending_trigger_events(
                    &mut game,
                    &mut ironsmith::triggers::TriggerQueue::new(),
                );
            }
            let returned = game.find_object_by_stable_id(chosen_stable).unwrap();
            assert_eq!(game.object(returned).unwrap().zone, Zone::Hand);
            assert!(game.player(carol).unwrap().hand.contains(&returned));
            assert!(game.exile.is_empty());
        }
    }
}

#[test]
fn shown_hand_exile_preserves_owner_and_card_qualifiers() {
    let mut payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deep-Cavern Bat",
    )
    .unwrap()
    .remove(0);
    payload.name = "Hand Exile Probe".into();
    payload.parse_name = None;
    for opening in [
        "look at target player's hand",
        "target player reveals their hand",
    ] {
        payload.parse_input = format!(
            "{}\nWhen this creature enters, {opening}. You may exile an instant card from it until this creature leaves the battlefield.",
            payload.metadata_lines.join("\n")
        );
        let definition = ironsmith_tools::compile_definition_from_payload(&payload).unwrap();
        let trigger = definition
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .unwrap();
        assert_eq!(trigger.choices.len(), 1);
        let optional = trigger
            .effects
            .flattened_default_effects()
            .iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::MayEffect>())
            .unwrap();
        let exile = optional.effects[0]
            .downcast_ref::<ironsmith::effects::ExileUntilEffect>()
            .unwrap();
        let ironsmith::target::ChooseSpec::Object(filter) = exile.spec.base() else {
            panic!("{:?}", exile.spec);
        };
        assert_eq!(filter.zone, Some(Zone::Hand), "{opening}");
        assert!(
            matches!(
                filter.owner,
                Some(ironsmith::target::PlayerFilter::Target(_))
                    | Some(ironsmith::target::PlayerFilter::AliasedTarget(_))
            ),
            "{filter:?}"
        );
        assert_eq!(filter.card_types, [CardType::Instant]);
    }
}
