//! CR audit (pending-entry pass, package p04): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::{
    CardDefinition, CardId, CardType, ChooseSpec, CounterType, DecisionMaker, EffectExecutor,
    ObjectId, PlayerFilter, PowerToughness, Subtype, Supertype, Zone,
};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn two_players() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}

fn three_players() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20)
}

fn compile_card(name: &str, text: &str) -> CardDefinition {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("fixture {name} must compile: {error:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
        "fixture {name} must compile without unimplemented content"
    );
    definition
}

fn try_compile(name: &str, text: &str) -> Result<CardDefinition, String> {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .map_err(|error| format!("{error:?}"))
}

fn creature(game: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Audit creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn planeswalker_def(name: &str, loyalty: u32, legendary: bool) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Jace])
        .supertypes(if legendary {
            vec![Supertype::Legendary]
        } else {
            vec![]
        })
        .loyalty(loyalty)
        .build()
}

fn legal_actions(game: &GameState, player: PlayerId) -> Vec<ironsmith::LegalAction> {
    ironsmith::compute_legal_actions(game, player).expect("legal actions must compute")
}

fn current_zone(game: &GameState, stable: ironsmith::ids::StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

/// Picks the first option whose label is in `wanted` (in preference order),
/// otherwise the first legal option.
struct PickNamed(Vec<String>);

impl DecisionMaker for PickNamed {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        for wanted in &self.0 {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|option| option.legal && option.description.eq_ignore_ascii_case(wanted))
            {
                return vec![option.index];
            }
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min.max(1))
            .map(|option| option.index)
            .collect()
    }
}

fn finish_cast<D: DecisionMaker>(
    game: &mut GameState,
    action: ironsmith::decision::LegalAction,
    dm: &mut D,
) -> ObjectId {
    use ironsmith::decision::GameProgress;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::triggers::TriggerQueue;
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut queue = TriggerQueue::new();
    let before = game.stack.len();
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .expect("an affordable cast must start");
    for _ in 0..32 {
        if game.stack.len() > before {
            return game.stack.last().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("cast stopped before reaching the stack: {progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &context, dm,
        )
        .expect("cast payment must finish");
    }
    panic!("cast did not finish");
}

// ---------------------------------------------------------------------------
// 305 Lands
// ---------------------------------------------------------------------------

#[test]
fn cr_305_4_effect_put_land_is_not_a_land_play() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::special_actions::{SpecialAction, perform};
    use ironsmith::{LegalAction, Phase};
    let mut game = two_players();
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let land = CardDefinitionBuilder::new(CardId::new(), "Put audit land")
        .card_types(vec![CardType::Land])
        .build();
    let put = game.create_object_from_definition(&land, A, Zone::Hand);
    let played = game.create_object_from_definition(&land, A, Zone::Hand);
    let put_stable = game.object(put).unwrap().stable_id;
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(put), Zone::Battlefield, false)
        .execute(&mut game, &mut EffectContext::new_default(put, A))
        .unwrap();
    assert_eq!(current_zone(&game, put_stable), Some(Zone::Battlefield));
    assert_eq!(
        game.player(A).unwrap().lands_played_this_turn,
        0,
        "CR 305.4: putting a land onto the battlefield is not playing a land"
    );
    assert!(
        legal_actions(&game, A)
            .iter()
            .any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == played)),
        "CR 305.4: the land put onto the battlefield does not use the turn's land play"
    );
    // Control: an actual land play does count.
    perform(
        SpecialAction::PlayLand { card_id: played },
        &mut game,
        A,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(game.player(A).unwrap().lands_played_this_turn, 1);
}

#[test]
fn cr_305_5_land_subtypes_are_separate_single_words() {
    let dual = compile_card("Audit Dual", "Type: Land — Forest Island");
    assert_eq!(dual.card.card_types, vec![CardType::Land]);
    assert_eq!(
        dual.card.subtypes,
        vec![Subtype::Forest, Subtype::Island],
        "CR 305.5: each word after the dash is a separate land type"
    );
    let basic = compile_card("Audit Mountain", "Type: Basic Land — Mountain");
    assert_eq!(basic.card.subtypes, vec![Subtype::Mountain]);
    assert_eq!(basic.card.supertypes, vec![Supertype::Basic]);
    let mut game = two_players();
    let id = game.create_object_from_definition(&dual, A, Zone::Battlefield);
    let subtypes = game.current_subtypes(id).unwrap();
    assert!(
        subtypes.contains(&Subtype::Forest) && subtypes.contains(&Subtype::Island),
        "CR 305.5: a land may have multiple land types at once"
    );
}

#[test]
fn cr_305_8_basic_supertype_alone_makes_a_land_basic() {
    use ironsmith::ObjectFilter;
    let mut game = two_players();
    let dual = compile_card("Audit Dual", "Type: Land — Forest Island");
    let basic = compile_card("Audit Mountain", "Type: Basic Land — Mountain");
    let dual = game.create_object_from_definition(&dual, A, Zone::Battlefield);
    let basic = game.create_object_from_definition(&basic, A, Zone::Battlefield);
    let ctx = EffectContext::new_default(dual, A);
    let matches = |filter: ObjectFilter| {
        ironsmith::effects::helpers::resolve_objects_from_spec(
            &game,
            &ChooseSpec::All(filter),
            &ctx,
        )
        .unwrap()
    };
    let basics = matches(ObjectFilter::land().with_supertype(Supertype::Basic));
    let nonbasics = matches(ObjectFilter::land().nonbasic());
    assert!(basics.contains(&basic) && !basics.contains(&dual));
    assert!(
        nonbasics.contains(&dual) && !nonbasics.contains(&basic),
        "CR 305.8: a land with basic land types but no basic supertype is nonbasic"
    );
}

#[test]
fn cr_305_9_land_creature_is_played_as_a_land_never_cast() {
    use ironsmith::{CastingMethod, LegalAction, Phase};
    for played in [0, 1] {
        let mut game = two_players();
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(A).unwrap().lands_played_this_turn = played;
        let arbor = compile_card(
            "Audit Arbor",
            "Type: Land Creature — Forest Dryad\nPower/Toughness: 1/1",
        );
        let id = game.create_object_from_definition(&arbor, A, Zone::Hand);
        let actions = legal_actions(&game, A);
        assert_eq!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == id)),
            played == 0,
            "CR 305.9: the land creature can be played as a land while a play remains"
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == id)),
            "CR 305.9: an object that is a land and another type can't be cast (plays used={played})"
        );
        assert!(
            !ironsmith::decision::can_cast_spell(
                &game,
                A,
                game.object(id).unwrap(),
                &CastingMethod::Normal
            ),
            "CR 305.9: can_cast_spell must reject a land creature"
        );
    }
}

// ---------------------------------------------------------------------------
// 306 Planeswalkers
// ---------------------------------------------------------------------------

#[test]
fn cr_306_3_planeswalker_subtypes_compile_as_separate_words() {
    let jace = compile_card(
        "Audit Jace",
        "Mana cost: {2}{U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 3\n+1: Draw a card.",
    );
    assert_eq!(jace.card.card_types, vec![CardType::Planeswalker]);
    assert_eq!(jace.card.subtypes, vec![Subtype::Jace]);
    let dual = compile_card(
        "Audit Dual Walker",
        "Mana cost: {2}{U}{U}\nType: Legendary Planeswalker — Jace Ajani\nLoyalty: 3\n+1: Draw a card.",
    );
    assert_eq!(
        dual.card.subtypes,
        vec![Subtype::Jace, Subtype::Ajani],
        "CR 306.3: each word after the dash is a separate planeswalker type"
    );
}

