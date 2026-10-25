//! CR audit (in-review completion pass, package q01): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::{DecisionMaker, EffectExecutor};

fn p(index: usize) -> PlayerId {
    PlayerId::from_index(index as _)
}

fn compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("fixture: strict compiler must accept {name}: {error:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
        "fixture: {name} compiled with unimplemented content"
    );
    definition
}

fn resolve_as_spell(
    game: &mut GameState,
    definition: &ironsmith::cards::CardDefinition,
    caster: PlayerId,
) {
    let spell = game.create_object_from_definition(definition, caster, ironsmith::Zone::Stack);
    game.stack
        .push(ironsmith::game_state::StackEntry::new(spell, caster));
    ironsmith::game_loop::resolve_stack_entry(game).unwrap();
}

fn plain_card(
    game: &mut GameState,
    name: &str,
    types: Vec<ironsmith::CardType>,
    owner: PlayerId,
    zone: ironsmith::Zone,
) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let mut builder =
        CardDefinitionBuilder::new(ironsmith::CardId::new(), name).card_types(types.clone());
    if types.contains(&ironsmith::CardType::Creature) {
        builder = builder.power_toughness(ironsmith::PowerToughness::fixed(2, 2));
    }
    game.create_object_from_definition(&builder.build(), owner, zone)
}

fn complete_priority_action_with_dm<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: ironsmith::LegalAction,
    dm: &mut D,
    queue: &mut ironsmith::triggers::TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        ironsmith::LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if let Some(pending) = &state.pending_method_selection {
            let index = pending
                .available_methods
                .iter()
                .position(|choice| Some(&choice.method) == requested_method.as_ref())
                .ok_or_else(|| "requested casting method lost".to_string())?;
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending native action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    if state.pending_cast.is_some()
        || state.pending_method_selection.is_some()
        || state.pending_activation.is_some()
    {
        return Err("native action exceeded decision bound".into());
    }
    Ok(())
}

struct NoChoices;
impl DecisionMaker for NoChoices {}

// ---------------------------------------------------------------------------
// 100.1a / 100.1b / 103.8c: two-player vs multiplayer is fixed by the number of
// players the game BEGAN with; in a multiplayer game the starting player draws.
// ---------------------------------------------------------------------------
#[test]
fn cr_100_1a_b_103_8c_first_draw_follows_the_starting_player_count() {
    use ironsmith::{CardType, Phase, Step, Zone};
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for (count, departed) in [
        (2usize, false),
        (3, false),
        (3, true),
        (4, false),
        (4, true),
    ] {
        let names = (0..count).map(|i| format!("Player {i}")).collect();
        let mut game = GameState::new(names, 20);
        game.turn.active_player = p(0);
        game.turn.turn_number = 1;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        for index in 0..count {
            plain_card(
                &mut game,
                "Top card",
                vec![CardType::Artifact],
                p(index),
                Zone::Library,
            );
        }
        if departed {
            // A nonstarting player leaves before the first draw step: the game
            // still began with more than two players (CR 100.1b).
            assert!(
                game.concede_game(p(count - 1)).unwrap(),
                "fixture: concession"
            );
            assert!(game.player(p(count - 1)).unwrap().has_left_game);
        }
        ironsmith::turn::execute_draw_step_with(&mut game, &mut NoChoices).unwrap();
        observed.push((count, departed, game.player(p(0)).unwrap().hand.len()));
        expected.push((count, departed, usize::from(count > 2)));
    }
    assert_eq!(
        observed, expected,
        "CR 100.1a/100.1b/103.8a/103.8c: only a game that began with two players skips the starting player's first draw"
    );
}

