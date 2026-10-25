//! CR audit (in-review completion pass, package q03): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::EffectExecutor;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effects::ApplyContinuousEffect;
use ironsmith::events::KeywordActionKind;
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    Ability, AbilityKind, CardId, CardType, ChooseSpec, CounterType, DecisionMaker, LegalAction,
    ObjectId, Phase, PowerToughness, PriorityLoopState, PriorityResponse, StaticAbility, Subtype,
    Until, Zone,
};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture compile failed for {name}: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture: {name} must be fully implemented"
    );
    def
}

fn new_game(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn vanilla(name: &str, p: i32, t: i32, types: Vec<CardType>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types)
        .power_toughness(PowerToughness::fixed(p, t))
        .build()
}

#[derive(Default)]
struct Script {
    x: Option<u32>,
    prefer: Vec<Target>,
    accept: bool,
}

impl DecisionMaker for Script {
    fn decide_boolean(&mut self, g: &GameState, c: &BooleanContext) -> bool {
        if self.accept { true } else { SelectFirstDecisionMaker.decide_boolean(g, c) }
    }
    fn decide_number(&mut self, _g: &GameState, c: &NumberContext) -> u32 {
        match self.x {
            Some(x) => x.clamp(c.min, c.max),
            None => c.max,
        }
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(g, c)
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, _g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for req in &c.requirements {
            if let Some(t) = self.prefer.iter().find(|t| req.legal_targets.contains(t)) {
                out.push(*t);
            } else if let Some(t) = req.legal_targets.first() {
                out.push(*t);
            }
        }
        out
    }
}

fn complete_action<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: LegalAction,
    dm: &mut D,
    queue: &mut TriggerQueue,
) -> Result<(), String> {
    let mut state = PriorityLoopState::new(game.players_in_game());
    assert_eq!(game.turn.priority_player, Some(player), "fixture priority holder");
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..24 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if state.pending_method_selection.is_some() {
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(0),
                dm,
            );
        } else {
            let context = match &result {
                Ok(GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    Ok(())
}

fn legal(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    ironsmith::compute_legal_actions(game, player).unwrap()
}

fn cast<D: DecisionMaker>(game: &mut GameState, player: PlayerId, spell: ObjectId, dm: &mut D) {
    let action = legal(game, player)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("fixture spell must be castable");
    let mut queue = TriggerQueue::new();
    complete_action(game, player, action, dm, &mut queue).expect("fixture cast");
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).expect("stack triggers");
}

fn activation_indices(game: &GameState, player: PlayerId, source: ObjectId) -> Vec<usize> {
    legal(game, player)
        .into_iter()
        .filter_map(|a| match a {
            LegalAction::ActivateAbility { source: s, ability_index } if s == source => {
                Some(ability_index)
            }
            _ => None,
        })
        .collect()
}

fn activate<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    ability_index: usize,
    dm: &mut D,
) -> Result<(), String> {
    let mut queue = TriggerQueue::new();
    complete_action(
        game,
        player,
        LegalAction::ActivateAbility { source, ability_index },
        dm,
        &mut queue,
    )?;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm)
        .map_err(|e| format!("{e:?}"))
}

/// Pass priority until the stack is empty, putting pending triggers on the stack.
fn resolve_all<D: DecisionMaker>(game: &mut GameState, dm: &mut D) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    for _ in 0..80 {
        if game.stack.is_empty() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm)
                .expect("stack triggers");
            if game.stack.is_empty() {
                break;
            }
        }
        match ironsmith::game_loop::apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .expect("pass priority")
        {
            GameProgress::NeedsDecisionCtx(ctx) => {
                ironsmith::game_loop::apply_decision_context_with_dm(
                    game, &mut queue, &mut state, &ctx, dm,
                )
                .expect("answer decision");
            }
            GameProgress::GameOver(r) => panic!("unexpected game over {r:?}"),
            _ => {}
        }
    }
    assert!(game.stack.is_empty(), "stack did not empty");
}