#[test]
fn cr_306_4_no_planeswalker_uniqueness_rule_only_legend_rule() {
    let mut game = two_players();
    let first = game.create_object_from_definition(
        &planeswalker_def("Jace audit one", 3, false),
        A,
        Zone::Battlefield,
    );
    let second = game.create_object_from_definition(
        &planeswalker_def("Jace audit two", 3, false),
        A,
        Zone::Battlefield,
    );
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(game.object(first).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.object(second).unwrap().zone,
        Zone::Battlefield,
        "CR 306.4: two planeswalkers sharing a planeswalker type coexist"
    );
    // Legendary planeswalkers are subject to the legend rule.
    let mut game = two_players();
    let legendary = planeswalker_def("Jace, Audit Legend", 3, true);
    let one = game.create_object_from_definition(&legendary, A, Zone::Battlefield);
    let two = game.create_object_from_definition(&legendary, A, Zone::Battlefield);
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    let remaining = [one, two]
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        })
        .count();
    assert_eq!(
        remaining, 1,
        "CR 306.4 / 704.5j: legendary planeswalkers with the same name obey the legend rule"
    );
}

#[test]
fn cr_306_5_loyalty_is_a_characteristic_only_planeswalkers_have() {
    use ironsmith::Until;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let mut game = two_players();
    let walker = game.create_object_from_definition(
        &planeswalker_def("Typeless audit walker", 3, false),
        A,
        Zone::Battlefield,
    );
    assert_eq!(
        game.calculated_characteristics(walker).unwrap().loyalty,
        Some(3)
    );
    ApplyContinuousEffect::new(
        EffectTarget::Specific(walker),
        Modification::SetCardTypes(vec![CardType::Artifact]),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(walker, A))
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_card_types(walker),
        Some(vec![CardType::Artifact])
    );
    assert_eq!(
        game.calculated_characteristics(walker).unwrap().loyalty,
        None,
        "CR 306.5: an object that is no longer a planeswalker has no loyalty characteristic"
    );
}

#[test]
fn cr_306_6_opposing_planeswalkers_can_be_attacked() {
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{Ability, Phase, StaticAbility, Step};
    let mut game = two_players();
    game.turn.active_player = A;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let attacker = CardDefinitionBuilder::new(CardId::new(), "Audit attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_ability(Ability::static_ability(StaticAbility::haste()))
        .build();
    let attacker = game.create_object_from_definition(&attacker, A, Zone::Battlefield);
    let theirs = game.create_object_from_definition(
        &planeswalker_def("Opposing audit walker", 4, false),
        B,
        Zone::Battlefield,
    );
    let mine = game.create_object_from_definition(
        &planeswalker_def("Own audit walker", 4, false),
        A,
        Zone::Battlefield,
    );
    let combat = combat_state::new_combat();
    let option = ironsmith::compute_legal_attackers(&game, &combat)
        .into_iter()
        .find(|option| option.creature == attacker)
        .expect("the hasty creature can attack");
    assert!(
        option
            .valid_targets
            .contains(&AttackTarget::Planeswalker(theirs)),
        "CR 306.6: an opponent's planeswalker can be attacked"
    );
    assert!(
        !option
            .valid_targets
            .contains(&AttackTarget::Planeswalker(mine)),
        "CR 506.2: a player's own planeswalker isn't an attack option"
    );
}

#[test]
fn cr_306_7_noncombat_damage_to_a_player_is_not_redirected_to_planeswalkers() {
    use ironsmith::effects::DealDamageEffect;
    let mut game = two_players();
    let source = creature(&mut game, A, 2, 2);
    let walker = game.create_object_from_definition(
        &planeswalker_def("Unredirected audit walker", 4, false),
        B,
        Zone::Battlefield,
    );
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    DealDamageEffect::new(3, ChooseSpec::Player(PlayerFilter::Specific(B)))
        .execute(
            &mut game,
            &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    assert_eq!(game.player(B).unwrap().life, 17);
    assert_eq!(
        game.counter_count(walker, CounterType::Loyalty),
        4,
        "CR 306.7: the removed redirection rule must not move player damage to a planeswalker"
    );
}

#[test]
fn cr_306_9_zero_loyalty_planeswalker_goes_to_owners_graveyard() {
    let mut game = two_players();
    let walker = game.create_object_from_definition(
        &planeswalker_def("Depleted audit walker", 3, false),
        B,
        Zone::Battlefield,
    );
    let _ = game.set_current_controller(walker, A);
    let stable = game.object(walker).unwrap().stable_id;
    game.remove_counters(walker, CounterType::Loyalty, 2, None, None);
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(
        current_zone(&game, stable),
        Some(Zone::Battlefield),
        "loyalty one survives"
    );
    game.remove_counters(walker, CounterType::Loyalty, 1, None, None);
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    let id = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        game.object(id).unwrap().zone,
        Zone::Graveyard,
        "CR 306.9: a planeswalker with loyalty 0 is put into a graveyard"
    );
    assert_eq!(game.object(id).unwrap().owner, B);
    assert!(
        game.player(B).unwrap().graveyard.contains(&id),
        "CR 306.9: it goes to its owner's graveyard"
    );
}

// ---------------------------------------------------------------------------
// 307 Sorceries / 308 Kindreds
// ---------------------------------------------------------------------------

#[test]
fn cr_307_2_sorcery_follows_its_text_then_goes_to_owners_graveyard() {
    use ironsmith::Effect;
    use ironsmith::game_state::StackEntry;
    let mut game = two_players();
    let sorcery = CardDefinitionBuilder::new(CardId::new(), "Audit sorcery")
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(2)])
        .build();
    let spell = game.create_object_from_definition(&sorcery, B, Zone::Stack);
    let stable = game.object(spell).unwrap().stable_id;
    game.stage_initial_controller_for_assembly(spell, A);
    game.stack.push(StackEntry::new(spell, A));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.player(A).unwrap().life,
        22,
        "CR 307.2: the sorcery's instructions are followed for its controller"
    );
    let id = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(id).unwrap().zone, Zone::Graveyard);
    assert!(
        game.player(B).unwrap().graveyard.contains(&id),
        "CR 307.2: then it's put into its owner's graveyard"
    );
}

#[test]
fn cr_307_3_spell_types_are_separate_single_word_subtypes() {
    let sorcery = compile_card(
        "Audit Arcane Lesson",
        "Mana cost: {R}\nType: Sorcery — Arcane Lesson\nAudit Arcane Lesson deals 1 damage to any target.",
    );
    assert_eq!(sorcery.card.card_types, vec![CardType::Sorcery]);
    assert_eq!(
        sorcery.card.subtypes,
        vec![Subtype::Arcane, Subtype::Lesson],
        "CR 307.3: each word after the dash is a separate spell type"
    );
    let instant = compile_card(
        "Audit Arcane Instant",
        "Mana cost: {R}\nType: Instant — Arcane\nAudit Arcane Instant deals 1 damage to any target.",
    );
    assert_eq!(
        instant.card.subtypes,
        vec![Subtype::Arcane],
        "CR 307.3: instants and sorceries share the spell types"
    );
}