// ---------------------------------------------------------------------------
// 101.2: "can't" beats "may"/instructions.
// ---------------------------------------------------------------------------
#[test]
fn cr_101_2_cant_play_lands_overrides_additional_land_permission() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, LegalAction, Phase, StaticAbility, Zone};
    let alice = p(0);
    let mut observed = Vec::new();
    for prohibition in [false, true] {
        for played in [0u32, 1] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(alice).unwrap().lands_played_this_turn = played as _;
            let land = plain_card(
                &mut game,
                "Land in hand",
                vec![CardType::Land],
                alice,
                Zone::Hand,
            );
            let grant = CardDefinitionBuilder::new(CardId::new(), "Extra land grant")
                .card_types(vec![CardType::Enchantment])
                .with_ability(Ability::static_ability(
                    StaticAbility::additional_land_plays(1),
                ))
                .build();
            game.create_object_from_definition(&grant, alice, Zone::Battlefield);
            if prohibition {
                let ban = compile(
                    "Land ban",
                    "Mana cost: {2}{R}\nType: Enchantment\nYou can't play lands.",
                );
                game.create_object_from_definition(&ban, alice, Zone::Battlefield);
            }
            game.refresh_continuous_state().unwrap();
            let legal = ironsmith::compute_legal_actions(&game, alice)
                .unwrap()
                .iter()
                .any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == land));
            observed.push(((prohibition, played), legal));
        }
    }
    assert_eq!(
        observed,
        vec![
            ((false, 0), true),
            ((false, 1), true),
            ((true, 0), false),
            ((true, 1), false)
        ],
        "CR 101.2: 'you can't play lands' takes precedence over an additional land play permission"
    );
}

#[test]
fn cr_101_2_cant_gain_life_overrides_a_gain_life_instruction() {
    use ironsmith::{CardType, Effect, Zone};
    let alice = p(0);
    for prohibition in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = plain_card(
            &mut game,
            "Life source",
            vec![CardType::Artifact],
            alice,
            Zone::Battlefield,
        );
        if prohibition {
            let ban = compile(
                "Life ban",
                "Mana cost: {2}{B}\nType: Enchantment\nPlayers can't gain life.",
            );
            game.create_object_from_definition(&ban, p(1), Zone::Battlefield);
        }
        game.stack.push(ironsmith::game_state::StackEntry::ability(
            source,
            alice,
            vec![Effect::gain_life(3)],
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.player(alice).unwrap().life,
            if prohibition { 20 } else { 23 },
            "CR 101.2: 'players can't gain life' overrides 'you gain 3 life' (prohibition={prohibition})"
        );
    }
}

// ---------------------------------------------------------------------------
// 104.2a: last player standing wins immediately despite "can't win" effects.
// ---------------------------------------------------------------------------
#[test]
fn cr_104_2a_last_remaining_player_wins_despite_cant_win_effect() {
    use ironsmith::{GameProgress, GameResult};
    let alice = p(0);
    let bob = p(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    use ironsmith::effect::Restriction;
    use ironsmith::{CardType, Effect, PlayerFilter, Until, Zone};
    // Bob's resolving instruction: "your opponents can't win the game this turn".
    let source = plain_card(
        &mut game,
        "Grace source",
        vec![CardType::Instant],
        bob,
        Zone::Graveyard,
    );
    game.stack.push(ironsmith::game_state::StackEntry::ability(
        source,
        bob,
        vec![Effect::cant_until(
            Restriction::win_game(PlayerFilter::Opponent),
            Until::EndOfTurn,
        )],
    ));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(
        !game.can_win_game(alice),
        "fixture: Bob's resolved spell must stop Alice from winning this turn"
    );
    assert!(
        game.concede_game(bob).unwrap(),
        "fixture: concession is unconditional"
    );
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let progress = ironsmith::game_loop::advance_priority(&mut game, &mut queue).unwrap();
    assert!(
        matches!(progress, GameProgress::GameOver(GameResult::Winner(id)) if id == alice),
        "CR 104.2a: Alice's only opponent left the game, so she wins despite 'can't win'; got {progress:?}"
    );
}

// ---------------------------------------------------------------------------
// 104.3c / 104.3d boundaries.
// ---------------------------------------------------------------------------
#[test]
fn cr_104_3c_overdraw_draws_the_remaining_cards_then_loses_at_the_next_check() {
    use ironsmith::effects::DrawCardsEffect;
    use ironsmith::{CardType, GameProgress, GameResult, Zone};
    let alice = p(0);
    let bob = p(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = plain_card(
        &mut game,
        "Draw source",
        vec![CardType::Artifact],
        alice,
        Zone::Battlefield,
    );
    for _ in 0..2 {
        plain_card(
            &mut game,
            "Library card",
            vec![CardType::Artifact],
            alice,
            Zone::Library,
        );
    }
    DrawCardsEffect::you(3)
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        2,
        "CR 104.3c: the player draws the remaining cards"
    );
    assert!(game.player(alice).unwrap().library.is_empty());
    assert!(
        !game.player(alice).unwrap().has_lost,
        "CR 104.3c: the loss waits until a player would receive priority"
    );
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let progress = ironsmith::game_loop::advance_priority(&mut game, &mut queue).unwrap();
    assert!(
        game.player(alice).unwrap().has_lost,
        "CR 104.3c: player loses at the SBA check"
    );
    assert!(matches!(progress, GameProgress::GameOver(GameResult::Winner(id)) if id == bob));
}

#[test]
fn cr_104_3d_exactly_ten_poison_counters_lose_but_nine_do_not() {
    let mut observed = Vec::new();
    for poison in [9u32, 10, 11] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.player_mut(p(0)).unwrap().poison_counters = poison as _;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::advance_priority(&mut game, &mut queue).unwrap();
        observed.push((poison, game.player(p(0)).unwrap().has_lost));
    }
    assert_eq!(
        observed,
        vec![(9, false), (10, true), (11, true)],
        "CR 104.3d: ten or more poison counters lose the game"
    );
}