fn find_named(game: &GameState, name: &str, zone: Zone) -> Option<ObjectId> {
    let ids: Vec<ObjectId> = match zone {
        Zone::Battlefield => game.battlefield.iter().copied().collect(),
        Zone::Graveyard => game.players.iter().flat_map(|p| p.graveyard.iter().copied()).collect(),
        _ => unreachable!(),
    };
    ids.into_iter()
        .find(|id| game.object(*id).is_some_and(|o| o.name.as_ref() as &str == name))
}

fn destroy_spell() -> CardDefinition {
    compile("Audit Destroy", "Mana cost: {0}\nType: Instant\nDestroy target creature.")
}

// ---------------------------------------------------------------------------
// 122.8 / 122.9
// ---------------------------------------------------------------------------

#[test]
fn cr_122_8_real_dies_and_leaves_triggers_place_each_kind_of_counter_the_departed_object_had() {
    let cards = [
        (
            "Iron Apprentice",
            "Mana cost: {1}\nType: Artifact Creature — Construct\nPower/Toughness: 0/0\nThis creature enters with a +1/+1 counter on it.\nWhen this creature dies, if it had counters on it, put those counters on target creature you control.",
        ),
        (
            "Hei Bai, Spirit of Balance",
            "Mana cost: {2}{W/B}{W/B}\nType: Legendary Creature — Bear Spirit\nPower/Toughness: 3/3\nWhenever Hei Bai enters or attacks, you may sacrifice another creature or artifact. If you do, put two +1/+1 counters on Hei Bai.\nWhen Hei Bai leaves the battlefield, put its counters on target creature you control.",
        ),
    ];
    for (name, text) in cards {
        let def = compile(name, text);
        let mut game = new_game(A);
        let donor = game.create_object_from_definition(&def, A, Zone::Battlefield);
        let recipient =
            game.create_object_from_definition(&vanilla("Recipient", 2, 2, vec![CardType::Creature]), A, Zone::Battlefield);
        game.add_counters(donor, CounterType::PlusOnePlusOne, 2);
        game.add_counters(donor, CounterType::Charge, 3);
        let before_plus = game.counter_count(donor, CounterType::PlusOnePlusOne);
        let spell = game.create_object_from_definition(&destroy_spell(), A, Zone::Hand);
        let mut dm = Script { prefer: vec![Target::Object(donor), Target::Object(recipient)], ..Default::default() };
        cast(&mut game, A, spell, &mut dm);
        resolve_all(&mut game, &mut dm);
        let gy = find_named(&game, name, Zone::Graveyard).expect("fixture: donor died");
        assert!(game.object(gy).unwrap().counters.is_empty(), "fixture: graveyard card is a new object");
        assert_eq!(
            (
                game.counter_count(recipient, CounterType::PlusOnePlusOne),
                game.counter_count(recipient, CounterType::Charge)
            ),
            (before_plus, 3),
            "CR 122.8: {name} must put the same number of each kind of counter the departed object had"
        );
    }
}

#[test]
fn cr_122_8_kind_restricted_modular_places_only_the_named_kind() {
    let worker = compile(
        "Arcbound Worker",
        "Mana cost: {1}\nType: Artifact Creature — Construct\nPower/Toughness: 0/0\nModular 1",
    );
    let mut game = new_game(A);
    let donor = game.create_object_from_definition(&worker, A, Zone::Battlefield);
    let recipient = game.create_object_from_definition(
        &vanilla("Artifact Recipient", 1, 1, vec![CardType::Artifact, CardType::Creature]),
        A,
        Zone::Battlefield,
    );
    game.add_counters(donor, CounterType::PlusOnePlusOne, 2);
    game.add_counters(donor, CounterType::Charge, 1);
    let plus = game.counter_count(donor, CounterType::PlusOnePlusOne);
    let spell = game.create_object_from_definition(&destroy_spell(), A, Zone::Hand);
    let mut dm = Script { prefer: vec![Target::Object(donor), Target::Object(recipient)], accept: true, ..Default::default() };
    cast(&mut game, A, spell, &mut dm);
    resolve_all(&mut game, &mut dm);
    assert!(find_named(&game, "Arcbound Worker", Zone::Graveyard).is_some(), "fixture: worker died");
    assert_eq!(
        (
            game.counter_count(recipient, CounterType::PlusOnePlusOne),
            game.counter_count(recipient, CounterType::Charge)
        ),
        (plus, 0),
        "CR 122.8: a kind-restricted instruction puts only that kind, in the departed object's number"
    );
}