#[test]
fn cr_307_5a_sorcery_timing_fact_requires_own_main_phase_and_empty_stack() {
    use ironsmith::game_state::StackEntry;
    use ironsmith::{Effect, LegalAction, Phase, Step};
    let reflexes = compile_card(
        "Audit Reflexes",
        "Mana cost: {0}\nType: Enchantment\nYou may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.",
    );
    let mut mismatches = Vec::new();
    // (own turn, phase, occupied stack, expected "could have cast a sorcery")
    let cases = [
        (true, Phase::FirstMain, false, true),
        (true, Phase::NextMain, false, true),
        (true, Phase::FirstMain, true, false),
        (false, Phase::FirstMain, false, false),
        (true, Phase::Combat, false, false),
        (true, Phase::Ending, false, false),
        (true, Phase::Beginning, false, false),
    ];
    for (own_turn, phase, occupied, expected) in cases {
        let mut game = two_players();
        game.turn.active_player = if own_turn { A } else { B };
        game.turn.priority_player = Some(A);
        game.turn.phase = phase;
        game.turn.step = match phase {
            Phase::Combat => Some(Step::DeclareAttackers),
            Phase::Ending => Some(Step::End),
            Phase::Beginning => Some(Step::Upkeep),
            _ => None,
        };
        let card = game.create_object_from_definition(&reflexes, A, Zone::Hand);
        if occupied {
            let source = creature(&mut game, A, 1, 1);
            game.stack
                .push(StackEntry::ability(source, A, vec![Effect::gain_life(1)]));
        }
        let Some(action) = legal_actions(&game, A)
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        else {
            mismatches.push(format!(
                "own_turn={own_turn} phase={phase:?} occupied={occupied}: flash cast not offered"
            ));
            continue;
        };
        let spell = finish_cast(
            &mut game,
            action,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        );
        let actual = game
            .object(spell)
            .unwrap()
            .optional_costs_paid
            .was_cast_at_sorcery_timing();
        if actual != expected {
            mismatches.push(format!(
                "own_turn={own_turn} phase={phase:?} occupied={occupied}: recorded sorcery-timing={actual}, expected {expected}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 307.5a: 'any time a sorcery couldn't have been cast' checks priority, own main phase and empty stack: {mismatches:#?}"
    );
}

#[test]
fn cr_308_1_kindred_cards_resolve_as_their_other_card_type() {
    use ironsmith::Effect;
    use ironsmith::game_state::StackEntry;
    for permanent in [false, true] {
        let mut game = two_players();
        let definition = CardDefinitionBuilder::new(CardId::new(), "Audit kindred card")
            .card_types(if permanent {
                vec![CardType::Kindred, CardType::Enchantment]
            } else {
                vec![CardType::Kindred, CardType::Instant]
            })
            .subtypes(vec![Subtype::Goblin])
            .with_spell_effect(if permanent {
                vec![]
            } else {
                vec![Effect::gain_life(2)]
            })
            .build();
        let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
        let stable = game.object(spell).unwrap().stable_id;
        game.stack.push(StackEntry::new(spell, A));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let expected = if permanent {
            Zone::Battlefield
        } else {
            Zone::Graveyard
        };
        assert_eq!(
            current_zone(&game, stable),
            Some(expected),
            "CR 308.1: a kindred card resolves following its other card type (permanent={permanent})"
        );
        if !permanent {
            assert_eq!(game.player(A).unwrap().life, 22);
        }
    }
}

#[test]
fn cr_308_2_kindred_subtypes_are_creature_types() {
    use ironsmith::ObjectFilter;
    let kindred = compile_card(
        "Audit Merfolk Kindred",
        "Mana cost: {1}{U}\nType: Kindred Enchantment — Merfolk\nMerfolk you control get +1/+1.",
    );
    assert_eq!(
        kindred.card.card_types,
        vec![CardType::Kindred, CardType::Enchantment]
    );
    assert_eq!(kindred.card.subtypes, vec![Subtype::Merfolk]);
    let multi = compile_card(
        "Audit Goblin Rogue Kindred",
        "Mana cost: {R}\nType: Kindred Instant — Goblin Rogue\nAudit Goblin Rogue Kindred deals 1 damage to any target.",
    );
    assert_eq!(
        multi.card.subtypes,
        vec![Subtype::Goblin, Subtype::Rogue],
        "CR 308.2: kindreds may have multiple creature types"
    );
    let mut game = two_players();
    let id = game.create_object_from_definition(&kindred, A, Zone::Battlefield);
    let ctx = EffectContext::new_default(id, A);
    assert!(
        ironsmith::effects::helpers::resolve_objects_from_spec(
            &game,
            &ChooseSpec::All(ObjectFilter::permanent().with_subtype(Subtype::Merfolk)),
            &ctx
        )
        .unwrap()
        .contains(&id),
        "CR 308.2: the kindred permanent is a Merfolk for creature-type queries"
    );
}

// ---------------------------------------------------------------------------
// 310 Battles
// ---------------------------------------------------------------------------

fn battle_def(name: &str, defense: u32, siege: bool) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Battle])
        .subtypes(if siege { vec![Subtype::Siege] } else { vec![] })
        .defense(defense)
        .build()
}

#[test]
fn cr_310_3_battle_subtypes_compile() {
    let siege = compile_card(
        "Audit Siege",
        "Mana cost: {2}{R}\nType: Battle — Siege\nDefense: 5\nWhen Audit Siege enters, it deals 2 damage to any target.",
    );
    assert_eq!(siege.card.card_types, vec![CardType::Battle]);
    assert_eq!(
        siege.card.subtypes,
        vec![Subtype::Siege],
        "CR 310.3: the word after the dash is the battle type"
    );
}

#[test]
fn cr_310_4c_310_6_battle_defense_is_its_counters_and_damage_removes_them() {
    use ironsmith::effects::DealDamageEffect;
    let mut game = two_players();
    let battle = game.create_object_from_definition(
        &battle_def("Audit battle", 5, false),
        B,
        Zone::Battlefield,
    );
    assert_eq!(game.battle_protector(battle), Some(B));
    let stable = game.object(battle).unwrap().stable_id;
    let source = creature(&mut game, A, 2, 2);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    DealDamageEffect::new(3, ChooseSpec::SpecificObject(battle))
        .execute(
            &mut game,
            &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    assert_eq!(
        game.counter_count(battle, CounterType::Defense),
        2,
        "CR 310.6: damage dealt to a battle removes that many defense counters"
    );
    assert_eq!(
        game.damage_on(battle),
        0,
        "CR 310.6: battle damage is not marked"
    );
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(current_zone(&game, stable), Some(Zone::Battlefield));
    game.add_counters(battle, CounterType::Defense, 2).unwrap();
    // Defense is now four (counters), above the printed five-minus-damage path.
    DealDamageEffect::new(3, ChooseSpec::SpecificObject(battle))
        .execute(
            &mut game,
            &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(
        current_zone(&game, stable),
        Some(Zone::Battlefield),
        "CR 310.4c: defense is the defense-counter count (four), so three damage leaves one"
    );
    assert_eq!(game.counter_count(battle, CounterType::Defense), 1);
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(battle))
        .execute(
            &mut game,
            &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm),
        )
        .unwrap();
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(
        current_zone(&game, stable),
        Some(Zone::Graveyard),
        "CR 310.4c / 704.5v: zero defense counters means zero defense"
    );
}