// ---------------------------------------------------------------------------
// 105.3: color-changing effects.
// ---------------------------------------------------------------------------
#[test]
fn cr_105_3_new_colors_replace_old_unless_in_addition_and_colorless_is_possible() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, ColorSet, ManaCost, ManaSymbol, Until, Zone};
    let alice = p(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let green = CardDefinitionBuilder::new(CardId::new(), "Green subject")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
        .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
        .build();
    let subject = game.create_object_from_definition(&green, alice, Zone::Battlefield);
    let colorless = plain_card(
        &mut game,
        "Colorless subject",
        vec![CardType::Artifact],
        alice,
        Zone::Battlefield,
    );
    assert_eq!(game.current_colors(subject), Some(ColorSet::GREEN));
    assert_eq!(game.current_colors(colorless), Some(ColorSet::COLORLESS));
    let apply = |game: &mut GameState, target, modification| {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(target),
            modification,
            Until::EndOfTurn,
        )
        .execute(game, &mut EffectContext::new_default(target, alice))
        .unwrap();
    };
    apply(&mut game, subject, Modification::SetColors(ColorSet::RED));
    assert_eq!(
        game.current_colors(subject),
        Some(ColorSet::RED),
        "CR 105.3: new color replaces green"
    );
    apply(&mut game, subject, Modification::AddColors(ColorSet::BLUE));
    assert_eq!(
        game.current_colors(subject),
        Some(ColorSet::RED.union(ColorSet::BLUE)),
        "CR 105.3: 'in addition' keeps red"
    );
    apply(&mut game, subject, Modification::MakeColorless);
    assert_eq!(
        game.current_colors(subject),
        Some(ColorSet::COLORLESS),
        "CR 105.3: becomes colorless"
    );
    apply(
        &mut game,
        colorless,
        Modification::SetColors(ColorSet::WHITE),
    );
    assert_eq!(
        game.current_colors(colorless),
        Some(ColorSet::WHITE),
        "CR 105.3: an effect can give a colorless object a color"
    );
}

// ---------------------------------------------------------------------------
// 106.12: only a mana ability with {T} in its cost is "tapping for mana".
// ---------------------------------------------------------------------------
#[test]
fn cr_106_12_only_tap_symbol_mana_abilities_count_as_tapping_for_mana() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{Ability, CardId, CardType, Effect, ObjectFilter, PlayerFilter, Zone};
    let alice = p(0);
    let mut observed = Vec::new();
    for (label, text) in [
        ("tap", "Mana cost: {0}\nType: Artifact\n{T}: Add {G}."),
        (
            "sacrifice",
            "Mana cost: {0}\nType: Artifact\nSacrifice this artifact: Add {G}.",
        ),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let observer = CardDefinitionBuilder::new(CardId::new(), "Tap-for-mana observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::permanent()),
                vec![Effect::gain_life(3)],
            ))
            .build();
        let observer = game.create_object_from_definition(&observer, alice, Zone::Battlefield);
        let definition = compile("Mana source", text);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(
            game.player(alice).unwrap().mana_pool.green,
            1,
            "fixture: {label} produced G"
        );
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let count = queue
            .entries
            .iter()
            .filter(|e| e.source == observer)
            .count();
        observed.push((label, count));
    }
    assert_eq!(
        observed,
        vec![("tap", 1), ("sacrifice", 0)],
        "CR 106.12: only activating a {{T}} mana ability is tapping a permanent for mana"
    );
}

