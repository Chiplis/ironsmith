#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

pub(super) fn resolve_irresistible_prey_targeting_attacker(
    blocker_tapped: bool,
) -> (
    crate::game_state::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
) {
    let prey = parse_oracle_card_definition("Irresistible Prey");
    let attacker_def = CardDefinitionBuilder::new(CardId::new(), "Irresistible Prey Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let blocker_def = CardDefinitionBuilder::new(CardId::new(), "Irresistible Prey Blocker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let draw_card = CardDefinitionBuilder::new(CardId::new(), "Irresistible Prey Draw Card")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&prey, alice, Zone::Stack);
    let attacker = game.create_object_from_definition(&attacker_def, alice, Zone::Battlefield);
    let blocker = game.create_object_from_definition(&blocker_def, bob, Zone::Battlefield);
    game.create_object_from_definition(&draw_card, alice, Zone::Library);
    let hand_size_before = game.objects_in_zone(Zone::Hand).len();
    game.remove_summoning_sickness(attacker);
    if blocker_tapped {
        game.tap(blocker);
    }

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(attacker)]);
    ctx.snapshot_targets(&game);
    for effect in prey
        .spell_effect
        .as_ref()
        .expect("Irresistible Prey should compile to spell effects")
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Irresistible Prey spell effect should resolve");
    }

    assert!(
        game.must_be_blocked(attacker),
        "Irresistible Prey should mark the targeted creature as must-be-blocked"
    );
    assert_eq!(
        game.objects_in_zone(Zone::Hand).len(),
        hand_size_before + 1,
        "Irresistible Prey should draw a card as its second spell effect"
    );

    (game, alice, bob, attacker, blocker)
}

#[test]
pub(super) fn irresistible_prey_runtime_requirement_survives_ability_removal() {
    let (mut game, alice, bob, attacker, _blocker) =
        resolve_irresistible_prey_targeting_attacker(false);

    let remove_abilities = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Object(crate::target::ObjectFilter::specific(attacker)),
        crate::continuous::Modification::RemoveAllAbilities,
        crate::effect::Until::EndOfTurn,
    ));
    let mut removal_ctx = crate::effects::ExecutionContext::new_default(attacker, alice);
    crate::effects::execute_effect(&mut game, &remove_abilities, &mut removal_ctx)
        .expect("remove all abilities effect should resolve");

    assert!(
        game.must_be_blocked(attacker),
        "Irresistible Prey's direct restriction should survive effects that remove abilities"
    );

    let mut combat = crate::combat_state::CombatState::default();
    crate::combat_state::declare_attackers(
        &mut game,
        &mut combat,
        vec![(attacker, crate::combat_state::AttackTarget::Player(bob))],
    )
    .expect("Irresistible Prey target should still be able to attack after ability removal");

    let missing_block = crate::combat_state::declare_blockers(&mut game, &mut combat, vec![]);
    assert!(
        matches!(
            missing_block,
            Err(crate::combat_state::CombatError::NotEnoughBlockers {
                attacker: blocked_attacker,
                required: 1,
                provided: 0,
            }) if blocked_attacker == attacker
        ),
        "Irresistible Prey should still require a block after ability removal, got {missing_block:?}"
    );
}

#[test]
pub(super) fn irresistible_prey_runtime_requires_available_blocker_and_honors_if_able() {
    let (mut game, _alice, bob, attacker, blocker) =
        resolve_irresistible_prey_targeting_attacker(false);
    let mut combat = crate::combat_state::CombatState::default();
    crate::combat_state::declare_attackers(
        &mut game,
        &mut combat,
        vec![(attacker, crate::combat_state::AttackTarget::Player(bob))],
    )
    .expect("Irresistible Prey target should be able to attack");

    let missing_block =
        crate::combat_state::declare_blockers(&mut game, &mut combat.clone(), vec![]);
    assert!(
        matches!(
            missing_block,
            Err(crate::combat_state::CombatError::NotEnoughBlockers {
                attacker: blocked_attacker,
                required: 1,
                provided: 0,
            }) if blocked_attacker == attacker
        ),
        "Irresistible Prey should require a block while a blocker can block, got {missing_block:?}"
    );

    crate::combat_state::declare_blockers(&mut game, &mut combat, vec![(blocker, attacker)])
        .expect("blocking the Irresistible Prey target should satisfy the requirement");

    let (mut unable_game, _alice, unable_bob, unable_attacker, _tapped_blocker) =
        resolve_irresistible_prey_targeting_attacker(true);
    let mut unable_combat = crate::combat_state::CombatState::default();
    crate::combat_state::declare_attackers(
        &mut unable_game,
        &mut unable_combat,
        vec![(
            unable_attacker,
            crate::combat_state::AttackTarget::Player(unable_bob),
        )],
    )
    .expect("Irresistible Prey target should be able to attack in tapped-blocker branch");
    crate::combat_state::declare_blockers(&mut unable_game, &mut unable_combat, vec![])
        .expect("Irresistible Prey should not require an impossible block");
}

#[test]
pub(super) fn blood_tyrant_strict_parser_text_and_structure_regression() {
    assert_oracle_card_parses_strict("Blood Tyrant");
    let def = parse_oracle_card_definition("Blood Tyrant");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let abilities_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains(
            "At the beginning of your upkeep, each player loses 1 life. Put a +1/+1 counter on this creature for each 1 life lost this way"
        ),
        "Blood Tyrant should render the life-lost counter clause, got {rendered}"
    );
    assert!(
        rendered
            .contains("Whenever a player loses the game, put five +1/+1 counters on this creature"),
        "Blood Tyrant should render the player-loses-game trigger, got {rendered}"
    );
    assert!(
        abilities_debug.contains("EffectMetric")
            && abilities_debug.contains("LifeLost")
            && abilities_debug.contains("PlayerLosesGameTrigger"),
        "Blood Tyrant should lower to life-lost metrics and a player-loses-game trigger, got {abilities_debug}"
    );
}

#[test]
pub(super) fn blood_tyrant_upkeep_life_loss_adds_counter_for_each_life_lost() {
    let (mut game, alice, bob, tyrant_id) = blood_tyrant_game();
    game.turn.active_player = alice;
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );

    assert_eq!(resolve_triggers_for_source(&mut game, tyrant_id, &event), 1);
    assert_eq!(game.life_total(alice), 19, "controller should lose 1 life");
    assert_eq!(game.life_total(bob), 19, "opponent should lose 1 life");
    assert_eq!(
        game.counter_count(tyrant_id, CounterType::PlusOnePlusOne),
        2,
        "Blood Tyrant should get one counter for each 1 life lost this way"
    );
}

#[test]
pub(super) fn blood_tyrant_player_loses_game_trigger_adds_five_counters_from_sba() {
    let (mut game, _alice, bob, tyrant_id) = blood_tyrant_game();
    game.player_mut(bob).expect("bob exists").life = 0;

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::check_and_apply_sbas(&mut game, &mut trigger_queue)
        .expect("state-based player loss should apply");
    assert!(
        game.player(bob).expect("bob exists").has_lost,
        "Bob should lose the game at 0 life"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Blood Tyrant player-loss trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Blood Tyrant player-loss trigger should resolve");
    assert_eq!(
        game.counter_count(tyrant_id, CounterType::PlusOnePlusOne),
        5,
        "Blood Tyrant should get five counters when a player loses the game"
    );
}

#[test]
pub(super) fn blood_tyrant_does_not_treat_life_loss_as_losing_the_game() {
    let (game, _alice, bob, tyrant_id) = blood_tyrant_game();
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::life::LifeLossEvent::from_effect(bob, 3),
        crate::provenance::ProvNodeId::default(),
    );

    let triggers = crate::triggers::check_triggers(&game, &event);
    assert!(
        triggers.iter().all(|entry| entry.source != tyrant_id),
        "Blood Tyrant should trigger on losing the game, not on life-loss events"
    );
}

pub(super) fn sulfuric_vortex_game() -> (crate::game_state::GameState, PlayerId, PlayerId, ObjectId)
{
    let def = parse_oracle_card_definition("Sulfuric Vortex");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let vortex_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    (game, alice, bob, vortex_id)
}

pub(super) fn sulfuric_vortex_triggered_ability(
    def: &CardDefinition,
) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Sulfuric Vortex should have an upkeep triggered ability")
}

#[test]
pub(super) fn sulfuric_vortex_oracle_parses_strictly_and_renders_life_replacement() {
    assert_oracle_card_parses_strict("Sulfuric Vortex");
    let def = parse_oracle_card_definition("Sulfuric Vortex");
    let rendered = canonical_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(
        rendered,
        "At the beginning of each player's upkeep, this enchantment deals 2 damage to that player.\nIf a player would gain life, that player gains no life instead."
    );
    assert!(
        ability_debug.contains("BeginningOfUpkeepTrigger")
            && ability_debug.contains("RuleRestriction")
            && ability_debug.contains("GainLife"),
        "Sulfuric Vortex should lower to an upkeep trigger plus a structural gain-life restriction, got {ability_debug}"
    );
}

#[test]
pub(super) fn sulfuric_vortex_prevents_life_gain_for_each_player_while_on_battlefield() {
    let (mut game, alice, bob, vortex_id) = sulfuric_vortex_game();
    game.update_cant_effects();

    let mut alice_ctx = crate::effects::ExecutionContext::new_default(vortex_id, alice);
    let alice_outcome = GainLifeEffect::you(3)
        .execute(&mut game, &mut alice_ctx)
        .expect("Alice life gain should resolve to no life gained");
    assert_eq!(game.life_total(alice), 20, "Alice should gain no life");
    assert_eq!(alice_outcome.as_count(), Some(0));
    assert!(
        alice_outcome.events.is_empty(),
        "prevented life gain should not emit a life-gain trigger event"
    );

    let mut bob_ctx = crate::effects::ExecutionContext::new_default(vortex_id, bob);
    let bob_outcome = GainLifeEffect::you(4)
        .execute(&mut game, &mut bob_ctx)
        .expect("Bob life gain should resolve to no life gained");
    assert_eq!(game.life_total(bob), 20, "Bob should gain no life");
    assert_eq!(bob_outcome.as_count(), Some(0));
    assert!(
        bob_outcome.events.is_empty(),
        "prevented opponent life gain should not emit a life-gain trigger event"
    );
}

#[test]
pub(super) fn sulfuric_vortex_life_gain_restriction_ends_when_enchantment_leaves() {
    let (mut game, alice, _bob, vortex_id) = sulfuric_vortex_game();
    game.update_cant_effects();
    let moved_vortex_id = game
        .move_object(
            vortex_id,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
        )
        .expect("Sulfuric Vortex should move to graveyard");
    game.update_cant_effects();

    let mut ctx = crate::effects::ExecutionContext::new_default(moved_vortex_id, alice);
    let outcome = GainLifeEffect::you(3)
        .execute(&mut game, &mut ctx)
        .expect("life gain should work after Sulfuric Vortex leaves");
    assert_eq!(game.life_total(alice), 23);
    assert_eq!(outcome.as_count(), Some(3));
    assert!(
        !outcome.events.is_empty(),
        "actual life gain should emit a life-gain trigger event"
    );
}

#[test]
pub(super) fn sulfuric_vortex_upkeep_trigger_damages_that_player() {
    let def = parse_oracle_card_definition("Sulfuric Vortex");
    let triggered = sulfuric_vortex_triggered_ability(&def);
    let (mut game, alice, bob, vortex_id) = sulfuric_vortex_game();
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    let ctx = crate::triggers::TriggerContext::for_source(vortex_id, alice, &game);
    let controller_upkeep = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let opponent_upkeep = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(bob),
        crate::provenance::ProvNodeId::default(),
    );

    assert!(
        triggered.trigger.matches(&controller_upkeep, &ctx)
            && triggered.trigger.matches(&opponent_upkeep, &ctx),
        "Sulfuric Vortex should trigger at each player's upkeep"
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, vortex_id, &opponent_upkeep),
        1
    );
    assert_eq!(
        game.life_total(alice),
        20,
        "controller should not be damaged on Bob's upkeep"
    );
    assert_eq!(
        game.life_total(bob),
        18,
        "that upkeep player should be dealt 2 damage"
    );
}

fn each_player_upkeep_ability(def: &CardDefinition) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:?}", triggered.trigger).contains("BeginningOfUpkeep") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("card should have a beginning-of-upkeep triggered ability")
}

