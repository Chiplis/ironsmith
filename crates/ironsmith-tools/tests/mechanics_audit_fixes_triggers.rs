//! September 2026 audit F1, F2, F3, F5, and F31.
//! CR 702.62, 702.55, 702.46, 702.153b, and 702.127/709.3.
use ironsmith::ability::AbilityKind;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder as B};
use ironsmith::cost::OptionalCostsPaid;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effects::{EffectContext as ExecutionContext, EffectExecutor as _};
use ironsmith::events::cause::EventCause;
use ironsmith::game_loop::{
    drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::mana::ManaCost;
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler::CardDefinitionBuilder as CB;

const A: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn creature() -> CardDefinition {
    B::new(CardId::new(), "Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}
fn runtime(def: ironsmith_compiler::CardDefinition) -> CardDefinition {
    // The compiler-to-runtime conversion holds large typed enum values in
    // debug builds. Give this adapter the same headroom as the compiler API.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || ironsmith_tools::into_runtime_definition(def).unwrap())
        .unwrap()
        .join()
        .unwrap()
}
fn suspend_def(compiler: bool) -> CardDefinition {
    if compiler {
        runtime(
            CB::new(CardId::new(), "Suspended spell")
                .card_types(vec![CardType::Sorcery])
                .suspend(3, ManaCost::new())
                .build(),
        )
    } else {
        B::new(CardId::new(), "Suspended spell")
            .card_types(vec![CardType::Sorcery])
            .suspend(3, ManaCost::new())
            .build()
    }
}
fn queue_event(game: &GameState, event: &TriggerEvent) -> TriggerQueue {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, event) {
        queue.add(trigger);
    }
    queue
}
fn suspend_ready(compiler: bool) -> (GameState, ObjectId, TriggerQueue) {
    let mut game = game();
    let id = game.create_object_from_definition(&suspend_def(compiler), A, Zone::Exile);
    game.add_counters(id, CounterType::Time, 3);
    let (_, event) = game
        .remove_counters(id, CounterType::Time, 3, None, None)
        .unwrap();
    let queue = queue_event(&game, &event);
    assert_eq!(
        queue.entries.len(),
        1,
        "one last-counter event, regardless of quantity"
    );
    (game, id, queue)
}

#[test]
fn suspend_ignores_other_counters_and_nonfinal_time_counter_removal() {
    for compiler in [false, true] {
        let mut game = game();
        let id = game.create_object_from_definition(&suspend_def(compiler), A, Zone::Exile);
        game.add_counters(id, CounterType::Charge, 1);
        let (_, event) = game
            .remove_counters(id, CounterType::Charge, 1, None, None)
            .unwrap();
        assert!(check_triggers(&game, &event).is_empty());
        game.add_counters(id, CounterType::Time, 2);
        let (_, event) = game
            .remove_counters(id, CounterType::Time, 1, None, None)
            .unwrap();
        assert!(check_triggers(&game, &event).is_empty());
    }
}

#[test]
fn suspend_last_counter_trigger_still_casts_after_a_time_counter_is_added() {
    for compiler in [false, true] {
        let (mut game, id, mut queue) = suspend_ready(compiler);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(game.stack.len(), 1);
        game.add_counters(id, CounterType::Time, 1);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(
            game.stack.len(),
            1,
            "the spell should be cast on trigger resolution"
        );
        assert!(!game.stack[0].is_ability);
    }
}

#[test]
fn suspend_last_counter_trigger_can_be_declined_and_requires_same_exiled_object() {
    for compiler in [false, true] {
        let (mut game, id, mut queue) = suspend_ready(compiler);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.object(id).unwrap().zone, Zone::Exile);

        let (mut game, id, mut queue) = suspend_ready(compiler);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        let hand = game
            .move_object(id, Zone::Hand, EventCause::from_game_rule())
            .unwrap();
        let new_id = game
            .move_object(hand, Zone::Exile, EventCause::from_game_rule())
            .unwrap();
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.object(new_id).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn granted_suspend_uses_the_same_last_counter_rule() {
    let abilities =
        ironsmith_compiler::runtime_static_ability_helpers::suspend_exile_triggered_abilities();
    let mut def = CB::new(CardId::new(), "Granted suspend").card_types(vec![CardType::Sorcery]);
    for ability in abilities {
        def = def.with_ability(ability);
    }
    let mut game = game();
    let id = game.create_object_from_definition(&runtime(def.build()), A, Zone::Exile);
    game.add_counters(id, CounterType::Time, 3);
    let (_, event) = game
        .remove_counters(id, CounterType::Time, 3, None, None)
        .unwrap();
    let triggered = check_triggers(&game, &event);
    assert_eq!(triggered.len(), 1);
    assert!(matches!(
        triggered[0].ability.intervening_if,
        Some(ironsmith::ConditionExpr::SourceIsInZone(Zone::Exile))
    ));
}

fn haunt_def(compiler: bool) -> CardDefinition {
    let mut def = if compiler {
        runtime(
            CB::new(CardId::new(), "Haunt spell")
                .card_types(vec![CardType::Sorcery])
                .haunt()
                .build(),
        )
    } else {
        B::new(CardId::new(), "Haunt spell")
            .card_types(vec![CardType::Sorcery])
            .haunt()
            .build()
    };
    def.spell_effect =
        Some(vec![Effect::destroy(ChooseSpec::target(ChooseSpec::creature()))].into());
    def
}

fn haunt_setup(compiler: bool) -> (GameState, ObjectId, ObjectId) {
    let mut game = game();
    let target = game.create_object_from_definition(&creature(), BOB, Zone::Battlefield);
    let spell = game.create_object_from_definition(&haunt_def(compiler), A, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, A).with_targets(vec![Target::Object(target)]));
    (game, spell, target)
}
fn pending_haunt_count(game: &mut GameState) -> usize {
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events(game, &mut queue);
    queue
        .entries
        .iter()
        .filter(|trigger| trigger.source_name == "Haunt spell")
        .count()
}

#[test]
fn haunt_spell_triggers_only_during_its_successful_resolution() {
    for compiler in [false, true] {
        let (mut game, _, _) = haunt_setup(compiler);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(pending_haunt_count(&mut game), 1);
    }
}

#[test]
fn haunt_spell_does_not_trigger_when_countered_by_another_resolving_spell() {
    for compiler in [false, true] {
        let (mut game, spell, _) = haunt_setup(compiler);
        let counter = B::new(CardId::new(), "Counter spell")
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![Effect::counter(ChooseSpec::SpecificObject(spell))])
            .build();
        let counter = game.create_object_from_definition(&counter, BOB, Zone::Stack);
        game.push_to_stack(StackEntry::new(counter, BOB));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(pending_haunt_count(&mut game), 0);
    }
}

#[test]
fn haunt_spell_does_not_trigger_when_all_targets_are_illegal() {
    for compiler in [false, true] {
        let (mut game, _, target) = haunt_setup(compiler);
        game.move_object(target, Zone::Exile, EventCause::from_game_rule())
            .unwrap();
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(pending_haunt_count(&mut game), 0);
    }
}

#[test]
fn haunt_spell_does_not_trigger_when_graveyard_move_is_replaced() {
    for compiler in [false, true] {
        let (mut game, spell, _) = haunt_setup(compiler);
        ironsmith::effects::ExileInsteadOfGraveyardEffect::you()
            .execute(&mut game, &mut ExecutionContext::new_default(spell, A))
            .unwrap();
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(pending_haunt_count(&mut game), 0);
        assert!(
            game.exile
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Haunt spell")
        );
    }
}

fn soulshift_def(compiler: bool) -> CardDefinition {
    if compiler {
        runtime(
            CB::new(CardId::new(), "Soulshift creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .soulshift(3)
                .build(),
        )
    } else {
        B::new(CardId::new(), "Soulshift creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .soulshift(3)
            .build()
    }
}
fn soulshift_setup(compiler: bool, target: bool) -> (GameState, Option<ObjectId>, TriggerQueue) {
    let mut game = game();
    let target = target.then(|| {
        game.create_object_from_definition(
            &B::new(CardId::new(), "Spirit")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![ironsmith::types::Subtype::Spirit])
                .mana_cost(ManaCost::new())
                .power_toughness(PowerToughness::fixed(1, 1))
                .build(),
            A,
            Zone::Graveyard,
        )
    });
    let source = game.create_object_from_definition(&soulshift_def(compiler), A, Zone::Battlefield);
    game.move_object(source, Zone::Graveyard, EventCause::from_game_rule())
        .unwrap();
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(queue.entries.len(), 1);
    (game, target, queue)
}

#[test]
fn soulshift_requires_a_target_then_allows_declining_at_resolution() {
    for compiler in [false, true] {
        let (mut game, target, mut queue) = soulshift_setup(compiler, true);
        let target = target.unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(game.stack[0].targets, vec![Target::Object(target)]);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.object(target).unwrap().zone, Zone::Graveyard);
        let (mut game, target, mut queue) = soulshift_setup(compiler, true);
        let stable = game.object(target.unwrap()).unwrap().stable_id;
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Hand
        );
    }
}