#[test]
fn cr_122_9_sacrifice_cost_ability_puts_the_sacrificed_objects_counters() {
    for (restricted, text) in [
        (false, "Sacrifice this creature: Put its counters on target creature."),
        (true, "Sacrifice this creature: Put its +1/+1 counters on target creature."),
    ] {
        let def = compile(
            "Audit Counter Donor",
            &format!("Mana cost: {{1}}\nType: Creature — Ooze\nPower/Toughness: 1/1\n{text}"),
        );
        let mut game = new_game(A);
        let donor = game.create_object_from_definition(&def, A, Zone::Battlefield);
        let recipient =
            game.create_object_from_definition(&vanilla("Recipient", 2, 2, vec![CardType::Creature]), A, Zone::Battlefield);
        game.add_counters(donor, CounterType::PlusOnePlusOne, 2);
        game.add_counters(donor, CounterType::Charge, 3);
        let index = *activation_indices(&game, A, donor).first().expect("fixture: sacrifice ability offered");
        let mut dm = Script { prefer: vec![Target::Object(recipient)], ..Default::default() };
        activate(&mut game, A, donor, index, &mut dm).expect("fixture activation");
        assert!(find_named(&game, "Audit Counter Donor", Zone::Graveyard).is_some(), "fixture: sacrificed as cost");
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            (
                game.counter_count(recipient, CounterType::PlusOnePlusOne),
                game.counter_count(recipient, CounterType::Charge)
            ),
            (2, if restricted { 0 } else { 3 }),
            "CR 122.9 (restricted={restricted}): the sacrificed object's counters are placed on the target"
        );
    }
}

// ---------------------------------------------------------------------------
// 123.x stickers
// ---------------------------------------------------------------------------

fn sticker_all_four(game: &mut GameState, target: ObjectId, source: ObjectId) {
    use ironsmith::effects::PutStickerEffect;
    game.add_accessible_art_sticker(A);
    game.add_accessible_name_sticker(A, "Bright");
    game.add_accessible_pt_sticker(A, 0, 6, 7);
    game.add_accessible_ability_sticker(A, 0, vec![Ability::static_ability(StaticAbility::flying())], "Flying");
    for action in [
        KeywordActionKind::ArtSticker,
        KeywordActionKind::NameSticker,
        KeywordActionKind::PowerToughnessSticker,
        KeywordActionKind::AbilitySticker,
    ] {
        PutStickerEffect::new(ChooseSpec::SpecificObject(target), action)
            .execute(game, &mut EffectContext::new_default(source, A))
            .unwrap();
    }
}

#[test]
fn cr_123_1_become_copy_effect_ignores_stickers_on_the_copied_object() {
    use ironsmith::effects::continuous::RuntimeModification;
    let mut game = new_game(A);
    let donor = game.create_object_from_definition(&vanilla("Donor", 3, 4, vec![CardType::Creature]), A, Zone::Battlefield);
    let host = game.create_object_from_definition(&vanilla("Host", 1, 1, vec![CardType::Creature]), A, Zone::Battlefield);
    sticker_all_four(&mut game, donor, donor);
    assert_eq!(game.current_name(donor).as_deref(), Some("Bright Donor"), "fixture: name sticker applied");
    assert_eq!(game.current_power(donor), Some(6), "fixture: P/T sticker applied");
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(host),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: vec![],
            copy_exception_surface: None,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(host, A))
    .unwrap();
    assert_eq!(game.current_name(host).as_deref(), Some("Donor"), "CR 123.1: name sticker is not copiable");
    assert_eq!(
        (game.current_power(host), game.current_toughness(host)),
        (Some(3), Some(4)),
        "CR 123.1: P/T sticker is not copiable"
    );
    assert!(!game.object_has_ability(host, &StaticAbility::flying()), "CR 123.1: ability sticker is not copiable");
    assert_eq!(
        game.sticker_count_on_object(host, KeywordActionKind::Sticker, None),
        0,
        "CR 123.1: stickers are not objects/copiable values and stay on the original"
    );
}