#[test]
pub(super) fn each_player_upkeep_cluster_keeps_exact_event_player_surfaces() {
    for (card, expected) in [
        (
            "Blood Clock",
            "At the beginning of each player's upkeep, that player returns a permanent they control to its owner's hand unless they pay 2 life.",
        ),
        (
            "Sunken Hope",
            "At the beginning of each player's upkeep, that player returns a creature they control to its owner's hand.",
        ),
        (
            "Dreamborn Muse",
            "At the beginning of each player's upkeep, that player mills X cards, where X is the number of cards in their hand.",
        ),
        (
            "Roiling Vortex",
            "At the beginning of each player's upkeep, this enchantment deals 1 damage to them.",
        ),
        (
            "Hokori, Dust Drinker",
            "At the beginning of each player's upkeep, that player untaps a land they control.",
        ),
        (
            "Rising Waters",
            "At the beginning of each player's upkeep, that player untaps a land they control.",
        ),
    ] {
        assert_oracle_card_parses_strict(card);
        let def = parse_oracle_card_definition(card);
        let compiled = compiled_text_lines(&def);
        assert!(
            compiled.iter().any(|line| line == expected),
            "{card} must retain the authored event-player surface: {compiled:#?}"
        );
    }
}

#[test]
pub(super) fn each_player_upkeep_cluster_lowers_bodies_to_typed_event_players() {
    for card in [
        "Blood Clock",
        "Sunken Hope",
        "Hokori, Dust Drinker",
        "Rising Waters",
    ] {
        let def = parse_oracle_card_definition(card);
        let triggered = each_player_upkeep_ability(&def);
        let body = format!("{:#?}", triggered.effects);
        assert!(
            body.contains("IteratedPlayer"),
            "{card} must bind its relative chooser/controller from the upkeep event: {body}"
        );
        assert!(
            !body.contains("Active"),
            "{card} must not substitute the game's active-player field for the event participant: {body}"
        );
    }

    let dreamborn = parse_oracle_card_definition("Dreamborn Muse");
    let dreamborn_trigger = each_player_upkeep_ability(&dreamborn);
    let mill = dreamborn_trigger
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::MillEffect>())
        .expect("Dreamborn Muse should lower to a typed mill effect");
    assert_eq!(mill.player, PlayerFilter::IteratedPlayer);
    let count_binds_upkeep_player = match mill.count.unhinted() {
        crate::effect::Value::CardsInHand(player) => player == &PlayerFilter::IteratedPlayer,
        crate::effect::Value::Count(filter) => {
            filter.zone == Some(Zone::Hand)
                && filter.owner.as_ref() == Some(&PlayerFilter::IteratedPlayer)
        }
        _ => false,
    };
    assert!(
        count_binds_upkeep_player,
        "Dreamborn Muse's hand count must be owned by the upkeep event player: {mill:#?}"
    );

    let roiling = parse_oracle_card_definition("Roiling Vortex");
    let roiling_trigger = each_player_upkeep_ability(&roiling);
    let damage = roiling_trigger
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
        .expect("Roiling Vortex should lower to a typed damage effect");
    assert_eq!(
        damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    );
}

#[test]
pub(super) fn roiling_vortex_uses_upkeep_event_player_even_when_active_player_differs() {
    let def = parse_oracle_card_definition("Roiling Vortex");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let vortex = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    // Deliberately make the game field disagree with the concrete event. The
    // trigger context must carry Bob through resolution instead of consulting
    // `turn.active_player` again.
    game.turn.active_player = alice;
    let bob_upkeep = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(bob),
        crate::provenance::ProvNodeId::default(),
    );

    assert_eq!(
        resolve_triggers_for_source(&mut game, vortex, &bob_upkeep),
        1
    );
    assert_eq!(game.life_total(alice), 20);
    assert_eq!(game.life_total(bob), 19);
}

#[test]
pub(super) fn oath_of_lieges_relative_target_and_search_remains_an_explicit_residual() {
    // This is intentionally separate from the event-player binding regression:
    // Oath also needs a reusable relative-player target + library-search shape.
    // Keep the exact assertion live so that gap cannot disappear from the
    // cluster merely because the common upkeep participant is now correct.
    assert_oracle_card_parses_strict("Oath of Lieges");
    let def = parse_oracle_card_definition("Oath of Lieges");
    let compiled = compiled_text_lines(&def);
    let expected = "At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle.";
    assert!(
        compiled.iter().any(|line| line == expected),
        "Oath of Lieges remains a separately routed relative-target/search residual: {compiled:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mistmeadow_skulk_strict_parser_text_structure_and_runtime_regression() {
    let def = parse_oracle_card_definition("Mistmeadow Skulk");
    let rendered = canonical_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Lifelink, protection from mana value 3 or greater"),
        "expected Mistmeadow Skulk to render its lifelink and mana-value protection clause, got {rendered}"
    );
    assert!(
        ability_debug.contains("Lifelink")
            && ability_debug.contains("Protection")
            && ability_debug.contains("GreaterThanOrEqual(3)"),
        "expected Mistmeadow Skulk to lower into lifelink plus mana-value protection, got {ability_debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let skulk_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    assert!(
        game.current_has_static_ability_id(skulk_id, StaticAbilityId::Lifelink),
        "Mistmeadow Skulk should have lifelink on the battlefield"
    );
    assert!(
        game.current_has_static_ability_id(skulk_id, StaticAbilityId::Protection),
        "Mistmeadow Skulk should have protection on the battlefield"
    );

    let high_mana_value_spell = CardDefinitionBuilder::new(CardId::new(), "Mana Value Three Bolt")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Instant])
        .build();
    let high_source_id =
        game.create_object_from_definition(&high_mana_value_spell, bob, Zone::Stack);
    assert!(
        crate::targeting::has_protection_from_source(&game, skulk_id, high_source_id,),
        "Mistmeadow Skulk should be protected from a source with mana value 3"
    );
    assert!(
        matches!(
            crate::targeting::can_target_object(&game, skulk_id, high_source_id, bob),
            crate::targeting::TargetingResult::Invalid(
                crate::targeting::TargetingInvalidReason::HasProtection
            )
        ),
        "Mistmeadow Skulk should be an illegal target for a source with mana value 3"
    );

    let low_mana_value_spell = CardDefinitionBuilder::new(CardId::new(), "Mana Value Two Bolt")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Instant])
        .build();
    let low_source_id = game.create_object_from_definition(&low_mana_value_spell, bob, Zone::Stack);
    assert!(
        !crate::targeting::has_protection_from_source(&game, skulk_id, low_source_id),
        "Mistmeadow Skulk should not be protected from a source with mana value 2"
    );
    assert!(
        crate::targeting::can_target_object(&game, skulk_id, low_source_id, bob).is_legal(),
        "Mistmeadow Skulk should remain targetable by a source with mana value 2"
    );
}

#[test]
pub(super) fn departed_deckhand_strict_parser_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Departed Deckhand");
    let rendered = canonical_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("This creature can't be blocked except by spirits"),
        "expected static Spirit-only blocking restriction, got {rendered}"
    );
    assert!(
        rendered.contains(
            "{3}{U}: Another target creature you control can't be blocked this turn except by Spirits"
        ),
        "expected activated Spirit-only blocking restriction, got {rendered}"
    );
    assert!(
        ability_debug.contains("BecomesTargetedBySpellTrigger")
            && ability_debug.contains("SacrificeTargetEffect"),
        "expected spell-targeted sacrifice trigger, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("RuleRestriction")
            && ability_debug.contains("BlockSpecificAttacker")
            && ability_debug.contains("excluded_subtypes: [Spirit]"),
        "expected static Spirit exception to lower structurally, got {ability_debug}"
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Departed Deckhand should have an activated ability");
    let cost_debug = format!("{:#?}", activated.mana_cost);
    assert!(
        cost_debug.contains("Generic") && cost_debug.contains("3,") && cost_debug.contains("Blue"),
        "expected {{3}}{{U}} activation cost, got {cost_debug}"
    );

    let effects = activated.effects.flattened_default_effects();
    let target_only = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<TaggedEffect>()
                .and_then(|tagged| tagged.effect.downcast_ref::<TargetOnlyEffect>())
                .or_else(|| effect.downcast_ref::<TargetOnlyEffect>())
        })
        .expect("Departed Deckhand activation should establish a target");
    let ChooseSpec::Object(target_filter) = target_only.target.base() else {
        panic!(
            "expected object target filter, got {:?}",
            target_only.target
        );
    };
    assert!(target_filter.other, "target should be another creature");
    assert_eq!(target_filter.controller, Some(PlayerFilter::You));
    assert!(target_filter.card_types.contains(&CardType::Creature));

    let cant = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::CantEffect>())
        .expect("Departed Deckhand activation should create a cant-block restriction");
    assert_eq!(cant.duration, crate::effect::Until::EndOfTurn);
    match &cant.restriction {
        crate::effect::Restriction::BlockSpecificAttacker { blockers, attacker } => {
            assert!(blockers.excluded_subtypes.contains(&Subtype::Spirit));
            assert!(attacker.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            }));
        }
        other => panic!("expected Spirit-only block-specific restriction, got {other:?}"),
    }
}

#[test]
pub(super) fn sengir_the_dark_baron_strict_parser_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Sengir, the Dark Baron");
    let rendered = canonical_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains(
            "Whenever another player loses the game, you gain life equal to that player's life total as the turn began"
        ),
        "expected player-loses-game life-total trigger text, got {rendered}"
    );
    assert!(
        rendered.contains("Whenever another creature dies, put two +1/+1 counters on Sengir"),
        "expected another-creature-dies counter trigger text, got {rendered}"
    );
    assert!(
        ability_debug.contains("PlayerLosesGameTrigger")
            && ability_debug.contains("LifeTotalAsTurnBegan"),
        "expected player-loses-game trigger to lower to life-total-as-turn-began gain life, got {ability_debug}"
    );
    let life_gain = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .flat_map(|triggered| triggered.effects.flattened_default_effects())
        .find_map(|effect| effect.downcast_ref::<crate::effects::GainLifeEffect>())
        .expect("Sengir should have a typed life-gain effect");
    assert_eq!(
        life_gain.amount,
        crate::effect::Value::LifeTotalAsTurnBegan(PlayerFilter::IteratedPlayer),
        "the life value must remain bound to the player who lost the game"
    );
}

#[test]
pub(super) fn sengir_the_dark_baron_another_creature_dies_adds_two_counters_only_for_other_creatures()
 {
    let def = parse_oracle_card_definition("Sengir, the Dark Baron");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let sengir_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let other_creature = CardDefinitionBuilder::new(CardId::new(), "Other Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let other_id = game.create_object_from_definition(&other_creature, alice, Zone::Battlefield);

    let other_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(other_id).expect("other creature exists"),
        &game,
    );
    let other_dies = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            other_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(other_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());
    let triggered = crate::triggers::check_triggers(&game, &other_dies);
    let entry = triggered
        .iter()
        .find(|entry| entry.source == sengir_id)
        .expect("Sengir should trigger when another creature dies");
    let mut ctx = crate::effects::ExecutionContext::new_default(sengir_id, alice)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Sengir's counter trigger should resolve");
    }
    assert_eq!(
        game.counter_count(sengir_id, crate::object::CounterType::PlusOnePlusOne),
        2,
        "Sengir should get two +1/+1 counters when another creature dies"
    );

    let sengir_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(sengir_id).expect("Sengir exists"),
        &game,
    );
    let self_dies = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            sengir_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(sengir_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());
    assert!(
        crate::triggers::check_triggers(&game, &self_dies)
            .into_iter()
            .all(|entry| entry.source != sengir_id),
        "Sengir should not trigger from its own death because the text says another creature"
    );
}

#[test]
pub(super) fn sengir_the_dark_baron_another_player_loses_game_gains_life_from_turn_start_total() {
    fn stage_life_loss(game: &mut crate::game_state::GameState, player: PlayerId, amount: u32) {
        game.lose_life(player, amount);
        let event = crate::events::Event::life_loss(player, amount, false).into_raw();
        game.stage_turn_history_event(&event);
    }

    let def = parse_oracle_card_definition("Sengir, the Dark Baron");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sengir_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.player_mut(alice).expect("alice exists").life = 10;

    stage_life_loss(&mut game, bob, 5);
    assert_eq!(game.player(bob).expect("bob exists").life, 15);
    stage_life_loss(&mut game, bob, 16);
    assert_eq!(game.player(bob).expect("bob exists").life, -1);
    game.add_player_counters_with_source(bob, crate::object::CounterType::Poison, 10, None, None);

    assert!(
        crate::rules::state_based::apply_state_based_actions(&mut game),
        "Bob at negative life and ten poison counters should lose the game as a state-based action"
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
    let sengir_entries = trigger_queue
        .entries
        .iter()
        .filter(|entry| entry.source == sengir_id)
        .collect::<Vec<_>>();
    assert_eq!(
        sengir_entries.len(),
        1,
        "Sengir should trigger once when a player loses the game, even with multiple simultaneous loss reasons"
    );
    let entry = sengir_entries[0];
    let mut ctx = crate::effects::ExecutionContext::new_default(sengir_id, alice)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Sengir's life-gain trigger should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        30,
        "Sengir should gain life equal to Bob's life total as the turn began, not Bob's current life total"
    );
}

