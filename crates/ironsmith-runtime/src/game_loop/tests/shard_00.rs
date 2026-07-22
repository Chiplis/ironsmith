#![allow(unused_imports)]
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
use super::shard_17::*;
use super::*;

#[test]
pub(super) fn targeted_cast_refreshes_after_proposal_metadata_before_wide_target_queries() {
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let creatures = (0..96)
        .map(|index| create_creature(&mut game, &format!("Target {index}"), alice, 2, 2))
        .collect::<Vec<_>>();
    for modification in [
        Modification::AddAbility(StaticAbility::flying()),
        Modification::AddAbility(StaticAbility::vigilance()),
        Modification::ModifyPowerToughness {
            power: 1,
            toughness: 1,
        },
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 2,
        },
    ] {
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                creatures[0],
                alice,
                EffectTarget::AllCreatures,
                modification,
            ));
    }

    let target_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
        crate::target::ObjectFilter::creature(),
    ));
    let spell = CardDefinitionBuilder::new(CardId::from_raw(991_001), "Wide Target Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::destroy(target_spec)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Green, 1);
    game.refresh_continuous_state();

    let before = game.work_counters();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("targeted cast should reach target selection");
    let after = game.work_counters();

    let targets = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            ctx,
        )) => ctx
            .requirements
            .into_iter()
            .flat_map(|requirement| requirement.legal_targets)
            .collect::<Vec<_>>(),
        other => panic!("expected target selection, got {other:?}"),
    };
    assert_eq!(targets.len(), creatures.len());
    let pending = state
        .pending_cast
        .as_ref()
        .expect("cast should remain pending");
    assert!(
        pending
            .optional_costs_paid
            .was_paid_label("CastDuringYourMainPhase"),
        "proposal metadata should be initialized before its single continuous-state refresh"
    );
    assert!(game.continuous_state_is_clean());
    assert!(
        after.characteristics_full_recomputes <= before.characteristics_full_recomputes + 2,
        "cast-time spell inspection and target enumeration should use batched characteristics: before={before:?}, after={after:?}"
    );
    assert!(
        after.dependency_sorts <= before.dependency_sorts + 4,
        "cast-time dependency sorting should be bounded by effect layers, not target count: before={before:?}, after={after:?}"
    );
    assert!(
        after.dependency_pairs_probed <= before.dependency_pairs_probed + 64,
        "cast-time dependency probes should scale with effects, not battlefield width: before={before:?}, after={after:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn aligned_heart_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(836_968_406), "Aligned Heart")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Flurry — Whenever you cast your second spell each turn, put a rally counter on this enchantment. Then create a 1/1 white Monk creature token with prowess for each rally counter on it.",
        )
        .expect("Aligned Heart should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn aligned_heart_monk_tokens(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token
                    && game.controller_of(object) == controller
                    && object.card_types.contains(&CardType::Creature)
                    && object.subtypes.contains(&Subtype::Monk)
            })
        })
        .collect()
}

pub(super) fn will_kenrith_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(567_555), "Will Kenrith")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .parse_text(
            "+2: Until your next turn, up to two target creatures each have base power and toughness 0/3 and lose all abilities.\n\
             -2: Target player draws two cards. Until your next turn, instant, sorcery, and planeswalker spells that player casts cost {2} less to cast.\n\
             -8: Target player gets an emblem with \"Whenever you cast an instant or sorcery spell, copy it. You may choose new targets for the copy.\"\n\
             Partner with Rowan Kenrith\n\
             Will Kenrith can be your commander.",
        )
        .expect("Will Kenrith should parse for runtime tests")
}

pub(super) fn resolve_will_kenrith_loyalty_ability(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    cost_markers: &[&str],
    targets: Vec<crate::effects::ResolvedTarget>,
) {
    let def = will_kenrith_definition();
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                let cost_debug = format!("{:?}", activated.mana_cost);
                cost_markers
                    .iter()
                    .all(|marker| cost_debug.contains(marker))
                    .then_some(activated)
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!("Will Kenrith should have loyalty ability matching {cost_markers:?}")
        });
    let mut ctx =
        crate::effects::ExecutionContext::new_default(source, controller).with_targets(targets);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Will Kenrith loyalty ability effect should resolve");
    }
}

pub(super) fn component_pouch_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(900_071), "Component Pouch")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}, Remove a component counter from this artifact: Add two mana of different colors.\n\
             {T}: Roll a d20.\n\
             1—9 | Put a component counter on this artifact.\n\
             10—20 | Put two component counters on this artifact.",
        )
        .expect("Component Pouch should parse for runtime tests")
}

pub(super) fn component_pouch_mana_ability_index(def: &crate::cards::CardDefinition) -> usize {
    def.abilities
        .iter()
        .position(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    effect
                        .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
                        .is_some_and(|add| add.distinct_colors)
                }),
            _ => false,
        })
        .expect("Component Pouch should have its component-counter mana ability")
}

pub(super) fn component_pouch_d20_ability_index(def: &crate::cards::CardDefinition) -> usize {
    def.abilities
        .iter()
        .position(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    effect
                        .downcast_ref::<crate::effects::WithIdEffect>()
                        .and_then(|with_id| {
                            with_id
                                .effect
                                .downcast_ref::<crate::effects::RollDieEffect>()
                        })
                        .is_some_and(|roll| roll.sides == 20)
                }),
            _ => false,
        })
        .expect("Component Pouch should have its d20 counter ability")
}

#[test]
pub(super) fn component_pouch_mana_activation_requires_counter_pays_cost_and_adds_distinct_mana_runtime()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let def = component_pouch_definition();
    let pouch_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ability_index = component_pouch_mana_ability_index(&def);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, ability_index: idx }
                    if *source == pouch_id && *idx == ability_index
            )),
        "Component Pouch mana ability should be illegal without a component counter"
    );

    game.add_counters(pouch_id, crate::object::CounterType::Named("component"), 1)
        .expect("component counter should be addable to Component Pouch");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, ability_index: idx }
                    if *source == pouch_id && *idx == ability_index
            )
        })
        .expect("Component Pouch mana ability should be legal with a component counter");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Component Pouch mana ability should pay costs and resolve");

    assert_eq!(
        game.counter_count(pouch_id, crate::object::CounterType::Named("component")),
        0,
        "activation cost should remove the component counter"
    );
    assert!(
        game.is_tapped(pouch_id),
        "activation cost should tap Component Pouch"
    );
    let pool = &game.player(alice).expect("Alice should exist").mana_pool;
    let colored_counts = [pool.white, pool.blue, pool.black, pool.red, pool.green];
    assert_eq!(pool.total(), 2, "mana ability should add exactly two mana");
    assert_eq!(
        colored_counts.iter().filter(|&&count| count > 0).count(),
        2,
        "mana ability should add two different colors, got {colored_counts:?}"
    );
}

