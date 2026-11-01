//! Regression for the custom Ambush reminder previously accepted as a global discount.
use ironsmith_compiler_runtime::compile_to_runtime_definition;

#[test]
fn ambush_does_not_silently_discard_casting_and_entry_rules() {
    // Actual compileDraftInput expansion of Ambush {3}{R}, with its named cost bound.
    let rules = "During your declare attackers step, you may cast this spell for {3}{R}. It costs {1} less for each attacking creature. This creature enters tapped and attacking.";
    let text =
        format!("Mana cost: {{6}}{{R}}\nType: Creature — Human\nPower/Toughness: 3/3\n{rules}");
    let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
        compile_to_runtime_definition("Ambush Test", &text, false)
    });
    let filter_only =
        ironsmith_compiler::word_claims::with_enforcement_options(true, false, || {
            compile_to_runtime_definition("Ambush Test", &text, false)
        });
    match filter_only {
        Err(error) => println!("Filter-only coverage: {error}"),
        Ok(definition) => println!(
            "Filter-only coverage ACCEPTED ({} alternative casting methods): {}",
            definition.alternative_casts.len(),
            ironsmith_text::compiled_text_lines(&definition).join("\n")
        ),
    }
    match result {
        Err(error) => {
            let diagnostic = error.to_string();
            assert!(
                diagnostic.contains("object filter does not read 'enters'"),
                "Unexpected rejection: {diagnostic}"
            );
            println!("Original Ambush with both coverage checks: {diagnostic}");
        }
        Ok(definition) => panic!(
            "Ambush was accepted with {} alternative casting methods. Verify timing, its own discount, and attacking entry: {}",
            definition.alternative_casts.len(),
            ironsmith_text::compiled_text_lines(&definition).join("\n")
        ),
    }
}

const COMBAT_PRICE: &str = "During your declare attackers step, any time you could cast an instant, you may cast this spell from your hand for {3}{R} rather than pay its mana cost. This cost is reduced by {1} for each attacking creature. If you cast this spell this way, it enters tapped and attacking.";

fn combat_definition() -> ironsmith::cards::CardDefinition {
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
        compile_to_runtime_definition("Combat entrant", &format!("Mana cost: {{6}}{{R}}\nType: Creature — Human\nPower/Toughness: 3/3\n{COMBAT_PRICE}"), false)
    }).unwrap()
}

#[test]
fn explicit_combat_price_compiles_with_complete_coverage() {
    let definition = combat_definition();
    assert_eq!(definition.alternative_casts.len(), 1);
    let instructions = definition.alternative_casts[0].battlefield_instructions();
    assert_eq!(instructions.generic_cost_reductions[0].0, 1);
    assert!(instructions.enters_tapped && instructions.enters_attacking);
    assert_eq!(
        instructions.casting_window,
        Some(ironsmith_core::ThisSpellCastTiming::DuringYourDeclareAttackersStep)
    );
    let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("for each attacking creature"),
        "{rendered}"
    );
}