#[test]
fn cr_310_9e_player_who_protects_a_battle_is_its_protector() {
    use ironsmith::ObjectFilter;
    let tracker = compile_card(
        "Portent Tracker",
        "Mana cost: {1}{G}\nType: Creature — Human Scout\nPower/Toughness: 2/2\n{T}: Untap target land.\n{T}: Choose target battle. If an opponent protects it, remove a defense counter from it. Otherwise, put a defense counter on it. Activate only as a sorcery.",
    );
    assert_eq!(tracker.abilities.len(), 2);
    let mut game = three_players();
    // Siege controlled by A, protected by B.
    let a_siege =
        game.create_object_from_definition(&battle_def("A siege", 4, true), A, Zone::Battlefield);
    assert!(game.set_battle_protector(a_siege, B));
    // Non-Siege battle controlled and protected by B.
    let b_battle =
        game.create_object_from_definition(&battle_def("B battle", 4, false), B, Zone::Battlefield);
    // Siege controlled by C, protected by A.
    let c_siege =
        game.create_object_from_definition(&battle_def("C siege", 4, true), C, Zone::Battlefield);
    assert!(game.set_battle_protector(c_siege, A));
    let ctx = EffectContext::new_default(a_siege, A);
    let protected_by = |player: PlayerFilter| {
        let mut filter = ObjectFilter::permanent().with_type(CardType::Battle);
        filter.protected_by = Some(player);
        let mut ids = ironsmith::effects::helpers::resolve_objects_from_spec(
            &game,
            &ChooseSpec::All(filter),
            &ctx,
        )
        .unwrap();
        ids.sort();
        ids
    };
    let mut expected_b = vec![a_siege, b_battle];
    expected_b.sort();
    assert_eq!(
        protected_by(PlayerFilter::Specific(B)),
        expected_b,
        "CR 310.9e: 'battles B protects' means battles whose protector is B, regardless of controller"
    );
    assert_eq!(protected_by(PlayerFilter::You), vec![c_siege]);
    let mut expected_opp = vec![a_siege, b_battle];
    expected_opp.sort();
    assert_eq!(protected_by(PlayerFilter::Opponent), expected_opp);
}

#[test]
fn cr_310_12b_siege_defeat_exiles_it_and_casts_the_back_face_free() {
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::triggers::TriggerQueue;
    let mut game = two_players();
    let front_id = CardId::new();
    let back_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Audit Siege Front")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .defense(2)
        .other_face(back_id)
        .other_face_name("Audit Siege Back")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let back = CardDefinitionBuilder::new(back_id, "Audit Siege Back")
        .card_types(vec![CardType::Sorcery])
        .other_face(front_id)
        .other_face_name("Audit Siege Front")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    game.register_linked_face_definition(&back);
    let battle = game.create_object_from_definition(&front, A, Zone::Battlefield);
    assert_eq!(game.battle_protector(battle), Some(B));
    // Removing a non-final counter does not trigger.
    let (_, first) = game
        .remove_counters(battle, CounterType::Defense, 1, None, Some(B))
        .unwrap();
    assert!(
        ironsmith::triggers::check_triggers(&game, &first).is_empty(),
        "CR 310.12b: only the last defense counter's removal triggers"
    );
    let (_, last) = game
        .remove_counters(battle, CounterType::Defense, 1, None, Some(B))
        .unwrap();
    let mut queue = TriggerQueue::new();
    for entry in ironsmith::triggers::check_triggers(&game, &last) {
        queue.add(entry);
    }
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "CR 310.12b: the intrinsic Siege ability triggers"
    );
    assert_eq!(game.stack[0].controller, A);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "CR 310.12b: the controller may cast it transformed"
    );
    let spell = game.object(game.stack[0].object_id).unwrap();
    assert_eq!(spell.name, "Audit Siege Back");
    assert_eq!(game.stack[0].controller, A);
}

// ---------------------------------------------------------------------------
// 309 Dungeons
// ---------------------------------------------------------------------------

const DUNGEON_A: &str = "P04 Audit Dungeon Alpha";
const DUNGEON_B: &str = "P04 Audit Dungeon Beta";

/// Entry -> {Left, Right}; Left -> Vault; Right -> Vault; Vault is bottommost.
/// Room abilities gain 1/2/3/4 life so each room's trigger is distinguishable.
fn register_audit_dungeon(name: &str) {
    use ironsmith::Effect;
    use ironsmith::ability::Ability;
    use ironsmith::triggers::Trigger;
    let mut definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Dungeon])
        .build();
    for (room, leads_to, life) in [
        ("Entry", vec!["Left", "Right"], 1),
        ("Left", vec!["Vault"], 2),
        ("Right", vec!["Vault"], 3),
        ("Vault", vec![], 4),
    ] {
        definition.abilities.push(Ability::triggered(
            Trigger::dungeon_room(room, leads_to.into_iter().map(String::from).collect()),
            vec![Effect::gain_life(life)],
        ));
    }
    ironsmith::dungeon::register_dungeon_definition(&definition).unwrap();
}

fn register_audit_dungeons() {
    register_audit_dungeon(DUNGEON_A);
    register_audit_dungeon(DUNGEON_B);
}

fn venture(game: &mut GameState, player: PlayerId, picks: &[&str]) {
    use ironsmith::effects::VentureIntoDungeonEffect;
    let source = creature(game, player, 1, 1);
    let mut dm = PickNamed(picks.iter().map(|s| s.to_string()).collect());
    let mut ctx = EffectContext::new_default(source, player).with_decision_maker(&mut dm);
    VentureIntoDungeonEffect::new(PlayerFilter::Specific(player))
        .execute(game, &mut ctx)
        .unwrap();
}

fn progress(game: &GameState, player: PlayerId) -> Option<(String, String)> {
    game.active_dungeon(player)
        .map(|p| (p.dungeon_name.clone(), p.room_name.clone()))
}

fn stack_and_resolve_room_abilities(game: &mut GameState) {
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::put_triggers_on_stack(game, &mut queue).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry(game).unwrap();
    }
}

#[test]
fn cr_309_2_2a_4a_venture_brings_the_chosen_dungeon_in_at_its_topmost_room() {
    register_audit_dungeons();
    let mut game = two_players();
    assert_eq!(
        progress(&game, A),
        None,
        "CR 309.2: dungeons begin outside the game"
    );
    venture(&mut game, A, &[DUNGEON_B]);
    assert_eq!(
        progress(&game, A),
        Some((DUNGEON_B.to_string(), "Entry".to_string())),
        "CR 309.2a / 309.4a: the chosen dungeon comes in with the marker on its topmost room"
    );
    assert_eq!(
        progress(&game, B),
        None,
        "only the venturing player's dungeon comes in"
    );
    assert!(
        ironsmith::dungeon::registered_dungeon_names()
            .iter()
            .any(|name| name == DUNGEON_A),
        "both dungeons remain available outside the game"
    );
}

#[test]
fn cr_309_3_5_5a_venture_moves_the_marker_along_a_chosen_arrow() {
    register_audit_dungeons();
    let mut game = two_players();
    venture(&mut game, A, &[DUNGEON_A]);
    venture(&mut game, A, &[DUNGEON_B, "Right"]);
    assert_eq!(
        progress(&game, A),
        Some((DUNGEON_A.to_string(), "Right".to_string())),
        "CR 309.3 / 309.5a: with a dungeon in command, venturing advances it along the chosen arrow instead of bringing in another"
    );
    venture(&mut game, A, &[DUNGEON_B]);
    assert_eq!(
        progress(&game, A),
        Some((DUNGEON_A.to_string(), "Vault".to_string())),
        "CR 309.5a: a single arrow is followed"
    );
    assert!(!game.has_completed_dungeon(A));
}