#[test]
pub(super) fn sengir_the_dark_baron_explicit_lose_game_effect_triggers_life_gain() {
    let def = parse_oracle_card_definition("Sengir, the Dark Baron");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sengir_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let lose_effect = crate::effect::Effect::lose_the_game_player(PlayerFilter::Opponent);
    let mut ctx = crate::effects::ExecutionContext::new_default(sengir_id, alice);
    crate::effects::execute_effect(&mut game, &lose_effect, &mut ctx)
        .expect("explicit lose-the-game effect should resolve");
    assert!(game.player(bob).expect("bob exists").has_lost);

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
    let entry = trigger_queue
        .entries
        .iter()
        .find(|entry| entry.source == sengir_id)
        .expect("Sengir should trigger from explicit lose-the-game effects");

    let mut ctx = crate::effects::ExecutionContext::new_default(sengir_id, alice)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Sengir's explicit-loss life-gain trigger should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        40,
        "Sengir should gain life equal to the losing player's turn-start life total for explicit loss effects"
    );
}

#[test]
pub(super) fn sengir_the_dark_baron_win_game_effect_triggers_for_other_players_losing() {
    let def = parse_oracle_card_definition("Sengir, the Dark Baron");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sengir_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let win_effect = crate::effect::Effect::win_the_game();
    let mut ctx = crate::effects::ExecutionContext::new_default(sengir_id, alice);
    crate::effects::execute_effect(&mut game, &win_effect, &mut ctx)
        .expect("explicit win-the-game effect should resolve");
    assert!(game.player(bob).expect("bob exists").has_lost);

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
    let entry = trigger_queue
        .entries
        .iter()
        .find(|entry| entry.source == sengir_id)
        .expect("Sengir should trigger when another player loses because its controller wins");

    let mut ctx = crate::effects::ExecutionContext::new_default(sengir_id, alice)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Sengir's win-effect life-gain trigger should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        40,
        "Sengir should gain life equal to the losing player's turn-start life total when another player loses from a win effect"
    );
}

#[test]
pub(super) fn sengir_the_dark_baron_does_not_trigger_when_its_controller_loses_game() {
    let def = parse_oracle_card_definition("Sengir, the Dark Baron");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let sengir_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.lose_life(alice, 21);

    assert!(
        crate::rules::state_based::apply_state_based_actions(&mut game),
        "Alice at negative life should lose the game as a state-based action"
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue
            .entries
            .iter()
            .all(|entry| entry.source != sengir_id),
        "Sengir should not trigger when its controller loses the game because the text says another player"
    );
}

#[test]
pub(super) fn departed_deckhand_runtime_targets_another_creature_and_allows_only_spirit_blockers() {
    fn creature_def(name: &str, subtype: Subtype) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    let deckhand = parse_oracle_card_definition("Departed Deckhand");
    let activated = deckhand
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Departed Deckhand should have an activated ability");
    let effects = activated.effects.flattened_default_effects();
    let target_only = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<TaggedEffect>()
                .and_then(|tagged| tagged.effect.downcast_ref::<TargetOnlyEffect>())
                .or_else(|| effect.downcast_ref::<TargetOnlyEffect>())
        })
        .expect("Departed Deckhand activation should establish a target");
    let ChooseSpec::Object(target_filter) = target_only.target.base() else {
        panic!(
            "expected object target filter, got {:?}",
            target_only.target
        );
    };

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let deckhand_id = game.create_object_from_definition(&deckhand, alice, Zone::Battlefield);
    let target_id = game.create_object_from_definition(
        &creature_def("Alice's Target", Subtype::Merfolk),
        alice,
        Zone::Battlefield,
    );
    let bob_creature_id = game.create_object_from_definition(
        &creature_def("Bob's Creature", Subtype::Merfolk),
        bob,
        Zone::Battlefield,
    );
    let non_spirit_blocker_id = game.create_object_from_definition(
        &creature_def("Non-Spirit Blocker", Subtype::Pirate),
        bob,
        Zone::Battlefield,
    );
    let spirit_blocker_id = game.create_object_from_definition(
        &creature_def("Spirit Blocker", Subtype::Spirit),
        bob,
        Zone::Battlefield,
    );

    let filter_ctx = crate::filter::FilterContext::new(alice)
        .with_source(deckhand_id)
        .with_opponents(vec![bob]);
    assert!(
        target_filter.matches(
            game.object(target_id).expect("target exists"),
            &filter_ctx,
            &game,
        ),
        "another creature Alice controls should be a legal activation target"
    );
    assert!(
        !target_filter.matches(
            game.object(deckhand_id).expect("deckhand exists"),
            &filter_ctx,
            &game,
        ),
        "Departed Deckhand should not target itself because the text says another"
    );
    assert!(
        !target_filter.matches(
            game.object(bob_creature_id).expect("Bob creature exists"),
            &filter_ctx,
            &game,
        ),
        "Departed Deckhand should not target creatures controlled by another player"
    );

    game.refresh_continuous_state();
    assert!(
        !crate::rules::combat::can_block(
            game.object(deckhand_id).expect("deckhand exists"),
            game.object(non_spirit_blocker_id)
                .expect("non-Spirit blocker exists"),
            &game,
        ),
        "Departed Deckhand's static restriction should stop non-Spirit blockers"
    );
    assert!(
        crate::rules::combat::can_block(
            game.object(deckhand_id).expect("deckhand exists"),
            game.object(spirit_blocker_id)
                .expect("Spirit blocker exists"),
            &game,
        ),
        "Departed Deckhand's static restriction should still allow Spirit blockers"
    );
    assert!(
        crate::rules::combat::can_block(
            game.object(target_id).expect("target exists"),
            game.object(non_spirit_blocker_id)
                .expect("non-Spirit blocker exists"),
            &game,
        ),
        "the chosen creature should be normally blockable before the activation resolves"
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(deckhand_id, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target_id)]);
    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Departed Deckhand activation effect should resolve");
    }

    assert!(
        !crate::rules::combat::can_block(
            game.object(target_id).expect("target exists"),
            game.object(non_spirit_blocker_id)
                .expect("non-Spirit blocker exists"),
            &game,
        ),
        "the activated effect should stop non-Spirit creatures from blocking the target"
    );
    assert!(
        crate::rules::combat::can_block(
            game.object(target_id).expect("target exists"),
            game.object(spirit_blocker_id)
                .expect("Spirit blocker exists"),
            &game,
        ),
        "the activated effect should still allow Spirit creatures to block the target"
    );

    let spell = CardDefinitionBuilder::new(CardId::new(), "Targeting Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_definition(&spell, bob, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell_id, bob));
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::BecomesTargetedEvent::new(deckhand_id, spell_id, bob, false),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == deckhand_id)
    {
        trigger_queue.add(entry);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Departed Deckhand should trigger once when targeted by a spell"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Departed Deckhand sacrifice trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Departed Deckhand sacrifice trigger should resolve");
    assert!(
        game.objects_in_zone(Zone::Graveyard).into_iter().any(|id| {
            game.object(id)
                .is_some_and(|object| object.name == "Departed Deckhand")
        }),
        "Departed Deckhand should be sacrificed after its targeting trigger resolves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cheering_fanatic_strict_parser_renders_chosen_name_cost_reduction() {
    let def = parse_oracle_card_definition("Cheering Fanatic");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cheering Fanatic should have an attack trigger");

    assert_eq!(
        triggered.trigger.display(),
        "Whenever this creature attacks",
        "Cheering Fanatic should trigger from attacking"
    );

    let effects = triggered.effects.flattened_default_effects();
    assert!(
        effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::ChooseCardNameEffect>()
            .is_some()),
        "Cheering Fanatic should choose a card name before granting the cost reduction"
    );
    let reduction = effects
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
        })
        .expect("Cheering Fanatic should grant a temporary matching-spell cost reduction");
    assert_eq!(reduction.player, PlayerFilter::Any);
    assert_eq!(reduction.filter.name.as_deref(), Some("{chosen name}"));
    assert_eq!(reduction.filter.cast_by, None);
    assert!(reduction.applies_to_all_matching_this_turn);
    assert!(
        matches!(reduction.generic_reduction, Some(Value::Fixed(1))),
        "Cheering Fanatic should reduce matching spells by one generic mana, got {:?}",
        reduction.generic_reduction
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("spells with the chosen name cost {1} less to cast this turn"),
        "Cheering Fanatic should render the chosen-name temporary cost reduction, got {rendered}"
    );
}

pub(super) fn assert_oracle_card_fails_strict(name: &str) {
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for regression card '{name}'"))
        .clone();
    let result = CardDefinitionBuilder::new(CardId::new(), name).parse_text(oracle.clone());
    assert!(
        result.is_err(),
        "strict parser regression expected failure for '{name}', but parse succeeded.\nOracle text:\n{}",
        oracle
    );
}

pub(super) fn season_of_the_burrow_modal_effect(def: &CardDefinition) -> &ChooseModeEffect {
    def.spell_effect
        .as_ref()
        .expect("Season of the Burrow should compile to spell effects")
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        .expect("Season of the Burrow should compile to one modal choice effect")
}

pub(super) fn target_assignments_for_requirements(
    requirements: &[crate::decision::TargetRequirement],
    targets: &[crate::game_state::Target],
) -> Vec<crate::game_state::TargetAssignment> {
    let requirement_contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                aggregate_constraint: requirement.aggregate_constraint.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
                distinct_player_group: requirement.distinct_player_group,
            },
        )
        .collect::<Vec<_>>();
    let ranges = crate::targeting::assigned_target_ranges(&requirement_contexts, targets)
        .expect("selected targets should satisfy requirements");
    requirements
        .iter()
        .zip(ranges)
        .map(|(requirement, range)| crate::game_state::TargetAssignment {
            spec: requirement.spec.clone(),
            range,
        })
        .collect()
}

pub(super) fn polliwallop_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(669_103), "Polliwallop")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Affinity for Frogs (This spell costs {1} less to cast for each Frog you control.)\n\
             Target creature you control deals damage equal to twice its power to target creature you don't control.",
        )
        .expect("Polliwallop should parse strictly")
}

#[test]
pub(super) fn polliwallop_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Polliwallop");
    let rendered = compiled_text_lines(&def).join("\n");
    let debug = format!("{:#?}", def.spell_effect);
    let affinity = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) if static_ability.has_affinity() => {
                Some(static_ability)
            }
            _ => None,
        });

    assert!(
        affinity.is_some_and(|static_ability| static_ability.id() == StaticAbilityId::Affinity),
        "expected Polliwallop to preserve affinity keyword identity, got {:#?}",
        def.abilities
    );

    assert!(
        rendered.contains("Affinity for Frogs"),
        "expected Polliwallop affinity keyword text, got {rendered}"
    );
    assert!(
        rendered.contains("damage equal to twice its power"),
        "expected Polliwallop scaled power damage text, got {rendered}"
    );
    assert!(
        debug.contains("TargetOnlyEffect")
            && debug.contains("ExecuteWithSourceEffect")
            && debug.contains("Scaled")
            && debug.contains("PowerOf"),
        "expected Polliwallop to tag the source creature and deal twice its power, got {debug}"
    );
}

#[test]
pub(super) fn polliwallop_affinity_for_frogs_reduces_only_for_your_frogs() {
    let def = polliwallop_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let spell = game
        .object(spell_id)
        .expect("Polliwallop should be in hand");
    let base_cost = spell
        .mana_cost
        .as_ref()
        .expect("Polliwallop has a mana cost")
        .clone();
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, spell, &base_cost)
            .mana_value(),
        4,
        "Polliwallop should not be reduced with no Frogs you control"
    );

    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_110), "Alice Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_111), "Alice Frog in Hand")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Frog])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Hand,
    );

    let spell = game
        .object(spell_id)
        .expect("Polliwallop should still be in hand");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, spell, &base_cost)
            .mana_value(),
        4,
        "Polliwallop affinity for Frogs should ignore artifacts and Frogs outside the battlefield"
    );

    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_104), "Alice Frog One")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Frog])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_105), "Alice Frog Two")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Frog])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_106), "Bob Frog")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Frog])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        bob,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_107), "Alice Rabbit")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Rabbit])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let spell = game
        .object(spell_id)
        .expect("Polliwallop should still be in hand");
    let reduced = crate::decision::calculate_effective_mana_cost(&game, alice, spell, &base_cost);
    assert_eq!(
        reduced.mana_value(),
        2,
        "Polliwallop should cost {{1}}{{G}} with exactly two Frogs you control"
    );
}