#[test]
fn soulshift_without_any_legal_target_is_not_put_on_stack() {
    for compiler in [false, true] {
        let (mut game, _, mut queue) = soulshift_setup(compiler, false);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert!(game.stack.is_empty());
    }
}

#[derive(Default)]
struct CountMay {
    count: usize,
}
impl DecisionMaker for CountMay {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.count += 1;
        true
    }
}
#[test]
fn soulshift_illegal_target_stops_resolution_before_the_optional_choice() {
    for compiler in [false, true] {
        let (mut game, target, mut queue) = soulshift_setup(compiler, true);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        game.move_object(target.unwrap(), Zone::Exile, EventCause::from_game_rule())
            .unwrap();
        let mut dm = CountMay::default();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.count, 0);
    }
}

#[test]
fn each_printed_casualty_instance_checks_only_its_own_payment() {
    for compiler in [false, true] {
        let def = if compiler {
            runtime(
                CB::new(CardId::new(), "Double casualty")
                    .card_types(vec![CardType::Sorcery])
                    .casualty(1)
                    .casualty(3)
                    .build(),
            )
        } else {
            B::new(CardId::new(), "Double casualty")
                .card_types(vec![CardType::Sorcery])
                .casualty(1)
                .casualty(3)
                .build()
        };
        assert_ne!(
            def.optional_costs[0].cost_ref(),
            def.optional_costs[1].cost_ref()
        );
        assert!(
            def.optional_costs
                .iter()
                .all(|cost| cost.display_label() == "Casualty")
        );
        for mask in 0_u32..4 {
            let mut game = game();
            let id = game.create_object_from_definition(&def, A, Zone::Stack);
            let mut paid = OptionalCostsPaid::from_costs(&def.optional_costs);
            for index in 0..2 {
                if mask & (1 << index) != 0 {
                    paid.pay(index);
                }
            }
            game.object_mut(id).unwrap().optional_costs_paid = paid.clone();
            let mut entry = StackEntry::new(id, A);
            entry.optional_costs_paid = paid;
            game.push_to_stack(entry);
            let event = TriggerEvent::new(
                ironsmith::events::spells::SpellCastEvent::new(id, A, Zone::Hand),
                Default::default(),
            );
            let mut queue = queue_event(&game, &event);
            assert_eq!(
                queue.entries.len(),
                mask.count_ones() as usize,
                "compiler={compiler}, mask={mask}"
            );
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
                .unwrap();
            let trigger_count = mask.count_ones() as usize;
            for _ in 0..trigger_count {
                // Each resolving copy is placed above remaining casualty triggers.
                resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                assert!(!game.stack.last().unwrap().is_ability);
                resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            }
            assert_eq!(
                game.stack.len(),
                1,
                "original spell remains after all copy triggers/copies resolve"
            );
        }
    }
}