// ---------------------------------------------------------------------------
// 107.3a / 107.4b: X is announced by the caster and generic X is payable with any type.
// ---------------------------------------------------------------------------
#[test]
fn cr_107_3a_107_4b_cast_x_is_announced_and_paid_with_any_mana_type() {
    use ironsmith::decisions::context::NumberContext;
    use ironsmith::{LegalAction, ManaSymbol, Phase, Zone};
    struct AnnounceX {
        value: u32,
        calls: usize,
    }
    impl DecisionMaker for AnnounceX {
        fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
            self.calls += 1;
            assert!(
                ctx.max >= self.value,
                "fixture: X={} must be selectable",
                self.value
            );
            self.value
        }
    }
    let definition = compile(
        "X life audit",
        "Mana cost: {X}{G}\nType: Sorcery\nYou gain X life.",
    );
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for caster in [p(0), p(1)] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            for x in [0u32, 3] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = caster;
                game.turn.priority_player = Some(caster);
                game.turn.phase = Phase::FirstMain;
                let card = game.create_object_from_definition(&definition, caster, Zone::Hand);
                let pool = &mut game.player_mut(caster).unwrap().mana_pool;
                pool.add(ManaSymbol::Green, 1);
                pool.add(symbol, x);
                let action = ironsmith::compute_legal_actions(&game, caster)
                    .unwrap()
                    .into_iter()
                    .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
                    .expect("fixture: X spell castable");
                let mut dm = AnnounceX { value: x, calls: 0 };
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                complete_priority_action_with_dm(&mut game, caster, action, &mut dm, &mut queue)
                    .unwrap();
                assert_eq!(dm.calls, 1, "CR 107.3a: the caster announces X once");
                assert_eq!(game.stack.len(), 1);
                assert_eq!(
                    game.stack[0].x_value,
                    Some(x),
                    "CR 107.3a: announced X on the stack"
                );
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let pool = &game.player(caster).unwrap().mana_pool;
                let left =
                    pool.white + pool.blue + pool.black + pool.red + pool.green + pool.colorless;
                observed.push((caster, symbol, x, game.player(caster).unwrap().life, left));
                expected.push((caster, symbol, x, 20 + x as i32, 0));
            }
        }
    }
    assert_eq!(
        observed, expected,
        "CR 107.3a/107.4b: X chosen by the caster is paid with any mana type and used on resolution"
    );
}