#[test]
pub(super) fn component_pouch_d20_branches_put_one_or_two_component_counters_runtime() {
    fn resolve_forced_roll(roll: u32) -> u32 {
        let def = component_pouch_definition();
        let ability_index = component_pouch_d20_ability_index(&def);
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let pouch_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.force_next_die_roll(roll);

        let AbilityKind::Activated(activated) = &def.abilities[ability_index].kind else {
            panic!("Component Pouch d20 ability should be activated")
        };
        let mut ctx = crate::effects::ExecutionContext::new_default(pouch_id, alice);
        for effect in activated.effects.flattened_default_effects() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .expect("Component Pouch d20 effect should resolve");
        }
        game.counter_count(pouch_id, crate::object::CounterType::Named("component"))
    }

    assert_eq!(
        resolve_forced_roll(7),
        1,
        "1-9 branch should put one component counter"
    );
    assert_eq!(
        resolve_forced_roll(15),
        2,
        "10-20 branch should put two component counters and not also take the low branch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn myrkuls_edict_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(563_018), "Myrkul's Edict")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Roll a d20.\n\
             1—9 | Choose an opponent. That player sacrifices a creature of their choice.\n\
             10—19 | Each opponent sacrifices a creature of their choice.\n\
             20 | Each opponent sacrifices a creature with the greatest power among creatures that player controls.",
        )
        .expect("Myrkul's Edict should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_myrkul_edict_creature(
    game: &mut GameState,
    name: &str,
    controller: PlayerId,
    power: i32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, power))
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_myrkuls_edict(game: &mut GameState, roll: u32) {
    let def = myrkuls_edict_definition();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.force_next_die_roll(roll);

    let mut dm = SelectFirstDecisionMaker;
    let mut ctx =
        crate::effects::ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
    for effect in def
        .spell_effect
        .as_ref()
        .expect("Myrkul's Edict should have spell effects")
        .flattened_default_effects()
    {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Myrkul's Edict effect should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn myrkul_edict_zone_contains(game: &GameState, zone: Zone, name: &str) -> bool {
    game.objects_in_zone(zone)
        .iter()
        .filter_map(|id| game.object(*id))
        .any(|object| object.name == name)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn myrkuls_edict_low_roll_only_chosen_opponent_sacrifices() {
    let mut game = setup_three_player_game();
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    create_myrkul_edict_creature(&mut game, "Bob Bear", bob, 2);
    create_myrkul_edict_creature(&mut game, "Charlie Bear", charlie, 2);

    resolve_myrkuls_edict(&mut game, 7);

    assert!(
        myrkul_edict_zone_contains(&game, Zone::Graveyard, "Bob Bear"),
        "1-9 should make the chosen opponent sacrifice a creature"
    );
    assert!(
        myrkul_edict_zone_contains(&game, Zone::Battlefield, "Charlie Bear"),
        "1-9 should not make each opponent sacrifice"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn myrkuls_edict_middle_roll_each_opponent_sacrifices_one_creature() {
    let mut game = setup_three_player_game();
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    create_myrkul_edict_creature(&mut game, "Bob Bear", bob, 2);
    create_myrkul_edict_creature(&mut game, "Charlie Bear", charlie, 2);

    resolve_myrkuls_edict(&mut game, 15);

    assert!(
        myrkul_edict_zone_contains(&game, Zone::Graveyard, "Bob Bear"),
        "10-19 should make Bob sacrifice a creature"
    );
    assert!(
        myrkul_edict_zone_contains(&game, Zone::Graveyard, "Charlie Bear"),
        "10-19 should make Charlie sacrifice a creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn myrkuls_edict_twenty_sacrifices_only_each_opponents_greatest_power_creature() {
    let mut game = setup_three_player_game();
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    create_myrkul_edict_creature(&mut game, "Bob Small", bob, 1);
    create_myrkul_edict_creature(&mut game, "Bob Large", bob, 5);
    create_myrkul_edict_creature(&mut game, "Charlie Small", charlie, 2);
    create_myrkul_edict_creature(&mut game, "Charlie Large", charlie, 4);

    resolve_myrkuls_edict(&mut game, 20);

    assert!(
        myrkul_edict_zone_contains(&game, Zone::Graveyard, "Bob Large"),
        "20 should make Bob sacrifice a greatest-power creature"
    );
    assert!(
        myrkul_edict_zone_contains(&game, Zone::Graveyard, "Charlie Large"),
        "20 should make Charlie sacrifice a greatest-power creature"
    );
    assert!(
        myrkul_edict_zone_contains(&game, Zone::Battlefield, "Bob Small"),
        "20 should not allow Bob to sacrifice a lower-power creature"
    );
    assert!(
        myrkul_edict_zone_contains(&game, Zone::Battlefield, "Charlie Small"),
        "20 should not allow Charlie to sacrifice a lower-power creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn reprocess_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(646_813), "Reprocess")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice any number of artifacts, creatures, and/or lands. Draw a card for each \
             permanent sacrificed this way.",
        )
        .expect("Reprocess should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_reprocess_permanent(
    game: &mut GameState,
    name: &str,
    card_types: Vec<CardType>,
    controller: PlayerId,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn add_reprocess_library_cards(game: &mut GameState, owner: PlayerId, count: usize) {
    for idx in 0..count {
        let card = CardBuilder::new(CardId::new(), &format!("Reprocess Draw Card {}", idx + 1))
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, owner, Zone::Library);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ReprocessDecisionMaker {
    pub(super) selected: Vec<ObjectId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ReprocessDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal && self.selected.contains(&candidate.id))
            .map(|candidate| candidate.id)
            .take(ctx.max.unwrap_or(self.selected.len()))
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_reprocess_with_selection(
    game: &mut GameState,
    controller: PlayerId,
    selected: Vec<ObjectId>,
) {
    let reprocess = reprocess_definition();
    let source = game.create_object_from_definition(&reprocess, controller, Zone::Stack);
    let mut dm = ReprocessDecisionMaker { selected };
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller)
        .with_decision_maker(&mut dm);

    for effect in reprocess
        .spell_effect
        .as_ref()
        .expect("Reprocess should have spell effects")
        .flattened_default_effects()
    {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Reprocess effect should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reprocess_sacrifices_selected_controlled_permanents_and_draws_that_many() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_reprocess_library_cards(&mut game, alice, 3);

    let artifact = create_reprocess_permanent(
        &mut game,
        "Reprocess Artifact",
        vec![CardType::Artifact],
        alice,
    );
    let land = create_reprocess_permanent(&mut game, "Reprocess Land", vec![CardType::Land], alice);
    let creature = create_reprocess_permanent(
        &mut game,
        "Reprocess Creature",
        vec![CardType::Creature],
        alice,
    );
    let bob_artifact =
        create_reprocess_permanent(&mut game, "Bob's Artifact", vec![CardType::Artifact], bob);

    resolve_reprocess_with_selection(&mut game, alice, vec![artifact, land, bob_artifact]);

    let graveyard_names = game
        .objects_in_zone(Zone::Graveyard)
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();

    assert!(
        graveyard_names
            .iter()
            .any(|name| name == "Reprocess Artifact"),
        "selected artifact should be sacrificed into a graveyard, got {graveyard_names:?}"
    );
    assert!(
        graveyard_names.iter().any(|name| name == "Reprocess Land"),
        "selected land should be sacrificed into a graveyard, got {graveyard_names:?}"
    );
    assert_eq!(
        game.object(creature).expect("unselected creature").zone,
        Zone::Battlefield,
        "unselected controlled permanents should remain on the battlefield"
    );
    assert_eq!(
        game.object(bob_artifact).expect("opponent artifact").zone,
        Zone::Battlefield,
        "Reprocess should not choose or sacrifice permanents controlled by another player"
    );
    assert_eq!(
        game.player(alice).expect("Alice").hand.len(),
        2,
        "Reprocess should draw one card for each permanent sacrificed this way"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reprocess_can_choose_zero_and_draws_no_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    add_reprocess_library_cards(&mut game, alice, 2);

    let artifact = create_reprocess_permanent(
        &mut game,
        "Reprocess Artifact",
        vec![CardType::Artifact],
        alice,
    );

    resolve_reprocess_with_selection(&mut game, alice, Vec::new());

    assert_eq!(
        game.object(artifact).expect("artifact").zone,
        Zone::Battlefield,
        "choosing zero permanents should leave available permanents untouched"
    );
    assert_eq!(
        game.player(alice).expect("Alice").hand.len(),
        0,
        "choosing zero permanents should not draw cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn necromancers_covenant_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(405_317), "Necromancer's Covenant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, exile all creature cards from target player's graveyard, then create a 2/2 black Zombie creature token for each card exiled this way.\n\
             Zombies you control have lifelink.",
        )
        .expect("Necromancer's Covenant should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_necromancers_covenant_graveyard_card(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    card_types: Vec<CardType>,
) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), name).card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    let card = builder.build();
    game.create_object_from_card(&card, owner, Zone::Graveyard)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_necromancers_covenant_etb(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    target_player: PlayerId,
) {
    let covenant = necromancers_covenant_definition();
    let triggered = covenant
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Necromancer's Covenant should have an enters trigger");
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(target_player)]);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Necromancer's Covenant ETB effect should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn necromancers_covenant_zombie_tokens(
    game: &GameState,
    controller: PlayerId,
) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token
                    && game.controller_of(object) == controller
                    && object.card_types.contains(&CardType::Creature)
                    && object.subtypes.contains(&Subtype::Zombie)
            })
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necromancers_covenant_exiles_target_graveyard_creatures_and_creates_that_many_zombies()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let covenant = necromancers_covenant_definition();
    let source = game.create_object_from_definition(&covenant, alice, Zone::Battlefield);

    create_necromancers_covenant_graveyard_card(
        &mut game,
        "Bob Graveyard Creature One",
        bob,
        vec![CardType::Creature],
    );
    create_necromancers_covenant_graveyard_card(
        &mut game,
        "Bob Graveyard Creature Two",
        bob,
        vec![CardType::Creature],
    );
    let bob_noncreature = create_necromancers_covenant_graveyard_card(
        &mut game,
        "Bob Graveyard Artifact",
        bob,
        vec![CardType::Artifact],
    );
    let alice_creature = create_necromancers_covenant_graveyard_card(
        &mut game,
        "Alice Graveyard Creature",
        alice,
        vec![CardType::Creature],
    );

    resolve_necromancers_covenant_etb(&mut game, source, alice, bob);

    let exiled_names = game
        .objects_in_zone(Zone::Exile)
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    assert!(
        exiled_names
            .iter()
            .any(|name| name == "Bob Graveyard Creature One")
            && exiled_names
                .iter()
                .any(|name| name == "Bob Graveyard Creature Two"),
        "target player's creature cards should be exiled, got {exiled_names:?}"
    );
    assert_eq!(
        game.object(bob_noncreature).expect("Bob artifact").zone,
        Zone::Graveyard,
        "noncreature cards in the target graveyard should stay in the graveyard"
    );
    assert_eq!(
        game.object(alice_creature).expect("Alice creature").zone,
        Zone::Graveyard,
        "creature cards in non-target graveyards should stay in the graveyard"
    );

    let zombies = necromancers_covenant_zombie_tokens(&game, alice);
    assert_eq!(
        zombies.len(),
        2,
        "controller should create one Zombie for each card exiled this way"
    );
    for zombie in zombies {
        let object = game.object(zombie).expect("Zombie token should exist");
        assert_eq!(object.power(), Some(2));
        assert_eq!(object.toughness(), Some(2));
        assert!(
            game.current_has_static_ability_id(
                zombie,
                crate::static_abilities::StaticAbilityId::Lifelink,
            ),
            "Necromancer's Covenant should grant lifelink to Zombies you control"
        );
    }
    assert!(
        necromancers_covenant_zombie_tokens(&game, bob).is_empty(),
        "target player should not create the Zombies"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necromancers_covenant_creates_no_zombies_when_target_graveyard_has_no_creature_cards()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let covenant = necromancers_covenant_definition();
    let source = game.create_object_from_definition(&covenant, alice, Zone::Battlefield);
    let bob_artifact = create_necromancers_covenant_graveyard_card(
        &mut game,
        "Bob Graveyard Artifact",
        bob,
        vec![CardType::Artifact],
    );

    resolve_necromancers_covenant_etb(&mut game, source, alice, bob);

    assert_eq!(
        game.object(bob_artifact).expect("Bob artifact").zone,
        Zone::Graveyard,
        "noncreature cards should not be exiled"
    );
    assert!(
        necromancers_covenant_zombie_tokens(&game, alice).is_empty(),
        "no cards exiled this way should create no Zombie tokens"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn tide_of_war_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(78_606), "Tide of War")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever one or more creatures block, flip a coin. If you win the flip, each blocking creature is sacrificed by its controller. If you lose the flip, each blocked creature is sacrificed by its controller.",
        )
        .expect("Tide of War should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_tide_of_war_creature(
    game: &mut GameState,
    name: &str,
    controller: PlayerId,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_tide_of_war_combat_state(
    game: &mut GameState,
    attacker: ObjectId,
    blocker: ObjectId,
    defending_player: PlayerId,
) {
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(defending_player),
    });
    combat.blockers.insert(attacker, vec![blocker]);
    game.combat = Some(combat);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_tide_of_war_trigger(
    game: &mut GameState,
    tide_id: ObjectId,
    controller: PlayerId,
    attacker: ObjectId,
    blocker: ObjectId,
) {
    use crate::effects::{ExecutionContext, execute_effect};

    let tide = tide_of_war_definition();
    let triggered = tide
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Tide of War should have one triggered ability");
    let blocker_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(blocker)
            .expect("blocking creature should exist"),
        game,
    );
    let attacker_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(attacker)
            .expect("blocked creature should exist"),
        game,
    );
    let trigger_event = TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureBlockedEvent::with_snapshots(
            blocker,
            attacker,
            blocker_snapshot,
            attacker_snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx =
        ExecutionContext::new_default(tide_id, controller).with_triggering_event(trigger_event);
    for effect in triggered.effects.flattened_default_effects() {
        execute_effect(game, effect, &mut ctx).expect("Tide of War trigger effect should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tide_of_war_queues_once_when_multiple_creatures_block() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let tide = tide_of_war_definition();
    let _ = game.create_object_from_definition(&tide, alice, Zone::Battlefield);
    let attacker_one = create_tide_of_war_creature(&mut game, "Blocked Attacker One", bob);
    let attacker_two = create_tide_of_war_creature(&mut game, "Blocked Attacker Two", bob);
    let blocker_one = create_tide_of_war_creature(&mut game, "Blocking Creature One", alice);
    let blocker_two = create_tide_of_war_creature(&mut game, "Blocking Creature Two", alice);

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_one,
        target: AttackTarget::Player(alice),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_two,
        target: AttackTarget::Player(alice),
    });
    let mut trigger_queue = TriggerQueue::new();

    apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[
            BlockerDeclaration {
                blocker: blocker_one,
                blocking: attacker_one,
            },
            BlockerDeclaration {
                blocker: blocker_two,
                blocking: attacker_two,
            },
        ],
        alice,
    )
    .expect("Tide of War blockers should be legal");

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Tide of War should trigger once for one or more creatures blocking"
    );
    assert_eq!(
        trigger_queue.entries[0].ability.trigger.display(),
        "Whenever one or more creatures block"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tide_of_war_win_flip_sacrifices_blocking_creatures_only() {
    let mut game = setup_game();
    game.set_random_seed(2);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let tide = tide_of_war_definition();
    let tide_id = game.create_object_from_definition(&tide, alice, Zone::Battlefield);
    let attacker = create_tide_of_war_creature(&mut game, "Blocked Attacker", bob);
    let blocker = create_tide_of_war_creature(&mut game, "Blocking Creature", alice);
    let noncombat = create_tide_of_war_creature(&mut game, "Noncombat Creature", alice);
    put_tide_of_war_combat_state(&mut game, attacker, blocker, alice);

    resolve_tide_of_war_trigger(&mut game, tide_id, alice, attacker, blocker);

    assert!(game.battlefield.contains(&attacker));
    assert!(!game.battlefield.contains(&blocker));
    assert!(game.battlefield.contains(&noncombat));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tide_of_war_lose_flip_sacrifices_blocked_creatures_only() {
    let mut game = setup_game();
    game.set_random_seed(7);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let tide = tide_of_war_definition();
    let tide_id = game.create_object_from_definition(&tide, alice, Zone::Battlefield);
    let attacker = create_tide_of_war_creature(&mut game, "Blocked Attacker", bob);
    let blocker = create_tide_of_war_creature(&mut game, "Blocking Creature", alice);
    let noncombat = create_tide_of_war_creature(&mut game, "Noncombat Creature", bob);
    put_tide_of_war_combat_state(&mut game, attacker, blocker, alice);

    resolve_tide_of_war_trigger(&mut game, tide_id, alice, attacker, blocker);

    assert!(!game.battlefield.contains(&attacker));
    assert!(game.battlefield.contains(&blocker));
    assert!(game.battlefield.contains(&noncombat));
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn the_eternity_elevator_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(645_444), "The Eternity Elevator")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Spacecraft])
        .parse_text(
            "{T}: Add {C}{C}{C}.\n\
             Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery.)\n\
             20+ | {T}: Add X mana of any one color, where X is the number of charge counters on The Eternity Elevator.",
        )
        .expect("The Eternity Elevator should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn the_eternity_elevator_threshold_ability_index(
    def: &crate::cards::CardDefinition,
) -> usize {
    def.abilities
        .iter()
        .position(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    effect
                        .downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>()
                        .is_some()
                }),
            _ => false,
        })
        .expect("The Eternity Elevator should have a threshold any-one-color mana ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn minds_dilation_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(658_545), "Mind's Dilation")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever an opponent casts their first spell each turn, that player exiles the top card of their library. If it's a nonland card, you may cast it without paying its mana cost.",
        )
        .expect("Mind's Dilation should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn nymris_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(658_546), "Nymris, Oona's Trickster")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 6))
        .parse_text(
            "Flash\n\
             Flying\n\
             Whenever you cast your first spell during each opponent's turn, look at the top two cards of your library. Put one of those cards into your hand and the other into your graveyard.",
        )
        .expect("Nymris, Oona's Trickster should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn necrotic_ooze_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(645_500), "Necrotic Ooze")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "As long as this creature is on the battlefield, it has all activated abilities of all creature cards in all graveyards.",
        )
        .expect("Necrotic Ooze should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn graveyard_sage_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(645_501), "Graveyard Sage")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Draw a card.")
        .expect("graveyard creature activated ability should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn graveyard_scroll_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(645_502), "Graveyard Scroll")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Draw a card.")
        .expect("graveyard noncreature activated ability should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn colfenors_urn_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(696_471), "Colfenor's Urn")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a creature with toughness 4 or greater is put into your graveyard from the battlefield, you may exile it.\n\
             At the beginning of the end step, if three or more cards have been exiled with this artifact, sacrifice it. If you do, return those cards to the battlefield under their owner's control.",
        )
        .expect("Colfenor's Urn should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn wondrous_crucible_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(696_480), "Wondrous Crucible")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Permanents you control have ward {2}.\n\
             At the beginning of your end step, mill two cards, then exile a nonland card at random from your graveyard. Copy it. You may cast the copy without paying its mana cost.",
        )
        .expect("Wondrous Crucible should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn wondrous_crucible_spell_definition(
    card_id: u32,
    name: &str,
) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(card_id), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Draw a card.")
        .expect("Wondrous Crucible test spell should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn wondrous_crucible_land_card(card_id: u32, name: &str) -> crate::card::Card {
    CardBuilder::new(CardId::from_raw(card_id), name)
        .card_types(vec![CardType::Land])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_wondrous_crucible_end_step_trigger_on_stack(game: &mut GameState) -> usize {
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(game, &mut trigger_queue);
    put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Wondrous Crucible end-step trigger processing should succeed");
    game.stack.len()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn feast_of_the_victorious_dead_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(111_668), "Feast of the Victorious Dead")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your end step, if one or more creatures died this turn, you gain that much life and distribute that many +1/+1 counters among any number of creatures you control.",
        )
        .expect("Feast of the Victorious Dead should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_feast_end_step_trigger_on_stack(game: &mut GameState) -> usize {
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(game, &mut trigger_queue);
    put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Feast end-step trigger processing should succeed");
    game.stack.len()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseAllLegalObjects;

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseAllLegalObjects {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.max.unwrap_or(usize::MAX))
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feast_of_the_victorious_dead_does_not_trigger_without_a_creature_death() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let feast = feast_of_the_victorious_dead_definition();
    let survivor = vanilla_creature_definition(111_669, "Feast Survivor", 2, 2);
    let survivor_id = game.create_object_from_definition(&survivor, alice, Zone::Battlefield);
    game.create_object_from_definition(&feast, alice, Zone::Battlefield);

    assert_eq!(
        game.turn_store
            .turn_history
            .total_creatures_died_this_turn(),
        0,
        "test setup should start with no creatures dying this turn"
    );
    assert_eq!(
        put_feast_end_step_trigger_on_stack(&mut game),
        0,
        "Feast should not trigger unless a creature died this turn"
    );
    assert_eq!(game.player(alice).expect("Alice exists").life, 20);
    assert_eq!(
        game.counter_count(survivor_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Feast should not distribute counters when its intervening-if condition is false"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feast_of_the_victorious_dead_gains_life_and_distributes_counters_for_creatures_died()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let feast = feast_of_the_victorious_dead_definition();
    game.create_object_from_definition(&feast, alice, Zone::Battlefield);

    let alice_doomed = vanilla_creature_definition(111_670, "Alice's Doomed Creature", 1, 1);
    let bob_doomed = vanilla_creature_definition(111_671, "Bob's Doomed Creature", 1, 1);
    let first_survivor = vanilla_creature_definition(111_672, "First Feast Survivor", 2, 2);
    let second_survivor = vanilla_creature_definition(111_673, "Second Feast Survivor", 2, 2);
    let bob_survivor = vanilla_creature_definition(111_674, "Bob's Feast Survivor", 2, 2);
    let alice_doomed_id =
        game.create_object_from_definition(&alice_doomed, alice, Zone::Battlefield);
    let bob_doomed_id = game.create_object_from_definition(&bob_doomed, bob, Zone::Battlefield);
    let first_survivor_id =
        game.create_object_from_definition(&first_survivor, alice, Zone::Battlefield);
    let second_survivor_id =
        game.create_object_from_definition(&second_survivor, alice, Zone::Battlefield);
    let bob_survivor_id = game.create_object_from_definition(&bob_survivor, bob, Zone::Battlefield);

    game.move_object_by_effect(alice_doomed_id, Zone::Graveyard);
    game.move_object_by_effect(bob_doomed_id, Zone::Graveyard);
    assert_eq!(
        game.turn_store
            .turn_history
            .total_creatures_died_this_turn(),
        2,
        "Feast should count all creatures that died this turn"
    );

    assert_eq!(
        put_feast_end_step_trigger_on_stack(&mut game),
        1,
        "Feast should trigger at the beginning of your end step after creatures died"
    );
    let mut dm = ChooseAllLegalObjects;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Feast trigger should resolve");

    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        22,
        "Feast should gain life equal to the number of creatures that died this turn"
    );
    assert_eq!(
        game.counter_count(
            first_survivor_id,
            crate::object::CounterType::PlusOnePlusOne
        ),
        1,
        "Feast should distribute the first counter to a controlled creature"
    );
    assert_eq!(
        game.counter_count(
            second_survivor_id,
            crate::object::CounterType::PlusOnePlusOne
        ),
        1,
        "Feast should distribute the second counter to another controlled creature"
    );
    assert_eq!(
        game.counter_count(bob_survivor_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Feast should not distribute counters to creatures controlled by opponents"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn vanilla_creature_definition(
    card_id: u32,
    name: &str,
    power: i32,
    toughness: i32,
) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(card_id), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_colfenor_end_step_trigger_on_stack(game: &mut GameState) -> usize {
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(game, &mut trigger_queue);
    put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Colfenor's Urn end-step trigger processing should succeed");
    game.stack.len()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn count_named_objects_in_zone(game: &GameState, zone: Zone, name: &str) -> usize {
    game.objects_in_zone(zone)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| object.name == name)
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activated_ability_count(game: &GameState, object_id: ObjectId) -> usize {
    game.calculated_characteristics(object_id)
        .expect("object should have calculated characteristics")
        .abilities
        .iter()
        .filter(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wondrous_crucible_grants_ward_two_to_permanents_you_control_runtime() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let crucible = wondrous_crucible_definition();
    game.create_object_from_definition(&crucible, alice, Zone::Battlefield);
    let protected = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(696_481), "Protected Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );

    let opponent_ward = crate::targeting::collect_ward_costs(&game, &[protected], bob);
    assert_eq!(opponent_ward.len(), 1, "opponent targeting should see ward");
    assert_eq!(opponent_ward[0].target, protected);
    assert_eq!(opponent_ward[0].ward_controller, alice);
    assert_eq!(opponent_ward[0].cost.display(), "{2}");
    assert!(
        crate::targeting::collect_ward_costs(&game, &[protected], alice).is_empty(),
        "ward should not tax the controller's own targeting"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wondrous_crucible_end_step_randomly_exiles_nonland_and_declining_cast_leaves_it_exiled()
 {
    struct DeclineMay;

    impl DecisionMaker for DeclineMay {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let crucible = wondrous_crucible_definition();
    game.create_object_from_definition(&crucible, alice, Zone::Battlefield);
    let spark = wondrous_crucible_spell_definition(696_482, "Crucible Spark");
    game.create_object_from_definition(&spark, alice, Zone::Library);
    let land = wondrous_crucible_land_card(696_483, "Milled Plains");
    game.create_object_from_card(&land, alice, Zone::Library);
    let random_before = game.irreversible_random_count();

    assert_eq!(
        put_wondrous_crucible_end_step_trigger_on_stack(&mut game),
        1,
        "Wondrous Crucible should trigger at the beginning of your end step"
    );
    let mut dm = DeclineMay;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Wondrous Crucible trigger should resolve");

    assert_eq!(
        game.irreversible_random_count(),
        random_before + 1,
        "the at-random graveyard selection should consume match randomness"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Milled Plains"),
        1
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Crucible Spark"),
        1
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Stack, "Crucible Spark"),
        0,
        "declining the may-cast branch should not put a copy on the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wondrous_crucible_end_step_accepting_may_casts_copy_without_paying_mana() {
    struct AcceptMay;

    impl DecisionMaker for AcceptMay {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let crucible = wondrous_crucible_definition();
    game.create_object_from_definition(&crucible, alice, Zone::Battlefield);
    let spark = wondrous_crucible_spell_definition(696_484, "Crucible Spark");
    game.create_object_from_definition(&spark, alice, Zone::Library);
    let land = wondrous_crucible_land_card(696_485, "Milled Island");
    game.create_object_from_card(&land, alice, Zone::Library);

    assert_eq!(
        put_wondrous_crucible_end_step_trigger_on_stack(&mut game),
        1
    );
    let mut dm = AcceptMay;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Wondrous Crucible trigger should resolve");

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Crucible Spark"),
        1
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Stack, "Crucible Spark"),
        1,
        "accepting the may-cast branch should cast a stack copy"
    );
    let copy_id = game
        .stack
        .last()
        .expect("the copied spell should be on the stack")
        .object_id;
    let copy = game.object(copy_id).expect("stack copy object exists");
    assert_eq!(copy.name, "Crucible Spark");
    assert_eq!(copy.zone, Zone::Stack);
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0,
        "the copied spell should be cast without paying its {{3}} mana cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wondrous_crucible_end_step_with_no_nonland_graveyard_card_mills_but_casts_no_copy() {
    struct AcceptMay;

    impl DecisionMaker for AcceptMay {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let crucible = wondrous_crucible_definition();
    game.create_object_from_definition(&crucible, alice, Zone::Battlefield);
    let plains = wondrous_crucible_land_card(696_486, "Only Plains");
    let island = wondrous_crucible_land_card(696_487, "Only Island");
    game.create_object_from_card(&plains, alice, Zone::Library);
    game.create_object_from_card(&island, alice, Zone::Library);

    assert_eq!(
        put_wondrous_crucible_end_step_trigger_on_stack(&mut game),
        1
    );
    let mut dm = AcceptMay;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Wondrous Crucible trigger should resolve");

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Only Plains"),
        1
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Only Island"),
        1
    );
    assert_eq!(
        game.objects_in_zone(Zone::Exile).len(),
        0,
        "without a nonland graveyard card, Wondrous Crucible should exile nothing"
    );
    assert!(
        game.stack.is_empty(),
        "without an exiled nonland card, the copy/may-cast branch should put nothing on the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn colfenors_urn_death_trigger_exiles_only_toughness_four_or_greater_creatures() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let urn = colfenors_urn_definition();
    let urn_id = game.create_object_from_definition(&urn, alice, Zone::Battlefield);

    let small = vanilla_creature_definition(696_472, "Small Creature", 3, 3);
    let large = vanilla_creature_definition(696_473, "Large Creature", 4, 4);
    let small_id = game.create_object_from_definition(&small, alice, Zone::Battlefield);
    let large_id = game.create_object_from_definition(&large, alice, Zone::Battlefield);

    game.move_object_by_effect(small_id, Zone::Graveyard)
        .expect("small creature should move to graveyard");
    let mut trigger_queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        0,
        "Colfenor's Urn should not trigger for toughness below four"
    );

    game.move_object_by_effect(large_id, Zone::Graveyard)
        .expect("large creature should move to graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Colfenor's Urn should trigger for a creature with toughness four or greater"
    );
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Colfenor's Urn death trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Colfenor's Urn death trigger should resolve");

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Large Creature"),
        1,
        "accepting Colfenor's Urn optional trigger should exile the large creature card"
    );
    assert_eq!(
        game.get_exiled_with_source_links(urn_id).len(),
        1,
        "the exiled card should be linked to Colfenor's Urn as cards exiled with it"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn colfenors_urn_end_step_requires_three_source_exiled_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let urn = colfenors_urn_definition();
    let urn_id = game.create_object_from_definition(&urn, alice, Zone::Battlefield);
    let card = vanilla_creature_definition(696_474, "Exiled Creature", 2, 2);
    let unrelated_id = game.create_object_from_definition(&card, alice, Zone::Exile);

    for idx in 0..2 {
        let card_id = game.create_object_from_definition(&card, alice, Zone::Exile);
        game.add_exiled_with_source_link(urn_id, card_id);
        assert_eq!(
            game.get_exiled_with_source_links(urn_id).len(),
            idx + 1,
            "test setup should link exiled cards to Colfenor's Urn"
        );
    }

    assert_eq!(
        put_colfenor_end_step_trigger_on_stack(&mut game),
        0,
        "Colfenor's Urn should not count unrelated exiled cards toward its threshold"
    );
    assert_eq!(
        game.object(unrelated_id)
            .expect("unrelated card exists")
            .zone,
        Zone::Exile,
        "unrelated exiled cards should remain exiled below the source-linked threshold"
    );
    assert_eq!(
        game.object(urn_id).expect("urn exists").zone,
        Zone::Battlefield,
        "below threshold Colfenor's Urn should remain on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn colfenors_urn_end_step_sacrifices_and_returns_source_exiled_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let urn = colfenors_urn_definition();
    let urn_id = game.create_object_from_definition(&urn, alice, Zone::Battlefield);
    let card = vanilla_creature_definition(696_475, "Exiled Creature", 2, 2);
    let unrelated_id = game.create_object_from_definition(&card, alice, Zone::Exile);
    let mut exiled = Vec::new();

    for _ in 0..3 {
        let card_id = game.create_object_from_definition(&card, alice, Zone::Exile);
        game.add_exiled_with_source_link(urn_id, card_id);
        exiled.push(card_id);
    }

    assert_eq!(
        put_colfenor_end_step_trigger_on_stack(&mut game),
        1,
        "Colfenor's Urn should trigger once when three cards have been exiled with it"
    );
    resolve_stack_entry(&mut game).expect("Colfenor's Urn end-step trigger should resolve");

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Colfenor's Urn"),
        1,
        "Colfenor's Urn should sacrifice itself when the threshold trigger resolves"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Battlefield, "Exiled Creature"),
        exiled.len(),
        "only cards exiled with Colfenor's Urn should return to the battlefield"
    );
    assert_eq!(
        game.object(unrelated_id)
            .expect("unrelated card exists")
            .zone,
        Zone::Exile,
        "unrelated exiled cards should not return with Colfenor's Urn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_eternity_elevator_threshold_mana_requires_twenty_charge_counters() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let elevator = the_eternity_elevator_definition();
    let elevator_id = game.create_object_from_definition(&elevator, alice, Zone::Battlefield);
    let threshold_ability_index = game
        .object(elevator_id)
        .expect("The Eternity Elevator should exist")
        .abilities
        .iter()
        .position(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    effect
                        .downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>()
                        .is_some()
                }),
            _ => false,
        })
        .expect("The Eternity Elevator object should have threshold mana ability");

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, ability_index }
                    if *source == elevator_id && *ability_index == threshold_ability_index
            )),
        "The Eternity Elevator's 20+ mana ability should be locked with no charge counters"
    );

    game.add_counters(elevator_id, crate::object::CounterType::Charge, 19)
        .expect("charge counters should be addable to The Eternity Elevator");
    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, ability_index }
                    if *source == elevator_id && *ability_index == threshold_ability_index
            )),
        "The Eternity Elevator's 20+ mana ability should remain locked at 19 charge counters"
    );

    game.add_counters(elevator_id, crate::object::CounterType::Charge, 1)
        .expect("the twentieth charge counter should be addable");
    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, ability_index }
                    if *source == elevator_id && *ability_index == threshold_ability_index
            )),
        "The Eternity Elevator's 20+ mana ability should unlock at 20 charge counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_eternity_elevator_threshold_mana_counts_current_charge_counters() {
    use crate::effects::{ExecutionContext, execute_effect};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let elevator = the_eternity_elevator_definition();
    let threshold_ability_index = the_eternity_elevator_threshold_ability_index(&elevator);
    let elevator_id = game.create_object_from_definition(&elevator, alice, Zone::Battlefield);
    game.add_counters(elevator_id, crate::object::CounterType::Charge, 23)
        .expect("charge counters should be addable to The Eternity Elevator");

    let ability = match &elevator.abilities[threshold_ability_index].kind {
        AbilityKind::Activated(activated) => activated,
        _ => panic!("threshold ability should be activated"),
    };
    let [effect] = ability.effects.flattened_default_effects() else {
        panic!("threshold ability should have one mana effect");
    };
    let mut ctx = ExecutionContext::new_default(elevator_id, alice);
    execute_effect(&mut game, effect, &mut ctx)
        .expect("The Eternity Elevator threshold mana ability should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.white,
        23,
        "The Eternity Elevator should add X mana of one color where X is its charge counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necrotic_ooze_copies_only_graveyard_creature_activated_abilities_on_battlefield() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let ooze = necrotic_ooze_definition();
    let sage = graveyard_sage_definition();
    let scroll = graveyard_scroll_definition();

    let ooze_id = game.create_object_from_definition(&ooze, alice, Zone::Battlefield);
    game.create_object_from_definition(&sage, alice, Zone::Graveyard);
    game.create_object_from_definition(&sage, bob, Zone::Graveyard);
    game.create_object_from_definition(&sage, alice, Zone::Hand);
    game.create_object_from_definition(&scroll, alice, Zone::Graveyard);
    game.remove_summoning_sickness(ooze_id);
    game.refresh_continuous_state();

    assert_eq!(
        activated_ability_count(&game, ooze_id),
        2,
        "Necrotic Ooze should copy creature-card activated abilities from all graveyards and ignore noncreature or non-graveyard cards"
    );

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::ActivateAbility { source, .. } if *source == ooze_id
        )),
        "Necrotic Ooze's copied activated ability should be available while it is on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necrotic_ooze_does_not_copy_activated_abilities_outside_battlefield() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let ooze = necrotic_ooze_definition();
    let sage = graveyard_sage_definition();

    let ooze_id = game.create_object_from_definition(&ooze, alice, Zone::Graveyard);
    game.create_object_from_definition(&sage, alice, Zone::Graveyard);
    game.refresh_continuous_state();

    assert_eq!(
        activated_ability_count(&game, ooze_id),
        0,
        "Necrotic Ooze should not copy graveyard activated abilities unless it is on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_eternity_elevator_station_adds_charge_counters_equal_to_tapped_creature_power() {
    use crate::effects::{ExecutionContext, execute_effect};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let elevator = the_eternity_elevator_definition();
    let elevator_id = game.create_object_from_definition(&elevator, alice, Zone::Battlefield);
    let creature = CardBuilder::new(CardId::new(), "Station Helper")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Construct])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

    let station_ability = elevator
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .any(|effect| {
                        effect
                            .downcast_ref::<crate::effects::PutCountersEffect>()
                            .is_some_and(|put| {
                                put.counter_type == crate::object::CounterType::Charge
                            })
                    }) =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("The Eternity Elevator should have a station ability");
    let [effect] = station_ability.effects.flattened_default_effects() else {
        panic!("station ability should have one counter effect");
    };

    let creature_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(creature_id).expect("station helper exists"),
        &game,
    );
    let mut ctx = ExecutionContext::new_default(elevator_id, alice);
    ctx.tag_object("tap_cost_0", creature_snapshot);
    execute_effect(&mut game, effect, &mut ctx)
        .expect("The Eternity Elevator station ability should put counters on the source");

    assert_eq!(
        game.counter_count(elevator_id, crate::object::CounterType::Charge),
        4,
        "Station should put charge counters equal to the tapped creature's power on The Eternity Elevator"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn rampaging_aetherhood_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_200), "Rampaging Aetherhood")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Snake, Subtype::Hydra])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Trample, ward {2}\n\
             At the beginning of your upkeep, you get an amount of {E} (energy counters) equal to this creature's power. Then you may pay one or more {E}. If you do, put that many +1/+1 counters on this creature.",
        )
        .expect("Rampaging Aetherhood should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct RampagingAetherhoodDecisionMaker {
    pub(super) accept_payment: bool,
    pub(super) energy_to_pay: u32,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for RampagingAetherhoodDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept_payment
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        self.energy_to_pay.clamp(ctx.min, ctx.max)
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_rampaging_aetherhood_upkeep_trigger_on_stack(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) {
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(game, trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Rampaging Aetherhood should trigger at the beginning of your upkeep"
    );
    put_triggers_on_stack(game, trigger_queue)
        .expect("Rampaging Aetherhood upkeep trigger should go on the stack");
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn aether_refinery_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_201), "Aether Refinery")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "If you would get one or more {E} (energy counters), you get twice that many {E} instead.\n\
             {T}: You get {E}, then you may pay one or more {E}. If you do, create an X/X black Aetherborn creature token, where X is the amount of {E} paid this way.",
        )
        .expect("Aether Refinery should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct AetherRefineryDecisionMaker {
    pub(super) accept_payment: bool,
    pub(super) energy_to_pay: u32,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for AetherRefineryDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept_payment
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        self.energy_to_pay.clamp(ctx.min, ctx.max)
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn aether_refinery_activation_index(def: &crate::cards::CardDefinition) -> usize {
    def.abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Aether Refinery should have its energy payment activation")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activate_aether_refinery(
    game: &mut GameState,
    refinery_id: ObjectId,
    ability_index: usize,
    dm: &mut impl DecisionMaker,
) {
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let activate_action = crate::decision::compute_legal_actions(game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == refinery_id && *idx == ability_index
            )
        })
        .expect("Aether Refinery activation should be legal while untapped");
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        dm,
    )
    .expect("Aether Refinery activation should go on the stack");
    resolve_stack_entry_with(game, dm).expect("Aether Refinery activation should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn aetherborn_tokens_controlled_by(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id).is_some_and(|object| {
                game.controller_of(object) == player
                    && object.kind == ObjectKind::Token
                    && object.name == "Aetherborn"
                    && object.card_types.contains(&CardType::Creature)
                    && object.subtypes.contains(&Subtype::Aetherborn)
                    && game.current_colors(id) == Some(crate::color::ColorSet::BLACK)
            })
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn lost_monarch_of_ifnir_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(692_108), "Lost Monarch of Ifnir")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Noble])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Afflict 3 (Whenever this creature becomes blocked, defending player loses 3 life.)\n\
             Other Zombies you control have afflict 3.\n\
             At the beginning of your second main phase, if a player was dealt combat damage by a Zombie this turn, mill three cards, then you may return a creature card from your graveyard to your hand.",
        )
        .expect("Lost Monarch of Ifnir should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn archon_of_coronation_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(559_764), "Archon of Coronation")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .flying()
        .parse_text(
            "When this creature enters, you become the monarch.\n\
             As long as you're the monarch, damage doesn't cause you to lose life.",
        )
        .expect("Archon of Coronation should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn deal_test_combat_damage_to_player(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
) -> CombatDamageEvent {
    let cause = crate::events::cause::EventCause::combat_damage(source);
    let processed = crate::events::processing::process_damage_assignments_with_event(
        game,
        source,
        crate::events::DamageTarget::Player(player),
        amount,
        true,
        cause.clone(),
    );
    let keywords = crate::rules::damage::source_damage_keywords(game, source, None);
    let mut damage_dealt = 0u32;
    let mut life_lost = 0u32;
    for assignment in processed.assignments {
        let applied = crate::rules::damage::apply_processed_damage_assignment(
            game,
            source,
            assignment.target,
            assignment.amount,
            keywords,
            cause.clone(),
        );
        assert!(applied.applied, "combat damage assignment should apply");
        damage_dealt = damage_dealt.saturating_add(assignment.amount);
        life_lost = life_lost.saturating_add(applied.life_lost);
    }

    CombatDamageEvent {
        source,
        target: DamageEventTarget::Player(player),
        amount: damage_dealt,
        life_lost,
        result: DamageResult {
            damage_dealt,
            ..DamageResult::default()
        },
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_typed_creature(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    subtypes: Vec<Subtype>,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn block_lost_monarch_attacker(
    game: &mut GameState,
    attacker: ObjectId,
    blocker: ObjectId,
    defending_player: PlayerId,
    attack_target: AttackTarget,
) -> TriggerQueue {
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker,
        target: attack_target,
    });
    let mut trigger_queue = TriggerQueue::new();
    apply_blocker_declarations(
        game,
        &mut combat,
        &mut trigger_queue,
        &[BlockerDeclaration {
            blocker,
            blocking: attacker,
        }],
        defending_player,
    )
    .expect("Lost Monarch combat block should be legal");
    trigger_queue
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn record_combat_damage_to_player(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
) {
    let event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(player),
            2,
            true,
            crate::events::cause::EventCause::combat_damage(source),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_lost_monarch_second_main_trigger_on_stack(game: &mut GameState) {
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Lost Monarch of Ifnir should have one second-main trigger"
    );
    put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Lost Monarch of Ifnir second-main trigger should go on the stack");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lost_monarch_of_ifnir_grants_afflict_only_to_other_zombies_you_control() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let monarch = lost_monarch_of_ifnir_definition();
    let monarch_id = game.create_object_from_definition(&monarch, alice, Zone::Battlefield);
    let other_zombie =
        create_typed_creature(&mut game, "Ifnir Loyalist", alice, vec![Subtype::Zombie]);
    let non_zombie = create_typed_creature(&mut game, "Ifnir Human", alice, vec![Subtype::Human]);
    let opposing_zombie =
        create_typed_creature(&mut game, "Opposing Zombie", bob, vec![Subtype::Zombie]);
    let bob_blocker_1 =
        create_typed_creature(&mut game, "Bob Blocker One", bob, vec![Subtype::Human]);
    let bob_blocker_2 =
        create_typed_creature(&mut game, "Bob Blocker Two", bob, vec![Subtype::Human]);
    let bob_blocker_3 =
        create_typed_creature(&mut game, "Bob Blocker Three", bob, vec![Subtype::Human]);
    let alice_blocker =
        create_typed_creature(&mut game, "Alice Blocker", alice, vec![Subtype::Human]);
    game.refresh_continuous_state();

    let mut queue = block_lost_monarch_attacker(
        &mut game,
        other_zombie,
        bob_blocker_1,
        bob,
        AttackTarget::Player(bob),
    );
    assert_eq!(
        queue.entries.len(),
        1,
        "other controlled Zombies should gain afflict 3"
    );
    put_triggers_on_stack(&mut game, &mut queue).expect("granted afflict should go on the stack");
    resolve_stack_entry(&mut game).expect("granted afflict should resolve");
    assert_eq!(game.player(bob).expect("bob exists").life, 17);

    let mut queue = block_lost_monarch_attacker(
        &mut game,
        monarch_id,
        bob_blocker_2,
        bob,
        AttackTarget::Player(bob),
    );
    assert_eq!(
        queue.entries.len(),
        1,
        "Lost Monarch should keep only its own afflict trigger; the static grant says other Zombies"
    );
    put_triggers_on_stack(&mut game, &mut queue).expect("intrinsic afflict should go on the stack");
    resolve_stack_entry(&mut game).expect("intrinsic afflict should resolve");
    assert_eq!(game.player(bob).expect("bob exists").life, 14);

    let queue = block_lost_monarch_attacker(
        &mut game,
        non_zombie,
        bob_blocker_3,
        bob,
        AttackTarget::Player(bob),
    );
    assert!(
        queue.entries.is_empty(),
        "non-Zombies should not gain afflict"
    );
    assert_eq!(game.player(bob).expect("bob exists").life, 14);

    let queue = block_lost_monarch_attacker(
        &mut game,
        opposing_zombie,
        alice_blocker,
        alice,
        AttackTarget::Player(alice),
    );
    assert!(
        queue.entries.is_empty(),
        "Zombies not controlled by Lost Monarch's controller should not gain afflict"
    );
    assert_eq!(game.player(alice).expect("alice exists").life, 20);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lost_monarch_of_ifnir_second_main_trigger_requires_zombie_combat_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let monarch = lost_monarch_of_ifnir_definition();
    game.create_object_from_definition(&monarch, alice, Zone::Battlefield);
    let non_zombie = create_typed_creature(&mut game, "Combat Human", alice, vec![Subtype::Human]);
    for idx in 0..3 {
        let card = CardBuilder::new(CardId::new(), &format!("Library Card {idx}"))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    let library_before = game.player(alice).expect("alice exists").library.len();
    let graveyard_before = game.player(alice).expect("alice exists").graveyard.len();

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;
    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Lost Monarch should not trigger without prior Zombie combat damage to a player"
    );

    record_combat_damage_to_player(&mut game, non_zombie, PlayerId::from_index(1));
    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Lost Monarch should not trigger for combat damage dealt by a non-Zombie"
    );

    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before,
        "Lost Monarch should not mill without prior Zombie combat damage to a player"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").graveyard.len(),
        graveyard_before,
        "Lost Monarch should not move cards when the condition is false"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lost_monarch_of_ifnir_second_main_trigger_mills_and_may_return_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let monarch = lost_monarch_of_ifnir_definition();
    game.create_object_from_definition(&monarch, alice, Zone::Battlefield);
    let zombie = create_typed_creature(&mut game, "Combat Zombie", alice, vec![Subtype::Zombie]);
    for idx in 0..3 {
        let card = CardBuilder::new(CardId::new(), &format!("Mill Card {idx}"))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    let creature_card = CardBuilder::new(CardId::new(), "Recoverable Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Graveyard);
    let creature_stable_id = game
        .object(creature_id)
        .expect("creature card exists")
        .stable_id;
    record_combat_damage_to_player(&mut game, zombie, bob);

    put_lost_monarch_second_main_trigger_on_stack(&mut game);
    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("condition-true second-main trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        0,
        "Lost Monarch should mill three cards after Zombie combat damage"
    );
    let returned_id = game
        .find_object_by_stable_id(creature_stable_id)
        .expect("returned creature card should still exist");
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&returned_id),
        "accepting the may choice should return a creature card to hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lost_monarch_of_ifnir_second_main_return_is_optional() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let monarch = lost_monarch_of_ifnir_definition();
    game.create_object_from_definition(&monarch, alice, Zone::Battlefield);
    let zombie = create_typed_creature(&mut game, "Combat Zombie", alice, vec![Subtype::Zombie]);
    for idx in 0..3 {
        let card = CardBuilder::new(CardId::new(), &format!("Decline Mill Card {idx}"))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    let creature_card = CardBuilder::new(CardId::new(), "Declined Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Graveyard);
    record_combat_damage_to_player(&mut game, zombie, bob);

    put_lost_monarch_second_main_trigger_on_stack(&mut game);
    let mut dm = AutoPassDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("declined second-main trigger should resolve");

    assert_eq!(
        game.object(creature_id).expect("creature card exists").zone,
        Zone::Graveyard,
        "declining the may choice should leave the creature card in the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aether_refinery_replacement_doubles_player_energy_gained() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let refinery = aether_refinery_definition();
    let refinery_id = game.create_object_from_definition(&refinery, alice, Zone::Battlefield);
    game.refresh_continuous_state();

    game.add_player_counters_with_source(
        alice,
        crate::object::CounterType::Energy,
        3,
        Some(refinery_id),
        Some(alice),
    )
    .expect("adding energy should emit a marker event");

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        6,
        "Aether Refinery should double energy counters its controller would get"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aether_refinery_activation_pays_energy_and_creates_paid_size_construct() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let refinery = aether_refinery_definition();
    let refinery_id = game.create_object_from_definition(&refinery, alice, Zone::Battlefield);
    let ability_index = aether_refinery_activation_index(&refinery);
    game.refresh_continuous_state();

    let mut dm = AetherRefineryDecisionMaker {
        accept_payment: true,
        energy_to_pay: 2,
    };
    activate_aether_refinery(&mut game, refinery_id, ability_index, &mut dm);

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        0,
        "the activation should get two energy after replacement, then spend the chosen two"
    );
    assert!(
        game.is_tapped(refinery_id),
        "activation cost should tap Aether Refinery"
    );
    let tokens = aetherborn_tokens_controlled_by(&game, alice);
    assert_eq!(
        tokens.len(),
        1,
        "paying energy should create one Aetherborn token"
    );
    let token_id = tokens[0];
    assert_eq!(game.current_power(token_id), Some(2));
    assert_eq!(game.current_toughness(token_id), Some(2));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aether_refinery_declining_payment_keeps_energy_and_creates_no_token() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let refinery = aether_refinery_definition();
    let refinery_id = game.create_object_from_definition(&refinery, alice, Zone::Battlefield);
    let ability_index = aether_refinery_activation_index(&refinery);
    game.refresh_continuous_state();

    let mut dm = AetherRefineryDecisionMaker {
        accept_payment: false,
        energy_to_pay: 2,
    };
    activate_aether_refinery(&mut game, refinery_id, ability_index, &mut dm);

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        2,
        "declining payment should keep the doubled energy from the activation"
    );
    assert!(
        aetherborn_tokens_controlled_by(&game, alice).is_empty(),
        "declining the optional energy payment should skip the if-you-do token branch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rampaging_aetherhood_upkeep_pays_energy_and_adds_that_many_counters() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let aetherhood = rampaging_aetherhood_definition();
    let aetherhood_id = game.create_object_from_definition(&aetherhood, alice, Zone::Battlefield);

    put_rampaging_aetherhood_upkeep_trigger_on_stack(&mut game, &mut trigger_queue);

    let mut dm = RampagingAetherhoodDecisionMaker {
        accept_payment: true,
        energy_to_pay: 3,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Rampaging Aetherhood upkeep trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        1,
        "the trigger should get four energy from power 4, then spend the chosen three"
    );
    assert_eq!(
        game.counter_count(aetherhood_id, crate::object::CounterType::PlusOnePlusOne),
        3,
        "Rampaging Aetherhood should get counters equal to the amount of energy paid"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rampaging_aetherhood_payment_choice_cannot_pay_zero() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let aetherhood = rampaging_aetherhood_definition();
    let aetherhood_id = game.create_object_from_definition(&aetherhood, alice, Zone::Battlefield);

    put_rampaging_aetherhood_upkeep_trigger_on_stack(&mut game, &mut trigger_queue);

    let mut dm = RampagingAetherhoodDecisionMaker {
        accept_payment: true,
        energy_to_pay: 0,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Rampaging Aetherhood upkeep trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        3,
        "one-or-more energy payment should force at least one energy to be paid"
    );
    assert_eq!(
        game.counter_count(aetherhood_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "the if-you-do branch should use the minimum paid amount"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rampaging_aetherhood_declining_payment_gets_energy_without_counters() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let aetherhood = rampaging_aetherhood_definition();
    let aetherhood_id = game.create_object_from_definition(&aetherhood, alice, Zone::Battlefield);

    put_rampaging_aetherhood_upkeep_trigger_on_stack(&mut game, &mut trigger_queue);

    let mut dm = RampagingAetherhoodDecisionMaker {
        accept_payment: false,
        energy_to_pay: 4,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Rampaging Aetherhood upkeep trigger should resolve when payment is declined");

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        4,
        "declining payment should leave the energy gained from the trigger"
    );
    assert_eq!(
        game.counter_count(aetherhood_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "declining the optional energy payment should not add +1/+1 counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sarulf_realm_eater_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(792_240), "Sarulf, Realm Eater")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Green],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wolf])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Whenever a permanent an opponent controls is put into a graveyard from the battlefield, put a +1/+1 counter on Sarulf.\n\
             At the beginning of your upkeep, if Sarulf has one or more +1/+1 counters on it, you may remove all of them. If you do, exile each other nonland permanent with mana value less than or equal to the number of counters removed this way.",
        )
        .expect("Sarulf, Realm Eater should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sarulf_test_permanent_definition(
    card_id: u32,
    name: &str,
    card_types: Vec<CardType>,
    mana_value: u8,
) -> crate::cards::CardDefinition {
    let mut builder =
        CardDefinitionBuilder::new(CardId::from_raw(card_id), name).card_types(card_types.clone());
    if mana_value > 0 {
        builder = builder.mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]));
    }
    if card_types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct SarulfUpkeepDecisionMaker {
    pub(super) remove_counters: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for SarulfUpkeepDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.remove_counters
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn queue_sarulf_upkeep_trigger(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(game, trigger_queue);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sarulf_realm_eater_death_trigger_adds_plus_one_counter_for_opponent_permanent() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sarulf = sarulf_realm_eater_definition();
    let sarulf_id = game.create_object_from_definition(&sarulf, alice, Zone::Battlefield);
    let opponent_permanent =
        sarulf_test_permanent_definition(792_241, "Opponent Relic", vec![CardType::Artifact], 2);
    let own_permanent =
        sarulf_test_permanent_definition(792_242, "Own Relic", vec![CardType::Artifact], 2);
    let opponent_id =
        game.create_object_from_definition(&opponent_permanent, bob, Zone::Battlefield);
    let own_id = game.create_object_from_definition(&own_permanent, alice, Zone::Battlefield);
    let mut trigger_queue = TriggerQueue::new();

    game.move_object_by_effect(own_id, Zone::Graveyard)
        .expect("own permanent should move to graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Sarulf should not trigger for its controller's permanent"
    );

    game.move_object_by_effect(opponent_id, Zone::Graveyard)
        .expect("opponent permanent should move to graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Sarulf should trigger for an opponent-controlled permanent"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Sarulf death trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Sarulf death trigger should resolve");

    assert_eq!(
        game.counter_count(sarulf_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Sarulf death trigger should add a +1/+1 counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sarulf_realm_eater_upkeep_without_plus_one_counters_does_not_trigger() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let sarulf = sarulf_realm_eater_definition();
    game.create_object_from_definition(&sarulf, alice, Zone::Battlefield);
    let mut trigger_queue = TriggerQueue::new();

    queue_sarulf_upkeep_trigger(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue.entries.is_empty(),
        "Sarulf upkeep ability has an intervening-if condition and should not trigger without +1/+1 counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sarulf_realm_eater_upkeep_removes_all_plus_one_counters_and_exiles_by_removed_count()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sarulf = sarulf_realm_eater_definition();
    let sarulf_id = game.create_object_from_definition(&sarulf, alice, Zone::Battlefield);
    game.add_counters(sarulf_id, crate::object::CounterType::PlusOnePlusOne, 3);
    game.add_counters(sarulf_id, crate::object::CounterType::Charge, 1);

    let low =
        sarulf_test_permanent_definition(792_243, "Low Permanent", vec![CardType::Artifact], 3);
    let high =
        sarulf_test_permanent_definition(792_244, "High Permanent", vec![CardType::Artifact], 4);
    let land = sarulf_test_permanent_definition(792_245, "Low Land", vec![CardType::Land], 0);
    game.create_object_from_definition(&low, bob, Zone::Battlefield);
    let high_id = game.create_object_from_definition(&high, bob, Zone::Battlefield);
    let land_id = game.create_object_from_definition(&land, bob, Zone::Battlefield);
    let mut trigger_queue = TriggerQueue::new();

    queue_sarulf_upkeep_trigger(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Sarulf upkeep ability should trigger while it has +1/+1 counters"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Sarulf upkeep trigger should go on the stack");
    let mut dm = SarulfUpkeepDecisionMaker {
        remove_counters: true,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Sarulf upkeep trigger should resolve");

    assert_eq!(
        game.counter_count(sarulf_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "accepting the upkeep choice should remove all +1/+1 counters"
    );
    assert_eq!(
        game.counter_count(sarulf_id, crate::object::CounterType::Charge),
        1,
        "all of them should refer to the +1/+1 counters from the intervening condition, not unrelated counters"
    );
    assert_eq!(
        game.object(sarulf_id).expect("Sarulf exists").zone,
        Zone::Battlefield,
        "Sarulf should not exile itself because the effect exiles each other permanent"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Low Permanent"),
        1,
        "permanents with mana value equal to the removed counter count should be exiled"
    );
    assert_eq!(
        game.object(high_id).expect("high permanent exists").zone,
        Zone::Battlefield,
        "permanents with mana value greater than the removed counter count should remain"
    );
    assert_eq!(
        game.object(land_id).expect("land exists").zone,
        Zone::Battlefield,
        "lands should remain even when their mana value is low enough"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sarulf_realm_eater_declining_upkeep_removal_skips_exile_branch() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sarulf = sarulf_realm_eater_definition();
    let sarulf_id = game.create_object_from_definition(&sarulf, alice, Zone::Battlefield);
    game.add_counters(sarulf_id, crate::object::CounterType::PlusOnePlusOne, 2);
    let low = sarulf_test_permanent_definition(
        792_246,
        "Decline Low Permanent",
        vec![CardType::Artifact],
        1,
    );
    let low_id = game.create_object_from_definition(&low, bob, Zone::Battlefield);
    let mut trigger_queue = TriggerQueue::new();

    queue_sarulf_upkeep_trigger(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Sarulf upkeep trigger should go on the stack");
    let mut dm = SarulfUpkeepDecisionMaker {
        remove_counters: false,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("declined Sarulf upkeep trigger should resolve");

    assert_eq!(
        game.counter_count(sarulf_id, crate::object::CounterType::PlusOnePlusOne),
        2,
        "declining the optional removal should leave Sarulf's +1/+1 counters"
    );
    assert_eq!(
        game.object(low_id).expect("low permanent exists").zone,
        Zone::Battlefield,
        "if no counters are removed, the if-you-do exile branch should not happen"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn assaultron_dominator_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_260), "Assaultron Dominator")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Robot])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "When this creature enters, you get {E}{E} (two energy counters).\n\
             Whenever an artifact creature you control attacks, you may pay {E}. If you do, put your choice of a +1/+1, first strike, or trample counter on that creature.",
        )
        .expect("Assaultron Dominator should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct AssaultronDecisionMaker {
    pub(super) pay_energy: bool,
    pub(super) mode_index: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for AssaultronDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.pay_energy
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        vec![self.mode_index.min(ctx.options.len().saturating_sub(1))]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn attack_with_assaultron_creature(
    game: &mut GameState,
    attacker_id: ObjectId,
) -> TriggerQueue {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.remove_summoning_sickness(attacker_id);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        }],
    )
    .expect("Assaultron attacker should be legal");
    trigger_queue
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn assaultron_dominator_enters_trigger_gets_two_energy() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let assaultron = assaultron_dominator_definition();
    let assaultron_id = game.create_object_from_definition(&assaultron, alice, Zone::Battlefield);
    let event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            assaultron_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    for trigger in crate::triggers::check_triggers(&game, &event) {
        if trigger.source == assaultron_id {
            trigger_queue.add(trigger);
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Assaultron Dominator should trigger when it enters"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Assaultron Dominator enters trigger should go on the stack");

    let mut dm = AutoPassDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Assaultron Dominator enters trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        2,
        "Assaultron Dominator should give two energy when it enters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn assaultron_dominator_attack_trigger_pays_energy_and_chooses_each_counter_mode() {
    for (mode_index, counter_type) in [
        (0, crate::object::CounterType::PlusOnePlusOne),
        (1, crate::object::CounterType::FirstStrike),
        (2, crate::object::CounterType::Trample),
    ] {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let assaultron = assaultron_dominator_definition();
        let assaultron_id =
            game.create_object_from_definition(&assaultron, alice, Zone::Battlefield);
        game.player_mut(alice)
            .expect("alice exists")
            .energy_counters = 1;

        let mut trigger_queue = attack_with_assaultron_creature(&mut game, assaultron_id);
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Assaultron Dominator should trigger when it attacks as an artifact creature"
        );
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("Assaultron Dominator attack trigger should go on the stack");

        let mut dm = AssaultronDecisionMaker {
            pay_energy: true,
            mode_index,
        };
        resolve_stack_entry_with(&mut game, &mut dm)
            .expect("Assaultron Dominator attack trigger should resolve");

        assert_eq!(
            game.player(alice).expect("alice exists").energy_counters,
            0,
            "Assaultron Dominator should spend one energy when the payment is accepted"
        );
        assert_eq!(
            game.counter_count(assaultron_id, counter_type),
            1,
            "Assaultron Dominator should receive the selected counter mode"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn assaultron_dominator_attack_trigger_puts_counter_on_that_attacking_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let assaultron = assaultron_dominator_definition();
    let assaultron_id = game.create_object_from_definition(&assaultron, alice, Zone::Battlefield);
    let attacker = CardBuilder::new(CardId::from_raw(74_261), "Artifact Attacker")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice exists")
        .energy_counters = 1;

    let mut trigger_queue = attack_with_assaultron_creature(&mut game, attacker_id);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Assaultron Dominator should trigger when another artifact creature attacks"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Assaultron Dominator attack trigger should go on the stack");

    let mut dm = AssaultronDecisionMaker {
        pay_energy: true,
        mode_index: 1,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Assaultron Dominator attack trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        0,
        "Assaultron Dominator should spend one energy when the payment is accepted"
    );
    assert_eq!(
        game.counter_count(attacker_id, crate::object::CounterType::FirstStrike),
        1,
        "the selected counter should be put on the attacking artifact creature"
    );
    assert_eq!(
        game.counter_count(assaultron_id, crate::object::CounterType::FirstStrike),
        0,
        "the selected counter should not default to Assaultron Dominator"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn assaultron_dominator_declining_energy_payment_adds_no_counter() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let assaultron = assaultron_dominator_definition();
    let assaultron_id = game.create_object_from_definition(&assaultron, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice exists")
        .energy_counters = 1;

    let mut trigger_queue = attack_with_assaultron_creature(&mut game, assaultron_id);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Assaultron Dominator attack trigger should go on the stack");

    let mut dm = AssaultronDecisionMaker {
        pay_energy: false,
        mode_index: 0,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Assaultron Dominator attack trigger should resolve when payment is declined");

    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        1,
        "declining the optional payment should leave the energy unspent"
    );
    assert_eq!(
        game.counter_count(assaultron_id, crate::object::CounterType::PlusOnePlusOne)
            + game.counter_count(assaultron_id, crate::object::CounterType::FirstStrike)
            + game.counter_count(assaultron_id, crate::object::CounterType::Trample),
        0,
        "declining the optional payment should add no counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn assaultron_dominator_ignores_nonartifact_attackers() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let assaultron = assaultron_dominator_definition();
    game.create_object_from_definition(&assaultron, alice, Zone::Battlefield);
    let nonartifact = create_creature(&mut game, "Nonartifact Attacker", alice, 2, 2);

    let trigger_queue = attack_with_assaultron_creature(&mut game, nonartifact);
    assert_eq!(
        trigger_queue.entries.len(),
        0,
        "Assaultron Dominator should not trigger for a nonartifact creature attacking"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn twenty_toed_toad_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_950), "Twenty-Toed Toad")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Frog, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Your maximum hand size is twenty.\n\
             Whenever you attack with two or more creatures, put a +1/+1 counter on this creature and draw a card.\n\
             Whenever this creature attacks, you win the game if there are twenty or more counters on it or you have twenty or more cards in hand.",
        )
        .expect("Twenty-Toed Toad should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn trusted_advisor_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_951), "Trusted Advisor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Advisor])
        .power_toughness(PowerToughness::fixed(1, 2))
        .parse_text(
            "Your maximum hand size is increased by two.\n\
             At the beginning of your upkeep, return a blue creature you control to its owner's hand.",
        )
        .expect("Trusted Advisor should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn ox_drover_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(73_950), "Ox Drover")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Peasant])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Vigilance\n\
             This creature can't be blocked by Oxen.\n\
             Whenever this creature enters or attacks, target opponent creates a 2/4 white Ox creature token and you draw a card.",
        )
        .expect("Ox Drover should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn ox_tokens_controlled_by(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id).is_some_and(|object| {
                game.controller_of(object) == player
                    && object.kind == ObjectKind::Token
                    && object.name == "Ox"
                    && object.card_types.contains(&CardType::Creature)
                    && object.subtypes.contains(&Subtype::Ox)
                    && game.current_power(id) == Some(2)
                    && game.current_toughness(id) == Some(4)
                    && game.current_colors(id) == Some(crate::color::ColorSet::WHITE)
            })
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn from_under_the_floorboards_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_210), "From Under the Floorboards")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Madness {X}{B}{B}\n\
             Create three tapped 2/2 black Zombie creature tokens and you gain 3 life. If this spell's madness cost was paid, instead create X of those tokens and you gain X life.",
        )
        .expect("From Under the Floorboards should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn zombie_tokens_controlled_by(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id).is_some_and(|object| {
                game.controller_of(object) == player
                    && object.kind == ObjectKind::Token
                    && object.name == "Zombie"
                    && object.card_types.contains(&CardType::Creature)
                    && object.subtypes.contains(&Subtype::Zombie)
                    && game.current_power(id) == Some(2)
                    && game.current_toughness(id) == Some(2)
                    && game.current_colors(id) == Some(crate::color::ColorSet::BLACK)
            })
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_from_under_the_floorboards_on_stack(
    game: &mut GameState,
    player: PlayerId,
    paid_madness: bool,
    x_value: Option<u32>,
) {
    let def = from_under_the_floorboards_definition();
    assert!(
        def.alternative_casts.iter().any(|method| matches!(
            method,
            crate::alternative_cast::AlternativeCastingMethod::Madness { cost }
                if cost.to_oracle() == "{X}{B}{B}"
        )),
        "From Under the Floorboards should expose its madness alternative cost"
    );
    let spell_id = game.create_object_from_definition(&def, player, Zone::Stack);
    if let Some(spell) = game.object_mut(spell_id) {
        spell.x_value = x_value;
        if paid_madness {
            spell.optional_costs_paid.mark_label_paid("Madness");
        }
    }
    let stable_id = game.object(spell_id).expect("spell on stack").stable_id;
    let mut entry = StackEntry::new(spell_id, player)
        .with_source_info(stable_id, "From Under the Floorboards".to_string());
    if let Some(x) = x_value {
        entry = entry.with_x(x);
    }
    game.push_to_stack(entry);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn from_under_the_floorboards_without_madness_uses_default_token_and_life_branch() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    put_from_under_the_floorboards_on_stack(&mut game, alice, false, Some(2));
    resolve_stack_entry(&mut game).expect("From Under the Floorboards should resolve normally");

    let zombies = zombie_tokens_controlled_by(&game, alice);
    assert_eq!(
        zombies.len(),
        3,
        "normal branch should create three Zombies"
    );
    assert!(
        zombies.iter().all(|id| game.is_tapped(*id)),
        "normal branch Zombies should enter tapped"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        23,
        "normal branch should gain 3 life"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn from_under_the_floorboards_paid_madness_uses_x_token_and_life_branch() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    put_from_under_the_floorboards_on_stack(&mut game, alice, true, Some(2));
    resolve_stack_entry(&mut game)
        .expect("From Under the Floorboards should resolve with madness paid");

    let zombies = zombie_tokens_controlled_by(&game, alice);
    assert_eq!(
        zombies.len(),
        2,
        "paid madness branch should replace the default with X Zombies"
    );
    assert!(
        zombies.iter().all(|id| game.is_tapped(*id)),
        "paid madness branch Zombies should also enter tapped"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        22,
        "paid madness branch should gain X life"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn torch_the_witness_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_260), "Torch the Witness")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Torch the Witness deals twice X damage to target creature. If excess damage was dealt to that creature this way, investigate.",
        )
        .expect("Torch the Witness should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn clue_tokens_controlled_by(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                game.controller_of(object) == player
                    && object.kind == ObjectKind::Token
                    && object.name == "Clue"
            })
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sophina_spearsage_deserter_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(105_941), "Sophina, Spearsage Deserter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::White],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Menace\nWhenever Sophina, Spearsage Deserter attacks, investigate once for each nontoken attacking creature. (To investigate, create a Clue token. It's an artifact with \"{2}, Sacrifice this artifact: Draw a card.\")\nPartner—Friends forever (You can have two commanders if both have this ability.)",
        )
        .expect("Sophina, Spearsage Deserter should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sophina_spearsage_deserter_strict_parse_and_compiled_text_preserve_investigate_count()
{
    let def = sophina_spearsage_deserter_definition();

    let rendered_lines = crate::compiled_text::compiled_text_lines(&def);
    let rendered = rendered_lines.join(" ");
    assert_eq!(
        rendered_lines,
        vec![
            "Menace".to_string(),
            "Whenever Sophina attacks, investigate X times, where X is the number of nontoken attacking creatures."
                .to_string(),
            "Partner—friends forever".to_string(),
        ],
        "Sophina compiled text should preserve the exact attack trigger, investigate count, and Partner variant label, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("InvestigateEffect")
            && debug.contains("Count")
            && debug.contains("attacking: true")
            && debug.contains("nontoken: true"),
        "Sophina should structurally investigate for each nontoken attacking creature, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sophina_spearsage_deserter_attack_trigger_counts_nontoken_attackers_and_ignores_tokens()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sophina = sophina_spearsage_deserter_definition();
    let sophina_id = game.create_object_from_definition(&sophina, alice, Zone::Battlefield);
    let nontoken_attacker = create_creature(&mut game, "Nontoken Attacker", alice, 2, 2);
    let token_attacker = create_creature(&mut game, "Token Attacker", alice, 1, 1);
    game.object_mut(token_attacker)
        .expect("token attacker should exist")
        .kind = ObjectKind::Token;

    for creature in [sophina_id, nontoken_attacker, token_attacker] {
        game.remove_summoning_sickness(creature);
    }

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![
        AttackerDeclaration {
            creature: sophina_id,
            target: AttackTarget::Player(bob),
        },
        AttackerDeclaration {
            creature: nontoken_attacker,
            target: AttackTarget::Player(bob),
        },
        AttackerDeclaration {
            creature: token_attacker,
            target: AttackTarget::Player(bob),
        },
    ];
    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("Sophina and other creatures should be able to attack");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Sophina attack trigger should go on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Sophina should create one attack trigger"
    );

    resolve_stack_entry(&mut game).expect("Sophina attack trigger should resolve");

    assert_eq!(
        clue_tokens_controlled_by(&game, alice).len(),
        2,
        "Sophina should investigate for Sophina and the other nontoken attacker, but not the token attacker"
    );
}