#[test]
fn printed_and_granted_casualty_payments_do_not_cross_match() {
    let mut def = B::new(CardId::new(), "Printed and granted casualty")
        .card_types(vec![CardType::Sorcery])
        .casualty(1)
        .build();
    let mut granted = def.abilities[0].clone();
    let AbilityKind::Triggered(trigger) = &mut granted.kind else {
        panic!()
    };
    trigger.intervening_if = Some(ironsmith::ConditionExpr::ThisSpellPaidLabel(
        "Granted Casualty 2".into(),
    ));
    def.abilities.push(granted);
    def.optional_costs
        .push(ironsmith::cost::OptionalCost::custom(
            "Granted Casualty 2",
            ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::sacrifice(
                ironsmith::target::ObjectFilter::creature(),
            )),
        ));
    for mask in 0_u32..4 {
        let mut game = game();
        let id = game.create_object_from_definition(&def, A, Zone::Stack);
        let mut paid = OptionalCostsPaid::from_costs(&def.optional_costs);
        for index in 0..2 {
            if mask & (1 << index) != 0 {
                paid.pay(index);
            }
        }
        game.object_mut(id).unwrap().optional_costs_paid = paid.clone();
        let mut entry = StackEntry::new(id, A);
        entry.optional_costs_paid = paid;
        game.push_to_stack(entry);
        let event = TriggerEvent::new(
            ironsmith::events::spells::SpellCastEvent::new(id, A, Zone::Hand),
            Default::default(),
        );
        assert_eq!(
            check_triggers(&game, &event).len(),
            mask.count_ones() as usize
        );
    }
}