pub(super) fn krang_master_mind_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(955_852), "Krang, Master Mind")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 4))
        .parse_text(
            "Affinity for artifacts (This spell costs {1} less to cast for each artifact you control.)\n\
             When Krang enters, if you have fewer than four cards in hand, draw cards equal to the difference.\n\
             Krang gets +1/+0 for each other artifact you control.",
        )
        .expect("Krang, Master Mind should parse strictly")
}

#[test]
pub(super) fn krang_master_mind_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Krang, Master Mind");
    let def = parse_oracle_card_definition("Krang, Master Mind");
    let rendered = compiled_text_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    let debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Affinity for artifacts"),
        "Krang should preserve affinity keyword text, got {rendered}"
    );
    assert!(
        rendered_lower.contains("draw cards equal to the difference"),
        "Krang should render the conditional difference draw count, got {rendered}"
    );
    assert!(
        rendered_lower.contains("gets +1/+0 for each other artifact you control"),
        "Krang should render its other-artifact power bonus, got {rendered}"
    );
    assert!(
        debug.contains("PlayerCardsInHandOrFewer")
            && debug.contains("CardsInHand")
            && debug.contains("Difference"),
        "Krang should lower the fewer-than-four hand condition and difference draw structurally, got {debug}"
    );
}

#[test]
pub(super) fn krang_master_mind_etb_draws_up_to_four_cards_in_hand() {
    let def = krang_master_mind_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let krang_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    for idx in 0..5 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(
                CardId::from_raw(955_860 + idx),
                &format!("Library Card {idx}"),
            )
            .card_types(vec![CardType::Artifact])
            .build(),
            alice,
            Zone::Library,
        );
    }

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            krang_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    assert_eq!(
        resolve_triggers_for_source(&mut game, krang_id, &event),
        1,
        "Krang should trigger when its controller has fewer than four cards in hand"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        4,
        "Krang should draw exactly the difference up to four cards in hand"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        1,
        "Krang should draw four cards from a five-card library"
    );
}

#[test]
pub(super) fn krang_master_mind_etb_does_not_trigger_with_four_cards_in_hand() {
    let def = krang_master_mind_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let krang_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    for idx in 0..4 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(
                CardId::from_raw(955_870 + idx),
                &format!("Hand Card {idx}"),
            )
            .card_types(vec![CardType::Artifact])
            .build(),
            alice,
            Zone::Hand,
        );
    }
    for idx in 0..3 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(
                CardId::from_raw(955_880 + idx),
                &format!("Library Card {idx}"),
            )
            .card_types(vec![CardType::Artifact])
            .build(),
            alice,
            Zone::Library,
        );
    }

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            krang_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    assert_eq!(
        resolve_triggers_for_source(&mut game, krang_id, &event),
        0,
        "Krang should use the hand-size condition as an intervening-if trigger gate"
    );
    assert_eq!(game.player(alice).expect("alice exists").hand.len(), 4);
    assert_eq!(game.player(alice).expect("alice exists").library.len(), 3);
}

#[test]
pub(super) fn krang_master_mind_etb_draw_condition_is_rechecked_on_resolution() {
    let def = krang_master_mind_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let krang_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    for idx in 0..3 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(
                CardId::from_raw(955_883 + idx),
                &format!("Library Card {idx}"),
            )
            .card_types(vec![CardType::Artifact])
            .build(),
            alice,
            Zone::Library,
        );
    }

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            krang_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggers
            .iter()
            .filter(|entry| entry.source == krang_id)
            .count(),
        1,
        "Krang should initially trigger while its controller has fewer than four cards in hand"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers
        .into_iter()
        .filter(|entry| entry.source == krang_id)
    {
        trigger_queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Krang trigger should go on the stack");
    for idx in 0..4 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(
                CardId::from_raw(955_886 + idx),
                &format!("Hand Card {idx}"),
            )
            .card_types(vec![CardType::Artifact])
            .build(),
            alice,
            Zone::Hand,
        );
    }

    crate::game_loop::resolve_stack_entry(&mut game).expect("Krang trigger should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        4,
        "Krang should not draw if the intervening-if condition fails on resolution"
    );
    assert_eq!(game.player(alice).expect("alice exists").library.len(), 3);
}

#[test]
pub(super) fn krang_master_mind_affinity_reduces_only_for_your_artifacts() {
    let def = krang_master_mind_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let krang_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let krang = game.object(krang_id).expect("Krang should be in hand");
    let base_cost = krang
        .mana_cost
        .as_ref()
        .expect("Krang has a mana cost")
        .clone();

    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, krang, &base_cost)
            .mana_value(),
        8,
        "Krang should cost eight mana with no artifacts you control"
    );

    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(955_890), "Alice Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(955_891), "Bob Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(955_892), "Alice Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let krang = game
        .object(krang_id)
        .expect("Krang should still be in hand");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, krang, &base_cost)
            .mana_value(),
        7,
        "Krang affinity should count only artifacts controlled by its controller"
    );
}

#[test]
pub(super) fn krang_master_mind_gets_plus_one_power_for_each_other_artifact_you_control() {
    let def = krang_master_mind_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let krang_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert_eq!(game.current_power(krang_id), Some(1));
    assert_eq!(game.current_toughness(krang_id), Some(4));

    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(955_900), "Bob Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    assert_eq!(
        game.current_power(krang_id),
        Some(1),
        "Krang should not count artifacts controlled by opponents"
    );

    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(955_901), "Alice Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    assert_eq!(
        game.current_power(krang_id),
        Some(2),
        "Krang should get +1/+0 for one other artifact you control"
    );
    assert_eq!(game.current_toughness(krang_id), Some(4));
}

#[test]
pub(super) fn polliwallop_targets_and_deals_twice_source_creature_power() {
    let def = polliwallop_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

    let alice_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_108), "Alice Biter")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let bob_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(669_109), "Bob Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(10, 10))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let program = def
        .spell_effect
        .as_ref()
        .expect("Polliwallop should have spell effects");
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        program,
        alice,
        Some(spell_id),
        None,
    );
    assert_eq!(
        requirements.len(),
        2,
        "Polliwallop should require two targets"
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(alice_creature)),
        "first Polliwallop target should include a creature you control"
    );
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(bob_creature)),
        "first Polliwallop target must not include creatures you don't control"
    );
    assert!(
        requirements[1]
            .legal_targets
            .contains(&crate::game_state::Target::Object(bob_creature)),
        "second Polliwallop target should include a creature you don't control"
    );
    assert!(
        !requirements[1]
            .legal_targets
            .contains(&crate::game_state::Target::Object(alice_creature)),
        "second Polliwallop target must not include creatures you control"
    );

    let selected_targets = vec![
        crate::game_state::Target::Object(alice_creature),
        crate::game_state::Target::Object(bob_creature),
    ];
    let assignments = target_assignments_for_requirements(&requirements, &selected_targets);
    let mut ctx = crate::effects::ExecutionContext::new_default(spell_id, alice)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(alice_creature),
            crate::effects::ResolvedTarget::Object(bob_creature),
        ])
        .with_target_assignments(assignments);
    for effect in program.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Polliwallop spell effect should resolve");
    }

    assert_eq!(
        game.damage_on(bob_creature),
        6,
        "Polliwallop should deal twice the 3-power source creature's power"
    );
    assert_eq!(
        game.damage_on(alice_creature),
        0,
        "Polliwallop is one-sided damage, not fight"
    );
}

#[test]
pub(super) fn season_of_the_burrow_strict_parser_and_weighted_modal_text_regression() {
    let def = parse_oracle_card_definition("Season of the Burrow");
    let modal = season_of_the_burrow_modal_effect(&def);

    assert_eq!(modal.choose_count, Value::Fixed(5));
    assert_eq!(modal.min_choose_count, Value::Fixed(0));
    assert!(modal.allow_repeated_modes);
    assert_eq!(modal.mode_point_costs, vec![1, 2, 3]);

    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Choose up to five {P} worth of modes")
            && rendered.contains("You may choose the same mode more than once")
            && rendered.contains("{P}{P} — Exile target nonland permanent")
            && rendered
                .contains("{P}{P}{P} — Return target permanent card with mana value 3 or less"),
        "expected Season of the Burrow weighted modal text, got {rendered}"
    );
}

#[test]
pub(super) fn season_of_the_burrow_rejects_over_budget_modes_and_keeps_target_filters() {
    let def = parse_oracle_card_definition("Season of the Burrow");
    let effects = def
        .spell_effect
        .as_ref()
        .expect("Season of the Burrow should compile to spell effects")
        .clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);

    let nonland = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_730), "Bob's Bauble")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let land = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_731), "Bob's Burrow")
            .card_types(vec![CardType::Land])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let small_permanent = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_732), "Alice's Keepsake")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Graveyard,
    );
    let expensive_permanent = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_733), "Alice's Monument")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Graveyard,
    );

    assert!(
        crate::game_loop::spell_program_has_legal_targets_with_modes(
            &game,
            &effects,
            alice,
            Some(source),
            Some(&[2, 1]),
        )
    );
    assert!(
        !crate::game_loop::spell_program_has_legal_targets_with_modes(
            &game,
            &effects,
            alice,
            Some(source),
            Some(&[2, 2]),
        )
    );

    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &effects,
        alice,
        Some(source),
        Some(&[1, 2]),
    );
    assert_eq!(requirements.len(), 2);
    assert!(
        requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(nonland))
    );
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(land))
    );
    assert!(
        requirements[1]
            .legal_targets
            .contains(&crate::game_state::Target::Object(small_permanent))
    );
    assert!(
        !requirements[1]
            .legal_targets
            .contains(&crate::game_state::Target::Object(expensive_permanent))
    );
}

#[test]
pub(super) fn season_of_the_burrow_return_and_exile_modes_resolve_in_selected_order() {
    let def = parse_oracle_card_definition("Season of the Burrow");
    let modal = season_of_the_burrow_modal_effect(&def).clone();
    let effects = def
        .spell_effect
        .as_ref()
        .expect("Season of the Burrow should compile to spell effects")
        .clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let graveyard_card = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_721), "Alice's Buried Keepsake")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Graveyard,
    );
    let battlefield_card = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_722), "Bob's Exiled Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_723), "Bob's Draw")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        bob,
        Zone::Library,
    );

    let chosen_modes = [2usize, 1usize];
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &effects,
        alice,
        Some(source),
        Some(&chosen_modes),
    );
    let selected_targets = vec![
        crate::game_state::Target::Object(graveyard_card),
        crate::game_state::Target::Object(battlefield_card),
    ];
    let assignments = target_assignments_for_requirements(&requirements, &selected_targets);
    assert_eq!(assignments[0].range, 0..1);
    assert_eq!(assignments[1].range, 1..2);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_chosen_modes(Some(chosen_modes.to_vec()))
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(graveyard_card),
            crate::effects::ResolvedTarget::Object(battlefield_card),
        ])
        .with_target_assignments(assignments);

    modal
        .execute(&mut game, &mut ctx)
        .expect("Season of the Burrow return plus exile modes should resolve");

    let returned_id = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Alice's Buried Keepsake")
        })
        .expect("returned permanent should be on the battlefield");
    let returned = game
        .object(returned_id)
        .expect("returned permanent should exist");
    assert_eq!(
        returned
            .counters
            .get(&crate::object::CounterType::Indestructible)
            .copied(),
        Some(1)
    );
    assert!(game.objects_in_zone(Zone::Exile).into_iter().any(|id| {
        game.object(id)
            .is_some_and(|object| object.name == "Bob's Exiled Relic")
    }));
    assert_eq!(game.player(bob).expect("Bob should exist").hand.len(), 1);
}

pub(super) fn garna_bloodfist_triggered_ability(
    def: &CardDefinition,
) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Garna, Bloodfist of Keld should have a triggered ability")
}

pub(super) fn parse_garna_bloodfist_card_definition() -> CardDefinition {
    let info = oracle_card_info_by_name()
        .get("Garna, Bloodfist of Keld")
        .expect("Garna, Bloodfist of Keld should exist in cards.json");
    let (supertypes, card_types, subtypes) =
        parse_type_line(info.type_line.as_deref().unwrap_or_default())
            .expect("Garna, Bloodfist of Keld type line should parse");
    CardDefinitionBuilder::new(CardId::new(), "Garna, Bloodfist of Keld")
        .supertypes(supertypes)
        .card_types(card_types)
        .subtypes(subtypes)
        .parse_text(info.oracle_text.clone())
        .expect("Garna, Bloodfist of Keld should parse strictly")
}

pub(super) fn add_garna_draw_card(game: &mut crate::game_state::GameState, player: PlayerId) {
    let card = crate::card::CardBuilder::new(CardId::new(), "Garna Draw Card")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_card(&card, player, Zone::Library);
}