#[test]
fn cr_309_4c_room_abilities_trigger_and_are_controlled_by_the_dungeon_owner() {
    register_audit_dungeons();
    for player in [A, B] {
        let mut game = two_players();
        let before = game.player(player).unwrap().life;
        venture(&mut game, player, &[DUNGEON_A]);
        let mut queue = ironsmith::TriggerQueue::new();
        ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.stack.len(),
            1,
            "CR 309.4c: entering a room triggers its room ability"
        );
        assert_eq!(
            game.stack[0].controller, player,
            "CR 309.4c: the dungeon's owner controls its room ability"
        );
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(player).unwrap().life, before + 1);
        venture(&mut game, player, &["Left"]);
        stack_and_resolve_room_abilities(&mut game);
        assert_eq!(
            game.player(player).unwrap().life,
            before + 3,
            "CR 309.4c: the Left room's ability (2 life) resolves"
        );
    }
}

#[test]
fn cr_309_5b_309_7_venturing_from_the_bottom_room_completes_and_starts_another() {
    register_audit_dungeons();
    let mut game = two_players();
    game.set_active_dungeon(
        A,
        ironsmith::dungeon::ActiveDungeonProgress::new(DUNGEON_B, "Vault"),
    );
    venture(&mut game, A, &[DUNGEON_A]);
    assert!(
        game.has_completed_named_dungeon(A, DUNGEON_B),
        "CR 309.5b / 309.7: removing the finished dungeon completes it"
    );
    assert_eq!(
        progress(&game, A),
        Some((DUNGEON_A.to_string(), "Entry".to_string())),
        "CR 309.5b: a new dungeon is chosen and the marker goes on its topmost room"
    );
}

#[test]
fn cr_309_6_bottom_room_dungeon_leaves_after_its_room_ability_leaves_the_stack() {
    register_audit_dungeons();
    let mut game = two_players();
    game.set_active_dungeon(
        A,
        ironsmith::dungeon::ActiveDungeonProgress::new(DUNGEON_A, "Left"),
    );
    venture(&mut game, A, &[]);
    assert_eq!(progress(&game, A).unwrap().1, "Vault");
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        !game.has_completed_dungeon(A) && game.active_dungeon(A).is_some(),
        "CR 309.6: the dungeon stays while its room ability has triggered but not left the stack"
    );
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        !game.has_completed_dungeon(A),
        "CR 309.6: the dungeon stays while its room ability is on the stack"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(A).unwrap().life, 24);
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        game.has_completed_named_dungeon(A, DUNGEON_A) && game.active_dungeon(A).is_none(),
        "CR 309.6 / 309.7: the finished dungeon is removed (completed) as a state-based action"
    );
}

#[test]
fn cr_309_4_compiled_dungeon_rooms_form_the_printed_graph() {
    let lost_mine = compile_card(
        "Lost Mine of Phandelver",
        "Type: Dungeon\nCave Entrance — Scry 1. (Leads to: Goblin Lair, Mine Tunnels)\nGoblin Lair — Create a 1/1 red Goblin creature token. (Leads to: Storeroom, Dark Pool)\nMine Tunnels — Create a Treasure token. (Leads to: Dark Pool, Fungi Cavern)\nStoreroom — Put a +1/+1 counter on target creature. (Leads to: Temple of Dumathoin)\nDark Pool — Each opponent loses 1 life and you gain 1 life. (Leads to: Temple of Dumathoin)\nFungi Cavern — Target creature gets -4/-0 until your next turn. (Leads to: Temple of Dumathoin)\nTemple of Dumathoin — Draw a card.",
    );
    assert_eq!(lost_mine.card.card_types, vec![CardType::Dungeon]);
    let dungeon = ironsmith::dungeon::DungeonDefinition::from_card_definition(&lost_mine).unwrap();
    assert_eq!(
        dungeon.first_room().name,
        "Cave Entrance",
        "CR 309.4a: topmost room"
    );
    assert_eq!(
        dungeon.room("Mine Tunnels").unwrap().leads_to,
        vec!["Dark Pool".to_string(), "Fungi Cavern".to_string()],
        "CR 309.4 / 309.5a: arrows connect the rooms"
    );
    assert!(
        dungeon
            .room("Temple of Dumathoin")
            .unwrap()
            .leads_to
            .is_empty()
    );
    assert_eq!(dungeon.rooms.len(), 7);
}

// ---------------------------------------------------------------------------
// Nontraditional card types (309.1, 311.1, 312.1, 313.1, 314.1, 315) via the compiler
// ---------------------------------------------------------------------------

fn assert_compiled_type(name: &str, text: &str, expected: CardType, rule: &str) {
    let definition =
        try_compile(name, text).unwrap_or_else(|error| panic!("{name} must compile: {error}"));
    assert_eq!(
        definition.card.card_types,
        vec![expected],
        "CR {rule}: compiling '{text}' must keep the {expected:?} card type"
    );
}

#[test]
fn cr_309_1_dungeon_card_type_survives_compilation() {
    assert_compiled_type(
        "Audit Dungeon Type",
        "Type: Dungeon\nEntry — You gain 1 life. (Leads to: Exit)\nExit — You gain 2 life.",
        CardType::Dungeon,
        "309.1",
    );
}

#[test]
fn cr_311_1_plane_card_type_survives_compilation() {
    assert_compiled_type(
        "Audit Plane Type",
        "Type: Plane — Dominaria\nCreatures get +1/+1.",
        CardType::Plane,
        "311.1",
    );
}

#[test]
fn cr_312_1_phenomenon_card_type_survives_compilation() {
    assert_compiled_type(
        "Audit Phenomenon Type",
        "Type: Phenomenon\nWhen you encounter Audit Phenomenon Type, each player gains 3 life. (Then planeswalk away from this phenomenon.)",
        CardType::Phenomenon,
        "312.1",
    );
}

#[test]
fn cr_313_1_vanguard_card_type_survives_compilation() {
    assert_compiled_type(
        "Audit Vanguard Type",
        "Type: Vanguard\nCreatures you control get +1/+1.",
        CardType::Vanguard,
        "313.1",
    );
}

#[test]
fn cr_314_1_scheme_card_type_survives_compilation() {
    assert_compiled_type(
        "Audit Scheme Type",
        "Type: Scheme\nWhen you set this scheme in motion, you gain 3 life.",
        CardType::Scheme,
        "314.1",
    );
}

#[test]
fn cr_315_2_conspiracy_card_type_survives_compilation() {
    assert_compiled_type(
        "Audit Conspiracy Type",
        "Type: Conspiracy\nCreatures you control get +1/+1.",
        CardType::Conspiracy,
        "315.2",
    );
}

#[test]
fn cr_311_3_multiword_planar_type_is_one_subtype() {
    let definition = try_compile(
        "Audit Serra Plane",
        "Type: Plane — Serra's Realm\nWhenever chaos ensues, you gain 2 life.",
    )
    .expect("plane must compile");
    let subtypes = definition
        .card
        .subtypes
        .iter()
        .map(|subtype| subtype.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        subtypes,
        vec!["Serra's Realm".to_string()],
        "CR 311.3: all words after the dash are, collectively, a single planar type (got {:?})",
        definition.card.subtypes
    );
}

