//! Six complete lifecycle programs, authored source regressions (unrun).
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effects::{
    DealDamageEffect, EffectExecutor, EffectContext as ExecutionContext, TransformEffect, TurnFaceUpEffect,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_builder_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/permanent_lifecycle_triggers.json.fixture"
    ))
    .unwrap()
}
fn compile(
    row: &serde_json::Value,
    id: ironsmith::CardId,
    other: Option<(ironsmith::CardId, &str)>,
) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(id, name);
    if let Some((id, name)) = other {
        builder = builder
            .other_face(id)
            .other_face_name(name)
            .linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike)
            .transforming_dfc(true);
    }
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_builder_to_artifact(builder, row["text"].as_str().unwrap(), false)
    });
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let transported = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &transported] {
        assert_eq!(definition.card.transforming_dfc, other.is_some());
        if let Some((_, name)) = other {
            assert_eq!(definition.card.other_face_name.as_deref(), Some(name));
            assert_eq!(
                definition.card.linked_face_layout,
                ironsmith::card::LinkedFaceLayout::TransformLike
            );
        }
    }
    [direct, transported]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    compile(&row, ironsmith::CardId::new(), None)
}
fn linked_heirloom(game: &mut GameState, route: usize) -> CardDefinition {
    let row = fixtures()
        .into_iter()
        .find(|r| r["name"] == "Neglected Heirloom")
        .unwrap();
    let front = ironsmith::CardId::new();
    let back = ironsmith::CardId::new();
    let f = compile(&row, front, Some((back, "Ashmouth Blade")))[route].clone();
    let b = compile(
        &row["other_face"],
        back,
        Some((front, "Neglected Heirloom")),
    )[route]
        .clone();
    game.register_linked_face_definition(&f);
    game.register_linked_face_definition(&b);
    f
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    for player in [A, B] {
        for _ in 0..20 {
            for color in [
                ironsmith::mana::ManaSymbol::White,
                ironsmith::mana::ManaSymbol::Blue,
                ironsmith::mana::ManaSymbol::Green,
                ironsmith::mana::ManaSymbol::Colorless,
            ] {
                game.player_mut(player).unwrap().mana_pool.add(color, 1);
            }
        }
    }
    game
}
fn object(game: &mut GameState, player: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let d = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&d, player, zone)
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target
            .filter(|id| {
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&Target::Object(*id)))
            })
            .map(|id| vec![Target::Object(id)])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_targets(game, ctx))
    }
}
fn action(game: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(game.players.len());
    let mut q = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut q, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut q, dm).unwrap();
}
fn cast(game: &mut GameState, player: PlayerId, id: ObjectId, alternative: bool, dm: &mut Choices) {
    action(
        game,
        player,
        LegalAction::CastSpell {
            spell_id: id,
            from_zone: Zone::Hand,
            casting_method: if alternative {
                ironsmith::alternative_cast::CastingMethod::Alternative(0)
            } else {
                ironsmith::alternative_cast::CastingMethod::Normal
            },
        },
        dm,
    );
}
fn stack(game: &mut GameState, events: Vec<TriggerEvent>, dm: &mut Choices) -> usize {
    for event in events {
        game.queue_trigger_event(Default::default(), event);
    }
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..20 {
        if game.stack.is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("nonempty stack");
}
fn transform(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    target: ObjectId,
) -> Vec<TriggerEvent> {
    TransformEffect::new(ChooseSpec::SpecificObject(target))
        .execute(game, &mut ExecutionContext::new_default(source, controller))
        .unwrap()
        .events
}
fn counters(game: &GameState, id: ObjectId) -> u32 {
    game.counter_count(id, ironsmith::object::CounterType::PlusOnePlusOne)
}
fn token_count(game: &GameState, subtype: ironsmith::types::Subtype) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            game.object(**id).is_some_and(|o| {
                o.kind == ironsmith::object::ObjectKind::Token
                    && game.calculated_subtypes(**id).contains(&subtype)
            })
        })
        .count()
}
fn host(game: &mut GameState, player: PlayerId, human_back: bool) -> ObjectId {
    host_with_subtype(
        game,
        player,
        if human_back {
            ironsmith::types::Subtype::Human
        } else {
            ironsmith::types::Subtype::Wolf
        },
    )
}
fn host_with_subtype(
    game: &mut GameState,
    player: PlayerId,
    subtype: ironsmith::types::Subtype,
) -> ObjectId {
    let front = ironsmith::CardId::new();
    let back = ironsmith::CardId::new();
    let front_name = format!("Front host {front:?}");
    let back_name = format!("Back host {back:?}");
    let f = ironsmith::card::CardBuilder::new(front, front_name.clone())
        .other_face(back)
        .other_face_name(back_name.clone())
        .linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike)
        .transforming_dfc(true)
        .card_types(vec![ironsmith::types::CardType::Creature])
        .subtypes(vec![ironsmith::types::Subtype::Human])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    let b = ironsmith::card::CardBuilder::new(back, back_name)
        .other_face(front)
        .other_face_name(front_name)
        .linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike)
        .transforming_dfc(true)
        .card_types(vec![ironsmith::types::CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3))
        .build();
    let f = CardDefinition::new(f);
    let b = CardDefinition::new(b);
    game.register_linked_face_definition(&f);
    game.register_linked_face_definition(&b);
    game.create_object_from_definition(&f, player, Zone::Battlefield)
}

