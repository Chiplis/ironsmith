//! Full linked source transport and exact physical-definition provenance.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn pairs() -> [(CardDefinition, CardDefinition); 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/revolution-experience-source.json"
    ))
    .unwrap();
    let card = &fixture["mechanics"][0]["representative_cards"][0];
    let faces = source_support::linked_definitions(card).unwrap();
    assert_eq!(faces.len(), 2);
    [
        (faces[0][0].clone(), faces[1][0].clone()),
        (faces[0][1].clone(), faces[1][1].clone()),
    ]
}
#[test]
fn full_source_back_copy_is_attached_on_entry_and_front_ability_copy_has_no_back() {
    for (front, back) in pairs() {
        for mode in [
            "normal",
            "source-left",
            "front-copy-token",
            "single-face-same-name",
            "wrong-subtype",
        ] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let mut back = back.clone();
            let required_subtype = *back
                .card
                .subtypes
                .iter()
                .find(|subtype| **subtype != ironsmith::Subtype::Aura)
                .unwrap();
            if mode == "wrong-subtype" {
                back.card
                    .subtypes
                    .retain(|subtype| *subtype != required_subtype);
            }
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let original = game.create_object_from_definition(&front, A, Zone::Battlefield);
            let mut source = original;
            if mode == "front-copy-token" {
                let id = game.new_object_id();
                let object =
                    ironsmith::object::Object::token_copy_of(game.object(original).unwrap(), id, A);
                game.add_object(object);
                source = id;
            }
            if mode == "single-face-same-name" {
                let mut def = front.clone();
                def.card.id = ironsmith::ids::CardId::new();
                def.card.other_face = None;
                def.card.other_face_name = None;
                source = game.create_object_from_definition(&def, A, Zone::Battlefield);
            }
            let mut ctx = EffectContext::new_default(source, A);
            if mode == "source-left" {
                ctx.source_snapshot = Some(ironsmith::snapshot::ObjectSnapshot::from_object(
                    game.object(source).unwrap(),
                    &game,
                ));
                game.remove_object(source);
            }
            let before = game.battlefield.len();
            let effect = Effect::new(ironsmith::effects::CreateLinkedFaceTokenEffect {
                required_subtype,
                attached_to: ChooseSpec::Player(ironsmith::filter::PlayerFilter::Specific(B)),
            });
            execute_effect(&mut game, &effect, &mut ctx).unwrap();
            let creates = matches!(mode, "normal" | "source-left");
            assert_eq!(
                game.battlefield.len(),
                before + usize::from(creates),
                "{mode}"
            );
            if creates {
                let token = game
                    .battlefield
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .find(|object| {
                        object.kind == ironsmith::object::ObjectKind::Token
                            && object.name.as_ref() == back.card.name
                    })
                    .unwrap();
                assert_eq!(token.mana_cost.as_deref(), back.card.mana_cost.as_ref());
                assert_eq!(&*token.abilities, &back.abilities);
                assert_eq!(
                    token.attached_to,
                    Some(ironsmith::object::AttachmentTarget::Player(B))
                );
            }
        }
    }
}
#[test]
fn actual_callaphe_damage_trigger_creates_complete_blessing_only_at_threshold() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    for (front, back) in pairs() {
        for starting in [0, 2] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            game.player_mut(A).unwrap().experience_counters = starting;
            let source = game.create_object_from_definition(&front, A, Zone::Battlefield);
            game.queue_trigger_event(
                Default::default(),
                ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::DamageEvent::with_cause(
                        source,
                        ironsmith::events::DamageTarget::Player(B),
                        1,
                        true,
                        ironsmith::events::EventCause::combat_damage(source),
                    ),
                    Default::default(),
                ),
            );
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(
                &mut game,
                &mut queue,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(
                &mut game,
                &mut queue,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.player(A).unwrap().experience_counters, starting + 1);
            let copies = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| {
                    object.kind == ironsmith::object::ObjectKind::Token
                        && object.name.as_ref() == back.card.name
                })
                .collect::<Vec<_>>();
            assert_eq!(copies.len(), usize::from(starting == 2));
            if let Some(token) = copies.first() {
                assert!(matches!(
                    token.attached_to,
                    Some(ironsmith::object::AttachmentTarget::Player(_))
                ));
                assert_eq!(&*token.abilities, &back.abilities);
                assert_eq!(token.mana_cost.as_deref(), back.card.mana_cost.as_ref());
            }
        }
    }
}
