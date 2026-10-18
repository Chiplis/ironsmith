//! Claude package 50 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::target::ChooseSpec;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

fn p50_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

/// CR 601.2b/601.2f: Explosive Singularity's optional additional cost taps
/// any number of untapped creatures you control, and each creature tapped
/// this way reduces the total cost by {1} before mana is paid — the same
/// cost-payment reduction as "for each creature sacrificed this way".
#[test]
fn explosive_singularity_tapped_creatures_reduce_the_actual_payment() {
    use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::decisions::context::{BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm};
    use ironsmith::mana::ManaSymbol;
    struct Tapper { wanted: usize, mine: Vec<ironsmith::ObjectId> }
    impl DecisionMaker for Tapper {
        fn answers_player_choices(&self) -> bool { true }
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &BooleanContext) -> bool { self.wanted > 0 }
        fn decide_objects(&mut self, game: &ironsmith::GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            let chosen: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal && self.mine.contains(&candidate.id))
                .take(self.wanted).map(|candidate| candidate.id).collect();
            if chosen.len() >= ctx.min { chosen } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
        }
        fn decide_options(&mut self, game: &ironsmith::GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
        fn decide_targets(&mut self, game: &ironsmith::GameState, ctx: &TargetsContext) -> Vec<ironsmith::game_state::Target> {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    let definition = p50_compile("Explosive Singularity",
        "Mana cost: {8}{R}{R}\nType: Sorcery\nAs an additional cost to cast this spell, you may tap any number of untapped creatures you control. This spell costs {1} less to cast for each creature tapped this way.\nExplosive Singularity deals 10 damage to any target.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Tapped"), "the reduction reads the tap payment: {debug}");
    let alice = PlayerId::from_index(0);
    let creature = CardBuilder::new(CardId::new(), "Tapped creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
    for wanted in [0usize, 1, 3] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 2);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 20);
        let mine: Vec<_> = (0..4).map(|_| game.create_object_from_card(&creature, alice, Zone::Battlefield)).collect();
        // An already tapped creature can't pay the tap cost.
        let tapped = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.tap(tapped);
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let action = compute_legal_actions(&game, alice).unwrap().into_iter()
            .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
            .expect("Explosive Singularity is castable");
        let before = game.player(alice).unwrap().mana_pool.total();
        let mut dm = Tapper { wanted, mine: mine.clone() };
        let mut state = PriorityLoopState::new(game.players.len());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..40 {
            if !game.stack.is_empty() { break; }
            progress = match progress {
                GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref ctx))
                    if ctx.description.starts_with("Choose optional costs") =>
                {
                    let chosen = if wanted == 0 { vec![] } else { vec![(0, 1)] };
                    apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                        &PriorityResponse::OptionalCosts(chosen), &mut dm).unwrap()
                }
                GameProgress::NeedsDecisionCtx(ref ctx) if !matches!(ctx, DecisionContext::Priority(_)) => {
                    apply_decision_context_with_dm(&mut game, &mut queue, &mut state, ctx, &mut dm).unwrap()
                }
                other => panic!("wanted={wanted}: the cast did not reach the stack: {other:?}"),
            };
        }
        assert!(!game.stack.is_empty(), "wanted={wanted}: the spell is on the stack");
        let paid = before - game.player(alice).unwrap().mana_pool.total();
        assert_eq!(paid as usize, 10 - wanted, "wanted={wanted}: each creature tapped this way pays {{1}}");
        assert_eq!(mine.iter().filter(|id| game.is_tapped(**id)).count(), wanted);
    }
}