use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn setup(attackers: usize) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.combat = Some(Default::default());
    let body = compile_to_runtime_definition(
        "Vigilant attacker",
        "Type: Creature\nPower/Toughness: 2/2\nVigilance",
        false,
    )
    .unwrap();
    for _ in 0..attackers {
        let id = game.create_object_from_definition(&body, A, Zone::Battlefield);
        game.combat
            .as_mut()
            .unwrap()
            .attackers
            .push(ironsmith::combat_state::AttackerInfo {
                creature: id,
                target: ironsmith::AttackTarget::Player(B),
            });
        assert!(!game.is_tapped(id));
    }
    let id = game.create_object_from_definition(&combat_definition(), A, Zone::Hand);
    (game, id)
}
fn fund(game: &mut GameState, generic: u32) {
    let pool = &mut game.player_mut(A).unwrap().mana_pool;
    pool.add(ManaSymbol::Red, 1);
    pool.add(ManaSymbol::Colorless, generic);
}
fn action(game: &GameState, id: ObjectId, alternative: bool) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == id &&
        if alternative { matches!(casting_method, CastingMethod::Alternative(_)) } else { matches!(casting_method, CastingMethod::Normal) }))
}
fn cast(game: &mut GameState, id: ObjectId, alternative: bool) {
    let action = action(game, id, alternative).expect("expected legal casting action");
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            assert_eq!(game.stack.len(), 1);
            return;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("cast did not finish");
}
#[test]
fn combat_price_counts_untapped_attackers_and_pays_its_own_discounted_cost() {
    for (count, generic) in [(0, 3), (2, 1), (5, 0)] {
        let (mut game, id) = setup(count);
        fund(&mut game, generic);
        assert!(
            action(&game, id, false).is_none(),
            "normal cast gets no instant permission"
        );
        cast(&mut game, id, true);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let permanent = game
            .objects_in_deterministic_order()
            .into_iter()
            .find(|o| o.zone == Zone::Battlefield && o.name.as_ref() == "Combat entrant")
            .unwrap()
            .id;
        assert!(game.is_tapped(permanent));
        assert!(
            game.combat
                .as_ref()
                .unwrap()
                .attackers
                .iter()
                .any(|a| a.creature == permanent && a.target == ironsmith::AttackTarget::Player(B))
        );
    }
}
#[test]
fn combat_price_is_unavailable_outside_its_window_and_normal_cast_has_no_riders() {
    let (mut game, id) = setup(2);
    fund(&mut game, 6);
    for step in [Step::BeginCombat, Step::DeclareBlockers, Step::CombatDamage] {
        game.turn.step = Some(step);
        assert!(action(&game, id, true).is_none(), "wrong step: {step:?}");
    }
    game.turn.step = Some(Step::DeclareAttackers);
    game.turn.active_player = B;
    assert!(action(&game, id, true).is_none(), "not your turn");
    game.turn.active_player = A;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    assert!(action(&game, id, true).is_none());
    cast(&mut game, id, false);
    assert_eq!(
        game.player(A).unwrap().mana_pool.total(),
        0,
        "normal cast pays printed cost despite attackers"
    );
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    let permanent = game
        .objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.zone == Zone::Battlefield && o.name.as_ref() == "Combat entrant")
        .unwrap()
        .id;
    assert!(!game.is_tapped(permanent));
    assert!(
        !game
            .combat
            .as_ref()
            .unwrap()
            .attackers
            .iter()
            .any(|a| a.creature == permanent)
    );
}

#[test]
fn combat_price_does_not_discount_other_spells_or_colored_mana() {
    let (mut game, id) = setup(5);
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 20);
    assert!(
        action(&game, id, true).is_none(),
        "attackers cannot pay the red pip"
    );
    game.player_mut(A).unwrap().mana_pool = Default::default();
    fund(&mut game, 0);
    cast(&mut game, id, true);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    let other = compile_to_runtime_definition(
        "Other spell",
        "Mana cost: {2}{R}\nType: Instant\nDraw a card.",
        false,
    )
    .unwrap();
    let other_id = game.create_object_from_definition(&other, A, Zone::Hand);
    game.turn.priority_player = Some(A);
    fund(&mut game, 1);
    assert!(
        action(&game, other_id, false).is_none(),
        "Ambush on the battlefield is not a global discount"
    );
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    assert!(action(&game, other_id, false).is_some());
}

#[test]
fn combat_price_artifact_round_trip_keeps_all_instructions() {
    let text =
        format!("Mana cost: {{6}}{{R}}\nType: Creature\nPower/Toughness: 3/3\n{COMBAT_PRICE}");
    let (artifact, direct) =
        ironsmith_compiler_runtime::compile_to_artifact("Combat entrant", &text, false).unwrap();
    let restored =
        ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap())
            .unwrap();
    assert_eq!(artifact, restored);
    let materialized =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    assert_eq!(direct.alternative_casts, materialized.alternative_casts);
}