pub(super) fn resolve_garna_bloodfist_trigger_for_dying_creature(
    was_attacking: bool,
) -> crate::game_state::GameState {
    let def = parse_oracle_card_definition("Garna, Bloodfist of Keld");
    let triggered = garna_bloodfist_triggered_ability(&def).clone();
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let garna_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let creature_def = CardDefinitionBuilder::new(CardId::new(), "Keld Witness")
        .card_types(vec![CardType::Creature])
        .build();
    let creature_id = game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);
    add_garna_draw_card(&mut game, alice);

    if was_attacking {
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: creature_id,
                target: crate::combat_state::AttackTarget::Player(bob),
            }],
            ..Default::default()
        });
    }

    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(creature_id)
            .expect("dying creature should exist before moving"),
        &game,
    );
    let graveyard_id = game
        .move_object_by_effect(creature_id, Zone::Graveyard)
        .expect("dying creature should move to graveyard");
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_results(
            creature_id,
            vec![graveyard_id],
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(garna_id, alice, &mut dm)
        .with_triggering_event(event);
    for effect in &triggered.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Garna trigger effect should resolve");
    }
    game
}

#[test]
pub(super) fn garna_bloodfist_of_keld_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Garna, Bloodfist of Keld");
    let def = parse_garna_bloodfist_card_definition();
    let rendered = compiled_text_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Whenever another creature you control dies, draw a card if it was attacking. Otherwise, Garna deals 1 damage to each opponent"
        ),
        "expected Garna's attacking conditional and otherwise damage clause to render oracle-like, got {rendered}"
    );
    assert!(
        !rendered
            .to_ascii_lowercase()
            .contains("that object matches"),
        "Garna rendered text should not expose tagged-object predicate internals, got {rendered}"
    );
    assert!(
        !rendered.contains("object-predicate-debug"),
        "Garna rendered text should not expose the parser debug marker, got {rendered}"
    );
}

#[test]
pub(super) fn garna_bloodfist_of_keld_triggers_only_for_another_creature_you_control_dying() {
    let def = parse_oracle_card_definition("Garna, Bloodfist of Keld");
    let triggered = garna_bloodfist_triggered_ability(&def);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let garna_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let creature_def = CardDefinitionBuilder::new(CardId::new(), "Dying Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let alice_creature =
        game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);
    let bob_creature = game.create_object_from_definition(&creature_def, bob, Zone::Battlefield);

    let ctx = crate::triggers::TriggerContext::for_source(garna_id, alice, &game);
    let allied_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(alice_creature).expect("allied creature exists"),
            &game,
        );
    let opposing_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(bob_creature).expect("opposing creature exists"),
            &game,
        );
    let garna_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(garna_id).expect("Garna exists"),
            &game,
        );

    let allied_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            alice_creature,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(allied_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let opposing_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            bob_creature,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(opposing_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let self_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            garna_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(garna_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    assert!(triggered.trigger.matches(&allied_event, &ctx));
    assert!(!triggered.trigger.matches(&opposing_event, &ctx));
    assert!(!triggered.trigger.matches(&self_event, &ctx));
}

#[test]
pub(super) fn garna_bloodfist_of_keld_draws_when_the_dying_creature_was_attacking() {
    let game = resolve_garna_bloodfist_trigger_for_dying_creature(true);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 1);
    assert_eq!(game.life_total(bob), 20);
    assert_eq!(game.life_total(charlie), 20);
}

#[test]
pub(super) fn garna_bloodfist_of_keld_damages_opponents_when_the_dying_creature_was_not_attacking()
{
    let game = resolve_garna_bloodfist_trigger_for_dying_creature(false);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 0);
    assert_eq!(game.life_total(alice), 20);
    assert_eq!(game.life_total(bob), 19);
    assert_eq!(game.life_total(charlie), 19);
}

#[test]
pub(super) fn frodo_adventurous_hobbit_strict_parser_and_compiled_text_regression() {
    let oracle = oracle_text_by_name()
        .get("Frodo, Adventurous Hobbit")
        .expect("missing Frodo, Adventurous Hobbit oracle text")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "Frodo, Adventurous Hobbit")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Frodo, Adventurous Hobbit should parse strictly");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Frodo, Adventurous Hobbit should parse its attack trigger strictly"
    );
    assert!(
        ability_debug.contains("RingTemptsYouEffect")
            && ability_debug.contains("ConditionalEffect")
            && ability_debug.contains("SourceIsRingBearer")
            && ability_debug.contains("PlayerRingTemptedThisGameOrMore")
            && ability_debug.contains("DrawCardsEffect"),
        "expected Ring temptation plus Ring-bearer draw gate, got {ability_debug}"
    );
    assert!(
        rendered_lower.contains(
            "if frodo is your ring-bearer and the ring has tempted you two or more times this game"
        ) && rendered_lower.contains("draw a card"),
        "expected Frodo's Ring-bearer temptation gate to render, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("unsupported predicate")
            && !rendered_lower.contains("unsupported effect"),
        "Frodo should compile without unsupported fallbacks, got {rendered}"
    );
}

#[test]
pub(super) fn martyr_of_spores_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Martyr of Spores");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);
    let ability_debug_compact = format!("{:?}", def.abilities);

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "Martyr of Spores should strictly parse its activated ability"
    );
    assert!(
        rendered.contains("Reveal X green cards from your hand"),
        "Martyr of Spores compiled text should preserve the X green reveal cost, got {rendered}"
    );
    assert!(
        rendered.contains("Target creature gets +X/+X until end of turn"),
        "Martyr of Spores compiled text should preserve the target pump effect, got {rendered}"
    );
    assert!(
        ability_debug.contains("RevealFromHandEffect")
            && ability_debug.contains("count: X")
            && ability_debug_compact.contains(&format!("{:?}", crate::color::ColorSet::GREEN)),
        "Martyr of Spores should lower reveal-X-green-cards structurally, got {ability_debug}"
    );
}

#[test]
pub(super) fn death_in_heaven_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Death in Heaven");
    let def = parse_oracle_card_definition("Death in Heaven");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "put all creature cards exiled with this card onto the battlefield face down under your control"
        ),
        "Death in Heaven chapter III should render the source-linked face-down return, got {rendered}"
    );
    assert!(
        rendered.contains("they're 2/2 cyberman artifact creatures"),
        "Death in Heaven chapter III should render the Cyberman artifact-creature effect, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ReturnAllToBattlefieldEffect")
            && debug.contains("face_down: true")
            && debug.contains("battlefield_controller: You")
            && debug.contains(crate::tag::SOURCE_EXILED_TAG)
            && debug.contains("Cyberman"),
        "Death in Heaven should lower chapter III to a face-down source-linked return plus Cyberman continuous effect, got {debug}"
    );
}

pub(super) fn death_in_heaven_saga_trigger(
    def: &CardDefinition,
    trigger_index: usize,
) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .filter_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            Some(triggered)
        })
        .nth(trigger_index)
        .unwrap_or_else(|| panic!("Death in Heaven should have saga trigger index {trigger_index}"))
}

pub(super) fn execute_death_in_heaven_trigger(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    controller: PlayerId,
    triggered: &crate::ability::TriggeredAbility,
    target_player: Option<PlayerId>,
) {
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller);
    if let Some(target_player) = target_player {
        ctx = ctx
            .with_targets(vec![crate::effects::ResolvedTarget::Player(target_player)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: triggered
                    .choices
                    .first()
                    .expect("Death in Heaven chapters I/II should target a player")
                    .clone(),
                range: 0..1,
            }]);
    }
    ctx.snapshot_targets(game);
    if let Some(source_object) = game.object(source) {
        ctx.tag_object(
            "triggering",
            crate::snapshot::ObjectSnapshot::from_object(source_object, game),
        );
    }

    for effect in triggered.effects.flattened_default_effects() {
        if effect
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
        {
            continue;
        }
        crate::effects::execute_effect(game, effect, &mut ctx).unwrap_or_else(|err| {
            panic!("Death in Heaven saga effect should resolve: {err:?}; effect: {effect:#?}")
        });
    }
}

