//! Native witnesses for independent MSEM damage consequences and graveyard costs.
//! Parent runs these tests; this source does not claim execution evidence.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{DealDamageEffect, EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Independent operation", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn body(extra: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition("Scenario creature", &format!("Type: Creature\nPower/Toughness: 2/8\n{extra}"), false).unwrap()
}
fn damage(game: &mut GameState, source: ObjectId, recipient: ObjectId, amount: i32) {
    execute_effect(game, &Effect::new(DealDamageEffect::new(amount, ChooseSpec::SpecificObject(recipient))), &mut EffectContext::new_default(source, A)).unwrap();
}
#[test]
fn damaged_creature_controller_loses_life_synchronously_once_even_with_wither() {
    const RIDER: &str = "Damage dealt to this creature also causes its controller to lose that much life.";
    for copies in [1, 2] {
        let text = format!("Type: Creature\nPower/Toughness: 2/8\n{}", vec![RIDER; copies].join("\n"));
        for definition in definitions(&text) {
            for wither in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let source = game.create_object_from_definition(&body(if wither {"Wither"} else {""}), A, Zone::Battlefield);
                let recipient = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.set_current_controller(recipient, B).unwrap();
                damage(&mut game, source, recipient, 0);
                assert_eq!(game.player(B).unwrap().life, 20);
                damage(&mut game, source, recipient, 3);
                assert_eq!(game.player(B).unwrap().life, 17, "current controller, redundant instances and no trigger resolution");
                assert_eq!(game.player(A).unwrap().life, 20);
                assert!(game.stack.is_empty(), "life loss is a damage result");
                assert_eq!(game.damage_on(recipient), if wither {0} else {3});
                assert_eq!(game.counter_count(recipient, ironsmith::CounterType::MinusOneMinusOne), if wither {3} else {0});
                let ordinary = game.create_object_from_definition(&body(""), B, Zone::Battlefield);
                damage(&mut game, source, ordinary, 2);
                assert_eq!(game.player(B).unwrap().life, 17, "ordinary damaged creatures have no rider");
            }
        }
    }
}
#[test]
fn prevented_damage_does_not_cause_recipient_controller_life_loss() {
    for definition in definitions("Type: Creature\nPower/Toughness: 2/8\nDamage dealt to this creature also causes its controller to lose that much life.\nPrevent all damage that would be dealt to this creature.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&body(""), A, Zone::Battlefield);
        let recipient = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        damage(&mut game, source, recipient, 3);
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(game.damage_on(recipient), 0);
    }
}
fn activation(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility {source: id, ..} if *id == source))
}
#[test]
fn kindle_uses_full_life_and_mana_cost_with_graveyard_and_sorcery_restrictions() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    for definition in definitions("Type: Instant\nYou gain 1 life.\n{B}{R}, Pay 3 life, Exile this card from your graveyard: Create 3 1/1 colorless Elemental creature tokens. Activate only as a sorcery.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A; game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        let wrong_zone = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(activation(&game, wrong_zone).is_none());
        assert!(activation(&game, source).is_none());
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 1);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        assert!(activation(&game, source).is_none());
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let action = activation(&game, source).unwrap();
        let mut dm = SelectFirstDecisionMaker;
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {break;}
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.player(A).unwrap().life, 17);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
        assert!(game.battlefield.is_empty(), "tokens wait for the ability to resolve");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.battlefield.len(), 3);
        for id in &game.battlefield {
            let token = game.object(*id).unwrap();
            assert_eq!(token.kind, ironsmith::object::ObjectKind::Token);
            assert!(token.subtypes.contains(&ironsmith::Subtype::Elemental));
            assert_eq!(game.calculated_power(*id), Some(1));
            assert_eq!(game.calculated_toughness(*id), Some(1));
        }
    }
}
#[test]
fn graveyard_revive_grant_filters_mana_value_and_leaves_with_grantor() {
    let reminder = "{2}{W/B}, Exile this card from your graveyard: Create a token that's a copy of this card, except it has no mana cost. Activate only as a sorcery.";
    let text = format!("Type: Creature — Human Knight\nPower/Toughness: 3/3\nLifelink\nCreature and Aura cards with mana value 3 or less in your graveyard have custom keyword \"revive\", and \"{reminder}\".");
    for grant in definitions(&text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A; game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
        let host = game.create_object_from_definition(&grant, A, Zone::Battlefield);
        let cheap = ironsmith_compiler_runtime::compile_to_runtime_definition("Cheap creature", "Mana cost: {2}{U}\nType: Creature\nPower/Toughness: 2/2", false).unwrap();
        let expensive = ironsmith_compiler_runtime::compile_to_runtime_definition("Expensive creature", "Mana cost: {3}{U}\nType: Creature\nPower/Toughness: 2/2", false).unwrap();
        let instant = ironsmith_compiler_runtime::compile_to_runtime_definition("Instant", "Mana cost: {U}\nType: Instant\nYou gain 1 life.", false).unwrap();
        let eligible = game.create_object_from_definition(&cheap, A, Zone::Graveyard);
        let wrong_controller = game.create_object_from_definition(&cheap, B, Zone::Graveyard);
        let wrong_cost = game.create_object_from_definition(&expensive, A, Zone::Graveyard);
        let wrong_type = game.create_object_from_definition(&instant, A, Zone::Graveyard);
        let wrong_zone = game.create_object_from_definition(&cheap, A, Zone::Hand);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
        assert!(activation(&game, eligible).is_some());
        for id in [wrong_controller, wrong_cost, wrong_type, wrong_zone] {
            assert!(activation(&game, id).is_none());
        }
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        assert!(activation(&game, eligible).is_none());
    }
}
fn settle(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
    while !game.stack.is_empty() {ironsmith::game_loop::resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();}
}
#[test]
fn shimmer_creates_a_full_tapped_attacking_copy_and_exiles_only_it_at_next_end_step() {
    const PROGRAM: &str = "Whenever this creature attacks, create a tapped and attacking token that's a copy of this creature. Exile that token at the beginning of the next end step.";
    for granted in [false, true] {
        let text = if granted {format!("Type: Enchantment\nCreatures you control have custom keyword \"shimmer\", and \"{PROGRAM}\".")}
            else {format!("Mana cost: {{2}}{{U}}\nType: Creature — Bird\nPower/Toughness: 2/3\nFlying\n{PROGRAM}")};
        for definition in definitions(&text) {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let grantor = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Attack witness", "Mana cost: {2}{U}\nType: Creature — Bird\nPower/Toughness: 2/3\nFlying", false).unwrap();
            let source = if granted {game.create_object_from_definition(&body, A, Zone::Battlefield)} else {grantor};
            let independent = game.create_object_from_definition(&body, A, Zone::Battlefield);
            let before = game.battlefield.clone();
            game.turn.phase = ironsmith::game_state::Phase::Combat;
            let mut combat = ironsmith::combat_state::CombatState::default();
            combat.attackers.push(ironsmith::combat_state::AttackerInfo {creature: source, target: ironsmith::combat_state::AttackTarget::Player(B)});
            game.combat = Some(combat);
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(source, ironsmith::events::AttackEventTarget::Player(B)), Default::default()));
            settle(&mut game);
            let token = *game.battlefield.iter().find(|id| !before.contains(id)).unwrap_or_else(||panic!("attacking copy; granted={granted}; abilities={:?}; spell_effect={:?}",definition.abilities,definition.spell_effect));
            assert_eq!(game.battlefield.len(), before.len() + 1, "entering attacking does not trigger another attack");
            assert_eq!(game.calculated_power(token), Some(2));
            assert_eq!(game.calculated_toughness(token), Some(3));
            assert_eq!(game.object(token).unwrap().mana_cost, game.object(source).unwrap().mana_cost);
            assert!(game.current_has_static_ability_id(token, ironsmith_core::StaticAbilityId::Flying));
            assert!(game.is_tapped(token));
            assert!(ironsmith::combat_state::is_attacking(game.combat.as_ref().unwrap(), token));
            game.combat = None;
            assert_eq!(game.object(token).unwrap().zone, Zone::Battlefield, "end of combat does not exile it");
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(B), Default::default()));
            settle(&mut game);
            assert!(game.object(token).is_none_or(|object| object.zone != Zone::Battlefield));
            for id in [source, independent, grantor] {assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);}
        }
    }
}
#[test]
fn engage_marks_only_its_creature_as_requiring_a_block() {
    for definition in definitions("Type: Creature — Human Soldier\nPower/Toughness: 0/1\nDouble strike\nThis creature must be blocked each combat if able.\n{1}{R}: This creature gets +1/+0 until end of turn.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let engaged = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ordinary = game.create_object_from_definition(&body(""), A, Zone::Battlefield);
        game.turn.active_player=A;
        game.turn.phase=ironsmith::game_state::Phase::Combat;
        game.turn.step=Some(ironsmith::game_state::Step::DeclareBlockers);
        let blocker=game.create_object_from_definition(&body(""),B,Zone::Battlefield);
        game.remove_summoning_sickness(blocker);
        game.refresh_continuous_state().unwrap();
        assert!(game.must_be_blocked(engaged), "abilities={:?};spell_effect={:?}",definition.abilities,definition.spell_effect);
        let mut combat=ironsmith::combat_state::CombatState::default();
        combat.attackers.push(ironsmith::combat_state::AttackerInfo {creature:engaged,target:ironsmith::combat_state::AttackTarget::Player(B)});
        game.combat=Some(combat.clone());
        assert!(ironsmith::combat_state::declare_blockers(&game,&mut combat.clone(),Vec::new()).is_err(), "an available blocker must block the engaged attacker");
        assert!(ironsmith::combat_state::declare_blockers(&game,&mut combat,vec![(blocker,engaged)]).is_ok());
        assert!(!game.must_be_blocked(ordinary));
        assert!(game.current_has_static_ability_id(engaged, ironsmith_core::StaticAbilityId::DoubleStrike));
    }
}
fn alternate_cast(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell {spell_id, casting_method: ironsmith::alternative_cast::CastingMethod::Alternative(_), ..} if *spell_id == source))
}
#[test]
fn selected_land_play_price_uses_allowance_on_success_without_restricting_instant_timing() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let text = "Mana cost: {2}{G}\nType: Instant\nDuring your turn, you may pay {G} rather than pay this spell's mana cost. Casting this spell this way counts as playing a land.\nYou gain 2 life.";
    for definition in definitions(text) {
        assert!(definition.alternative_casts.iter().any(|method| method.uses_land_play()));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A; game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        let watcher = ironsmith_compiler_runtime::compile_to_runtime_definition("Land observer", "Type: Enchantment\nWhenever you play a land, you gain 1 life.", false).unwrap();
        game.create_object_from_definition(&watcher, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let next = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(alternate_cast(&game, source).is_none(), "price requires its mana");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 4);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 6);
        game.turn.active_player = B;
        assert!(alternate_cast(&game, source).is_none(), "price is own-turn only");
        game.turn.active_player = A;
        let action = alternate_cast(&game, source).expect("instant price works during combat");
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {break;}
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 0, "no actual land was played");
        assert_eq!(game.player(A).unwrap().nonland_land_play_usage_this_turn, 1);
        assert!(alternate_cast(&game, next).is_none(), "land allowance is consumed before resolution");
        assert!(compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action,
            LegalAction::CastSpell {spell_id, casting_method: ironsmith::alternative_cast::CastingMethod::Normal, ..} if *spell_id == next)), "ordinary price remains available");
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        while !game.stack.is_empty() {ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();}
        assert_eq!(game.player(A).unwrap().life, 22, "MSEM702.215b spends only the allowance; no land-play observer fires");
        let allowance=ironsmith_compiler_runtime::compile_to_runtime_definition("Additional land permission","Type: Enchantment\nYou may play an additional land on each of your turns.",false).unwrap();
        game.create_object_from_definition(&allowance,A,Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.player(A).unwrap().land_plays_per_turn,2);
        assert_eq!(game.player(A).unwrap().nonland_land_play_usage_this_turn,1);
        assert!(alternate_cast(&game, next).is_some(), "a real additional-land permission enables another price use");
    }
}