#[test]
fn price_clauses_compose_without_a_specific_mechanic_paragraph() {
    let head = "You may cast this spell from your hand for {2}{U} rather than pay its mana cost.";
    for (prefix, riders, reductions, tapped, attacking) in [
        ("", "", 0, false, false),
        ("During your turn, ", "", 0, false, false),
        ("Any time you could cast an instant, ", "", 0, false, false),
        (
            "",
            "This cost is reduced by {2} for each artifact you control.",
            1,
            false,
            false,
        ),
        (
            "",
            "If you cast this spell this way, it enters tapped.",
            0,
            true,
            false,
        ),
        (
            "",
            "If you cast this spell this way, it enters attacking.",
            0,
            false,
            true,
        ),
        (
            "During the declare blockers step, any time you could cast an instant, ",
            "If you cast this spell this way, it enters tapped and attacking. This cost is reduced by {1} for each untapped creature you control. This cost is reduced by {2} for each artifact an opponent controls.",
            2,
            true,
            true,
        ),
    ] {
        let rules = format!("{prefix}{head} {riders}");
        let definition =
            ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
                compile_to_runtime_definition(
                    "Independent price clauses",
                    &format!(
                        "Mana cost: {{5}}{{U}}\nType: Creature\nPower/Toughness: 2/2\n{rules}"
                    ),
                    false,
                )
            })
            .unwrap_or_else(|error| panic!("{rules}: {error}"));
        let instructions = definition.alternative_casts[0].battlefield_instructions();
        assert_eq!(
            instructions.generic_cost_reductions.len(),
            reductions,
            "{rules}"
        );
        assert_eq!(instructions.enters_tapped, tapped, "{rules}");
        assert_eq!(instructions.enters_attacking, attacking, "{rules}");
        assert_eq!(
            instructions.instant_timing,
            prefix.contains("instant"),
            "{rules}"
        );
        assert_eq!(
            instructions.casting_window.is_some(),
            prefix.starts_with("During"),
            "{rules}"
        );
    }
}

#[test]
fn unknown_price_riders_and_filter_qualifiers_are_not_silently_consumed() {
    for rider in [
        "This cost is reduced by {1} for each imaginary creature.",
        "If you cast this spell this way, it enters tapped and draws a card.",
        "This cost is reduced by {1} for each attacking creature. You win the potato.",
    ] {
        let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            compile_to_runtime_definition(
                "Bad price",
                &format!(
                    "Type: Creature\nPower/Toughness: 2/2\nYou may cast this spell from your hand for {{R}} rather than pay its mana cost. {rider}"
                ),
                false,
            )
        });
        assert!(result.is_err(), "unrecognized rider was accepted: {rider}");
    }
}

#[test]
fn filtered_price_reduction_counts_only_matching_objects() {
    let rules = "Any time you could cast an instant, you may cast this spell from your hand for {3}{R} rather than pay its mana cost. This cost is reduced by {1} for each artifact you control.";
    let definition = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
        compile_to_runtime_definition(
            "Artifact price",
            &format!("Mana cost: {{6}}{{R}}\nType: Creature\nPower/Toughness: 3/3\n{rules}"),
            false,
        )
    })
    .unwrap();
    let (mut game, _) = setup(3);
    let artifact = compile_to_runtime_definition("Resource", "Type: Artifact", false).unwrap();
    game.create_object_from_definition(&artifact, A, Zone::Battlefield);
    game.create_object_from_definition(&artifact, B, Zone::Battlefield);
    game.create_object_from_definition(&artifact, A, Zone::Hand);
    let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
    fund(&mut game, 1);
    assert!(
        action(&game, spell, true).is_none(),
        "opponent artifacts, hand cards and attackers don't count"
    );
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    cast(&mut game, spell, true);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    let permanent = game
        .objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.zone == Zone::Battlefield && o.name.as_ref() == "Artifact price")
        .unwrap()
        .id;
    assert!(!game.is_tapped(permanent));
    assert!(
        !game
            .combat
            .as_ref()
            .unwrap()
            .attackers
            .iter()
            .any(|a| a.creature == permanent)
    );
}

#[test]
fn price_reductions_combine_with_tax_before_flooring_at_zero() {
    let (mut game, id) = setup(5);
    let tax = compile_to_runtime_definition(
        "Tax",
        "Type: Artifact\nSpells cost {2} more to cast.",
        false,
    )
    .unwrap();
    game.create_object_from_definition(&tax, B, Zone::Battlefield);
    fund(&mut game, 0);
    cast(&mut game, id, true);
    assert_eq!(
        game.player(A).unwrap().mana_pool.total(),
        0,
        "five reductions pay three generic plus two tax, leaving red"
    );
}