/// False Orders: the removed blocker's sole-blocked attackers become
/// unblocked (CR 506.4 / 509.1h), and the caster may then have it block an
/// attacking creature of their choice that is attacking its controller.
#[test]
fn false_orders_moves_a_blocker_onto_a_chosen_attacker() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState, is_blocked};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    struct Pick { attacker: Option<ironsmith::ObjectId>, prompts: usize }
    impl ironsmith::decision::DecisionMaker for Pick {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.attacker.is_some()
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.prompts += 1;
            self.attacker.filter(|id| ctx.candidates.iter().any(|candidate| candidate.id == *id)).into_iter().collect()
        }
    }
    let definition = p50_compile("False Orders",
        "Mana cost: {R}\nType: Instant\nCast this spell only during the declare blockers step.\nRemove target creature defending player controls from combat. Creatures it was blocking that had become blocked by only that creature this combat become unblocked. You may have it block an attacking creature of your choice.");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("UnblockSoleBlockedAttackersEffect") && debug.contains("BlockAttackerOfChoiceEffect"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = |name: &str| CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    for choose in [true, false] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        let first = game.create_object_from_card(&creature("Blocked attacker"), alice, Zone::Battlefield);
        let second = game.create_object_from_card(&creature("Unblocked attacker"), alice, Zone::Battlefield);
        let blocker = game.create_object_from_card(&creature("Bob's blocker"), bob, Zone::Battlefield);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo { creature: first, target: AttackTarget::Player(bob) });
        combat.attackers.push(AttackerInfo { creature: second, target: AttackTarget::Player(bob) });
        combat.blockers.insert(first, vec![blocker]);
        combat.blocked_attackers.insert(first);
        combat.block_declaration_complete = true;
        game.combat = Some(combat);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::CreatureBlockedEvent::new(blocker, first), ironsmith::ProvNodeId::default());
        game.turn_store.turn_history.record_event(&event, None, None);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut dm = Pick { attacker: choose.then_some(second), prompts: 0 };
        let mut ctx = EffectContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(blocker)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert!(dm.prompts >= 1, "the caster is offered an attacker");
        let combat = game.combat.as_ref().unwrap();
        assert!(!is_blocked(combat, first), "its only blocker left combat, so it is unblocked");
        assert!(!combat.blockers.get(&first).is_some_and(|blockers| blockers.contains(&blocker)));
        assert_eq!(is_blocked(combat, second), choose, "choose={choose}");
        assert_eq!(combat.blockers.get(&second).is_some_and(|blockers| blockers.contains(&blocker)), choose);
    }
}

fn p50_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p50_visit(child, f));
}

/// Eye of the Storm: the trigger reads "casts an instant or sorcery card"
/// (a card, never a copy, CR 707.12), exiles the spell, then the caster may
/// cast a free copy of every instant or sorcery card in the enchantment's
/// exile pile.
#[test]
fn eye_of_the_storm_copies_each_card_in_its_exile_pile_for_free_casts() {
    let definition = p50_compile("Eye of the Storm",
        "Mana cost: {5}{U}{U}\nType: Enchantment\nWhenever a player casts an instant or sorcery card, exile it. Then that player copies each instant or sorcery card exiled with this enchantment. For each copy, the player may cast the copy without paying its mana cost.");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("cast trigger");
    let trigger_debug = format!("{:?}", trigger.trigger);
    assert!(trigger_debug.contains("physical_card: true"), "only cards trigger it: {trigger_debug}");
    assert!(trigger_debug.contains("Instant") && trigger_debug.contains("Sorcery"), "{trigger_debug}");
    let mut exiles = 0;
    let mut pile_loops = 0;
    let mut free_copy_casts = 0;
    for effect in trigger.effects.all_effects() {
        p50_visit(effect, &mut |effect| {
            let debug = format!("{effect:?}");
            if effect.downcast_ref::<ironsmith::effects::ExileEffect>().is_some()
                || (effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>().is_some() && debug.contains("Exile"))
            {
                exiles += 1;
            }
            if let Some(for_each) = effect.downcast_ref::<ironsmith::effects::ForEachObject>() {
                let filter = format!("{:?}", for_each.filter);
                assert!(filter.contains("zone: Some(Exile)"), "{filter}");
                assert!(filter.contains("Instant") && filter.contains("Sorcery"), "{filter}");
                pile_loops += 1;
            }
            if let Some(cast) = effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>() {
                assert!(cast.as_copy && cast.without_paying_mana_cost, "{cast:?}");
                free_copy_casts += 1;
            }
        });
    }
    assert!(exiles >= 1, "the cast spell is exiled: {:?}", trigger.effects);
    assert_eq!(pile_loops, 1, "{:?}", trigger.effects);
    assert_eq!(free_copy_casts, 1, "{:?}", trigger.effects);
}