#[test]
pub(super) fn death_in_heaven_mills_exiles_then_returns_only_source_exiled_creatures_face_down() {
    let def = parse_oracle_card_definition("Death in Heaven");
    let chapter_one_two = death_in_heaven_saga_trigger(&def, 0);
    let chapter_three = death_in_heaven_saga_trigger(&def, 1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let creature_card = crate::card::CardBuilder::new(CardId::new(), "Milled Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let noncreature_card = crate::card::CardBuilder::new(CardId::new(), "Milled Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let graveyard_creature_card = crate::card::CardBuilder::new(CardId::new(), "Buried Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let unrelated_creature = crate::card::CardBuilder::new(CardId::new(), "Unrelated Exile")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();

    let creature_id = game.create_object_from_card(&creature_card, bob, Zone::Library);
    let creature_stable_id = game
        .object(creature_id)
        .expect("milled creature should exist")
        .stable_id;
    let spell_id = game.create_object_from_card(&noncreature_card, bob, Zone::Library);
    let spell_stable_id = game
        .object(spell_id)
        .expect("milled spell should exist")
        .stable_id;
    let graveyard_creature_id =
        game.create_object_from_card(&graveyard_creature_card, bob, Zone::Graveyard);
    let graveyard_creature_stable_id = game
        .object(graveyard_creature_id)
        .expect("graveyard creature should exist")
        .stable_id;
    let unrelated_id = game.create_object_from_card(&unrelated_creature, bob, Zone::Exile);
    let unrelated_stable_id = game
        .object(unrelated_id)
        .expect("unrelated exiled creature should exist")
        .stable_id;

    execute_death_in_heaven_trigger(&mut game, source, alice, chapter_one_two, Some(bob));
    assert!(
        game.player(bob).expect("bob exists").library.is_empty(),
        "Death in Heaven chapters I/II should mill two cards from the targeted player"
    );
    assert!(
        game.player(bob).expect("bob exists").graveyard.is_empty(),
        "Death in Heaven chapters I/II should exile the targeted player's graveyard after milling"
    );

    execute_death_in_heaven_trigger(&mut game, source, alice, chapter_three, None);

    let returned_creature = game
        .find_object_by_stable_id(creature_stable_id)
        .expect("milled creature should still be tracked");
    let returned = game
        .object(returned_creature)
        .expect("milled creature should be on the battlefield");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(returned), alice);
    assert!(
        game.is_face_down(returned_creature),
        "Death in Heaven chapter III should return creature cards face down"
    );
    assert_eq!(game.calculated_power(returned_creature), Some(2));
    assert_eq!(game.calculated_toughness(returned_creature), Some(2));
    assert!(
        game.calculated_card_types(returned_creature)
            .contains(&CardType::Artifact)
    );
    assert!(
        game.calculated_card_types(returned_creature)
            .contains(&CardType::Creature)
    );
    assert!(
        game.calculated_subtypes(returned_creature)
            .contains(&Subtype::Cyberman),
        "returned face-down creature should be a Cyberman"
    );
    let returned_graveyard_creature = game
        .find_object_by_stable_id(graveyard_creature_stable_id)
        .expect("graveyard creature should still be tracked");
    assert_eq!(
        game.object(returned_graveyard_creature)
            .expect("graveyard creature should be on the battlefield")
            .zone,
        Zone::Battlefield,
        "Death in Heaven chapter III should also return creature cards already in the targeted graveyard"
    );
    assert_eq!(
        game.controller_of(
            game.object(returned_graveyard_creature)
                .expect("graveyard creature should be on the battlefield"),
        ),
        alice
    );
    assert!(
        game.is_face_down(returned_graveyard_creature),
        "returned graveyard creature should enter face down"
    );

    let exiled_spell = game
        .find_object_by_stable_id(spell_stable_id)
        .expect("milled noncreature should still be tracked");
    assert_eq!(
        game.object(exiled_spell).expect("milled spell exists").zone,
        Zone::Exile,
        "Death in Heaven chapter III should not return noncreature cards"
    );
    let unrelated_after = game
        .find_object_by_stable_id(unrelated_stable_id)
        .expect("unrelated exiled creature should still be tracked");
    assert_eq!(
        game.object(unrelated_after)
            .expect("unrelated exiled creature exists")
            .zone,
        Zone::Exile,
        "Death in Heaven chapter III should not return creature cards exiled by other sources"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pyxis_reveals_every_source_exiled_card_but_returns_only_linked_permanents() {
    let pyxis = parse_oracle_card_definition("Pyxis of Pandemonium");
    let release = pyxis
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .nth(1)
        .expect("Pyxis should have its seven-mana release ability");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&pyxis, alice, Zone::Battlefield);

    let linked_creature = crate::card::CardBuilder::new(CardId::new(), "Linked Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let linked_land = crate::card::CardBuilder::new(CardId::new(), "Linked Land")
        .card_types(vec![CardType::Land])
        .build();
    let linked_instant = crate::card::CardBuilder::new(CardId::new(), "Linked Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let unrelated_creature = crate::card::CardBuilder::new(CardId::new(), "Unrelated Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();

    let creature = game.create_object_from_card(&linked_creature, alice, Zone::Exile);
    let land = game.create_object_from_card(&linked_land, bob, Zone::Exile);
    let instant = game.create_object_from_card(&linked_instant, bob, Zone::Exile);
    let unrelated = game.create_object_from_card(&unrelated_creature, bob, Zone::Exile);
    let creature_stable = game.object(creature).unwrap().stable_id;
    let land_stable = game.object(land).unwrap().stable_id;
    let instant_stable = game.object(instant).unwrap().stable_id;
    let unrelated_stable = game.object(unrelated).unwrap().stable_id;
    for object in [creature, land, instant, unrelated] {
        game.set_face_down(object);
    }
    for object in [creature, land, instant] {
        game.add_exiled_with_source_link(source, object);
    }

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in release.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Pyxis release effect should resolve");
    }

    let returned_creature = game.find_object_by_stable_id(creature_stable).unwrap();
    let returned_land = game.find_object_by_stable_id(land_stable).unwrap();
    assert_eq!(
        game.object(returned_creature).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(game.object(returned_land).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.controller_of(game.object(returned_creature).unwrap()),
        alice
    );
    assert_eq!(game.controller_of(game.object(returned_land).unwrap()), bob);

    let revealed_instant = game.find_object_by_stable_id(instant_stable).unwrap();
    assert_eq!(game.object(revealed_instant).unwrap().zone, Zone::Exile);
    assert!(
        !game.is_face_down(revealed_instant),
        "linked nonpermanents should remain in exile face up"
    );
    let unrelated_after = game.find_object_by_stable_id(unrelated_stable).unwrap();
    assert_eq!(game.object(unrelated_after).unwrap().zone, Zone::Exile);
    assert!(
        game.is_face_down(unrelated_after),
        "cards exiled by other sources must not be revealed or returned"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rohgahh_declined_upkeep_payment_taps_and_transfers_the_full_coordinated_set() {
    let rohgahh = parse_oracle_card_definition("Rohgahh of Kher Keep");
    let upkeep = rohgahh
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Rohgahh should have an upkeep trigger");
    let kobold = CardDefinitionBuilder::new(CardId::new(), "Kobolds of Kher Keep")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Kobold])
        .power_toughness(PowerToughness::fixed(0, 1))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&rohgahh, alice, Zone::Battlefield);
    let first = game.create_object_from_definition(&kobold, alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&kobold, alice, Zone::Battlefield);
    let opponent_owned = game.create_object_from_definition(&kobold, bob, Zone::Battlefield);
    let unrelated_kobold = CardDefinitionBuilder::new(CardId::new(), "Other Kobold")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Kobold])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let unrelated = game.create_object_from_definition(&unrelated_kobold, alice, Zone::Battlefield);

    assert_eq!(game.current_power(first), Some(2));
    assert_eq!(game.current_toughness(first), Some(3));
    assert_eq!(game.current_power(opponent_owned), Some(0));
    assert_eq!(game.current_power(unrelated), Some(1));

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &upkeep.effects,
        None,
        &[],
    )
    .expect("Rohgahh upkeep should resolve when its controller cannot pay");

    for object in [source, first, second, opponent_owned] {
        assert!(
            game.is_tapped(object),
            "the full coordinated set should be tapped"
        );
        assert_eq!(
            game.controller_of(game.object(object).unwrap()),
            bob,
            "the chosen opponent should gain control of the full tapped set"
        );
    }
    assert!(!game.is_tapped(unrelated));
    assert_eq!(
        game.controller_of(game.object(unrelated).unwrap()),
        alice,
        "a same-subtype creature with another name must not enter the tapped/transfer set"
    );
    assert_eq!(game.current_power(first), Some(2));
    assert_eq!(game.current_power(opponent_owned), Some(2));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rohgahh_paid_upkeep_payment_spends_three_red_and_preserves_the_full_set() {
    let rohgahh = parse_oracle_card_definition("Rohgahh of Kher Keep");
    let upkeep = rohgahh
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Rohgahh should have an upkeep trigger");
    let kobold = CardDefinitionBuilder::new(CardId::new(), "Kobolds of Kher Keep")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Kobold])
        .power_toughness(PowerToughness::fixed(0, 1))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&rohgahh, alice, Zone::Battlefield);
    let controlled = game.create_object_from_definition(&kobold, alice, Zone::Battlefield);
    let opponent_owned = game.create_object_from_definition(&kobold, bob, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 3);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &upkeep.effects,
        None,
        &[],
    )
    .expect("Rohgahh upkeep should resolve after its controller pays");

    assert_eq!(game.player(alice).expect("Alice exists").mana_pool.red, 0);
    for (object, controller) in [(source, alice), (controlled, alice), (opponent_owned, bob)] {
        assert!(!game.is_tapped(object));
        assert_eq!(
            game.controller_of(game.object(object).unwrap()),
            controller,
            "paying must prevent every tap and control-change consequence"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn losheel_clockwork_scholar_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Losheel, Clockwork Scholar");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Prevent all combat damage that would be dealt to attacking artifact creatures you control."
        ),
        "expected Losheel's filtered combat prevention clause to render, got {rendered}"
    );
    assert!(
        rendered.contains("This ability triggers only once each turn"),
        "expected Losheel's once-each-turn trigger text to render, got {rendered}"
    );

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("PreventAllCombatDamageToPermanentsMatching")
            && ability_debug.contains("attacking: true")
            && ability_debug.contains("Artifact")
            && ability_debug.contains("Creature")
            && ability_debug.contains("controller: Some(You)"),
        "expected Losheel to lower prevention to a filtered static replacement ability, got {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn losheel_test_creature(
    game: &mut crate::game_state::GameState,
    name: &str,
    controller: PlayerId,
    card_types: Vec<CardType>,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn losheel_clockwork_scholar_prevents_only_attacking_artifact_creature_combat_damage() {
    let losheel = parse_oracle_card_definition("Losheel, Clockwork Scholar");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.create_object_from_definition(&losheel, alice, Zone::Battlefield);
    let protected_attacker = losheel_test_creature(
        &mut game,
        "Attacking Artifact Creature",
        alice,
        vec![CardType::Artifact, CardType::Creature],
        2,
        4,
    );
    let unprotected_attacker = losheel_test_creature(
        &mut game,
        "Attacking Nonartifact Creature",
        alice,
        vec![CardType::Creature],
        2,
        4,
    );
    let artifact_blocker = losheel_test_creature(
        &mut game,
        "Artifact Blocker",
        bob,
        vec![CardType::Creature],
        3,
        4,
    );
    let nonartifact_blocker = losheel_test_creature(
        &mut game,
        "Nonartifact Blocker",
        bob,
        vec![CardType::Creature],
        3,
        4,
    );

    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: protected_attacker,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: unprotected_attacker,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    combat
        .blockers
        .insert(protected_attacker, vec![artifact_blocker]);
    combat
        .blockers
        .insert(unprotected_attacker, vec![nonartifact_blocker]);
    game.combat = Some(combat.clone());
    game.refresh_continuous_state();

    crate::game_loop::execute_combat_damage_step(&mut game, &combat, false);

    assert_eq!(
        game.damage_on(protected_attacker),
        0,
        "Losheel should prevent combat damage to an attacking artifact creature Alice controls"
    );
    assert_eq!(
        game.damage_on(unprotected_attacker),
        3,
        "Losheel should not prevent combat damage to a nonartifact attacking creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn losheel_clockwork_scholar_prevention_rejects_nonattacking_and_opponent_artifacts() {
    let losheel = parse_oracle_card_definition("Losheel, Clockwork Scholar");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let losheel_id = game.create_object_from_definition(&losheel, alice, Zone::Battlefield);
    let alice_attacking_artifact = losheel_test_creature(
        &mut game,
        "Alice Attacking Artifact",
        alice,
        vec![CardType::Artifact, CardType::Creature],
        2,
        2,
    );
    let alice_nonattacking_artifact = losheel_test_creature(
        &mut game,
        "Alice Nonattacking Artifact",
        alice,
        vec![CardType::Artifact, CardType::Creature],
        2,
        2,
    );
    let bob_attacking_artifact = losheel_test_creature(
        &mut game,
        "Bob Attacking Artifact",
        bob,
        vec![CardType::Artifact, CardType::Creature],
        2,
        2,
    );
    let damage_source = losheel_test_creature(
        &mut game,
        "Damage Source",
        bob,
        vec![CardType::Creature],
        3,
        3,
    );

    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: alice_attacking_artifact,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: bob_attacking_artifact,
        target: crate::combat_state::AttackTarget::Player(alice),
    });
    game.combat = Some(combat);
    game.refresh_continuous_state();

    let replacement = game
        .effect_store
        .replacement_effects
        .effects()
        .iter()
        .find(|effect| effect.source == losheel_id)
        .expect("Losheel should generate a static replacement effect");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("Losheel replacement should have a matcher");
    let ctx =
        crate::events::context::EventContext::for_replacement_effect(alice, losheel_id, &game);

    let protected = crate::events::damage::DamageEvent::with_cause(
        damage_source,
        crate::events::DamageTarget::Object(alice_attacking_artifact),
        3,
        true,
        crate::events::cause::EventCause::combat_damage(damage_source),
    );
    assert!(matcher.matches_event(&protected, &ctx));

    let nonattacking = crate::events::damage::DamageEvent::with_cause(
        damage_source,
        crate::events::DamageTarget::Object(alice_nonattacking_artifact),
        3,
        true,
        crate::events::cause::EventCause::combat_damage(damage_source),
    );
    assert!(!matcher.matches_event(&nonattacking, &ctx));

    let opponent_controlled = crate::events::damage::DamageEvent::with_cause(
        damage_source,
        crate::events::DamageTarget::Object(bob_attacking_artifact),
        3,
        true,
        crate::events::cause::EventCause::combat_damage(damage_source),
    );
    assert!(!matcher.matches_event(&opponent_controlled, &ctx));

    let noncombat = crate::events::damage::DamageEvent::with_cause(
        damage_source,
        crate::events::DamageTarget::Object(alice_attacking_artifact),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&noncombat, &ctx));

    let unpreventable = crate::events::damage::DamageEvent::unpreventable_with_cause(
        damage_source,
        crate::events::DamageTarget::Object(alice_attacking_artifact),
        3,
        true,
        crate::events::cause::EventCause::combat_damage(damage_source),
    );
    assert!(!matcher.matches_event(&unpreventable, &ctx));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn losheel_clockwork_scholar_artifact_creature_enter_trigger_is_once_each_turn() {
    let losheel = parse_oracle_card_definition("Losheel, Clockwork Scholar");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let losheel_id = game.create_object_from_definition(&losheel, alice, Zone::Battlefield);
    let artifact = losheel_test_creature(
        &mut game,
        "Entering Artifact Creature",
        alice,
        vec![CardType::Artifact, CardType::Creature],
        1,
        1,
    );
    let etb_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            artifact,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &etb_event) {
        if trigger.source == losheel_id {
            trigger_queue.add(trigger);
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Losheel should trigger when an artifact creature Alice controls enters"
    );
    let trigger_debug = format!("{:#?}", trigger_queue.entries[0]);
    assert!(
        trigger_debug.contains("DrawCardsEffect") || trigger_debug.contains("Draw"),
        "Losheel's trigger should draw a card, got {trigger_debug}"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Losheel trigger should move to the stack");

    let second_trigger_count = crate::triggers::check_triggers(&game, &etb_event)
        .iter()
        .filter(|entry| entry.source == losheel_id)
        .count();
    assert_eq!(
        second_trigger_count, 0,
        "Losheel's artifact-creature-enter trigger should trigger only once each turn"
    );
}

#[test]
pub(super) fn alena_kessig_trapper_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Alena, Kessig Trapper");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("first strike") && rendered_lower.contains("partner"),
        "expected Alena's keywords to render, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Add an amount of {R} equal to the greatest power among creatures you control that entered this turn"
        ),
        "expected Alena's aggregate red mana clause to render, got {rendered}"
    );

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("AddScaledManaEffect")
            && ability_debug.contains("GreatestPower")
            && ability_debug.contains("entered_battlefield_this_turn: true"),
        "expected Alena to lower to scaled red mana from entered-this-turn greatest power, got {ability_debug}"
    );
}