#[test]
fn cr_123_4_stickered_condition_tracks_current_stickers_only() {
    use ironsmith::effects::PutStickerEffect;
    let tabby = compile(
        "Grabby Tabby",
        "Mana cost: {3}{G}\nType: Creature — Cat Guest\nPower/Toughness: 4/3\nThis creature has vigilance as long as you control a stickered permanent.",
    );
    for kind in [KeywordActionKind::NameSticker, KeywordActionKind::ArtSticker] {
        let mut game = new_game(A);
        let cat = game.create_object_from_definition(&tabby, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&vanilla("Other", 2, 2, vec![CardType::Creature]), A, Zone::Battlefield);
        let stable = game.object(other).unwrap().stable_id;
        assert!(!game.object_has_ability(cat, &StaticAbility::vigilance()), "CR 123.4: no stickered permanent yet");
        match kind {
            KeywordActionKind::NameSticker => {
                game.add_accessible_name_sticker(A, "Shiny");
            }
            _ => {
                game.add_accessible_art_sticker(A);
            }
        }
        PutStickerEffect::new(ChooseSpec::SpecificObject(other), kind)
            .execute(&mut game, &mut EffectContext::new_default(other, A))
            .unwrap();
        assert_eq!(game.sticker_count_on_object(other, KeywordActionKind::Sticker, None), 1, "fixture: placed");
        assert!(
            game.object_has_ability(cat, &StaticAbility::vigilance()),
            "CR 123.4 ({kind:?}): a permanent with any kind of sticker is stickered"
        );
        let in_hand = game.move_object_by_effect(other, Zone::Hand).unwrap();
        game.move_object_by_effect(in_hand, Zone::Battlefield).unwrap();
        let back = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.sticker_count_on_object(back, KeywordActionKind::Sticker, None), 0, "fixture: sticker left via hand");
        assert!(
            !game.object_has_ability(cat, &StaticAbility::vigilance()),
            "CR 123.4 ({kind:?}): an object that previously had stickers is not stickered"
        );
    }
}

#[test]
fn cr_123_7_activated_ability_sticker_grants_an_activatable_ability() {
    use ironsmith::effects::PutStickerEffect;
    let mut game = new_game(A);
    let host = game.create_object_from_definition(&vanilla("Host", 2, 2, vec![CardType::Creature]), A, Zone::Battlefield);
    game.remove_summoning_sickness(host);
    assert!(activation_indices(&game, A, host).is_empty(), "fixture: host has no activated ability");
    game.add_accessible_ability_sticker(
        A,
        0,
        vec![Ability::activated(ironsmith::cost::TotalCost::free(), vec![ironsmith::Effect::gain_life(2)])],
        "Gain 2 life",
    );
    PutStickerEffect::new(ChooseSpec::SpecificObject(host), KeywordActionKind::AbilitySticker)
        .execute(&mut game, &mut EffectContext::new_default(host, A))
        .unwrap();
    let indices = activation_indices(&game, A, host);
    assert_eq!(indices.len(), 1, "CR 123.7: the stickered object gains the printed activated ability");
    let mut dm = Script::default();
    activate(&mut game, A, host, indices[0], &mut dm).expect("activation");
    resolve_all(&mut game, &mut dm);
    assert_eq!(game.player(A).unwrap().life, 22, "CR 123.7: the gained ability works");
}

// ---------------------------------------------------------------------------
// 202.3e: actual X announcement
// ---------------------------------------------------------------------------

#[test]
fn cr_202_3e_cast_x_spell_has_chosen_x_on_stack_and_zero_elsewhere() {
    use ironsmith::snapshot::ObjectSnapshot;
    let blaze = compile("Blaze", "Mana cost: {X}{R}\nType: Sorcery\nBlaze deals X damage to any target.");
    let mut game = new_game(A);
    let spell = game.create_object_from_definition(&blaze, A, Zone::Hand);
    let stable = game.object(spell).unwrap().stable_id;
    let mv = |g: &GameState| {
        let id = g.find_object_by_stable_id(stable).unwrap();
        ObjectSnapshot::from_object(g.object(id).unwrap(), g).mana_value()
    };
    assert_eq!(mv(&game), 1, "CR 202.3e: X is 0 in hand");
    game.player_mut(A).unwrap().mana_pool.red = 4;
    let mut dm = Script { x: Some(3), prefer: vec![Target::Player(B)], ..Default::default() };
    cast(&mut game, A, spell, &mut dm);
    assert_eq!(game.stack.len(), 1, "fixture: Blaze on the stack");
    assert_eq!(mv(&game), 4, "CR 202.3e: X is the chosen 3 on the stack");
    resolve_all(&mut game, &mut dm);
    assert_eq!(game.player(B).unwrap().life, 17, "fixture: X=3 resolved");
    assert_eq!(mv(&game), 1, "CR 202.3e: X is 0 again in the graveyard");
}