/// Finale of Promise: two independent "up to one target" graveyard slots,
/// each cast for free with its own exile-instead replacement, and only after
/// both casts (both spells on the stack) is each of those spells copied
/// twice when X is 10 or more.
#[test]
fn finale_of_promise_casts_both_slots_before_copying_each_spell() {
    let definition = p50_compile("Finale of Promise",
        "Mana cost: {X}{R}{R}\nType: Sorcery\nYou may cast up to one target instant card and/or up to one target sorcery card from your graveyard each with mana value X or less without paying their mana costs. If a spell cast this way would be put into your graveyard, exile it instead. If X is 10 or more, copy each of those spells twice. You may choose new targets for the copies.");
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let top: Vec<&Effect> = program.all_effects();
    let mut targets = Vec::new();
    let mut casts = Vec::new();
    let mut replacements = 0;
    let mut copies = Vec::new();
    let mut first_copy_at = None;
    let mut last_cast_at = None;
    for (position, effect) in top.iter().enumerate() {
        p50_visit(effect, &mut |effect| {
            if let Some(target) = effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>() {
                targets.push(format!("{:?}", target.target));
            }
            if let Some(cast) = effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>() {
                assert!(cast.without_paying_mana_cost && !cast.as_copy, "{cast:?}");
                casts.push(cast.tag.clone());
                last_cast_at = Some(position);
            }
            if effect.downcast_ref::<ironsmith::effects::RegisterFutureZoneReplacementEffect>().is_some() {
                replacements += 1;
            }
            if let Some(copy) = effect.downcast_ref::<ironsmith::effects::CopySpellEffect>() {
                copies.push(format!("{copy:?}"));
                first_copy_at.get_or_insert(position);
            }
        });
    }
    assert_eq!(targets.len(), 2, "{targets:?}");
    assert!(targets[0].contains("Instant") && targets[1].contains("Sorcery"), "{targets:?}");
    assert!(targets.iter().all(|target| target.contains("Graveyard")), "{targets:?}");
    assert_eq!(casts.len(), 2);
    assert_ne!(casts[0], casts[1], "each slot casts its own card");
    assert_eq!(replacements, 2, "each spell cast this way is exiled instead");
    assert_eq!(copies.len(), 2, "{copies:?}");
    assert!(copies.iter().all(|copy| copy.contains("Fixed(2)")), "{copies:?}");
    assert!(last_cast_at < first_copy_at, "both spells are cast before either is copied");
    let debug = format!("{:?}", program);
    assert!(debug.contains("XValueAtLeast(10)"), "{debug}");
}

/// Fblthp, Lost on the Range: the library's top card has plot with a cost
/// equal to its mana cost, and its controller may take the plot special
/// action for a nonland top card (CR 702.170a) — never for a land, and never
/// without Fblthp.
#[test]
fn fblthp_lost_on_the_range_plots_the_top_card_of_the_library() {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::special_actions::SpecialAction;
    let definition = p50_compile("Fblthp, Lost on the Range",
        "Mana cost: {1}{U}{U}\nType: Legendary Creature — Homunculus\nPower/Toughness: 1/1\nWard {2}\nYou may look at the top card of your library any time.\nThe top card of your library has plot. The plot cost is equal to its mana cost.\nYou may plot nonland cards from the top of your library.");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::MayPlotNonlandCardsFromTopOfLibrary)),
        "{:?}", definition.abilities);
    assert!(format!("{:?}", definition.abilities).contains("PlotFromCardManaCost"), "{:?}", definition.abilities);
    let alice = PlayerId::from_index(0);
    let spell = CardBuilder::new(CardId::new(), "Library sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::new().add_generic(2))
        .build();
    let land = CardBuilder::new(CardId::new(), "Library land").card_types(vec![CardType::Land]).build();
    let setup = |with_fblthp: bool, top_is_land: bool| {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.create_object_from_card(&spell, alice, Zone::Library);
        let top = game.create_object_from_card(if top_is_land { &land } else { &spell }, alice, Zone::Library);
        if with_fblthp {
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        }
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        game.refresh_continuous_state().unwrap();
        (game, top)
    };
    let plot_offered = |game: &ironsmith::GameState, top| compute_legal_actions(game, alice).unwrap()
        .contains(&LegalAction::SpecialAction(SpecialAction::Plot { card_id: top }));
    let (game, top) = setup(true, false);
    assert_eq!(game.player(alice).unwrap().library.last(), Some(&top));
    assert!(plot_offered(&game, top), "the nonland top card can be plotted");
    let (game, top) = setup(true, true);
    assert!(!plot_offered(&game, top), "lands are never plotted");
    let (game, top) = setup(false, false);
    assert!(!plot_offered(&game, top), "without the permission the library card can't be plotted");

    // Plotting pays the card's mana cost and exiles it plotted.
    let (mut game, top) = setup(true, false);
    let stable = game.object(top).unwrap().stable_id;
    let mut dm = SelectFirstDecisionMaker;
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::SpecialAction(SpecialAction::Plot { card_id: top })), &mut dm).unwrap();
    for _ in 0..20 {
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = &progress else { break };
        if matches!(context, ironsmith::decisions::context::DecisionContext::Priority(_)) { break; }
        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, context, &mut dm).unwrap();
    }
    let plotted = game.find_object_by_stable_id(stable).expect("the plotted card");
    assert_eq!(game.object(plotted).unwrap().zone, Zone::Exile);
    assert!(game.is_plotted_by(plotted, alice));
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0, "the plot cost is its mana cost");
}