#[test]
pub(super) fn kjeldoran_elite_guard_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Kjeldoran Elite Guard");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("target creature gets +2/+2 until end of turn"),
        "expected Kjeldoran Elite Guard to render the target pump, got {rendered}"
    );
    assert!(
        rendered_lower.contains(
            "when that creature leaves the battlefield this turn, sacrifice this creature"
        ),
        "expected the delayed target leaves-battlefield clause to render, got {rendered}"
    );
    assert!(
        rendered_lower.contains("activate only during combat"),
        "expected the combat-only activation restriction to render, got {rendered}"
    );

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("ScheduleDelayedTriggerEffect")
            && ability_debug.contains("target_tag: Some")
            && ability_debug.contains("targeted_0")
            && ability_debug.contains("from: Specific(")
            && ability_debug.contains("Battlefield")
            && ability_debug.contains("to: Any")
            && ability_debug.contains("this_object: true"),
        "expected delayed trigger to watch the targeted creature leaving, got {ability_debug}"
    );
}

#[test]
pub(super) fn alena_kessig_trapper_mana_runtime_uses_only_your_creatures_that_entered_this_turn() {
    fn record_entered_this_turn(game: &mut crate::game_state::GameState, id: ObjectId) {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(id)
                .expect("entered object should exist on the battlefield"),
            game,
        );
        let entry_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                id,
                Zone::Hand,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&entry_event);
    }

    fn create_creature(
        game: &mut crate::game_state::GameState,
        controller: PlayerId,
        name: &str,
        power: i32,
        entered_this_turn: bool,
    ) -> ObjectId {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 1))
            .build();
        let id = game.create_object_from_definition(&def, controller, Zone::Battlefield);
        if entered_this_turn {
            record_entered_this_turn(game, id);
        }
        id
    }

    let def = parse_oracle_card_definition("Alena, Kessig Trapper");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Alena should have an activated mana ability");
    let add_scaled = activated
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::AddScaledManaEffect>())
        .expect("Alena should produce scaled red mana");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alena_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    create_creature(&mut game, alice, "Old Behemoth", 7, false);
    create_creature(&mut game, bob, "Bob's New Behemoth", 6, true);
    create_creature(&mut game, alice, "Alice's New Scout", 3, true);
    create_creature(&mut game, alice, "Alice's New Giant", 5, true);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(alena_id, alice, &mut dm);
    let result = add_scaled
        .execute(&mut game, &mut ctx)
        .expect("Alena's mana ability should resolve");

    assert_eq!(
        result.value,
        crate::effect::OutcomeValue::ManaAdded(vec![
            ManaSymbol::Red,
            ManaSymbol::Red,
            ManaSymbol::Red,
            ManaSymbol::Red,
            ManaSymbol::Red,
        ])
    );
    assert_eq!(game.player(alice).expect("alice").mana_pool.red, 5);

    game.player_mut(alice).expect("alice").mana_pool.red = 0;
    game.turn_store.turn_history.clear_for_new_turn();

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(alena_id, alice, &mut dm);
    let result = add_scaled
        .execute(&mut game, &mut ctx)
        .expect("Alena's mana ability should resolve with no entered creatures");

    assert_eq!(result.value, crate::effect::OutcomeValue::ManaAdded(vec![]));
    assert_eq!(game.player(alice).expect("alice").mana_pool.red, 0);
}

#[test]
pub(super) fn selvala_eager_trailblazer_strict_parser_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Selvala, Eager Trailblazer");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Vigilance")
            && rendered.contains("Whenever you cast a creature spell")
            && rendered.contains(
                "You choose a color. Add one mana of the chosen color for each different power among creatures you control"
            ),
        "expected Selvala's keyword, token trigger, and distinct-power mana clause to render, got {rendered}"
    );
    assert!(
        ability_debug.contains("ChooseColorEffect")
            && ability_debug.contains("AddManaOfChosenColorEffect")
            && ability_debug.contains("DistinctPowers"),
        "expected Selvala's mana ability to choose a color and scale by distinct powers, got {ability_debug}"
    );
}

#[test]
pub(super) fn selvala_eager_trailblazer_mana_runtime_counts_distinct_controlled_powers() {
    fn selvala_definition() -> CardDefinition {
        let oracle = oracle_text_by_name()
            .get("Selvala, Eager Trailblazer")
            .expect("Selvala oracle text should be present")
            .clone();
        CardDefinitionBuilder::new(CardId::new(), "Selvala, Eager Trailblazer")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 5))
            .parse_text(oracle)
            .expect("Selvala should parse for runtime regression")
    }

    fn create_creature(
        game: &mut crate::game_state::GameState,
        controller: PlayerId,
        name: &str,
        power: i32,
    ) {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 1))
            .build();
        game.create_object_from_definition(&def, controller, Zone::Battlefield);
    }

    let def = selvala_definition();
    let add_chosen = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_mana_ability() => {
                activated.effects.iter().find_map(|effect| {
                    effect.downcast_ref::<crate::effects::AddManaOfChosenColorEffect>()
                })
            }
            _ => None,
        })
        .expect("Selvala should have a chosen-color mana effect");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let selvala_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    create_creature(&mut game, alice, "One-Power Scout", 1);
    create_creature(&mut game, alice, "Duplicate-Power Ally", 4);
    create_creature(&mut game, bob, "Opposing Giant", 7);
    game.set_chosen_color(selvala_id, Color::Blue);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(selvala_id, alice, &mut dm);
    let result = add_chosen
        .execute(&mut game, &mut ctx)
        .expect("Selvala's mana effect should resolve");

    assert_eq!(
        result.value,
        crate::effect::OutcomeValue::Count(2),
        "Selvala should add one mana for each distinct controlled power, ignoring duplicates and opponents"
    );
    assert_eq!(game.player(alice).expect("alice").mana_pool.blue, 2);

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let selvala_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.set_chosen_color(selvala_id, Color::Red);
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(selvala_id, alice, &mut dm);
    let result = add_chosen
        .execute(&mut game, &mut ctx)
        .expect("Selvala's mana effect should count Selvala herself");

    assert_eq!(
        result.value,
        crate::effect::OutcomeValue::Count(1),
        "Selvala should count her own power when she is the only creature you control"
    );
    assert_eq!(game.player(alice).expect("alice").mana_pool.red, 1);
}

#[test]
pub(super) fn selvala_eager_trailblazer_creature_spell_trigger_creates_mercenary_token() {
    fn spell_def(name: &str, card_types: Vec<CardType>) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build()
    }

    fn spell_cast_event(spell: ObjectId, caster: PlayerId) -> crate::triggers::TriggerEvent {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        )
    }

    let def = parse_oracle_card_definition("Selvala, Eager Trailblazer");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let selvala_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let instant = spell_def("Not a Creature", vec![CardType::Instant]);
    let instant_id = game.create_object_from_definition(&instant, alice, Zone::Stack);
    let instant_event = spell_cast_event(instant_id, alice);
    assert!(
        crate::triggers::check_triggers(&game, &instant_event).is_empty(),
        "Selvala should not trigger from your noncreature spell"
    );

    let opponent_creature = spell_def("Opponent Creature", vec![CardType::Creature]);
    let opponent_creature_id =
        game.create_object_from_definition(&opponent_creature, bob, Zone::Stack);
    let opponent_event = spell_cast_event(opponent_creature_id, bob);
    assert!(
        crate::triggers::check_triggers(&game, &opponent_event).is_empty(),
        "Selvala should not trigger from an opponent's creature spell"
    );

    let creature = spell_def("Your Creature", vec![CardType::Creature]);
    let creature_id = game.create_object_from_definition(&creature, alice, Zone::Stack);
    let creature_event = spell_cast_event(creature_id, alice);
    let triggered = crate::triggers::check_triggers(&game, &creature_event);
    assert_eq!(
        triggered.len(),
        1,
        "Selvala should trigger once from your creature spell"
    );

    let entry = &triggered[0];
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(selvala_id, alice, &mut dm)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Selvala's token trigger should resolve");
    }

    let mercenaries = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|&id| id != selvala_id)
        .filter_map(|id| game.object(id))
        .filter(|object| {
            object.card_types == [CardType::Creature]
                && object.subtypes == [Subtype::Mercenary]
                && object.color_override == Some(crate::color::ColorSet::RED)
                && object.base_power == Some(crate::card::PtValue::Fixed(1))
                && object.base_toughness == Some(crate::card::PtValue::Fixed(1))
                && game.controller_of(object) == alice
                && object
                    .abilities
                    .iter()
                    .any(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        })
        .count();
    assert_eq!(
        mercenaries, 1,
        "Selvala should create one 1/1 red Mercenary token with an activated ability"
    );
}

pub(super) fn dovescape_test_spell(
    name: &str,
    card_types: Vec<CardType>,
    mana_cost: ManaCost,
) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .mana_cost(mana_cost)
        .build()
}

pub(super) fn dovescape_spell_cast_event(
    spell: ObjectId,
    caster: PlayerId,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    )
}

pub(super) fn spell_cast_event_with_current_snapshot(
    game: &crate::game_state::GameState,
    spell: ObjectId,
    caster: PlayerId,
) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell)
            .expect("spell object should exist for cast snapshot"),
        game,
    );
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            spell,
            caster,
            Zone::Hand,
            snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
pub(super) fn vexing_bauble_retargets_no_mana_condition_to_triggering_spell() {
    let def = parse_oracle_card_definition("Vexing Bauble");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        ability_debug.contains("TriggeringSpellManaSpentToCastAtLeast")
            && !ability_debug.contains("TargetSpellManaSpentToCastAtLeast")
            && ability_debug.contains("CounterEffect")
            && ability_debug.contains("triggering"),
        "Vexing Bauble should test mana spent on the triggering spell, got {ability_debug}"
    );
}

#[test]
pub(super) fn vexing_bauble_does_not_counter_spells_cast_with_mana() {
    let def = parse_oracle_card_definition("Vexing Bauble");
    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Bob's Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Instant])
        .build();

    let mut paid_game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let paid_bauble = paid_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let paid_spell = paid_game.create_object_from_definition(&spell_def, bob, Zone::Stack);
    paid_game
        .object_mut(paid_spell)
        .expect("paid spell should exist")
        .mana_spent_to_cast = crate::player::ManaPool {
        colorless: 1,
        ..crate::player::ManaPool::default()
    };
    paid_game.push_to_stack(crate::game_state::StackEntry::new(paid_spell, bob));
    let paid_event = spell_cast_event_with_current_snapshot(&paid_game, paid_spell, bob);
    let paid_triggers = crate::triggers::check_triggers(&paid_game, &paid_event);
    assert_eq!(
        paid_triggers
            .iter()
            .filter(|entry| entry.source == paid_bauble)
            .count(),
        0,
        "Vexing Bauble should not trigger for spells cast with mana"
    );

    let mut free_game = crate::tests::test_helpers::setup_two_player_game();
    let free_bauble = free_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let free_spell = free_game.create_object_from_definition(&spell_def, bob, Zone::Stack);
    let free_stable_id = free_game
        .object(free_spell)
        .expect("free spell should exist")
        .stable_id;
    free_game.push_to_stack(crate::game_state::StackEntry::new(free_spell, bob));
    let free_event = spell_cast_event_with_current_snapshot(&free_game, free_spell, bob);
    let free_triggers = crate::triggers::check_triggers(&free_game, &free_event);
    assert_eq!(
        free_triggers
            .iter()
            .filter(|entry| entry.source == free_bauble)
            .count(),
        1,
        "Vexing Bauble should trigger for spells cast without spending mana"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in free_triggers {
        trigger_queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut free_game, &mut trigger_queue)
        .expect("Vexing Bauble trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut free_game)
        .expect("Vexing Bauble trigger should resolve");

    let moved_spell = free_game
        .find_object_by_stable_id(free_stable_id)
        .expect("countered spell should still be tracked");
    assert_eq!(
        free_game
            .object(moved_spell)
            .expect("countered spell should still exist")
            .zone,
        Zone::Graveyard,
        "Vexing Bauble should counter the unpaid spell"
    );
}

#[test]
pub(super) fn dovescape_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Dovescape");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Dovescape should parse its noncreature-spell trigger strictly"
    );
    assert!(
        ability_debug.contains("CounterEffect")
            && ability_debug.contains("CreateTokenEffect")
            && ability_debug.contains("ManaValueOf")
            && ability_debug.contains("triggering")
            && ability_debug.contains("WhereXIs"),
        "Dovescape should structurally counter the triggering spell and create X Birds from its mana value, got {ability_debug}"
    );
    assert!(
        rendered.contains("whenever a player casts a noncreature spell, counter it")
            && rendered.contains(
                "that player creates x 1/1 white and blue bird creature tokens with flying, where x is that spell's mana value"
            ),
        "Dovescape compiled text should preserve the where-X token clause, got {rendered}"
    );
}