// ---------------------------------------------------------------------------
// 208.3a / 301.7b
// ---------------------------------------------------------------------------

#[test]
fn cr_208_3a_base_setting_created_before_animation_applies_once_a_creature() {
    use ironsmith::effect::Value;
    let mut game = new_game(A);
    let def = CardDefinitionBuilder::new(CardId::new(), "Audit Vehicle")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(3, 4))
        .build();
    let id = game.create_object_from_definition(&def, A, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(id, A);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::SetPowerToughness {
            power: Value::Fixed(5),
            toughness: Value::Fixed(5),
            sublayer: ironsmith::continuous::PtSublayer::Setting,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(
        (game.current_power(id), game.current_toughness(id)),
        (Some(5), Some(5)),
        "CR 208.3a: a base-setting effect created before animation applies once the permanent is a creature"
    );
}

#[test]
fn cr_301_7b_crewed_vehicle_immediately_has_printed_pt() {
    let vehicle = compile(
        "Audit Crew Vehicle",
        "Mana cost: {2}\nType: Artifact — Vehicle\nPower/Toughness: 3/4\nCrew 1",
    );
    let mut game = new_game(A);
    let v = game.create_object_from_definition(&vehicle, A, Zone::Battlefield);
    let crew = game.create_object_from_definition(&vanilla("Crewer", 1, 1, vec![CardType::Creature]), A, Zone::Battlefield);
    game.remove_summoning_sickness(v);
    game.remove_summoning_sickness(crew);
    let index = *activation_indices(&game, A, v).first().expect("fixture: crew offered");
    let mut dm = Script::default();
    activate(&mut game, A, v, index, &mut dm).expect("crew activation");
    resolve_all(&mut game, &mut dm);
    assert!(game.current_is_creature(v), "fixture: crewed vehicle is a creature");
    assert_eq!(
        (game.current_power(v), game.current_toughness(v)),
        (Some(3), Some(4)),
        "CR 301.7b: a Vehicle that becomes a creature immediately has its printed P/T"
    );
}

// ---------------------------------------------------------------------------
// 302.6: summoning sickness after a control change
// ---------------------------------------------------------------------------

#[test]
fn cr_302_6_gaining_control_this_turn_blocks_tap_abilities() {
    let def = compile(
        "Audit Tapper",
        "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1\n{T}: You gain 1 life.",
    );
    let mut game = new_game(A);
    let tapper = game.create_object_from_definition(&def, B, Zone::Battlefield);
    game.remove_summoning_sickness(tapper);
    let own = game.create_object_from_definition(&def, A, Zone::Battlefield);
    game.remove_summoning_sickness(own);
    assert_eq!(activation_indices(&game, A, own).len(), 1, "control: a continuously controlled creature can tap");
    let src = game.create_object_from_definition(&vanilla("Thief", 1, 1, vec![CardType::Creature]), A, Zone::Battlefield);
    ApplyContinuousEffect::new(EffectTarget::Specific(tapper), Modification::ChangeController(A), Until::EndOfTurn)
        .execute(&mut game, &mut EffectContext::new_default(src, A))
        .unwrap();
    assert_eq!(game.current_controller(tapper), Some(A), "fixture: control changed");
    assert!(
        activation_indices(&game, A, tapper).is_empty(),
        "CR 302.6: a creature not continuously controlled since the turn began can't use {{T}} abilities"
    );
}

// ---------------------------------------------------------------------------
// 123.6c: name sticker position survives copy effects / fewer words
// ---------------------------------------------------------------------------

struct StickerPosition(usize);
impl DecisionMaker for StickerPosition {
    fn decide_options(&mut self, _g: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description == "Choose the name sticker's position" { vec![self.0] } else { vec![0] }
    }
}

#[test]
fn cr_123_6c_name_sticker_keeps_word_position_through_copy_effects() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::effects::continuous::RuntimeModification;
    for (copied, expected) in [
        ("Seeker of the Way", "Seeker of the Eldrazi Way"),
        ("Ox", "Ox Eldrazi"),
    ] {
        let mut game = new_game(A);
        let host = game.create_object_from_definition(
            &vanilla("It That Betrays", 11, 11, vec![CardType::Creature]),
            A,
            Zone::Battlefield,
        );
        let donor = game.create_object_from_definition(&vanilla(copied, 2, 3, vec![CardType::Creature]), B, Zone::Battlefield);
        game.add_accessible_name_sticker(A, "Eldrazi");
        let mut dm = StickerPosition(3);
        PutStickerEffect::new(ChooseSpec::SpecificObject(host), KeywordActionKind::NameSticker)
            .execute(&mut game, &mut EffectContext::new_default(host, A).with_decision_maker(&mut dm))
            .unwrap();
        assert_eq!(game.current_name(host).as_deref(), Some("It That Betrays Eldrazi"), "fixture: placed after word 3");
        ApplyContinuousEffect::new_runtime(
            EffectTarget::Specific(host),
            RuntimeModification::CopyOf {
                source: ChooseSpec::SpecificObject(donor),
                preserve_source_abilities: false,
                name_override: None,
                name_override_surface: None,
                add_supertypes: vec![],
                copy_exception_surface: None,
            },
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(host, A))
        .unwrap();
        assert_eq!(
            game.current_name(host).as_deref(),
            Some(expected),
            "CR 123.6c: the sticker applies to the new copiable name after the same number of words (or at the end if fewer)"
        );
    }
}