#[test]
fn cr_313_3_314_3_315_4_supplementary_cards_have_no_subtypes() {
    use ironsmith::{ArchenemyVariant, ConspiracySetupCard, PlanarCardKind};
    let with_subtype = |card_type: CardType, name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .subtypes(vec![Subtype::Goblin])
            .build()
    };
    let plain = |card_type: CardType, name: String| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build()
    };
    let mut rejected = Vec::new();
    // 313.3 Vanguard
    let mut game = two_players();
    rejected.push((
        "313.3 vanguard",
        game.enable_vanguard(vec![
            (A, with_subtype(CardType::Vanguard, "Subtyped vanguard")),
            (B, plain(CardType::Vanguard, "Plain vanguard".into())),
        ])
        .is_err(),
    ));
    // 314.3 Scheme
    let mut game = two_players();
    let mut deck: Vec<_> = (0..19)
        .map(|i| plain(CardType::Scheme, format!("Plain scheme {i}")))
        .collect();
    deck.push(with_subtype(CardType::Scheme, "Subtyped scheme"));
    rejected.push((
        "314.3 scheme",
        game.enable_archenemy(ArchenemyVariant::Default, vec![(A, deck)])
            .is_err(),
    ));
    // 315.4 Conspiracy
    let mut game = two_players();
    rejected.push((
        "315.4 conspiracy",
        game.enable_conspiracy(vec![(
            A,
            vec![ConspiracySetupCard {
                definition: with_subtype(CardType::Conspiracy, "Subtyped conspiracy"),
                agenda_names: vec![],
            }],
        )])
        .is_err(),
    ));
    let failures: Vec<_> = rejected.iter().filter(|(_, ok)| !ok).collect();
    assert!(
        failures.is_empty(),
        "CR 313.3/314.3/315.4: these card types have no subtypes, so a subtyped card must not enter play as one: {failures:?}"
    );
}

// ---------------------------------------------------------------------------
// 311 / 312 Planes and phenomena
// ---------------------------------------------------------------------------

fn plain_planar_deck(
    prefix: &str,
    count: usize,
    phenomenon_at: Option<usize>,
) -> Vec<(CardDefinition, ironsmith::PlanarCardKind)> {
    use ironsmith::PlanarCardKind;
    (0..count)
        .map(|index| {
            let phenomenon = phenomenon_at == Some(index);
            (
                CardDefinitionBuilder::new(CardId::new(), format!("{prefix} planar {index}"))
                    .card_types(vec![if phenomenon {
                        CardType::Phenomenon
                    } else {
                        CardType::Plane
                    }])
                    .build(),
                if phenomenon {
                    PlanarCardKind::Phenomenon
                } else {
                    PlanarCardKind::Plane
                },
            )
        })
        .collect()
}

fn compiled_plane_deck(
    prefix: &str,
    count: usize,
    rules: &str,
) -> Vec<(CardDefinition, ironsmith::PlanarCardKind)> {
    (0..count)
        .map(|index| {
            let mut definition = compile_card(
                &format!("{prefix} Plane {index}"),
                &format!("Type: Plane — Dominaria\n{rules}"),
            );
            definition.card.card_types = vec![CardType::Plane];
            definition.card.subtypes.clear();
            (definition, ironsmith::PlanarCardKind::Plane)
        })
        .collect()
}

fn assert_stays_in_command_and_uncastable(game: &mut GameState, id: ObjectId, label: &str) {
    use ironsmith::CastingMethod;
    for zone in [
        Zone::Graveyard,
        Zone::Hand,
        Zone::Library,
        Zone::Exile,
        Zone::Battlefield,
    ] {
        let result = game.move_object_by_effect(id, zone);
        assert_eq!(
            result,
            Some(id),
            "{label}: move to {zone:?} keeps the same object"
        );
        assert_eq!(
            game.object(id).unwrap().zone,
            Zone::Command,
            "{label}: an attempt to move it to {zone:?} leaves it in the command zone"
        );
    }
    assert!(
        !game.battlefield.contains(&id),
        "{label}: it is not a permanent"
    );
    assert!(
        !ironsmith::decision::can_cast_spell(
            game,
            game.object(id).unwrap().owner,
            game.object(id).unwrap(),
            &CastingMethod::Normal
        ),
        "{label}: it can't be cast"
    );
}

#[test]
fn cr_311_2_312_2_planar_cards_stay_in_command_and_cannot_be_cast() {
    use ironsmith::PlanarCardKind;
    let mut game = two_players();
    game.turn.active_player = A;
    game.enable_planechase(vec![
        (A, plain_planar_deck("Alice", 10, Some(4))),
        (B, plain_planar_deck("Bob", 10, None)),
    ])
    .unwrap();
    let plane = game.reveal_starting_plane().unwrap();
    assert_eq!(game.planar_card_kind(plane), Some(PlanarCardKind::Plane));
    assert_stays_in_command_and_uncastable(&mut game, plane, "CR 311.2 face-up plane");
    assert!(game.is_face_up_planar_object(plane));
    let face_down = game.planar_deck(B).unwrap()[0];
    assert_stays_in_command_and_uncastable(&mut game, face_down, "CR 311.2 face-down planar card");
    let mut current = plane;
    for _ in 0..10 {
        if game.planar_card_kind(current) == Some(PlanarCardKind::Phenomenon) {
            break;
        }
        current = game.planeswalk(A, current).unwrap();
    }
    assert_eq!(
        game.planar_card_kind(current),
        Some(PlanarCardKind::Phenomenon)
    );
    assert_stays_in_command_and_uncastable(&mut game, current, "CR 312.2 face-up phenomenon");
}

#[test]
fn cr_311_4_only_the_face_up_planes_static_ability_applies() {
    let mut game = two_players();
    game.turn.active_player = A;
    let bear = creature(&mut game, A, 2, 2);
    game.enable_planechase(vec![
        (
            A,
            compiled_plane_deck("Alice Anthem", 10, "Creatures get +1/+1."),
        ),
        (
            B,
            compiled_plane_deck("Bob Anthem", 10, "Creatures get +1/+1."),
        ),
    ])
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_power(bear),
        Some(2),
        "CR 311.4: face-down planar cards' static abilities don't apply"
    );
    let plane = game.reveal_starting_plane().unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_power(bear),
        Some(3),
        "CR 311.4: the face-up plane's static ability affects the game"
    );
    game.planeswalk(A, plane).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_power(bear), Some(3));
}

#[test]
fn cr_311_5_312_4_planar_controller_is_the_active_player_and_passes_on_departure() {
    let mut game = three_players();
    game.turn.active_player = A;
    game.enable_planechase(vec![
        (A, plain_planar_deck("Alice", 10, None)),
        (B, plain_planar_deck("Bob", 10, None)),
        (C, plain_planar_deck("Carol", 10, None)),
    ])
    .unwrap();
    let plane = game.reveal_starting_plane().unwrap();
    assert_eq!(game.planar_controller(), Some(A));
    assert_eq!(game.planar_controller_of_face(plane), Some(A));
    assert_eq!(
        game.controller_of_id(plane),
        Some(A),
        "CR 311.5: the controller of the face-up plane is the planar controller"
    );
    game.next_turn();
    assert_eq!(game.turn.active_player, B);
    assert_eq!(
        game.planar_controller(),
        Some(B),
        "CR 311.5: normally the planar controller is the active player"
    );
    assert_eq!(game.controller_of_id(plane), Some(B));
    game.leave_game(B).unwrap();
    assert_eq!(
        game.planar_controller(),
        Some(C),
        "CR 311.5 / 312.4: when the planar controller leaves, the next player in turn order becomes planar controller"
    );
    if let Some(face) = game.face_up_planar_objects().first().copied() {
        assert_eq!(game.controller_of_id(face), Some(C));
    }
}