#[test]
pub(super) fn dovescape_counters_noncreature_spell_and_creates_birds_equal_to_mana_value() {
    let def = parse_oracle_card_definition("Dovescape");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let dovescape_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let spell_def = dovescape_test_spell(
        "Bob's Probe",
        vec![CardType::Instant],
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Blue]]),
    );
    let spell_id = game.create_object_from_definition(&spell_def, bob, Zone::Stack);
    let spell_stable_id = game
        .object(spell_id)
        .expect("Bob's spell should exist on the stack")
        .stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(spell_id, bob));

    let event = dovescape_spell_cast_event(spell_id, bob);
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggers
            .iter()
            .filter(|entry| entry.source == dovescape_id)
            .count(),
        1,
        "Dovescape should trigger once for a noncreature spell"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Dovescape trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Dovescape trigger should resolve");

    let moved_spell_id = game
        .find_object_by_stable_id(spell_stable_id)
        .expect("countered spell should still be findable by stable id");
    assert_eq!(
        game.object(moved_spell_id)
            .expect("countered spell should still exist")
            .zone,
        Zone::Graveyard,
        "Dovescape should counter the triggering spell into its owner's graveyard"
    );
    assert!(
        !game.stack.iter().any(|entry| entry.object_id == spell_id),
        "countered spell should no longer be on the stack"
    );

    let bird_tokens = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|&id| id != dovescape_id)
        .filter_map(|id| game.object(id).map(|object| (id, object)))
        .filter(|(_, object)| {
            matches!(object.kind, crate::object::ObjectKind::Token)
                && object.name == "Bird"
                && object.subtypes.contains(&Subtype::Bird)
                && game.controller_of(object) == bob
        })
        .collect::<Vec<_>>();
    assert_eq!(
        bird_tokens.len(),
        3,
        "Dovescape should create one Bird per mana value of the triggering spell"
    );
    for (token_id, token) in bird_tokens {
        assert_eq!(game.current_power(token_id), Some(1));
        assert_eq!(game.current_toughness(token_id), Some(1));
        assert_eq!(
            token.colors(),
            crate::color::ColorSet::WHITE.union(crate::color::ColorSet::BLUE)
        );
        assert!(
            game.object_has_static_ability_id(token_id, StaticAbilityId::Flying),
            "Dovescape's Bird tokens should have flying"
        );
    }
}

#[test]
pub(super) fn dovescape_does_not_trigger_for_creature_spells() {
    let def = parse_oracle_card_definition("Dovescape");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let dovescape_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let creature_def = dovescape_test_spell(
        "Bob's Creature",
        vec![CardType::Creature],
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]),
    );
    let creature_id = game.create_object_from_definition(&creature_def, bob, Zone::Stack);

    let event = dovescape_spell_cast_event(creature_id, bob);
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggers
            .iter()
            .filter(|entry| entry.source == dovescape_id)
            .count(),
        0,
        "Dovescape should not trigger for creature spells"
    );
}

#[test]
pub(super) fn when_we_were_young_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("When We Were Young");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("Up to two target creatures each get +2/+2 until end of turn"),
        "expected up-to-two pump clause to render, got {rendered}"
    );
    assert!(
        rendered.contains(
            "If you control an artifact and an enchantment, those creatures also gain lifelink until end of turn"
        ),
        "expected conditional lifelink clause to render, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(
        compact_debug.contains("choicecount{min:0,max:some(2")
            && debug.contains("and(")
            && debug.contains("artifact")
            && debug.contains("enchantment")
            && debug.contains("lifelink"),
        "expected structural up-to-two targets plus artifact/enchantment lifelink condition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn path_of_the_pyromancer_strict_parser_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Path of the Pyromancer");
    let rendered = canonical_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Discard all the cards in your hand. Add {R} for each card discarded this way, then draw that many cards plus one"
        ),
        "expected discarded-this-way mana and draw clause to render, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Will of the Planeswalkers — Starting with you, each player votes for planeswalk or chaos"
        ),
        "expected planeswalker vote clause to render, got {rendered}"
    );
    assert!(
        rendered.contains("If chaos gets more votes or the vote is tied, chaos ensues"),
        "expected tied chaos branch to render, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("DiscardEffect")
            && debug.contains("AddScaledManaEffect")
            && debug.contains("EffectMetric")
            && debug.contains("VoteEffect")
            && debug.contains("Planeswalk")
            && debug.contains("ChaosEnsues"),
        "expected discard-count mana, vote, and planar keyword actions structurally, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct PathVoteDecisionMaker {
    pub(super) votes: Vec<usize>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for PathVoteDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if !self.votes.is_empty() {
            vec![self.votes.remove(0)]
        } else {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn vanilla_sorcery_for_path_test(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_path_of_the_pyromancer_with_votes(
    votes: Vec<usize>,
) -> (
    crate::game_state::GameState,
    Vec<crate::triggers::TriggerEvent>,
) {
    let def = parse_oracle_card_definition("Path of the Pyromancer");
    let program = def
        .spell_effect
        .as_ref()
        .expect("Path of the Pyromancer should compile to spell effects");
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let current_plane =
        CardDefinitionBuilder::new(CardId::from_raw(91_102), "Path Test Plane").build();
    let next_plane =
        CardDefinitionBuilder::new(CardId::from_raw(91_103), "Path Test Destination").build();
    let current_plane_id = game.create_object_from_definition(&current_plane, alice, Zone::Command);
    let next_plane_id = game.create_object_from_definition(&next_plane, alice, Zone::Command);
    game.planechase = Some(crate::game_state::PlanechaseState {
        decks: std::collections::HashMap::from([(alice, vec![next_plane_id, current_plane_id])]),
        communal_deck: None,
        deck_owners: std::collections::HashMap::from([
            (current_plane_id, alice),
            (next_plane_id, alice),
        ]),
        card_kinds: std::collections::HashMap::from([
            (current_plane_id, crate::game_state::PlanarCardKind::Plane),
            (next_plane_id, crate::game_state::PlanarCardKind::Plane),
        ]),
        face_up: Vec::new(),
        planar_controller: alice,
        planar_controllers: std::collections::HashSet::from([alice]),
        face_up_controllers: std::collections::HashMap::new(),
        voluntary_rolls_this_turn: std::collections::HashMap::new(),
        planeswalk_count: 0,
    });
    game.reveal_starting_plane()
        .expect("Path runtime fixture should reveal its starting plane");
    let filler = vanilla_sorcery_for_path_test(91_101, "Path Filler");
    for _ in 0..3 {
        game.create_object_from_definition(&filler, alice, Zone::Hand);
    }
    for _ in 0..4 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = PathVoteDecisionMaker { votes };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let mut events = crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Path of the Pyromancer should resolve");
    // Planeswalking performs a real game-state transition and queues its
    // observable keyword event on GameState. This direct-program harness does
    // not run the stack-resolution drain that production play does.
    events.extend(game.take_pending_trigger_events());
    (game, events)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn keyword_action_count(
    events: &[crate::triggers::TriggerEvent],
    action: crate::events::KeywordActionKind,
) -> usize {
    events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::KeywordActionEvent>())
        .filter(|event| event.action == action)
        .count()
}

fn first_created_token_definition(definition: &CardDefinition) -> CardDefinition {
    fn from_effect(effect: &crate::effect::Effect) -> Option<CardDefinition> {
        if let Some(create) = effect.downcast_ref::<CreateTokenEffect>() {
            return Some(create.token.clone());
        }
        if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
            return from_effect(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
            return from_effect(&with_id.effect);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().find_map(from_effect);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
            return may.effects.iter().find_map(from_effect);
        }
        if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
            return for_each.effects.iter().find_map(from_effect);
        }
        if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
            return for_each.effects.iter().find_map(from_effect);
        }
        if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
            return for_players.effects.iter().find_map(from_effect);
        }
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
            return conditional
                .if_true
                .iter()
                .chain(&conditional.if_false)
                .find_map(from_effect);
        }
        if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
            return if_effect
                .then
                .iter()
                .chain(&if_effect.else_)
                .find_map(from_effect);
        }
        None
    }

    if let Some(program) = &definition.spell_effect
        && let Some(token) = program.all_effects().into_iter().find_map(from_effect)
    {
        return token;
    }
    for ability in &definition.abilities {
        let program = match &ability.kind {
            AbilityKind::Activated(activated) => &activated.effects,
            AbilityKind::Triggered(triggered) => &triggered.effects,
            AbilityKind::Static(_) => continue,
        };
        if let Some(token) = program.all_effects().into_iter().find_map(from_effect) {
            return token;
        }
    }
    panic!("{} should create a token", definition.card.name);
}

#[test]
pub(super) fn spirit_token_reciprocal_blocking_cards_compile_one_typed_rule_surface() {
    const RULE: &str = "This token can't block or be blocked by non-Spirit creatures.";
    for name in [
        "Baboon Spirit",
        "Foggy Swamp Spirit Keeper",
        "Hei Bai, Forest Guardian",
        "Lost in the Spirit World",
        "Realm of Koh",
    ] {
        let definition = parse_oracle_card_definition(name);
        let token = first_created_token_definition(&definition);
        let rendered = canonical_compiled_lines(&definition).join("\n");
        let rule_abilities = token
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::RuleRestriction =>
                {
                    Some(static_ability)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            rule_abilities.len(),
            1,
            "{name} should give its Spirit token one compound rule ability: {:#?}",
            token.abilities
        );
        assert_eq!(rule_abilities[0].display(), RULE, "unexpected {name} rule");
        assert!(
            token.abilities.iter().all(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => !matches!(
                    static_ability.id(),
                    StaticAbilityId::CantBlock | StaticAbilityId::Unblockable
                ),
                _ => true,
            }),
            "{name} must not infer unconditional blocking abilities: {:#?}",
            token.abilities
        );

        let rule_debug = format!("{:#?}", rule_abilities[0]);
        assert_eq!(
            rule_debug.matches("BlockSpecificAttacker").count(),
            2,
            "{name} should lower both blocking directions: {rule_debug}"
        );
        assert_eq!(
            rule_debug.matches("excluded_subtypes: [Spirit]").count(),
            2,
            "{name} should type both non-Spirit filters: {rule_debug}"
        );
        assert!(
            rendered.contains(&format!(
                "1/1 colorless Spirit creature token with \"{RULE}\""
            )),
            "{name} should round-trip the single quoted rule: {rendered}"
        );
        assert!(
            !rendered.contains("\"This token can't block.\"")
                && !rendered.contains("\"This token can't be blocked.\""),
            "{name} should not render unconditional fallback rules: {rendered}"
        );
    }
}

#[test]
pub(super) fn spirit_token_reciprocal_blocking_rule_enforces_both_directions() {
    fn creature(name: &str, subtype: Subtype) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    let realm = parse_oracle_card_definition("Realm of Koh");
    let token = first_created_token_definition(&realm);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let token_id = game.create_object_from_definition(&token, alice, Zone::Battlefield);
    let non_spirit_attacker_id = game.create_object_from_definition(
        &creature("Non-Spirit Attacker", Subtype::Pirate),
        bob,
        Zone::Battlefield,
    );
    let spirit_attacker_id = game.create_object_from_definition(
        &creature("Spirit Attacker", Subtype::Spirit),
        bob,
        Zone::Battlefield,
    );
    let non_spirit_blocker_id = game.create_object_from_definition(
        &creature("Non-Spirit Blocker", Subtype::Pirate),
        bob,
        Zone::Battlefield,
    );
    let spirit_blocker_id = game.create_object_from_definition(
        &creature("Spirit Blocker", Subtype::Spirit),
        bob,
        Zone::Battlefield,
    );
    game.refresh_continuous_state();

    assert!(
        !crate::rules::combat::can_block(
            game.object(non_spirit_attacker_id)
                .expect("non-Spirit attacker exists"),
            game.object(token_id).expect("Spirit token exists"),
            &game,
        ),
        "the Spirit token must not block a non-Spirit attacker"
    );
    assert!(
        crate::rules::combat::can_block(
            game.object(spirit_attacker_id)
                .expect("Spirit attacker exists"),
            game.object(token_id).expect("Spirit token exists"),
            &game,
        ),
        "the Spirit token should be allowed to block a Spirit attacker"
    );
    assert!(
        !crate::rules::combat::can_block(
            game.object(token_id).expect("Spirit token exists"),
            game.object(non_spirit_blocker_id)
                .expect("non-Spirit blocker exists"),
            &game,
        ),
        "a non-Spirit creature must not block the Spirit token"
    );
    assert!(
        crate::rules::combat::can_block(
            game.object(token_id).expect("Spirit token exists"),
            game.object(spirit_blocker_id)
                .expect("Spirit blocker exists"),
            &game,
        ),
        "a Spirit creature should be allowed to block the Spirit token"
    );
}