// ---------------------------------------------------------------------------
// 301.5d / 303.4e: who may activate attachment and granted abilities
// ---------------------------------------------------------------------------

#[test]
fn cr_301_5d_303_4e_attachment_controller_activates_its_own_abilities_host_controller_activates_granted_ones() {
    use ironsmith::effects::AttachToEffect;
    let equipment = compile(
        "Audit Granting Equipment",
        "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature has \"{T}: You gain 1 life.\"\n{0}: You gain 2 life.\nEquip {1}",
    );
    let aura = compile(
        "Audit Granting Aura",
        "Mana cost: {1}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has \"{T}: You gain 1 life.\"\n{0}: You gain 2 life.",
    );
    for (label, def) in [("Equipment", &equipment), ("Aura", &aura)] {
        let mut game = new_game(A);
        let attachment = game.create_object_from_definition(def, A, Zone::Battlefield);
        let host = game.create_object_from_definition(&vanilla("Bob Host", 2, 2, vec![CardType::Creature]), B, Zone::Battlefield);
        game.remove_summoning_sickness(host);
        AttachToEffect::new(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new_default(attachment, A))
            .unwrap();
        assert!(
            game.object(attachment).unwrap().attached_to.is_some(),
            "fixture: {label} attached to Bob's creature"
        );
        assert_eq!(game.current_controller(attachment), Some(A), "CR 301.5d/303.4e: attachment controller unchanged");
        assert_eq!(game.current_controller(host), Some(B), "CR 301.5d/303.4e: host controller unchanged");
        // Alice holds priority.
        let alice_attach = activation_indices(&game, A, attachment);
        let alice_host = activation_indices(&game, A, host);
        game.turn.priority_player = Some(B);
        let bob_attach = activation_indices(&game, B, attachment);
        let bob_host = activation_indices(&game, B, host);
        assert_eq!(alice_attach.len(), 1, "CR 301.5d/303.4e ({label}): the attachment's controller activates its abilities");
        assert!(bob_attach.is_empty(), "CR 301.5d/303.4e ({label}): the host's controller can't activate the attachment's abilities");
        assert_eq!(bob_host.len(), 1, "CR 301.5d/303.4e ({label}): the host's controller activates the granted ability");
        assert!(alice_host.is_empty(), "CR 301.5d/303.4e ({label}): the attachment's controller can't activate the granted ability");
    }
}