#[test]
fn cr_311_7_chaos_ensues_triggers_only_the_face_up_planes_chaos_ability() {
    let mut game = two_players();
    game.turn.active_player = B;
    game.enable_planechase(vec![
        (
            A,
            compiled_plane_deck("Alice Chaos", 10, "Whenever chaos ensues, you gain 2 life."),
        ),
        (
            B,
            compiled_plane_deck("Bob Chaos", 10, "Whenever chaos ensues, you gain 2 life."),
        ),
    ])
    .unwrap();
    let plane = game.reveal_starting_plane().unwrap();
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert!(
        game.stack.is_empty(),
        "turning the plane face up isn't chaos"
    );
    game.chaos_ensues(B, plane).unwrap();
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "CR 311.7: only the face-up plane's chaos ability triggers"
    );
    assert_eq!(
        game.stack[0].controller, B,
        "the planar controller controls it"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(B).unwrap().life, 22);
    assert_eq!(game.player(A).unwrap().life, 20);
}

#[test]
fn cr_312_5_312_7_encounter_triggers_then_the_controller_planeswalks_away() {
    use ironsmith::PlanarCardKind;
    let mut game = two_players();
    game.turn.active_player = A;
    let mut alice = plain_planar_deck("Alice", 9, None);
    let mut phenomenon = compile_card(
        "Audit Encounter",
        "Type: Phenomenon\nWhen you encounter Audit Encounter, each player gains 3 life. (Then planeswalk away from this phenomenon.)",
    );
    phenomenon.card.card_types = vec![CardType::Phenomenon];
    alice.push((phenomenon, PlanarCardKind::Phenomenon));
    game.enable_planechase(vec![(A, alice), (B, plain_planar_deck("Bob", 10, None))])
        .unwrap();
    let mut current = game.reveal_starting_plane().unwrap();
    let mut queue = ironsmith::TriggerQueue::new();
    for _ in 0..10 {
        if game.planar_card_kind(current) == Some(PlanarCardKind::Phenomenon) {
            break;
        }
        // Drain earlier planeswalk observations before the next step.
        ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert!(game.stack.is_empty());
        current = game.planeswalk(A, current).unwrap();
    }
    assert_eq!(
        game.planar_card_kind(current),
        Some(PlanarCardKind::Phenomenon)
    );
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        game.is_face_up_planar_object(current),
        "CR 312.7: no planeswalk while the encounter ability has triggered but not left the stack"
    );
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "CR 312.5: encountering the phenomenon triggers it"
    );
    assert_eq!(game.stack[0].controller, A);
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(game.is_face_up_planar_object(current));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(A).unwrap().life, 23);
    assert_eq!(game.player(B).unwrap().life, 23);
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        game.object(current).is_none() || !game.is_face_up_planar_object(current),
        "CR 312.7: once the encounter ability has left the stack the planar controller planeswalks"
    );
    assert_eq!(game.face_up_planar_objects().len(), 1);
    assert_eq!(
        game.planar_card_kind(game.face_up_planar_objects()[0]),
        Some(PlanarCardKind::Plane)
    );
}

// ---------------------------------------------------------------------------
// 313 Vanguard (structural) / 314 Schemes
// ---------------------------------------------------------------------------

fn scheme_deck(prefix: &str, rules: Option<&str>, ongoing: bool) -> Vec<CardDefinition> {
    (0..20)
        .map(|index| {
            let name = format!("{prefix} Scheme {index}");
            let mut definition = match rules {
                Some(rules) => compile_card(&name, &format!("Type: Scheme\n{rules}")),
                None => CardDefinitionBuilder::new(CardId::new(), name).build(),
            };
            definition.card.card_types = vec![CardType::Scheme];
            definition.card.subtypes.clear();
            definition.card.supertypes = if ongoing {
                vec![Supertype::Ongoing]
            } else {
                vec![]
            };
            definition
        })
        .collect()
}

#[test]
fn cr_314_2_scheme_cards_stay_in_command_and_cannot_be_cast() {
    use ironsmith::ArchenemyVariant;
    let mut game = three_players();
    game.enable_archenemy(
        ArchenemyVariant::Default,
        vec![(A, scheme_deck("Plain", None, true))],
    )
    .unwrap();
    let face_down = game.scheme_deck(A).unwrap()[0];
    assert_stays_in_command_and_uncastable(&mut game, face_down, "CR 314.2 face-down scheme");
    let face_up = game.set_scheme_in_motion(A).unwrap();
    assert_stays_in_command_and_uncastable(&mut game, face_up, "CR 314.2 face-up scheme");
    assert!(game.is_face_up_scheme(face_up));
}

#[test]
fn cr_314_4_only_face_up_scheme_static_abilities_apply() {
    use ironsmith::ArchenemyVariant;
    let mut game = three_players();
    let bear = creature(&mut game, A, 2, 2);
    game.enable_archenemy(
        ArchenemyVariant::Default,
        vec![(
            A,
            scheme_deck("Anthem", Some("Creatures you control get +1/+1."), true),
        )],
    )
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_power(bear),
        Some(2),
        "CR 314.4: face-down schemes have no effect"
    );
    game.set_scheme_in_motion(A).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_power(bear),
        Some(3),
        "CR 314.4: a face-up scheme's static ability affects the game"
    );
}

#[test]
fn cr_314_7_this_scheme_means_the_source_scheme_card() {
    use ironsmith::ArchenemyVariant;
    let mut game = three_players();
    game.enable_archenemy(
        ArchenemyVariant::Default,
        vec![(
            A,
            scheme_deck(
                "Motion",
                Some("When you set this scheme in motion, you gain 3 life."),
                true,
            ),
        )],
    )
    .unwrap();
    let life_before = game.player(A).unwrap().life;
    let first = game.set_scheme_in_motion(A).unwrap();
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "CR 314.7 / 314.4: only the scheme set in motion triggers ('this scheme')"
    );
    assert_eq!(game.stack[0].object_id, first);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(A).unwrap().life, life_before + 3);
    // A second scheme set in motion triggers only itself, not the ongoing first one.
    let second = game.set_scheme_in_motion(A).unwrap();
    ironsmith::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].object_id, second);
}

#[test]
fn cr_314_5_scheme_owner_is_the_archenemy_and_controls_it() {
    use ironsmith::ArchenemyVariant;
    let mut game = three_players();
    game.enable_archenemy(
        ArchenemyVariant::Default,
        vec![(B, scheme_deck("Owned", None, true))],
    )
    .unwrap();
    let scheme = game.set_scheme_in_motion(B).unwrap();
    assert_eq!(game.object(scheme).unwrap().owner, B);
    assert_eq!(game.controller_of_id(scheme), Some(B));
    let _ = game.set_current_controller(scheme, A);
    assert_eq!(
        game.controller_of_id(scheme),
        Some(B),
        "CR 314.5: the controller of a face-up scheme is its owner"
    );
}

// ---------------------------------------------------------------------------
// 315 Conspiracies
// ---------------------------------------------------------------------------

fn conspiracy_def(name: &str, rules: &str) -> CardDefinition {
    let mut definition = compile_card(name, &format!("Type: Conspiracy\n{rules}"));
    definition.card.card_types = vec![CardType::Conspiracy];
    definition
}