#[test]
fn land_allowance_price_has_own_turn_instant_timing_and_resets_without_land_history() {
    for definition in definitions("Mana cost: {2}{G}\nType: Sorcery\nDuring your turn, you may pay {G} rather than pay this spell's mana cost. Casting this spell this way counts as playing a land.\nYou gain 2 life.") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.turn.active_player=A;game.turn.priority_player=Some(A);
        game.turn.phase=ironsmith::game_state::Phase::Beginning;game.turn.step=Some(ironsmith::game_state::Step::Upkeep);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green,1);
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);
        assert!(alternate_cast(&game,source).is_some(),"MSEM702.215c permits own upkeep");
        game.player_mut(A).unwrap().consume_land_play_for_cast();
        assert!(alternate_cast(&game,source).is_none());
        assert_eq!(game.player(A).unwrap().lands_played_this_turn,0);
        game.player_mut(A).unwrap().begin_turn();
        assert!(alternate_cast(&game,source).is_some());
    }
}

#[test]
fn showcase_bard_token_copy_casts_the_card_its_spell_exiled() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let text = "Mana cost: {1}{U}\nType: Instant\nDraw a card.\nYou may pay {3}{U} rather than pay this spell's mana cost. If its showcase cost was paid, exile this card, then create a 1/1 red Bard creature token with \"This creature can't block\" and \"Whenever this creature attacks, you may copy the exiled card and cast the copy without paying its mana cost.\"";
    for definition in definitions(text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A; game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let filler = body("");
        for _ in 0..4 { game.create_object_from_definition(&filler, A, Zone::Library); }
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 4);
        let action = alternate_cast(&game, source).expect("the showcase price is castable");
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "a paid showcase cost exiles the resolving card");
        let bard = game.battlefield.iter().copied().find(|id| game.object(*id).is_some_and(|object|
            object.kind == ironsmith::object::ObjectKind::Token && object.name.as_str().contains("Bard"))).expect("the Bard token");
        let hand_before = game.player(A).unwrap().hand.len();
        // CR 607.1: the Bard's attack trigger names the card its creating spell
        // exiled, so it copy-casts that card when it resolves.
        let trigger = game.object(bard).unwrap().abilities_vec().into_iter().find_map(|ability| match ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger), _ => None,
        }).expect("the attack trigger");
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(bard, A, trigger.effects.clone()));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.iter().any(|entry| game.object(entry.object_id).is_some_and(|object|
            object.kind == ironsmith::object::ObjectKind::SpellCopy && object.name == game.object(exiled).unwrap().name)),
            "the copy of the exiled card is cast");
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before + 1, "the cast copy resolves and draws a card");
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "only a copy is cast; the card stays exiled");
    }
}