#[test]
fn aftermath_half_permission_stays_specific_but_other_grants_can_allow_both_halves() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::grant_registry::{GrantSource, PlayFromConstraints};
    use ironsmith::mana::ManaSymbol;
    let defs = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Cut // Ribbons",
    )
    .unwrap()
    .iter()
    .map(|p| ironsmith_tools::compile_definition_from_payload(p).unwrap())
    .collect::<Vec<_>>();
    let mut game = game();
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    for def in &defs {
        game.register_linked_face_definition(def);
    }
    let cut = defs.iter().find(|def| def.card.name == "Cut").unwrap();
    let card = game.create_object_from_definition(cut, A, Zone::Graveyard);
    game.create_object_from_definition(&creature(), BOB, Zone::Battlefield);
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 4);
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 2);
    let routes = |game: &GameState| {
        compute_legal_actions(game, A).expect("fixture has complete replacement state")
            .into_iter()
            .filter_map(|action| {
                if let LegalAction::CastSpell {
                    spell_id,
                    casting_method,
                    ..
                } = action
                {
                    (spell_id == card).then_some(casting_method)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    let original_routes = routes(&game);
    assert!(!original_routes.is_empty());
    assert!(original_routes.iter().all(|method| matches!(
        method,
        CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. }
    )));
    let grant_source = game.create_object_from_definition(&creature(), A, Zone::Battlefield);
    game.effect_store.grant_registry.grant_play_from_to_card(
        card,
        Zone::Graveyard,
        A,
        PlayFromConstraints::default(),
        GrantSource::Effect {
            source_id: grant_source,
            expires_end_of_turn: u32::MAX,
        },
    );
    let extra_routes = routes(&game);
    assert!(
        extra_routes
            .iter()
            .any(|method| matches!(method, CastingMethod::PlayFrom { .. })),
        "an independent graveyard permission still authorizes Cut"
    );
    assert!(extra_routes.iter().any(|method| matches!(
        method,
        CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. }
    )));
}

#[test]
fn graveyard_flashback_grants_filter_the_adventure_card_before_selecting_its_spell_face() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::grant::Grantable;
    use ironsmith::grant_registry::GrantSource;
    use ironsmith::target::ObjectFilter;
    use ironsmith::types::Subtype;

    let mut game = game();
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let front_id = CardId::new();
    let adventure_id = CardId::new();
    let front = B::new(front_id, "Adventure creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::new())
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(adventure_id)
        .other_face_name("Adventure sorcery")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let adventure = B::new(adventure_id, "Adventure sorcery")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Adventure])
        .mana_cost(ManaCost::new())
        .other_face(front_id)
        .other_face_name("Adventure creature")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&adventure);
    let card = game.create_object_from_definition(&front, A, Zone::Graveyard);
    let ordinary_sorcery = game.create_object_from_definition(
        &B::new(CardId::new(), "Ordinary sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::new())
            .build(),
        A,
        Zone::Graveyard,
    );
    let source = game.create_object_from_definition(&creature(), A, Zone::Battlefield);
    let grant_source = GrantSource::Effect {
        source_id: source,
        expires_end_of_turn: u32::MAX,
    };
    game.effect_store.grant_registry.grant_to_filter(
        ObjectFilter::default().with_type(CardType::Sorcery),
        Zone::Graveyard,
        A,
        Grantable::flashback_from_cards_mana_cost(),
        grant_source.clone(),
    );
    let actions = compute_legal_actions(&game, A).expect("fixture has complete replacement state");
    assert!(
        actions.iter().any(|action| matches!(
            action, LegalAction::CastSpell { spell_id, .. } if *spell_id == ordinary_sorcery
        )),
        "the grant authorizes an ordinary sorcery card"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card
        )),
        "the Adventure's sorcery face does not make its graveyard card a sorcery"
    );

    game.effect_store.grant_registry.grant_to_filter(
        ObjectFilter::default().with_type(CardType::Creature),
        Zone::Graveyard,
        A,
        Grantable::flashback_from_cards_mana_cost(),
        grant_source,
    );
    assert!(
        compute_legal_actions(&game, A).expect("fixture has complete replacement state")
            .iter()
            .any(|action| matches!(
                action, LegalAction::CastSpell {
                    spell_id, casting_method: CastingMethod::SplitOtherHalfPlayFrom { .. }, ..
                } if *spell_id == card
            )),
        "a grant matching the creature card permits casting its Adventure face"
    );
}

#[test]
fn corrected_suspend_and_soulshift_keep_their_keyword_surfaces() {
    for compiler in [false, true] {
        let suspend = ironsmith::compiled_text::compiled_text_lines(&suspend_def(compiler));
        assert!(
            suspend.iter().any(|line| line.contains("Suspend 3")),
            "{suspend:?}"
        );
        assert!(
            !suspend
                .iter()
                .any(|line| line.contains("last time counter")),
            "suspend implementation triggers should be bundled: {suspend:?}"
        );
        assert_eq!(
            ironsmith::compiled_text::ability_surface_text(&soulshift_def(compiler).abilities[0]),
            "Soulshift 3"
        );
    }
}