#[test]
fn cr_315_2_315_5b_315_7_setup_puts_conspiracies_in_command_hidden_agendas_face_down() {
    use ironsmith::ConspiracySetupCard;
    let mut game = two_players();
    let hidden = conspiracy_def("Audit Hidden Agenda", "Hidden agenda");
    let public = conspiracy_def(
        "Audit Public Conspiracy",
        "Creatures you control get +1/+1.",
    );
    game.enable_conspiracy(vec![(
        A,
        vec![
            ConspiracySetupCard {
                definition: hidden,
                agenda_names: vec!["Grizzly Bears".into()],
            },
            ConspiracySetupCard {
                definition: public,
                agenda_names: vec![],
            },
        ],
    )])
    .unwrap();
    let cards = game.conspiracy_cards();
    assert_eq!(cards.len(), 2);
    let hidden = *cards
        .iter()
        .find(|id| game.is_face_down_conspiracy(**id))
        .expect("CR 315.2: hidden agenda conspiracies start face down");
    let public = *cards.iter().find(|id| **id != hidden).unwrap();
    for id in [hidden, public] {
        assert_eq!(game.object(id).unwrap().zone, Zone::Command, "CR 315.2");
        assert_eq!(game.object(id).unwrap().owner, A);
    }
    assert!(!game.is_face_down_conspiracy(public));
    assert_eq!(
        game.current_name(hidden).as_deref(),
        Some(""),
        "CR 315.5b: a face-down conspiracy has no name"
    );
    assert!(game.current_card_types(hidden).unwrap().is_empty());
    assert!(game.current_subtypes(hidden).unwrap_or_default().is_empty());
    assert_eq!(
        game.agenda_names_for(A, hidden).map(|names| names.to_vec()),
        Some(vec!["Grizzly Bears".to_string()]),
        "CR 315.7: you may look at your face-down conspiracy"
    );
    assert!(
        game.agenda_names_for(B, hidden).is_none(),
        "CR 315.7: you can't look at an opponent's face-down conspiracy"
    );
}

#[test]
fn cr_315_3_conspiracies_stay_in_command_and_cannot_be_cast() {
    use ironsmith::ConspiracySetupCard;
    let mut game = two_players();
    game.enable_conspiracy(vec![(
        A,
        vec![
            ConspiracySetupCard {
                definition: conspiracy_def(
                    "Audit Static Conspiracy",
                    "Creatures you control get +1/+1.",
                ),
                agenda_names: vec![],
            },
            ConspiracySetupCard {
                definition: conspiracy_def("Audit Secret Conspiracy", "Hidden agenda"),
                agenda_names: vec!["Shock".into()],
            },
        ],
    )])
    .unwrap();
    for id in game.conspiracy_cards() {
        assert_stays_in_command_and_uncastable(&mut game, id, "CR 315.3 conspiracy");
    }
}

#[test]
fn cr_315_5_only_face_up_conspiracy_static_abilities_apply() {
    use ironsmith::ConspiracySetupCard;
    let mut game = two_players();
    let bear = creature(&mut game, A, 2, 2);
    let hidden = conspiracy_def(
        "Audit Secret Anthem",
        "Hidden agenda\nCreatures you control get +1/+1.",
    );
    game.enable_conspiracy(vec![(
        A,
        vec![
            ConspiracySetupCard {
                definition: conspiracy_def("Audit Open Anthem", "Creatures you control get +1/+1."),
                agenda_names: vec![],
            },
            ConspiracySetupCard {
                definition: hidden,
                agenda_names: vec!["Grizzly Bears".into()],
            },
        ],
    )])
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_power(bear),
        Some(3),
        "CR 315.5: the face-up conspiracy's static ability applies, the face-down one's doesn't"
    );
    let hidden = game
        .conspiracy_cards()
        .into_iter()
        .find(|id| game.is_face_down_conspiracy(*id))
        .unwrap();
    game.turn.priority_player = Some(A);
    game.turn_conspiracy_face_up(A, hidden).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_power(bear),
        Some(4),
        "CR 315.5: once face up its static ability applies"
    );
}

#[test]
fn cr_315_6_conspiracy_owner_is_its_controller() {
    use ironsmith::ConspiracySetupCard;
    let mut game = two_players();
    game.enable_conspiracy(vec![(
        B,
        vec![ConspiracySetupCard {
            definition: conspiracy_def(
                "Audit Owned Conspiracy",
                "Creatures you control get +1/+1.",
            ),
            agenda_names: vec![],
        }],
    )])
    .unwrap();
    let id = game.conspiracy_cards()[0];
    assert_eq!(game.object(id).unwrap().owner, B);
    let _ = game.set_current_controller(id, A);
    assert_eq!(
        game.controller_of_id(id),
        Some(B),
        "CR 315.6: the controller of a conspiracy card is its owner"
    );
}

#[test]
fn cr_315_5a_power_play_conspiracy_changes_the_starting_player() {
    use ironsmith::ConspiracySetupCard;
    let compiled = try_compile(
        "Power Play",
        "Type: Conspiracy\n(Start the game with this conspiracy face up in the command zone.)\nYou are the starting player. If multiple players would be the starting player, one of those players is chosen at random.",
    );
    let definition = match compiled {
        Ok(definition) => definition,
        Err(error) => panic!("CR 315.5a: Power Play's start-of-game ability must compile: {error}"),
    };
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
        "CR 315.5a: Power Play's start-of-game ability must be implemented"
    );
    let mut definition = definition;
    definition.card.card_types = vec![CardType::Conspiracy];
    let mut mismatches = Vec::new();
    for owner in [A, B, C] {
        let mut game = three_players();
        game.enable_conspiracy(vec![(
            owner,
            vec![ConspiracySetupCard {
                definition: definition.clone(),
                agenda_names: vec![],
            }],
        )])
        .unwrap();
        let starting = game.randomize_starting_player();
        if starting != Some(owner) {
            mismatches.push(format!("owner={owner:?}: starting player {starting:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 315.5a: a conspiracy's ability can affect the start-of-game procedure (Power Play's owner starts): {mismatches:?}"
    );
}

#[test]
fn cr_312_3_phenomenon_cards_have_no_subtypes() {
    use ironsmith::PlanarCardKind;
    let with_subtype = |card_type: CardType, name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .subtypes(vec![Subtype::Goblin])
            .build()
    };
    let plain = |card_type: CardType, name: String| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build()
    };
    let mut rejected = Vec::new();
    // 312.3 Phenomenon
    let mut game = two_players();
    let deck = |owner: &str, subtyped: bool| {
        let mut cards: Vec<_> = (0..9)
            .map(|i| {
                (
                    plain(CardType::Plane, format!("{owner} plane {i}")),
                    PlanarCardKind::Plane,
                )
            })
            .collect();
        cards.push((
            if subtyped {
                with_subtype(
                    CardType::Phenomenon,
                    &format!("{owner} subtyped phenomenon"),
                )
            } else {
                plain(CardType::Phenomenon, format!("{owner} phenomenon"))
            },
            PlanarCardKind::Phenomenon,
        ));
        cards
    };
    let phenomenon_rejected = game
        .enable_planechase(vec![(A, deck("Alice", true)), (B, deck("Bob", false))])
        .is_err();
    let phenomenon_subtypes = if phenomenon_rejected {
        Vec::new()
    } else {
        game.objects_in_zone(Zone::Command)
            .into_iter()
            .filter(|id| game.planar_card_kind(*id) == Some(PlanarCardKind::Phenomenon))
            .flat_map(|id| game.current_subtypes(id).unwrap_or_default())
            .collect::<Vec<_>>()
    };
    rejected.push((
        "312.3 phenomenon",
        phenomenon_rejected || phenomenon_subtypes.is_empty(),
    ));
    assert!(
        rejected.iter().all(|(_, ok)| *ok),
        "CR 312.3: phenomenon cards have no subtypes, but planechase setup accepted a subtyped phenomenon and kept its subtypes: {phenomenon_subtypes:?}"
    );
}