// ---------------------------------------------------------------------------
// 110.4a: "permanent card" = artifact, battle, creature, enchantment, land, planeswalker card.
// ---------------------------------------------------------------------------
#[test]
fn cr_110_4a_permanent_card_targets_only_permanent_type_cards() {
    use ironsmith::decisions::context::TargetsContext;
    use ironsmith::game_state::Target;
    use ironsmith::{CardType, LegalAction, ObjectId, Phase, Zone};
    struct Capture {
        legal: Vec<Target>,
    }
    impl DecisionMaker for Capture {
        fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            self.legal = ctx.requirements[0].legal_targets.clone();
            vec![self.legal[0].clone()]
        }
    }
    let alice = p(0);
    let definition = compile(
        "Permanent card return",
        "Mana cost: {0}\nType: Sorcery\nReturn target permanent card from your graveyard to your hand.",
    );
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    let kinds: Vec<(&str, Vec<CardType>, bool)> = vec![
        ("artifact", vec![CardType::Artifact], true),
        ("battle", vec![CardType::Battle], true),
        ("creature", vec![CardType::Creature], true),
        ("enchantment", vec![CardType::Enchantment], true),
        ("land", vec![CardType::Land], true),
        ("planeswalker", vec![CardType::Planeswalker], true),
        (
            "kindred artifact",
            vec![CardType::Kindred, CardType::Artifact],
            true,
        ),
        ("instant", vec![CardType::Instant], false),
        ("sorcery", vec![CardType::Sorcery], false),
        (
            "kindred instant",
            vec![CardType::Kindred, CardType::Instant],
            false,
        ),
    ];
    let ids: Vec<(String, ObjectId, bool)> = kinds
        .into_iter()
        .map(|(name, types, permanent)| {
            (
                name.to_string(),
                plain_card(&mut game, name, types, alice, Zone::Graveyard),
                permanent,
            )
        })
        .collect();
    let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let action = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        .expect("fixture: graveyard has permanent cards to target");
    let mut dm = Capture { legal: vec![] };
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    complete_priority_action_with_dm(&mut game, alice, action, &mut dm, &mut queue).unwrap();
    let observed: Vec<(String, bool)> = ids
        .iter()
        .map(|(name, id, _)| (name.clone(), dm.legal.contains(&Target::Object(*id))))
        .collect();
    let expected: Vec<(String, bool)> = ids
        .iter()
        .map(|(name, _, permanent)| (name.clone(), *permanent))
        .collect();
    assert_eq!(
        observed, expected,
        "CR 110.4a: permanent cards are exactly artifact/battle/creature/enchantment/land/planeswalker cards"
    );
}

// ---------------------------------------------------------------------------
// 110.5c: status is retained while it is irrelevant.
// ---------------------------------------------------------------------------
#[test]
fn cr_110_5c_flipped_and_tapped_status_survive_a_copy_effect_that_makes_them_irrelevant() {
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, RuntimeModification};
    use ironsmith::{CardId, CardType, ChooseSpec, PowerToughness, Until, Zone};
    let alice = p(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let front_id = CardId::new();
    let back_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Status flip front")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .other_face(back_id)
        .other_face_name("Status flip back")
        .linked_face_layout(LinkedFaceLayout::Flip)
        .build();
    let back = CardDefinitionBuilder::new(back_id, "Status flip back")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .other_face(front_id)
        .other_face_name("Status flip front")
        .linked_face_layout(LinkedFaceLayout::Flip)
        .build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let permanent = game.create_object_from_definition(&front, alice, Zone::Battlefield);
    assert!(game.flip_permanent(permanent), "fixture: flip");
    game.tap(permanent);
    let plain = CardDefinitionBuilder::new(CardId::new(), "Plain copy donor")
        .card_types(vec![CardType::Artifact])
        .build();
    let donor = game.create_object_from_definition(&plain, alice, Zone::Battlefield);
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(permanent),
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
    .execute(&mut game, &mut EffectContext::new_default(permanent, alice))
    .unwrap();
    assert_eq!(
        game.current_name(permanent).as_deref(),
        Some("Plain copy donor")
    );
    assert!(
        game.is_flipped(permanent),
        "CR 110.5c: flipped status is retained even though it is irrelevant to a non-flip copy"
    );
    assert!(
        game.is_tapped(permanent),
        "CR 110.5c: tapped status is retained"
    );
    assert!(!game.is_face_down(permanent));
}

// ---------------------------------------------------------------------------
// 111.4: default token name is its subtypes plus "Token".
// ---------------------------------------------------------------------------
#[test]
fn cr_111_4_unnamed_tokens_are_named_after_their_subtypes_plus_token() {
    use ironsmith::{CardType, Zone};
    let alice = p(0);
    let mut observed = Vec::new();
    for (text, expected) in [
        ("Create a 1/1 red Goblin creature token.", "Goblin Token"),
        (
            "Create a 1/1 green Elf Warrior creature token.",
            "Elf Warrior Token",
        ),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = compile(
            "Token maker",
            &format!("Mana cost: {{1}}\nType: Sorcery\n{text}"),
        );
        resolve_as_spell(&mut game, &definition, alice);
        let token = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
            .expect("fixture: token created");
        observed.push(game.current_name(token).unwrap_or_default());
        let _ = expected;
    }
    assert_eq!(
        observed,
        vec!["Goblin Token".to_string(), "Elf Warrior Token".to_string()],
        "CR 111.4: an unnamed token's name is its subtypes plus the word Token"
    );
}