#[test]
fn all_six_exact_oracle_identities_and_both_equipment_faces_materialize() {
    assert_eq!(fixtures().len(), 6);
    for row in fixtures() {
        if row["other_face"].is_object() {
            for route in 0..2 {
                let mut game = game();
                let d = linked_heirloom(&mut game, route);
                assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
            }
        } else {
            for d in definitions(row["name"].as_str().unwrap()) {
                assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));
            }
        }
    }
}
#[test]
fn cult_reads_completed_nonhuman_characteristics_and_captured_controller() {
    for d in definitions("Cult of the Waxing Moon") {
        let mut game = game();
        let cult = game.create_object_from_definition(&d, A, Zone::Battlefield);
        let mut dm = Choices::default();
        let wolf = host(&mut game, A, false);
        let human = host(&mut game, A, true);
        let enemy = host(&mut game, B, false);
        game.take_pending_trigger_events();
        let events = transform(&mut game, cult, A, wolf);
        game.set_current_controller(wolf, B).unwrap(); // dispatch after a later, independent change
        assert_eq!(stack(&mut game, events, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(token_count(&game, ironsmith::types::Subtype::Wolf), 1);
        for target in [human, enemy] {
            let events = transform(&mut game, cult, A, target);
            assert_eq!(stack(&mut game, events, &mut dm), 0);
        }
        // Becoming Human again is not the qualifying transition.
        game.set_current_controller(wolf, A).unwrap();
        let events = transform(&mut game, cult, A, wolf);
        assert_eq!(stack(&mut game, events, &mut dm), 0);
    }
}
#[test]
fn inquisitor_full_entry_incubate_and_paid_token_transform_bind_exact_permanent() {
    for d in definitions("Norn's Inquisitor") {
        let mut game = game();
        let inquisitor = game.create_object_from_definition(&d, A, Zone::Hand);
        let mut dm = Choices::default();
        cast(&mut game, A, inquisitor, false, &mut dm);
        settle(&mut game, &mut dm);
        let token = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().name.as_ref() == "Incubator Token")
            .unwrap();
        assert_eq!(counters(&game, token), 2);
        assert!(!game.current_is_creature(token));
        action(
            &mut game,
            A,
            LegalAction::ActivateAbility {
                source: token,
                ability_index: 0,
            },
            &mut dm,
        );
        resolve(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 1);
        assert!(game.current_is_creature(token));
        assert_eq!(game.object(token).unwrap().name.as_ref(), "Phyrexian Token");
        assert_eq!(counters(&game, token), 2);
        resolve(&mut game, &mut dm);
        assert_eq!(counters(&game, token), 3);
        // Transforming back is a real event, but Incubator is not Phyrexian.
        let events = transform(&mut game, token, A, token);
        assert_eq!(stack(&mut game, events, &mut dm), 0);
        assert_eq!(counters(&game, token), 3);
        assert_eq!(game.object(token).unwrap().name.as_ref(), "Incubator Token");
        assert!(!game.current_is_creature(token));
        // A non-token card can legally blink. The later incarnation is not
        // the permanent identified by the transformation trigger.
        let permanent = host_with_subtype(&mut game, A, ironsmith::types::Subtype::Phyrexian);
        let events = transform(&mut game, permanent, A, permanent);
        assert_eq!(stack(&mut game, events, &mut dm), 1);
        let departed = game.move_object_by_effect(permanent, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(departed, Zone::Battlefield)
            .unwrap();
        settle(&mut game, &mut dm);
        assert_eq!(counters(&game, returned), 0);
    }
}
#[test]
fn heirloom_real_double_face_preserves_attachment_and_both_equip_costs() {
    for route in 0..2 {
        let mut game = game();
        let d = linked_heirloom(&mut game, route);
        let equipment = game.create_object_from_definition(&d, A, Zone::Battlefield);
        let creature = host(&mut game, A, false);
        let other = host(&mut game, A, false);
        let mut dm = Choices {
            target: Some(creature),
        };
        let index = d
            .abilities
            .iter()
            .position(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .unwrap();
        action(
            &mut game,
            A,
            LegalAction::ActivateAbility {
                source: equipment,
                ability_index: index,
            },
            &mut dm,
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.current_power(creature), Some(3));
        let events = transform(&mut game, creature, A, creature);
        assert_eq!(stack(&mut game, events, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(
            game.object(equipment).unwrap().name.as_ref(),
            "Ashmouth Blade"
        );
        assert_eq!(game.current_power(creature), Some(6));
        assert!(game.current_has_static_ability_id(
            creature,
            ironsmith::static_abilities::StaticAbilityId::FirstStrike
        ));
        assert_eq!(
            game.object(equipment).unwrap().attached_to,
            Some(ironsmith::object::AttachmentTarget::Object(creature))
        );
        let events = transform(&mut game, creature, A, creature);
        assert_eq!(stack(&mut game, events, &mut dm), 0);
        assert_eq!(
            game.object(equipment).unwrap().name.as_ref(),
            "Ashmouth Blade"
        );
        let index = game
            .object(equipment)
            .unwrap()
            .abilities
            .iter()
            .position(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .unwrap();
        dm.target = Some(other);
        let before = game.player(A).unwrap().mana_pool.total();
        action(
            &mut game,
            A,
            LegalAction::ActivateAbility {
                source: equipment,
                ability_index: index,
            },
            &mut dm,
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 3);
        settle(&mut game, &mut dm);
        assert_eq!(game.current_power(other), Some(5));
        assert_eq!(game.current_power(creature), Some(2));
    }
}
#[test]
fn equipped_transform_qualification_uses_event_time_attachment() {
    for route in 0..2 {
        let mut game = game();
        let d = linked_heirloom(&mut game, route);
        let equipment = game.create_object_from_definition(&d, A, Zone::Battlefield);
        let creature = host(&mut game, A, false);
        let other = host(&mut game, A, false);
        let mut dm = Choices::default();
        game.attach_object_to_target(
            equipment,
            ironsmith::object::AttachmentTarget::Object(creature),
        );
        game.take_pending_trigger_events();
        let events = transform(&mut game, creature, A, creature);
        game.attach_object_to_target(
            equipment,
            ironsmith::object::AttachmentTarget::Object(other),
        );
        game.take_pending_trigger_events();
        assert_eq!(stack(&mut game, events, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(
            game.object(equipment).unwrap().name.as_ref(),
            "Ashmouth Blade"
        );
        assert_eq!(game.current_power(other), Some(5));
    }
}
#[test]
fn essence_symbiote_real_mutate_cast_uses_surviving_permanent_and_postmerge_controller() {
    for d in definitions("Essence Symbiote") {
        for controller in [A, B] {
            let mut game = game();
            game.create_object_from_definition(&d, A, Zone::Battlefield);
            let host = object(
                &mut game,
                A,
                Zone::Battlefield,
                "Mutate host",
                "Type: Creature — Beast\nPower/Toughness: 2/2",
            );
            game.set_current_controller(host, controller).unwrap();
            let spell = object(
                &mut game,
                A,
                Zone::Hand,
                "Mutation program",
                "Mana cost: {5}\nType: Creature — Beast\nPower/Toughness: 4/4\nMutate {0}\nFlying",
            );
            let mut dm = Choices { target: Some(host) };
            game.take_pending_trigger_events();
            cast(&mut game, A, spell, true, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.mutation_count(host), 1);
            assert_eq!(game.stack.len(), usize::from(controller == A));
            settle(&mut game, &mut dm);
            assert_eq!(counters(&game, host), u32::from(controller == A));
            assert_eq!(
                game.player(A).unwrap().life,
                if controller == A { 22 } else { 20 }
            );
            assert_eq!(game.object(host).unwrap().name.as_ref(), "Mutation program");
        }
    }
}
#[test]
fn growing_dread_full_manifest_and_paid_face_up_bind_actor_and_exact_recipient() {
    for d in definitions("Growing Dread") {
        let mut game = game();
        for _ in 0..2 {
            object(
                &mut game,
                A,
                Zone::Library,
                "Manifest creature",
                "Mana cost: {1}\nType: Creature — Beast\nPower/Toughness: 3/3",
            );
        }
        let dread = game.create_object_from_definition(&d, A, Zone::Hand);
        let mut dm = Choices::default();
        cast(&mut game, A, dread, false, &mut dm);
        settle(&mut game, &mut dm);
        let manifested = *game
            .battlefield
            .iter()
            .find(|id| game.is_face_down(**id))
            .unwrap();
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(game.current_power(manifested), Some(2));
        action(
            &mut game,
            A,
            LegalAction::TurnFaceUp {
                creature_id: manifested,
                method: ironsmith::special_actions::TurnFaceUpMethod::PrintedManaCost,
            },
            &mut dm,
        );
        assert_eq!(game.stack.len(), 1);
        settle(&mut game, &mut dm);
        assert_eq!(game.current_power(manifested), Some(4));
        assert_eq!(counters(&game, manifested), 1);
        for (actor, recipient, expected) in [(A, B, 1), (B, A, 0)] {
            let target = object(
                &mut game,
                recipient,
                Zone::Battlefield,
                "Face-down recipient",
                "Type: Creature — Beast\nPower/Toughness: 3/3",
            );
            game.set_face_down(target);
            game.take_pending_trigger_events();
            let source = game.new_object_id();
            TurnFaceUpEffect::new(ChooseSpec::SpecificObject(target))
                .execute(&mut game, &mut ExecutionContext::new_default(source, actor))
                .unwrap();
            assert_eq!(stack(&mut game, vec![], &mut dm), expected);
            settle(&mut game, &mut dm);
            assert_eq!(counters(&game, target), expected as u32);
        }
    }
}
#[test]
fn wardens_real_renown_damage_counters_once_and_filtered_other_creatures() {
    for d in definitions("Valeron Wardens") {
        let mut game = game();
        for _ in 0..5 {
            object(&mut game, A, Zone::Library, "Draw resource", "Type: Land");
        }
        let wardens = game.create_object_from_definition(&d, A, Zone::Battlefield);
        let mut dm = Choices::default();
        game.take_pending_trigger_events();
        for expected in [1, 1] {
            let outcome = DealDamageEffect::new(
                1,
                ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(B)),
            )
            .with_combat(true)
            .execute(&mut game, &mut ExecutionContext::new_default(wardens, A))
            .unwrap();
            stack(&mut game, outcome.events, &mut dm);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), expected);
            assert!(game.is_renowned(wardens));
            assert_eq!(counters(&game, wardens), 2);
        }
        for controller in [A, B] {
            let source = object(
                &mut game,
                controller,
                Zone::Battlefield,
                "Other renown creature",
                "Type: Creature — Beast\nPower/Toughness: 2/2\nRenown 1",
            );
            let opponent = if controller == A { B } else { A };
            let outcome = DealDamageEffect::new(
                1,
                ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(opponent)),
            )
            .with_combat(true)
            .execute(
                &mut game,
                &mut ExecutionContext::new_default(source, controller),
            )
            .unwrap();
            stack(&mut game, outcome.events, &mut dm);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), 2);
            assert_eq!(counters(&game, source), 1);
        }
    }
}
#[test]
fn lifecycle_wire_variants_round_trip_without_reusing_old_variant_ordinals() {
    for trigger in [
        ironsmith_core::trigger_model::Trigger::permanent_transforms_into(
            ironsmith_core::ObjectFilter::permanent().you_control(),
            ironsmith_core::ObjectFilter::creature(),
        ),
        ironsmith_core::trigger_model::Trigger::permanent_mutates(
            ironsmith_core::ObjectFilter::creature().you_control(),
        ),
        ironsmith_core::trigger_model::Trigger::player_turns_face_up(
            ironsmith_core::PlayerFilter::You,
            ironsmith_core::ObjectFilter::permanent(),
        ),
    ] {
        let json = serde_json::to_string(&trigger).unwrap();
        let restored: ironsmith_core::trigger_model::Trigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, restored);
    }
}

#[test]
fn completed_face_up_batch_uses_all_new_continuous_characteristics() {
    let mut game = game();
    let mut dm = Choices::default();
    object(
        &mut game,
        A,
        Zone::Battlefield,
        "Face-up observer",
        "Type: Enchantment\nWhenever a non-Human creature is turned face up, you gain 1 life.",
    );
    let beast = object(
        &mut game,
        A,
        Zone::Battlefield,
        "Beast",
        "Type: Creature — Beast\nPower/Toughness: 2/2",
    );
    let lord = object(
        &mut game,
        A,
        Zone::Battlefield,
        "Human lord",
        "Type: Creature — Human\nPower/Toughness: 2/2\nOther creatures you control are Humans in addition to their other types.",
    );
    game.set_face_down(beast);
    game.set_face_down(lord);
    game.take_pending_trigger_events();
    let mut filter = ironsmith::target::ObjectFilter::creature();
    filter.face_down = Some(true);
    let source = game.new_object_id();
    TurnFaceUpEffect::new(ChooseSpec::All(filter))
        .execute(&mut game, &mut ExecutionContext::new_default(source, A))
        .unwrap();
    assert_eq!(
        stack(&mut game, vec![], &mut dm),
        0,
        "the Beast is Human in the single completed batch"
    );
    assert_eq!(game.player(A).unwrap().life, 20);
}

#[test]
fn lifecycle_trigger_observers_are_captured_before_a_later_instruction_removes_them() {
    for d in definitions("Cult of the Waxing Moon") {
        let mut game = game();
        let cult = game.create_object_from_definition(&d, A, Zone::Battlefield);
        let host = host(&mut game, A, false);
        let mut dm = Choices { target: Some(host) };
        let spell = object(
            &mut game,
            A,
            Zone::Hand,
            "Two instructions",
            "Mana cost: {0}\nType: Sorcery\nTransform target permanent. Destroy all creatures.",
        );
        game.take_pending_trigger_events();
        cast(&mut game, A, spell, false, &mut dm);
        resolve(&mut game, &mut dm);
        assert!(game.object(cult).is_none());
        assert_eq!(game.stack.len(), 1);
        settle(&mut game, &mut dm);
        assert_eq!(token_count(&game, ironsmith::types::Subtype::Wolf), 1);
    }
}

#[test]
fn day_night_changes_publish_complete_post_batch_transform_receipts_before_sbas() {
    for d in definitions("Cult of the Waxing Moon") {
        let mut game = game();
        game.create_object_from_definition(&d, A, Zone::Battlefield);
        for index in 0..2 {
            let front = ironsmith::CardId::new();
            let back = ironsmith::CardId::new();
            let front_name = format!("Day creature {index}");
            let back_name = format!("Night creature {index}");
            let f = serde_json::json!({"name":front_name,"text":"Type: Creature — Human\nPower/Toughness: 2/2\nDaybound"});
            let b = serde_json::json!({"name":back_name,"text":"Type: Creature — Wolf\nPower/Toughness: 3/3\nNightbound"});
            let f = compile(&f, front, Some((back, &back_name)))[0].clone();
            let b = compile(&b, back, Some((front, &front_name)))[0].clone();
            game.register_linked_face_definition(&f);
            game.register_linked_face_definition(&b);
            game.create_object_from_definition(&f, A, Zone::Battlefield);
        }
        game.set_daytime(true);
        game.take_pending_trigger_events();
        game.set_daytime(false);
        // The real turn/SBA owner finishes all as-transform programs before
        // any filtered transform event can be matched.
        let mut queue = TriggerQueue::new();
        let mut dm = Choices::default();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 2);
        settle(&mut game, &mut dm);
        assert_eq!(token_count(&game, ironsmith::types::Subtype::Wolf), 2);
    }
}