/// Ferris Wheel: the visited target phases out and stays phased out through
/// untap steps until its controller's visit roll is 3 or less (CR 701.52a),
/// and the Attraction remembers what it phased out.
#[test]
fn ferris_wheel_holds_its_target_phased_out_until_a_low_visit_roll() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    let definition = p50_compile("Ferris Wheel",
        "Type: Artifact — Attraction\nVisit — Choose target creature that hasn't been phased out with this Attraction. That creature phases out until you roll a 3 or less while rolling to visit your Attractions.");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let wheel = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let trigger = game.object(wheel).unwrap().abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(trigger) => Some(trigger.clone()),
        _ => None,
    }).expect("materialized visit trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("UntilVisitRollAtMost(3)"), "{debug}");
    assert!(debug.contains("not_phased_out_with_source: true"), "{debug}");
    let creature = CardBuilder::new(CardId::new(), "Rider")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let rider = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(wheel, alice).with_targets(vec![ResolvedTarget::Object(rider)]);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert!(game.is_phased_out(rider));
    assert!(game.was_phased_out_with_source(wheel, rider), "the Attraction remembers it");
    // Its controller's untap step doesn't phase it in.
    game.turn.active_player = bob;
    ironsmith::turn::execute_untap_step(&mut game);
    assert!(game.is_phased_out(rider), "held through the untap step");
    game.release_visit_roll_phase_out_holds(alice, 4);
    assert!(game.is_phased_out(rider), "a 4 isn't low enough");
    game.release_visit_roll_phase_out_holds(bob, 1);
    assert!(game.is_phased_out(rider), "only the effect's controller's visit roll counts");
    game.release_visit_roll_phase_out_holds(alice, 3);
    assert!(!game.is_phased_out(rider), "a 3 or less phases it in");
    assert!(game.was_phased_out_with_source(wheel, rider), "the history persists after phasing in");
}

/// Firestorm Phoenix: dying returns it to its owner's hand instead, where it
/// stays revealed and can't be played until that player's next turn begins.
#[test]
fn firestorm_phoenix_returns_revealed_and_unplayable_until_its_owners_next_turn() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    let definition = p50_compile("Firestorm Phoenix",
        "Mana cost: {4}{R}{R}\nType: Creature — Phoenix\nPower/Toughness: 3/2\nFlying\nIf this creature would die, return it to its owner's hand instead. Until that player's next turn, that player plays with that card revealed in their hand and can't play it.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("EventReplacementWithEffects"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let phoenix = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let stable = game.object(phoenix).unwrap().stable_id;
    game.push_to_stack(ironsmith::game_state::StackEntry::ability(phoenix, bob,
        vec![Effect::destroy(ChooseSpec::SpecificObject(phoenix))]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let card = game.find_object_by_stable_id(stable).expect("the Phoenix card");
    assert_eq!(game.object(card).unwrap().zone, Zone::Hand, "it returns to its owner's hand");
    assert!(game.is_hand_card_revealed_until_owners_next_turn(card));
    assert!(game.hand_card_cant_be_played_until_owners_next_turn(card));
    // Alice can't cast it even with the mana during her opponent's turn's
    // end, nor in her own main phase until her next turn has begun.
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 6);
    let castable = |game: &ironsmith::GameState| compute_legal_actions(game, alice).unwrap().iter()
        .any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card));
    assert!(!castable(&game), "that player can't play it");
    // Bob's turn begins: nothing changes for Alice's card.
    game.turn.active_player = alice;
    game.next_turn();
    assert_eq!(game.turn.active_player, bob);
    assert!(game.hand_card_cant_be_played_until_owners_next_turn(card));
    // Alice's next turn begins: the reveal and the restriction end.
    game.next_turn();
    assert_eq!(game.turn.active_player, alice);
    assert!(!game.is_hand_card_revealed_until_owners_next_turn(card));
    assert!(!game.hand_card_cant_be_played_until_owners_next_turn(card));
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 6);
    assert!(castable(&game), "after that player's next turn begins it can be cast again");
}
