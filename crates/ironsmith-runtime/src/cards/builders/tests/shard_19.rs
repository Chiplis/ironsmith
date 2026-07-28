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
use super::shard_17::*;
use super::shard_18::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jasmine_dragon_tea_shop_token_activation_creates_white_ally() {
    let def = parse_oracle_card_definition("Jasmine Dragon Tea Shop");
    let token_activated = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            activated
                .effects
                .iter()
                .any(|effect| effect.downcast_ref::<CreateTokenEffect>().is_some())
                .then_some(activated)
        })
        .expect("Jasmine Dragon Tea Shop should have a token-creating activated ability");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let tea_shop_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 5);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        tea_shop_id,
        &token_activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("Jasmine Dragon Tea Shop token activation cost should be payable");
    assert!(
        game.is_tapped(tea_shop_id),
        "Jasmine Dragon Tea Shop should tap to pay its token activation cost"
    );

    let mut ctx = crate::effects::ExecutionContext::new(tea_shop_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        tea_shop_id,
        &token_activated.effects,
        None,
        &[],
    )
    .expect("Jasmine Dragon Tea Shop token activation should resolve");

    let ally_tokens: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|object| (id, object)))
        .filter(|(_, object)| {
            matches!(object.kind, crate::object::ObjectKind::Token)
                && object.subtypes.contains(&Subtype::Ally)
                && game.controller_of(*object) == alice
        })
        .collect();
    assert_eq!(ally_tokens.len(), 1, "expected one Ally token");
    let (token_id, token) = ally_tokens[0];
    assert!(
        token.colors().contains(Color::White),
        "Jasmine Dragon Tea Shop should create a white Ally token"
    );
    assert_eq!(game.current_power(token_id), Some(1));
    assert_eq!(game.current_toughness(token_id), Some(1));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn throne_of_eldraine_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Throne of Eldraine");
    let def = parse_oracle_card_definition("Throne of Eldraine");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("spend this mana only to cast monocolored spells of that color"),
        "expected Throne of Eldraine mana spend restriction to render, got {rendered}"
    );
    assert!(
        rendered_lower.contains("spend only mana of the chosen color to activate this ability"),
        "expected Throne of Eldraine activation mana-source restriction to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn throne_of_eldraine_models_mana_and_activation_restrictions() {
    let def = parse_oracle_card_definition("Throne of Eldraine");
    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::ChooseColorAsEnters),
        "Throne of Eldraine should choose a color as it enters, got {ids:?}"
    );

    let mana_activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_mana_ability() => Some(activated),
            _ => None,
        })
        .expect("Throne of Eldraine should have a mana ability");
    let has_monocolored_chosen_restriction =
        mana_activated
            .mana_usage_restrictions
            .iter()
            .any(|restriction| {
                matches!(
                    restriction,
                    crate::ability::ManaUsageRestriction::CastSpellMatching {
                        filter,
                        restrict_to_matching_spell: true,
                        ..
                    } if filter.monocolored && filter.chosen_color
                )
            });
    assert!(
        has_monocolored_chosen_restriction,
        "Throne of Eldraine mana should be restricted to monocolored spells of the chosen color"
    );

    let draw_activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated
                    .effects
                    .iter()
                    .any(|effect| effect.downcast_ref::<DrawCardsEffect>().is_some()) =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Throne of Eldraine should have a draw activated ability");
    assert!(
        draw_activated
            .additional_restrictions
            .iter()
            .any(|restriction| {
                restriction.eq_ignore_ascii_case(
                    "spend only mana of the chosen color to activate this ability",
                )
            }),
        "draw ability should preserve chosen-color activation mana restriction"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn throne_of_eldraine_runtime_adds_chosen_color_mana_and_draws_two_cards() {
    let def = parse_oracle_card_definition("Throne of Eldraine");
    let mana_activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_mana_ability() => {
                Some(activated.clone())
            }
            _ => None,
        })
        .expect("Throne of Eldraine should have a mana ability");
    let draw_activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated
                    .effects
                    .iter()
                    .any(|effect| effect.downcast_ref::<DrawCardsEffect>().is_some()) =>
            {
                Some(activated.clone())
            }
            _ => None,
        })
        .expect("Throne of Eldraine should have a draw activated ability");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let throne_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    game.set_chosen_color(throne_id, Color::White);

    let mut mana_ctx = crate::effects::ExecutionContext::new(throne_id, alice, &mut dm)
        .with_mana_usage_restrictions(mana_activated.mana_usage_restrictions.clone());
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut mana_ctx,
        alice,
        throne_id,
        &mana_activated.effects,
        None,
        &[],
    )
    .expect("Throne of Eldraine mana ability should resolve");
    let player = game.player(alice).expect("alice exists");
    assert_eq!(player.mana_pool.white, 4);
    assert_eq!(player.restricted_mana.len(), 4);

    game.empty_mana_pools();
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 2);
    let insufficient = crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        throne_id,
        &draw_activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    );
    assert!(
        insufficient.is_err(),
        "draw ability should not be payable with only two mana"
    );

    game.empty_mana_pools();
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 3);
    let wrong_color = crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        throne_id,
        &draw_activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    );
    assert!(
        wrong_color.is_err(),
        "draw ability should require mana of Throne's chosen color"
    );

    game.empty_mana_pools();
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 3);
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        throne_id,
        &draw_activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("draw ability should be payable with three mana and an untapped Throne");

    let filler = CardDefinitionBuilder::new(CardId::new(), "Library Filler")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_definition(&filler, alice, Zone::Library);
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let mut draw_ctx = crate::effects::ExecutionContext::new(throne_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut draw_ctx,
        alice,
        throne_id,
        &draw_activated.effects,
        None,
        &[],
    )
    .expect("Throne of Eldraine draw ability should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before + 2,
        "draw ability should draw exactly two cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jetfire_ingenious_scientist_card_parses_strictly() {
    let def = parse_oracle_card_definition("Jetfire, Ingenious Scientist // Jetfire, Air Guardian");
    assert!(
        !def.abilities.is_empty(),
        "Jetfire should compile to a card definition with abilities"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jetfire_ingenious_scientist_compiled_text_keeps_nonartifact_mana_spend_restriction() {
    let def = parse_oracle_card_definition("Jetfire, Ingenious Scientist // Jetfire, Air Guardian");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("add an amount of {c} equal to x"),
        "expected Jetfire compiled text to preserve scaled mana output from removed counters, got {rendered}"
    );
    assert!(
        rendered.contains("this mana can't be spent to cast nonartifact spells"),
        "expected Jetfire compiled text to preserve nonartifact spend restriction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn is_negative_nonartifact_cast_payment_restriction(
    restriction: &crate::ability::ManaUsageRestriction,
) -> bool {
    let crate::ability::ManaUsageRestriction::PaymentTransaction {
        restriction: Some(crate::ability::ManaPaymentPredicate::Not(forbidden)),
        on_spend,
    } = restriction
    else {
        return false;
    };
    let crate::ability::ManaPaymentPredicate::All(parts) = forbidden.as_ref() else {
        return false;
    };
    on_spend.is_empty()
        && parts.iter().any(|part| {
            matches!(
                part,
                crate::ability::ManaPaymentPredicate::Purpose(
                    crate::ability::ManaPaymentPurpose::CastSpell
                )
            )
        })
        && parts.iter().any(|part| {
            matches!(
                part,
                crate::ability::ManaPaymentPredicate::SourceMatches(filter)
                    if filter.excluded_card_types == vec![CardType::Artifact]
            )
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jetfire_ingenious_scientist_mana_ability_restricts_nonartifact_spells() {
    let def = parse_oracle_card_definition("Jetfire, Ingenious Scientist // Jetfire, Air Guardian");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated.mana_output.is_some()
                    && !activated.mana_usage_restrictions.is_empty() =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Jetfire should have a restricted mana ability");

    assert!(
        activated
            .mana_usage_restrictions
            .iter()
            .any(is_negative_nonartifact_cast_payment_restriction),
        "expected Jetfire mana ability to forbid only nonartifact spell casting"
    );

    assert_eq!(
        activated.mana_usage_restrictions.len(),
        1,
        "Jetfire mana ability should carry a single cast restriction"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hydraulic_helper_keeps_its_artifact_only_mana_restriction() {
    let def = parse_oracle_card_definition("Hydraulic Helper");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.mana_output.is_some() => Some(activated),
            _ => None,
        })
        .expect("Hydraulic Helper should have a mana ability");

    assert!(
        activated
            .mana_usage_restrictions
            .iter()
            .any(is_negative_nonartifact_cast_payment_restriction),
        "Hydraulic Helper's mana must forbid nonartifact spells without forbidding other payments: {activated:#?}"
    );

    let rendered = canonical_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("{T}: Add {U}. This mana can't be spent to cast a nonartifact spell."),
        "the compiled surface must retain the authored negative spend restriction: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oran_rief_the_vastwood_compiled_text_keeps_entered_this_turn_green_filter() {
    let def = parse_oracle_card_definition("Oran-Rief, the Vastwood");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("each green creature that entered this turn")
            || rendered.contains("each green creature that entered the battlefield this turn"),
        "expected Oran-Rief compiled text to preserve entered-this-turn green creature filter, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oran_rief_the_vastwood_activation_runtime_only_counters_green_creatures_that_entered_this_turn()
 {
    let def = parse_oracle_card_definition("Oran-Rief, the Vastwood");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if !activated.effects.segments.is_empty() => {
                Some(activated)
            }
            _ => None,
        })
        .expect("Oran-Rief should have a countering activated ability");
    let [segment] = activated.effects.segments.as_slice() else {
        panic!("expected one Oran-Rief resolution segment");
    };
    let [effect] = segment.default_effects.as_slice() else {
        panic!("expected one Oran-Rief default effect");
    };
    let for_each = effect
        .downcast_ref::<crate::effects::ForEachObject>()
        .expect("Oran-Rief should iterate matching creatures");
    let put_counters = for_each.effects[0]
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .expect("Oran-Rief should resolve a put-counters effect");
    assert_eq!(
        put_counters.counter_type,
        crate::object::CounterType::PlusOnePlusOne,
        "expected Oran-Rief to place +1/+1 counters"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let oran_rief_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let green_def = CardDefinitionBuilder::new(CardId::new(), "Green Test Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let white_def = CardDefinitionBuilder::new(CardId::new(), "White Test Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let old_green = game.create_object_from_definition(&green_def, alice, Zone::Battlefield);
    game.turn_store.turn_history.clear_for_new_turn();
    let green_entered = game.create_object_from_definition(&green_def, alice, Zone::Battlefield);
    let opponent_green_entered =
        game.create_object_from_definition(&green_def, bob, Zone::Battlefield);
    let white_entered = game.create_object_from_definition(&white_def, alice, Zone::Battlefield);

    for obj_id in [green_entered, opponent_green_entered, white_entered] {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(obj_id)
                .expect("entered test creature should exist on the battlefield"),
            &game,
        );
        let entry_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                obj_id,
                Zone::Hand,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&entry_event);
    }

    let mut ctx = crate::effects::ExecutionContext::new_default(oran_rief_id, alice);
    effect
        .0
        .execute(&mut game, &mut ctx)
        .expect("Oran-Rief counter effect should resolve");

    assert_eq!(
        game.counter_count(green_entered, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Oran-Rief should counter your green creature that entered this turn"
    );
    assert_eq!(
        game.counter_count(
            opponent_green_entered,
            crate::object::CounterType::PlusOnePlusOne
        ),
        1,
        "Oran-Rief should counter each green creature, including opponents'"
    );
    assert_eq!(
        game.counter_count(white_entered, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Oran-Rief should not counter nongreen creatures"
    );
    assert_eq!(
        game.counter_count(old_green, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Oran-Rief should not counter green creatures that did not enter this turn"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    effect
        .0
        .execute(&mut game, &mut ctx)
        .expect("Oran-Rief counter effect should resolve on later turns");
    assert_eq!(
        game.counter_count(green_entered, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Oran-Rief should stop counting prior-turn entries after the turn changes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sundial_of_the_infinite_strict_regression() {
    assert_oracle_card_parses_strict("Sundial of the Infinite");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sundial_of_the_infinite_compiled_text_keeps_end_the_turn_clause() {
    let def = parse_oracle_card_definition("Sundial of the Infinite");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("end the turn") && rendered.contains("activate only during your turn"),
        "expected Sundial rendered text to preserve end-turn and timing clauses, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sundial_of_the_infinite_end_turn_effect_runtime_branches_by_active_player() {
    let def = parse_oracle_card_definition("Sundial of the Infinite");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if !activated.effects.segments.is_empty() => {
                Some(activated)
            }
            _ => None,
        })
        .expect("Sundial should have an activated ability");
    let effect = activated.effects.flattened_default_effects()[0]
        .downcast_ref::<crate::effects::EndTurnEffect>()
        .expect("Sundial activation should lower to EndTurnEffect")
        .clone();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = game
        .players
        .iter()
        .find(|p| p.name == "Alice")
        .expect("Alice should exist")
        .id;
    let bob = game
        .players
        .iter()
        .find(|p| p.name == "Bob")
        .expect("Bob should exist")
        .id;
    let sundial_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    let mut ctx = crate::effects::ExecutionContext::new_default(sundial_id, alice);
    effect
        .execute(&mut game, &mut ctx)
        .expect("Sundial effect should resolve on your turn");
    assert!(game.turn_store.end_turn_procedure_pending);
    let mut runner = crate::turn_runner::TurnRunner::from_state_for_sync(
        crate::turn_runner::TurnState::FirstMainPriority,
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue),
        Ok(crate::turn_runner::TurnAction::Continue)
    ));
    assert_eq!(game.turn.phase, crate::game_state::Phase::Ending);
    assert_eq!(game.turn.step, Some(crate::game_state::Step::Cleanup));

    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::BeginCombat);
    let mut ctx = crate::effects::ExecutionContext::new_default(sundial_id, alice);
    effect
        .execute(&mut game, &mut ctx)
        .expect("Sundial effect should no-op for non-active player");
    assert_eq!(game.turn.phase, crate::game_state::Phase::Combat);
    assert_eq!(game.turn.step, Some(crate::game_state::Step::BeginCombat));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cabaretti_ascendancy_strict_regression() {
    assert_oracle_card_parses_strict("Cabaretti Ascendancy");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cabaretti_ascendancy_compiled_text_keeps_hand_or_bottom_branch() {
    let def = parse_oracle_card_definition("Cabaretti Ascendancy");
    let rendered = canonical_compiled_lines(&def);

    assert_eq!(
        rendered,
        vec![
            concat!(
                "At the beginning of your upkeep, look at the top card of your library. ",
                "If it's a creature or a planeswalker card, you may reveal it and put it ",
                "into your hand. If you don't put the card into your hand, you may put ",
                "it on the bottom of your library."
            )
            .to_string()
        ],
        "expected Cabaretti Ascendancy compiled text to preserve both matching card types, the reveal-to-hand branch, and the conditional bottom branch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cabaretti_ascendancy_trigger_keeps_conditional_bottom_branch_runtime_shape() {
    let def = parse_oracle_card_definition("Cabaretti Ascendancy");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cabaretti Ascendancy should compile to a triggered upkeep ability");

    let debug = format!("{triggered:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("beginningofupkeep"),
        "expected upkeep trigger, got {debug}"
    );
    assert!(
        debug.contains("withideffect")
            && debug.contains("ifeffect")
            && debug.contains("predicate: didnothappen"),
        "expected effect-result condition for the declined hand branch, got {debug}"
    );
    assert!(
        debug.contains("zone: library")
            && debug.contains("to_top: false")
            && debug.contains("mayeffect"),
        "expected optional move-to-bottom effect gated by the condition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Default)]
pub(super) struct CabarettiSequenceDecisionMaker {
    pub(super) decisions: Vec<bool>,
    pub(super) index: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for CabarettiSequenceDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let choice = self.decisions.get(self.index).copied().unwrap_or(false);
        self.index += 1;
        choice
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn execute_cabaretti_ascendancy_top_card(
    top_name: &str,
    top_card_types: Vec<CardType>,
    decisions: Vec<bool>,
) -> (
    crate::game_state::GameState,
    PlayerId,
    ObjectId,
    ObjectId,
    usize,
) {
    let def = parse_oracle_card_definition("Cabaretti Ascendancy");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cabaretti Ascendancy should compile to a triggered upkeep ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let bottom_id = game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(93_001), "Bottom Filler")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    let top_id = game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(93_002), top_name)
            .card_types(top_card_types)
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = CabarettiSequenceDecisionMaker {
        decisions,
        index: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source_id, alice, &mut dm);
    for effect in &triggered.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Cabaretti Ascendancy trigger should resolve");
    }
    drop(ctx);
    let decision_count = dm.index;

    (game, alice, top_id, bottom_id, decision_count)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn cabaretti_player_zone_names(
    game: &crate::game_state::GameState,
    player: PlayerId,
    zone: Zone,
) -> Vec<String> {
    let player = game.player(player).expect("player exists");
    let ids = match zone {
        Zone::Hand => &player.hand,
        Zone::Library => &player.library,
        _ => panic!("unsupported Cabaretti test zone {zone:?}"),
    };
    ids.iter()
        .map(|id| {
            game.object(*id)
                .expect("zone object exists")
                .name
                .to_string()
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cabaretti_ascendancy_planeswalker_top_card_can_go_to_hand() {
    let (game, alice, _top_id, _bottom_id, decision_count) = execute_cabaretti_ascendancy_top_card(
        "Top Planeswalker",
        vec![CardType::Planeswalker],
        vec![true],
    );

    assert_eq!(
        decision_count, 1,
        "matching top card should ask the hand decision once"
    );
    assert_eq!(
        cabaretti_player_zone_names(&game, alice, Zone::Hand),
        vec!["Top Planeswalker".to_string()],
        "accepted planeswalker branch should put exactly the top card into hand"
    );
    assert_eq!(
        cabaretti_player_zone_names(&game, alice, Zone::Library),
        vec!["Bottom Filler".to_string()],
        "only the filler card should remain in library after the top card moves to hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cabaretti_ascendancy_creature_top_card_can_decline_hand_and_move_bottom() {
    let (game, alice, _top_id, _bottom_id, decision_count) = execute_cabaretti_ascendancy_top_card(
        "Top Creature",
        vec![CardType::Creature],
        vec![false, true],
    );

    assert_eq!(
        decision_count, 2,
        "matching top card should ask hand decision and then bottom decision when hand is declined"
    );
    assert_eq!(game.player(alice).expect("alice exists").hand.len(), 0);
    assert_eq!(
        cabaretti_player_zone_names(&game, alice, Zone::Library),
        vec!["Top Creature".to_string(), "Bottom Filler".to_string()],
        "declined creature card should move from top to bottom when the bottom branch is accepted"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cabaretti_ascendancy_matching_top_card_can_decline_both_optional_actions() {
    let (game, alice, _top_id, _bottom_id, decision_count) = execute_cabaretti_ascendancy_top_card(
        "Top Creature",
        vec![CardType::Creature],
        vec![false, false],
    );

    assert_eq!(
        decision_count, 2,
        "both optional decisions should be offered"
    );
    assert_eq!(game.player(alice).expect("alice exists").hand.len(), 0);
    assert_eq!(
        cabaretti_player_zone_names(&game, alice, Zone::Library),
        vec!["Bottom Filler".to_string(), "Top Creature".to_string()],
        "declining the bottom branch should leave the matching card on top"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cabaretti_ascendancy_nonmatching_top_card_skips_hand_branch_and_can_move_bottom() {
    let (game, alice, _top_id, _bottom_id, decision_count) =
        execute_cabaretti_ascendancy_top_card("Top Artifact", vec![CardType::Artifact], vec![true]);

    assert_eq!(
        decision_count, 1,
        "nonmatching top card should skip the hand decision and only ask the bottom decision"
    );
    assert_eq!(game.player(alice).expect("alice exists").hand.len(), 0);
    assert_eq!(
        cabaretti_player_zone_names(&game, alice, Zone::Library),
        vec!["Top Artifact".to_string(), "Bottom Filler".to_string()],
        "nonmatching card should move to bottom when the fallback branch is accepted"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_bucolic_ranch_strict_regression() {
    assert_oracle_card_parses_strict("Bucolic Ranch");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bucolic_ranch_compiled_text_keeps_it_hand_predicate_and_bottom_branch() {
    let def = parse_oracle_card_definition("Bucolic Ranch");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("if you don't put it into your hand")
            || rendered.contains("if you dont put it into your hand")
            || rendered.contains("if not"),
        "expected negative hand-placement predicate in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("you may put it on the bottom of your library")
            || rendered.contains("you may put it on the bottom of its owner's library"),
        "expected optional bottom-of-library branch in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bucolic_ranch_activated_ability_nonmount_top_card_skips_reveal_branch() {
    #[derive(Default)]
    struct SequenceBooleanDecisionMaker {
        decisions: Vec<bool>,
        index: usize,
    }

    impl crate::decision::DecisionMaker for SequenceBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let choice = self.decisions.get(self.index).copied().unwrap_or(false);
            self.index += 1;
            choice
        }
    }

    let def = parse_oracle_card_definition("Bucolic Ranch");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                let debug = format!("{activated:#?}").to_ascii_lowercase();
                if debug.contains("look") && debug.contains("top") {
                    Some(activated)
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("Bucolic Ranch should have the look-at-top activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let ranch_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(92_002), "Top Nonmount")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = SequenceBooleanDecisionMaker {
        decisions: vec![true],
        index: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(ranch_id, alice, &mut dm);
    for effect in &activated.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Bucolic Ranch ability should resolve");
    }

    assert_eq!(
        dm.index, 1,
        "when the top card is not a Mount, Bucolic Ranch should only ask the fallback bottom-of-library decision"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bucolic_ranch_activated_ability_can_decline_hand_and_bottom_the_card() {
    #[derive(Default)]
    struct SequenceBooleanDecisionMaker {
        decisions: Vec<bool>,
        index: usize,
    }

    impl crate::decision::DecisionMaker for SequenceBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let choice = self.decisions.get(self.index).copied().unwrap_or(false);
            self.index += 1;
            choice
        }
    }

    let def = parse_oracle_card_definition("Bucolic Ranch");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                let debug = format!("{activated:#?}").to_ascii_lowercase();
                if debug.contains("look") && debug.contains("top") {
                    Some(activated)
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("Bucolic Ranch should have the look-at-top activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let ranch_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(92_003), "Bottom Card")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(92_004), "Top Mount")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Mount])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = SequenceBooleanDecisionMaker {
        decisions: vec![false, true],
        index: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(ranch_id, alice, &mut dm);
    for effect in &activated.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Bucolic Ranch ability should resolve");
    }

    assert_eq!(
        dm.index, 2,
        "Bucolic Ranch should ask both optional branch decisions when the top card is a Mount"
    );
    assert!(
        game.player(alice).expect("alice exists").library.len() >= 1,
        "Bucolic Ranch should keep cards in library after resolving fallback branch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scholar_of_new_horizons_strict_regression_parses() {
    assert_oracle_card_parses_strict("Scholar of New Horizons");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scholar_of_new_horizons_compiled_text_keeps_optional_battlefield_and_fallback_hand() {
    let def = parse_oracle_card_definition("Scholar of New Horizons");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();

    assert!(
        lower.contains("search your library for a plains card and reveal it"),
        "expected Plains search and reveal text, got {rendered}"
    );
    assert!(
        lower.contains("if an opponent controls more lands than you, you may put that card onto the battlefield tapped"),
        "expected optional battlefield branch keyed by more lands, got {rendered}"
    );
    assert!(
        lower.contains("if you don't put the card onto the battlefield, put it into your hand")
            || lower
                .contains("if you dont put the card onto the battlefield, put it into your hand"),
        "expected fallback hand branch keyed by not putting onto battlefield, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scholar_of_new_horizons_enters_with_plus_one_counter() {
    let def = parse_oracle_card_definition("Scholar of New Horizons");
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let scholar_in_hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let scholar = game
        .move_object_with_etb_processing(scholar_in_hand, Zone::Battlefield)
        .expect("Scholar of New Horizons should enter")
        .new_id;

    assert_eq!(game.counter_count(scholar, CounterType::PlusOnePlusOne), 1);
    assert!(
        game.object(scholar).is_some(),
        "Scholar should remain on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Default)]
pub(super) struct ScholarOfNewHorizonsDecisionMaker {
    pub(super) object_choice: Option<ObjectId>,
    pub(super) boolean_decisions: Vec<bool>,
    pub(super) boolean_index: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for ScholarOfNewHorizonsDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if let Some(choice) = self.object_choice
            && ctx
                .candidates
                .iter()
                .any(|candidate| candidate.legal && candidate.id == choice)
        {
            return vec![choice];
        }
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min)
            .collect()
    }

    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let choice = self
            .boolean_decisions
            .get(self.boolean_index)
            .copied()
            .unwrap_or(false);
        self.boolean_index += 1;
        choice
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn scholar_of_new_horizons_activated_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Scholar of New Horizons should have an activated ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn scholar_test_land(name: &str) -> crate::card::Card {
    crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn scholar_named_objects_in_zone(
    game: &crate::game_state::GameState,
    player: PlayerId,
    zone: Zone,
    name: &str,
) -> Vec<ObjectId> {
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|object| object.name == name && game.controller_of(object) == player)
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn execute_scholar_of_new_horizons_activation(
    opponent_has_more_lands: bool,
    put_onto_battlefield: bool,
) -> (
    crate::game_state::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
    usize,
) {
    let def = parse_oracle_card_definition("Scholar of New Horizons");
    let activated = scholar_of_new_horizons_activated_ability(&def);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let scholar_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let plains_id =
        game.create_object_from_card(&scholar_test_land("Searched Plains"), alice, Zone::Library);

    let (alice_lands, bob_lands) = if opponent_has_more_lands {
        (1, 2)
    } else {
        (2, 1)
    };
    for idx in 0..alice_lands {
        game.create_object_from_card(
            &scholar_test_land(&format!("Alice Land {idx}")),
            alice,
            Zone::Battlefield,
        );
    }
    for idx in 0..bob_lands {
        game.create_object_from_card(
            &scholar_test_land(&format!("Bob Land {idx}")),
            bob,
            Zone::Battlefield,
        );
    }

    let mut dm = ScholarOfNewHorizonsDecisionMaker {
        object_choice: Some(plains_id),
        boolean_decisions: vec![put_onto_battlefield],
        boolean_index: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(scholar_id, alice, &mut dm);
    for effect in &activated.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Scholar of New Horizons activation should resolve");
    }
    drop(ctx);
    let boolean_count = dm.boolean_index;

    (game, alice, bob, scholar_id, plains_id, boolean_count)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scholar_of_new_horizons_activation_cost_keeps_tap_and_remove_counter_from_permanent()
{
    let def = parse_oracle_card_definition("Scholar of New Horizons");
    let activated = scholar_of_new_horizons_activated_ability(&def);
    let cost_debug = format!("{:#?}", activated.mana_cost).to_ascii_lowercase();

    assert!(
        cost_debug.contains("tapeffect"),
        "expected tap cost, got {cost_debug}"
    );
    assert!(
        cost_debug.contains("removeanycountersamongeffect")
            && cost_debug.contains("min_count: 1")
            && cost_debug.contains("controller: some(")
            && cost_debug.contains("you"),
        "expected remove-a-counter-from-a-permanent-you-control cost, got {cost_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scholar_of_new_horizons_more_lands_accepts_battlefield_branch_tapped() {
    let (game, alice, _bob, _scholar_id, _plains_id, boolean_count) =
        execute_scholar_of_new_horizons_activation(true, true);

    assert_eq!(
        boolean_count, 1,
        "more-lands branch should offer the optional battlefield choice"
    );
    let battlefield_plains =
        scholar_named_objects_in_zone(&game, alice, Zone::Battlefield, "Searched Plains");
    assert_eq!(
        battlefield_plains.len(),
        1,
        "searched Plains should be on the battlefield"
    );
    assert!(
        game.is_tapped(battlefield_plains[0]),
        "accepted Plains should enter tapped"
    );
    assert!(
        scholar_named_objects_in_zone(&game, alice, Zone::Hand, "Searched Plains").is_empty(),
        "accepted Plains should not also be in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scholar_of_new_horizons_more_lands_declines_battlefield_and_puts_card_in_hand() {
    let (game, alice, _bob, _scholar_id, _plains_id, boolean_count) =
        execute_scholar_of_new_horizons_activation(true, false);

    assert_eq!(
        boolean_count, 1,
        "more-lands branch should offer the optional battlefield choice"
    );
    assert_eq!(
        scholar_named_objects_in_zone(&game, alice, Zone::Hand, "Searched Plains").len(),
        1,
        "declined Plains should be put into hand"
    );
    assert!(
        scholar_named_objects_in_zone(&game, alice, Zone::Battlefield, "Searched Plains")
            .is_empty(),
        "declined Plains should not be on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scholar_of_new_horizons_without_more_lands_skips_battlefield_choice_and_puts_card_in_hand()
 {
    let (game, alice, _bob, _scholar_id, _plains_id, boolean_count) =
        execute_scholar_of_new_horizons_activation(false, true);

    assert_eq!(
        boolean_count, 0,
        "without more lands, the optional battlefield choice should not be offered"
    );
    assert_eq!(
        scholar_named_objects_in_zone(&game, alice, Zone::Hand, "Searched Plains").len(),
        1,
        "without more lands, Plains should be put into hand"
    );
    assert!(
        scholar_named_objects_in_zone(&game, alice, Zone::Battlefield, "Searched Plains")
            .is_empty(),
        "without more lands, Plains should not be on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn woodlurker_mimic_strict_regression_parses() {
    assert_oracle_card_parses_strict("Woodlurker Mimic");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn woodlurker_mimic_compiled_text_keeps_base_pt_and_wither_clause() {
    let def = parse_oracle_card_definition("Woodlurker Mimic");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("base power and toughness 4/5"),
        "expected base P/T clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("wither") && rendered.contains("until end of turn"),
        "expected temporary wither grant in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn woodlurker_mimic_trigger_runtime_shape_keeps_color_filter_and_wither_effect() {
    let def = parse_oracle_card_definition("Woodlurker Mimic");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Woodlurker Mimic should compile to a triggered cast ability");

    let debug = format!("{triggered:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("spellcasttrigger") && debug.contains("colorset") && debug.contains("spell"),
        "expected a color-constrained spell-cast trigger shape, got {debug}"
    );
    assert!(
        debug.contains("wither")
            && debug.contains("setpowertoughness")
            && debug.contains("fixed(")
            && debug.contains("4")
            && debug.contains("5"),
        "expected trigger effects to keep the base 4/5 + wither linkage, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn base_pt_animation_representatives_lower_to_continuous_setters() {
    for (name, expected_type_modification) in [
        ("Ascendant Spirit", "SetCardTypes"),
        ("Evolved Sleeper", "SetCardTypes"),
        ("Skilled Animator", "AddCardTypes"),
        ("Living Brain, Mechanical Marvel", "AddCardTypes"),
        ("Unctus's Retrofitter", "AddCardTypes"),
    ] {
        let def = parse_oracle_card_definition(name);
        let debug = format!("{def:#?}");
        assert!(
            debug.contains("ApplyContinuousEffect")
                && debug.contains(expected_type_modification)
                && debug.contains("Creature")
                && debug.contains("SetPowerToughness")
                && debug.contains("sublayer: Setting"),
            "expected {name} to lower through {expected_type_modification} plus continuous SetPowerToughness, got {debug}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn base_pt_animation_representatives_render_base_power_toughness_surface() {
    for (name, expected) in [
        (
            "Ascendant Spirit",
            "this creature becomes a spirit warrior with base power and toughness 2/3",
        ),
        (
            "Evolved Sleeper",
            "this creature becomes a human cleric with base power and toughness 2/2",
        ),
        (
            "Skilled Animator",
            "target artifact you control becomes an artifact creature with base power and toughness 5/5 for as long as this creature remains on the battlefield",
        ),
        (
            "Living Brain, Mechanical Marvel",
            "target non-equipment artifact you control becomes an artifact creature with base power and toughness 3/3 until end of turn",
        ),
        (
            "Unctus's Retrofitter",
            "up to one target artifact you control becomes an artifact creature with base power and toughness 4/4 for as long as this creature remains on the battlefield",
        ),
        (
            "Mimic",
            "this artifact becomes a shapeshifter artifact creature with base power and toughness 3/3 until end of turn",
        ),
    ] {
        let def = parse_oracle_card_definition(name);
        let rendered = canonical_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains(expected),
            "expected {name} compiled text to preserve oracle-style base P/T animation, got {rendered}"
        );
    }
}

pub(super) const STRICT_PARSE_REGRESSION_SUCCESS_CARDS: &[&str] = &[
    "Banefire",
    "Barrowin of Clan Undurr",
    "Biophagus",
    "Blast Zone",
    "Cabaretti Ascendancy",
    "Boseiju, Who Endures",
    "Cabal Ritual",
    "Caves of Chaos Adventurer",
    "Cavern of Souls",
    "Clown Car",
    "Cultivator Colossus",
    "Dungeon Crawler",
    "Echoing Deeps",
    "Encroaching Mycosynth",
    "Escaped Null",
    "Fatal Push",
    "Golgari Thug",
    "Gloom Stalker",
    "Grief",
    "Imoen, Mystic Trickster",
    "Maskwood Nexus",
    "Mox Amber",
    "Nesting Grounds",
    "Nexus of Fate",
    "Nykthos, Shrine to Nyx",
    "Nine-Lives Familiar",
    "Otawara, Soaring City",
    "Orcish Bowmasters",
    "Pawn of Ulamog",
    "Genesis Chamber",
    "Gwen Stacy // Ghost-Spider",
    "Sacrifice",
    "Sefris of the Hidden Ways",
    "Sephiroth, Fabled SOLDIER",
    "Susurian Voidborn",
    "Shifting Woodland",
    "Spelunking",
    "Talon Gates of Madara",
    "The Mycosynth Gardens",
    "The Soul Stone",
    "Tolaria West",
    "Turn the Earth",
    "Unmarked Grave",
    "Vesuva",
    "White Plume Adventurer",
];

pub(super) const STRICT_PARSE_REGRESSION_EXPECTED_FAILURE_CARDS: &[&str] =
    &["Hancock, Ghoulish Mayor", "Lake of the Dead"];

macro_rules! strict_parse_card_test {
    ($test_name:ident, $card_name:expr) => {
        #[cfg(ironsmith_runtime_parser_tests)]
        #[test]
        fn $test_name() {
            assert_oracle_card_parses_strict($card_name);
        }
    };
}

macro_rules! strict_parse_card_expected_fail_test {
    ($test_name:ident, $card_name:expr) => {
        #[test]
        fn $test_name() {
            assert_oracle_card_fails_strict($card_name);
        }
    };
}

strict_parse_card_test!(strict_parse_banefire, "Banefire");
strict_parse_card_test!(strict_parse_blast_zone, "Blast Zone");
strict_parse_card_test!(strict_parse_bridge_from_below, "Bridge from Below");
strict_parse_card_test!(strict_parse_cabal_ritual, "Cabal Ritual");
strict_parse_card_test!(strict_parse_cavern_of_souls, "Cavern of Souls");
strict_parse_card_test!(strict_parse_clown_car, "Clown Car");
strict_parse_card_test!(strict_parse_encroaching_mycosynth, "Encroaching Mycosynth");
strict_parse_card_test!(strict_parse_escaped_null, "Escaped Null");
strict_parse_card_test!(strict_parse_exuberant_fuseling, "Exuberant Fuseling");
strict_parse_card_test!(strict_parse_fatal_push, "Fatal Push");
strict_parse_card_test!(strict_parse_feudkillers_verdict, "Feudkiller's Verdict");
strict_parse_card_test!(strict_parse_gemstone_caverns, "Gemstone Caverns");
strict_parse_card_test!(strict_parse_golgari_thug, "Golgari Thug");
strict_parse_card_test!(strict_parse_gravecrawler, "Gravecrawler");
strict_parse_card_test!(strict_parse_grief, "Grief");
strict_parse_card_test!(
    strict_parse_gwen_stacy_ghost_spider,
    "Gwen Stacy // Ghost-Spider"
);
strict_parse_card_expected_fail_test!(
    strict_parse_hancock_ghoulish_mayor,
    "Hancock, Ghoulish Mayor"
);
strict_parse_card_expected_fail_test!(strict_parse_lake_of_the_dead, "Lake of the Dead");
strict_parse_card_test!(strict_parse_maskwood_nexus, "Maskwood Nexus");
strict_parse_card_test!(
    strict_parse_mighty_servant_of_leuk_o,
    "Mighty Servant of Leuk-o"
);
strict_parse_card_test!(strict_parse_mistmeadow_skulk, "Mistmeadow Skulk");
strict_parse_card_test!(strict_parse_mox_amber, "Mox Amber");
strict_parse_card_test!(strict_parse_nesting_grounds, "Nesting Grounds");
strict_parse_card_test!(strict_parse_nine_lives_familiar, "Nine-Lives Familiar");
strict_parse_card_test!(strict_parse_nykthos_shrine_to_nyx, "Nykthos, Shrine to Nyx");
strict_parse_card_test!(strict_parse_orcish_bowmasters, "Orcish Bowmasters");
strict_parse_card_test!(strict_parse_pawn_of_ulamog, "Pawn of Ulamog");
strict_parse_card_test!(strict_parse_profane_memento, "Profane Memento");
strict_parse_card_test!(strict_parse_genesis_chamber, "Genesis Chamber");
strict_parse_card_test!(strict_parse_inviolability, "Inviolability");
strict_parse_card_test!(strict_parse_saving_grace, "Saving Grace");
strict_parse_card_test!(strict_parse_sacrifice, "Sacrifice");
strict_parse_card_test!(strict_parse_skeleton_crew, "Skeleton Crew");
strict_parse_card_test!(
    strict_parse_sephiroth_fabled_soldier,
    "Sephiroth, Fabled SOLDIER"
);
strict_parse_card_test!(strict_parse_susurian_voidborn, "Susurian Voidborn");
strict_parse_card_test!(strict_parse_talon_gates_of_madara, "Talon Gates of Madara");
strict_parse_card_test!(strict_parse_the_soul_stone, "The Soul Stone");
strict_parse_card_test!(strict_parse_unmarked_grave, "Unmarked Grave");

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn strict_parse_animate_land() {
    assert_oracle_card_parses_strict("Animate Land");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn animate_land_compiled_text_keeps_animation_clause() {
    let def = parse_oracle_card_definition("Animate Land");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target land becomes a 3/3 creature")
            && rendered.contains("until end of turn")
            && rendered.contains("still a land"),
        "expected Animate Land compiled text to preserve animation, duration, and still-a-land clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn inviolability_compiled_text_keeps_damage_prevention_clause() {
    let def = parse_oracle_card_definition("Inviolability");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("prevent all damage that would be dealt to enchanted creature"),
        "expected Inviolability compiled text to preserve enchanted-creature prevention semantics, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn skeleton_crew_compiled_text_keeps_graveyard_leave_trigger() {
    let def = parse_oracle_card_definition("Skeleton Crew");
    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Each other Skeleton or Pirate creature you control gets +1/+1")
            && rendered.contains("Whenever one or more creature cards leave your graveyard, create a 2/2 black Skeleton Pirate creature token")
            && rendered.contains("{5}{B}: Return this card from your graveyard to the battlefield tapped"),
        "expected Skeleton Crew compiled text to preserve anthem, graveyard-leave trigger, and graveyard activation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mighty_servant_of_leuk_o_compiled_text_keeps_crew_count_granted_trigger() {
    let def = parse_oracle_card_definition("Mighty Servant of Leuk-o");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let abilities_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Whenever this Vehicle becomes crewed for the first time each turn")
            && rendered.contains("crewed by exactly two creatures")
            && rendered.contains("Whenever this Vehicle deals combat damage to a player")
            && rendered.contains("draw two cards")
            && rendered.contains("until end of turn")
            && rendered.contains("Crew 4"),
        "expected Mighty Servant of Leuk-o compiled text to preserve crew-count trigger and temporary combat-damage draw grant, got {rendered}"
    );
    assert!(
        abilities_debug.contains("SourceFirstCrewedThisTurn")
            && abilities_debug.contains("SourceCrewedByExactly")
            && abilities_debug.contains("AddAbilityGeneric")
            && abilities_debug.contains("ThisDealsCombatDamageToPlayerTrigger")
            && abilities_debug.contains("CrewCostEffect"),
        "expected Mighty Servant of Leuk-o structure to include first-crew, exact crew count, granted damage trigger, and Crew 4, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn inviolability_runtime_prevents_damage_to_enchanted_creature_only() {
    let aura_def = parse_oracle_card_definition("Inviolability");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let protected_creature = CardBuilder::new(CardId::from_raw(98_001), "Protected Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let protected_id =
        game.create_object_from_card(&protected_creature, alice, crate::zone::Zone::Battlefield);

    let other_creature = CardBuilder::new(CardId::from_raw(98_002), "Other Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let other_id =
        game.create_object_from_card(&other_creature, alice, crate::zone::Zone::Battlefield);

    let source_creature = CardBuilder::new(CardId::from_raw(98_003), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let source_id =
        game.create_object_from_card(&source_creature, bob, crate::zone::Zone::Battlefield);

    let aura_id =
        game.create_object_from_definition(&aura_def, alice, crate::zone::Zone::Battlefield);
    game.object_mut(aura_id)
        .expect("Inviolability object should exist")
        .attached_to = Some(crate::object::AttachmentTarget::Object(protected_id));
    game.object_mut(protected_id)
        .expect("protected creature should exist")
        .attachments
        .push(aura_id);
    assert_eq!(
        game.object(aura_id).and_then(|obj| obj.attached_to),
        Some(crate::object::AttachmentTarget::Object(protected_id)),
        "Inviolability should attach to the selected creature"
    );

    let (damage_to_protected, protected_prevented) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(protected_id),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        damage_to_protected, 0,
        "Inviolability should prevent all damage to enchanted creature"
    );
    assert!(
        protected_prevented || damage_to_protected == 0,
        "damage to enchanted creature should be prevented"
    );

    let (damage_to_other, other_prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        source_id,
        crate::events::DamageTarget::Object(other_id),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        damage_to_other, 3,
        "Inviolability should not prevent damage to unenchanted creatures"
    );
    assert!(
        !other_prevented,
        "damage to unenchanted creature should remain unprevented"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn saving_grace_compiled_text_keeps_damage_redirection_clause() {
    let def = parse_oracle_card_definition("Saving Grace");

    assert_eq!(
        def.aura_attach_filter,
        Some(AuraAttachmentFilter::Object(
            ObjectFilter::creature().you_control()
        )),
        "Saving Grace should enchant only creatures you control"
    );

    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "all damage that would be dealt this turn to you and permanents you control is dealt to enchanted creature instead"
        ),
        "expected Saving Grace compiled text to preserve the temporary enchanted-creature redirection clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn saving_grace_runtime_redirects_controller_and_permanent_damage_this_turn_only() {
    let def = parse_oracle_card_definition("Saving Grace");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .display()
                    .to_ascii_lowercase()
                    .contains("enters") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Saving Grace should have an Aura-enters triggered ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let protected_creature = CardBuilder::new(CardId::from_raw(99_001), "Protected Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let protected_id =
        game.create_object_from_card(&protected_creature, alice, crate::zone::Zone::Battlefield);

    let other_creature = CardBuilder::new(CardId::from_raw(99_002), "Other Alice Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let other_id =
        game.create_object_from_card(&other_creature, alice, crate::zone::Zone::Battlefield);

    let bob_creature = CardBuilder::new(CardId::from_raw(99_003), "Bob Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let bob_creature_id =
        game.create_object_from_card(&bob_creature, bob, crate::zone::Zone::Battlefield);

    let damage_source = CardBuilder::new(CardId::from_raw(99_004), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let source_id =
        game.create_object_from_card(&damage_source, bob, crate::zone::Zone::Battlefield);

    let aura_id = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);
    game.object_mut(aura_id)
        .expect("Saving Grace object should exist")
        .attached_to = Some(crate::object::AttachmentTarget::Object(protected_id));
    game.object_mut(protected_id)
        .expect("protected creature should exist")
        .attachments
        .push(aura_id);

    let mut ctx = crate::effects::ExecutionContext::new_default(aura_id, alice);
    for effect in &triggered.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Saving Grace ETB effect should resolve");
    }

    let damage_to_alice = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        source_id,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        damage_to_alice.assignments,
        vec![crate::events::processing::ProcessedDamageAssignment {
            target: crate::events::DamageTarget::Object(protected_id),
            amount: 3,
        }],
        "damage to Saving Grace's controller should be redirected to enchanted creature"
    );

    let damage_to_other_permanent =
        crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(other_id),
            2,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        damage_to_other_permanent.assignments,
        vec![crate::events::processing::ProcessedDamageAssignment {
            target: crate::events::DamageTarget::Object(protected_id),
            amount: 2,
        }],
        "damage to permanents controlled by Saving Grace's controller should be redirected"
    );

    let damage_to_opponent_permanent =
        crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(bob_creature_id),
            4,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        damage_to_opponent_permanent.assignments,
        vec![crate::events::processing::ProcessedDamageAssignment {
            target: crate::events::DamageTarget::Object(bob_creature_id),
            amount: 4,
        }],
        "Saving Grace should not redirect damage to permanents controlled by another player"
    );

    let damage_to_bob = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        source_id,
        crate::events::DamageTarget::Player(bob),
        4,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        damage_to_bob.assignments,
        vec![crate::events::processing::ProcessedDamageAssignment {
            target: crate::events::DamageTarget::Player(bob),
            amount: 4,
        }],
        "Saving Grace should not redirect damage to another player"
    );

    game.effect_store
        .replacement_effects
        .clear_until_end_of_turn_effects();
    let after_turn_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        source_id,
        crate::events::DamageTarget::Player(alice),
        5,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        after_turn_damage.assignments,
        vec![crate::events::processing::ProcessedDamageAssignment {
            target: crate::events::DamageTarget::Player(alice),
            amount: 5,
        }],
        "Saving Grace's damage redirection should expire at end of turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn animate_land_runtime_animates_target_land_until_end_of_turn() {
    let def = parse_oracle_card_definition("Animate Land");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Animate Land should produce spell effects")
        .clone();
    let apply = spell
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                .or_else(|| {
                    effect
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .and_then(|tagged| {
                            tagged
                                .effect
                                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                        })
                })
        })
        .expect("Animate Land should lower to an ApplyContinuousEffect");

    let target_spec = apply
        .target_spec
        .as_ref()
        .expect("Animate Land should carry an explicit target spec");
    let ChooseSpec::Target(inner) = target_spec else {
        panic!("expected Animate Land target to be target-only, got {target_spec:?}");
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        panic!("expected Animate Land target to be an object filter, got {inner:?}");
    };
    assert!(
        filter.card_types == vec![CardType::Land]
            && !filter.card_types.contains(&CardType::Creature),
        "expected Animate Land to target lands only, got {filter:?}"
    );

    assert_eq!(apply.until, crate::effect::Until::EndOfTurn);
    assert!(
        matches!(apply.modification, Some(crate::continuous::Modification::AddCardTypes(ref added)) if added == &vec![CardType::Creature]),
        "expected Animate Land to add creature card type, got {apply:?}"
    );
    assert!(
        apply.additional_modifications.iter().any(|modification| {
            matches!(
                modification,
                crate::continuous::Modification::SetPowerToughness {
                    power: Value::Fixed(3),
                    toughness: Value::Fixed(3),
                    ..
                }
            )
        }),
        "expected Animate Land to set base power/toughness to 3/3, got {apply:?}"
    );
}

#[test]
pub(super) fn escaped_null_compiled_text_keeps_blocks_or_becomes_blocked_trigger() {
    let def = parse_oracle_card_definition("Escaped Null");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever this creature blocks or this creature becomes blocked")
            && rendered.contains("gets +5/+0 until end of turn"),
        "expected Escaped Null to keep its combat trigger and pump clause, got {rendered}"
    );
}

#[test]
pub(super) fn exuberant_fuseling_compiled_text_keeps_oil_counter_scaling_clause() {
    let def = parse_oracle_card_definition("Exuberant Fuseling");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("this creature gets +1/+0 for each oil counter on it"),
        "expected Exuberant Fuseling to keep its oil-counter scaling clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "when this creature enters and whenever another creature or artifact you control is put into a graveyard from the battlefield"
        ),
        "expected Exuberant Fuseling trigger wording to preserve enters-and-whenever structure, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn exuberant_fuseling_trigger_adds_oil_counter_for_etb_and_other_controlled_death_only()
{
    let def = parse_oracle_card_definition("Exuberant Fuseling");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let fuseling_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let etb_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            fuseling_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut etb_queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &etb_event)
        .into_iter()
        .filter(|entry| entry.source == fuseling_id)
    {
        etb_queue.add(entry);
    }
    assert_eq!(
        etb_queue.entries.len(),
        1,
        "expected Exuberant Fuseling ETB branch to trigger once"
    );

    let allied_artifact = CardDefinitionBuilder::new(CardId::new(), "Allied Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let allied_artifact_id =
        game.create_object_from_definition(&allied_artifact, alice, Zone::Battlefield);
    let allied_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(allied_artifact_id)
            .expect("allied artifact should exist"),
        &game,
    );
    let allied_dies_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            allied_artifact_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(allied_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());
    let mut allied_dies_queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &allied_dies_event)
        .into_iter()
        .filter(|entry| entry.source == fuseling_id)
    {
        allied_dies_queue.add(entry);
    }
    assert_eq!(
        allied_dies_queue.entries.len(),
        1,
        "expected another controlled artifact dying to trigger Exuberant Fuseling"
    );

    let opposing_artifact = CardDefinitionBuilder::new(CardId::new(), "Opposing Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let opposing_artifact_id =
        game.create_object_from_definition(&opposing_artifact, bob, Zone::Battlefield);
    let opposing_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(opposing_artifact_id)
            .expect("opposing artifact should exist"),
        &game,
    );
    let opposing_dies_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            opposing_artifact_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(opposing_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());
    let opposing_triggers = crate::triggers::check_triggers(&game, &opposing_dies_event)
        .into_iter()
        .filter(|entry| entry.source == fuseling_id)
        .count();
    assert_eq!(
        opposing_triggers, 0,
        "expected opponent permanent dying to not trigger Exuberant Fuseling"
    );
    let fuseling_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(fuseling_id).expect("fuseling should exist"),
        &game,
    );
    let fuseling_dies_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            fuseling_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(fuseling_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());
    let self_dies_triggers = crate::triggers::check_triggers(&game, &fuseling_dies_event)
        .into_iter()
        .filter(|entry| entry.source == fuseling_id)
        .count();
    assert_eq!(
        self_dies_triggers, 0,
        "expected Exuberant Fuseling to not trigger from its own death under the 'another' clause"
    );
}

#[test]
pub(super) fn escaped_null_trigger_models_both_combat_branches_and_temporary_pump() {
    let def = parse_oracle_card_definition("Escaped Null");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("OrTrigger")
            && debug.contains("ThisBlocksTrigger")
            && debug.contains("ThisBecomesBlockedTrigger"),
        "expected Escaped Null to compile a trigger that covers blocking and becoming blocked, got {debug}"
    );
    assert!(
        debug.contains("ModifyPowerToughness")
            && debug.contains("power: Fixed(5)")
            && debug.contains("toughness: Fixed(0)")
            && debug.contains("until: EndOfTurn"),
        "expected Escaped Null trigger effect to be +5/+0 until end of turn, got {debug}"
    );
    assert!(
        !debug.contains("ThisBlocksObject") && !debug.contains("ThisBecomesBlockedByObject"),
        "expected Escaped Null trigger to not require a blocker filter, got {debug}"
    );
}

#[test]
pub(super) fn nesting_grounds_compiled_text_matches_counter_move_clause() {
    let def = parse_oracle_card_definition("Nesting Grounds");
    let rendered = debug_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "{1}, {T}: Move a counter from target permanent you control onto a second target permanent. Activate only as a sorcery."
        ),
        "expected Nesting Grounds counter-move clause in compiled text, got {rendered}"
    );
}

#[test]
pub(super) fn nesting_grounds_move_counter_effect_moves_exactly_one_counter() {
    let def = parse_oracle_card_definition("Nesting Grounds");
    let move_effect = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .find_map(|effect| effect.downcast_ref::<crate::effects::MoveOneCounterEffect>()),
            _ => None,
        })
        .expect("Nesting Grounds should compile to a move-one-counter activated effect");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let controller = PlayerId::from_index(0);
    let source = game.new_object_id();
    let from_id = game.new_object_id();
    let to_id = game.new_object_id();
    let mut from_obj = crate::object::Object::new_token(
        from_id,
        controller,
        "From Permanent".to_string(),
        vec![CardType::Creature],
        Vec::new(),
        Some(2),
        Some(2),
        crate::color::ColorSet::COLORLESS,
    );
    from_obj
        .counters
        .insert(crate::object::CounterType::PlusOnePlusOne, 2);
    let to_obj = crate::object::Object::new_token(
        to_id,
        controller,
        "To Permanent".to_string(),
        vec![CardType::Creature],
        Vec::new(),
        Some(2),
        Some(2),
        crate::color::ColorSet::COLORLESS,
    );
    game.add_object(from_obj);
    game.add_object(to_obj);

    let mut ctx =
        crate::effects::EffectContext::new_default(source, controller).with_targets(vec![
            crate::effects::ResolvedTarget::Object(from_id),
            crate::effects::ResolvedTarget::Object(to_id),
        ]);
    let result = move_effect
        .execute(&mut game, &mut ctx)
        .expect("Nesting Grounds move-counter effect should execute");
    assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
    assert_eq!(
        game.counter_count(from_id, crate::object::CounterType::PlusOnePlusOne),
        1
    );
    assert_eq!(
        game.counter_count(to_id, crate::object::CounterType::PlusOnePlusOne),
        1
    );
}

#[test]
pub(super) fn strict_parse_regression_batch_target_cards() {
    let mut failures = Vec::new();
    for &name in STRICT_PARSE_REGRESSION_SUCCESS_CARDS {
        let oracle = match oracle_text_by_name().get(name) {
            Some(text) => text.clone(),
            None => {
                failures.push(format!("{name}: missing oracle text in cards.json"));
                continue;
            }
        };
        if let Err(err) = CardDefinitionBuilder::new(CardId::new(), name).parse_text(oracle.clone())
        {
            failures.push(format!("{name}: {err:?}"));
        }
    }
    for &name in STRICT_PARSE_REGRESSION_EXPECTED_FAILURE_CARDS {
        let oracle = match oracle_text_by_name().get(name) {
            Some(text) => text.clone(),
            None => {
                failures.push(format!("{name}: missing oracle text in cards.json"));
                continue;
            }
        };
        if CardDefinitionBuilder::new(CardId::new(), name)
            .parse_text(oracle.clone())
            .is_ok()
        {
            failures.push(format!(
                "{name}: expected strict parse failure, but parse succeeded"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "strict parse regression batch failures:\n{}",
        failures.join("\n")
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feudkillers_verdict_compiled_text_mentions_life_lead_condition() {
    let def = parse_oracle_card_definition("Feudkiller's Verdict");
    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("You gain 10 life")
            && rendered.contains("If you have more life than an opponent")
            && rendered.contains("create a 5/5 white Giant Warrior creature token"),
        "expected compiled text to preserve Feudkiller's Verdict condition and token clause, got: {rendered}"
    );
}

#[test]
pub(super) fn strict_parse_shared_parser_regression_cards() {
    for name in [
        "Tarmogoyf",
        "Carnage Interpreter",
        "Narset, Parter of Veils",
        "Leovold, Emissary of Trest",
        "Emberwilde Captain",
        "Palace Jailer",
        "Aragorn, King of Gondor",
        "Lightning Greaves",
        "Skullclamp",
        "Eagles of the North",
        "Lórien Revealed",
        "Loran of the Third Path",
        "Phelia, Exuberant Shepherd",
        "Sage of the Skies",
        "Creepy Puppeteer",
        "Serpentine Ambush",
    ] {
        assert_oracle_card_parses_strict(name);
    }
}

#[test]
pub(super) fn strict_parse_nighthawk_scavenger_regression() {
    assert_oracle_card_parses_strict("Nighthawk Scavenger");
}

#[test]
pub(super) fn strict_parse_olivias_midnight_ambush_regression() {
    assert_oracle_card_parses_strict("Olivia's Midnight Ambush");
}

#[test]
pub(super) fn strict_parse_branching_evolution_regression() {
    assert_oracle_card_parses_strict("Branching Evolution");
}

#[test]
pub(super) fn branching_evolution_compiled_text_preserves_counter_doubling_clause() {
    let def = parse_oracle_card_definition("Branching Evolution");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("+1/+1 counters")
            && rendered.contains("you control")
            && rendered.contains("twice that many"),
        "expected Branching Evolution compiled text to preserve +1/+1 counter doubling clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn branching_evolution_runtime_doubles_only_your_plus_one_counters() {
    let branching = parse_oracle_card_definition("Branching Evolution");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source = game.create_object_from_definition(&branching, alice, Zone::Battlefield);

    let creature = CardDefinitionBuilder::new(CardId::new(), "Counter Test Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let alice_creature = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let bob_creature = game.create_object_from_definition(&creature, bob, Zone::Battlefield);

    game.update_replacement_effects();

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.targets = vec![crate::effects::ResolvedTarget::Object(alice_creature)];
    let plus_one = crate::effects::PutCountersEffect {
        counter_type: crate::object::CounterType::PlusOnePlusOne,
        amount: crate::effect::Value::Fixed(1),
        target: crate::target::ChooseSpec::target_creature(),
        target_count: Some(crate::effect::ChoiceCount::exactly(1)),
        distributed: false,
    };
    plus_one
        .execute(&mut game, &mut ctx)
        .expect("putting +1/+1 counter on your creature should resolve");
    assert_eq!(
        game.counter_count(alice_creature, crate::object::CounterType::PlusOnePlusOne),
        2,
        "Branching Evolution should double +1/+1 counters put on creatures you control"
    );

    ctx.targets = vec![crate::effects::ResolvedTarget::Object(bob_creature)];
    plus_one
        .execute(&mut game, &mut ctx)
        .expect("putting +1/+1 counter on opponent creature should resolve");
    assert_eq!(
        game.counter_count(bob_creature, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Branching Evolution should not double +1/+1 counters on creatures you do not control"
    );

    ctx.targets = vec![crate::effects::ResolvedTarget::Object(alice_creature)];
    let deathtouch_counter = crate::effects::PutCountersEffect {
        counter_type: crate::object::CounterType::Deathtouch,
        amount: crate::effect::Value::Fixed(1),
        target: crate::target::ChooseSpec::target_creature(),
        target_count: Some(crate::effect::ChoiceCount::exactly(1)),
        distributed: false,
    };
    deathtouch_counter
        .execute(&mut game, &mut ctx)
        .expect("putting deathtouch counter on your creature should resolve");
    assert_eq!(
        game.counter_count(alice_creature, crate::object::CounterType::Deathtouch),
        1,
        "Branching Evolution should not double non-+1/+1 counters"
    );
}

#[test]
pub(super) fn olivias_midnight_ambush_compiled_text_preserves_night_branch() {
    let def = parse_oracle_card_definition("Olivia's Midnight Ambush");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("target creature gets -2/-2 until end of turn")
            && rendered.contains("if it's night, it gets -13/-13 until end of turn instead"),
        "expected Olivia's Midnight Ambush to preserve both base and night conditional branches, got {rendered}"
    );
}

#[test]
pub(super) fn olivias_midnight_ambush_runtime_applies_correct_day_night_branch() {
    use crate::effects::ResolvedTarget;

    let def = parse_oracle_card_definition("Olivia's Midnight Ambush");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Olivia's Midnight Ambush should produce spell effects")
        .clone();

    let resolve_with_night = |is_night: bool| {
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        game.is_night = is_night;
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
        let target = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::from_raw(91_301), "Ambush Target")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(20, 20))
                .build(),
            bob,
            Zone::Battlefield,
        );

        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            spell_source,
            &spell,
            None,
            &[],
        )
        .expect("Olivia's Midnight Ambush should resolve");

        let power = game.current_power(target).unwrap_or(0);
        let toughness = game.current_toughness(target).unwrap_or(0);
        (power, toughness)
    };

    let (day_power, day_toughness) = resolve_with_night(false);
    assert_eq!(
        (day_power, day_toughness),
        (18, 18),
        "day branch should apply the default -2/-2 effect"
    );

    let (night_power, night_toughness) = resolve_with_night(true);
    assert_eq!(
        (night_power, night_toughness),
        (7, 7),
        "night branch should replace the default effect with -13/-13"
    );
}

#[test]
pub(super) fn nighthawk_scavenger_compiled_text_preserves_card_types_among_clause() {
    let def = parse_oracle_card_definition("Nighthawk Scavenger");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        (rendered.contains("power is 1 plus") || rendered.contains("power is equal to 1 plus"))
            && (rendered.contains(
                "1 plus the number of card types among cards in your opponents' graveyard"
            ) || rendered.contains(
                "1 plus the number of card types among cards in your opponents' graveyards"
            ) || rendered
                .contains("1 plus the number of distinct card types in your opponents' graveyard")
                || rendered.contains(
                    "1 plus the number of distinct card types in your opponents' graveyards"
                )),
        "expected Nighthawk Scavenger to keep a card-types-in-opponents-graveyards scaling clause, got {rendered}"
    );
}

#[test]
pub(super) fn nighthawk_scavenger_characteristic_runtime_scaling_regression() {
    let def = parse_oracle_card_definition("Nighthawk Scavenger");
    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .expect("Nighthawk Scavenger should have a characteristic-defining power ability");

    let game = crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let effects = static_ability.generate_effects(
        crate::ids::ObjectId::from_raw(1),
        crate::ids::PlayerId::from_index(0),
        &game,
    );
    let crate::continuous::Modification::SetPowerToughness {
        power,
        toughness,
        sublayer: _,
    } = &effects[0].modification
    else {
        panic!("expected Nighthawk Scavenger to use a SetPowerToughness CDA");
    };

    let is_expected_power = match power {
        crate::effect::Value::Add(left, right) => {
            matches!(
                (&**left, &**right),
                (
                    crate::effect::Value::Fixed(1),
                    crate::effect::Value::CardTypesInGraveyard(PlayerFilter::Opponent)
                ) | (
                    crate::effect::Value::CardTypesInGraveyard(PlayerFilter::Opponent),
                    crate::effect::Value::Fixed(1)
                )
            )
        }
        _ => false,
    };
    assert!(
        is_expected_power,
        "expected Nighthawk Scavenger power to be 1 plus card types in opponents' graveyards, got {:?}",
        power
    );
    assert!(
        matches!(toughness, crate::effect::Value::SourceToughness),
        "expected Nighthawk Scavenger toughness CDA axis to keep source toughness, got {:?}",
        toughness
    );
}

#[test]
pub(super) fn polygoyf_strict_parser_and_compiled_text_preserves_all_graveyards_card_types() {
    assert_oracle_card_parses_strict("Polygoyf");

    let def = parse_oracle_card_definition("Polygoyf");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .expect("Polygoyf should have a characteristic-defining P/T ability");
    let game = crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let effects = static_ability.generate_effects(
        crate::ids::ObjectId::from_raw(1),
        crate::ids::PlayerId::from_index(0),
        &game,
    );
    let crate::continuous::Modification::SetPowerToughness {
        power,
        toughness,
        sublayer: _,
    } = &effects[0].modification
    else {
        panic!("expected Polygoyf to use a SetPowerToughness CDA");
    };

    assert!(
        matches!(
            power,
            crate::effect::Value::CardTypesInGraveyard(PlayerFilter::Any)
        ),
        "Polygoyf should structurally count card types across all graveyards, got {power:?}"
    );
    assert!(
        matches!(
            toughness,
            crate::effect::Value::Add(left, right)
                if matches!(&**left, crate::effect::Value::CardTypesInGraveyard(PlayerFilter::Any))
                    && matches!(&**right, crate::effect::Value::Fixed(1))
        ) || matches!(
            toughness,
            crate::effect::Value::Add(left, right)
                if matches!(&**right, crate::effect::Value::CardTypesInGraveyard(PlayerFilter::Any))
                    && matches!(&**left, crate::effect::Value::Fixed(1))
        ),
        "Polygoyf toughness should be card types across all graveyards plus 1, got {toughness:?}"
    );
    assert!(
        rendered.contains("trample") && rendered.contains("myriad"),
        "Polygoyf compiled text should preserve keyword identity, got {rendered}"
    );
    assert!(
        rendered.contains("number of card types among cards in all graveyards"),
        "Polygoyf compiled text should preserve the all-graveyards card-types-among clause, got {rendered}"
    );
    assert!(
        rendered.contains("toughness is equal to that number plus 1"),
        "Polygoyf compiled text should render the shared toughness value as that number plus 1, got {rendered}"
    );
    assert!(
        !rendered.contains("number of cards in all graveyards"),
        "Polygoyf must not collapse card types among graveyards into a plain card count, got {rendered}"
    );
}

#[test]
pub(super) fn altar_of_the_goyf_preserves_card_types_among_marker_in_triggered_pump() {
    assert_oracle_card_parses_strict("Altar of the Goyf");

    let def = parse_oracle_card_definition("Altar of the Goyf");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("number of card types among cards in all graveyards"),
        "Altar of the Goyf must preserve the exact card-types-among semantic marker, got {rendered}"
    );
    assert!(
        !rendered.contains("number of cards in all graveyards"),
        "Altar of the Goyf must not collapse distinct card types into a card count, got {rendered}"
    );
}

#[test]
pub(super) fn polygoyf_runtime_counts_distinct_card_types_in_all_graveyards() {
    let def = parse_oracle_card_definition("Polygoyf");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let polygoyf = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert_eq!(
        (
            game.current_power(polygoyf),
            game.current_toughness(polygoyf)
        ),
        (Some(0), Some(1)),
        "with empty graveyards, Polygoyf should be 0/1"
    );

    let artifact_creature = CardDefinitionBuilder::new(CardId::new(), "Artifact Creature Probe")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .build();
    let enchantment = CardDefinitionBuilder::new(CardId::new(), "Enchantment Probe")
        .card_types(vec![CardType::Enchantment])
        .build();
    let duplicate_artifact = CardDefinitionBuilder::new(CardId::new(), "Duplicate Artifact Probe")
        .card_types(vec![CardType::Artifact])
        .build();

    game.create_object_from_definition(&artifact_creature, alice, Zone::Graveyard);
    game.create_object_from_definition(&enchantment, bob, Zone::Graveyard);
    game.create_object_from_definition(&duplicate_artifact, bob, Zone::Graveyard);
    game.refresh_continuous_state();

    assert_eq!(
        (
            game.current_power(polygoyf),
            game.current_toughness(polygoyf)
        ),
        (Some(3), Some(4)),
        "Polygoyf should count distinct card types across both players' graveyards and add 1 toughness"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_gwen_stacy_ghost_spider_compiled_text_regression() {
    let def = parse_oracle_card_definition("Gwen Stacy // Ghost-Spider");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("play that card for as long as you control this creature"),
        "expected Gwen Stacy // Ghost-Spider permission duration clause, got {rendered}"
    );
    assert!(
        rendered.contains("Transform Gwen Stacy"),
        "expected Gwen Stacy transform clause to preserve the explicit source-name surface, got {rendered}"
    );
}

#[test]
pub(super) fn scroll_of_isildur_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Scroll of Isildur");
    let def = parse_oracle_card_definition("Scroll of Isildur");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains(
            "Gain control of up to one target artifact for as long as you control this Saga"
        ),
        "expected Scroll of Isildur chapter I to render the source-control duration, got {rendered}"
    );
    assert!(
        rendered.to_ascii_lowercase().contains("ring tempts you"),
        "expected Scroll of Isildur chapter I to render the Ring tempts clause, got {rendered}"
    );
    assert!(
        rendered.contains("Tap up to two target creatures. Put a stun counter on each of them"),
        "expected Scroll of Isildur chapter II to render counters on the tapped targets, got {rendered}"
    );
    assert!(
        debug.contains("YouStopControllingThis")
            && debug.contains("RingTemptsYouEffect")
            && debug.contains("ForEachObject")
            && debug.contains("Stun"),
        "expected Scroll of Isildur to keep source-control, Ring, and tagged stun-counter structures, got {debug}"
    );
}

#[test]
pub(super) fn creepy_puppeteer_regression_renders_base_power_toughness_followup() {
    let def = parse_oracle_card_definition("Creepy Puppeteer");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:#?}", def.abilities);
    assert!(
        (rendered.contains("base power and toughness become 4/3")
            || rendered.contains("base power and toughness 4/3"))
            && rendered.contains("until end of turn"),
        "expected Creepy Puppeteer to keep its temporary base power/toughness setting, got {rendered}"
    );
    assert!(
        rendered.contains("you may have that creature's base power and toughness become 4/3"),
        "expected Creepy Puppeteer to render the exact other attacker as that creature, got {rendered}"
    );
    assert!(
        !rendered.contains("you may each creature"),
        "expected Creepy Puppeteer not to render the tagged attacker as each creature, got {rendered}"
    );
    assert!(
        debug.contains("other_attacker"),
        "expected Creepy Puppeteer to bind the exact other attacker, got {debug}"
    );
}

#[test]
pub(super) fn optional_continuous_effects_render_causative_have() {
    for (name, expected) in [
        (
            "Creepy Puppeteer",
            "you may have that creature's base power and toughness become 4/3 until end of turn",
        ),
        (
            "Cultivator of Blades",
            "you may have each other attacking creature get +x/+x until end of turn",
        ),
        (
            "Vihaan, Goldwaker",
            "you may have each treasure you control become a 3/3 construct assassin artifact creature in addition to its other types until end of turn",
        ),
    ] {
        let def = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains(expected),
            "expected {name} to render optional continuous effect as causative have, got {rendered}"
        );
        assert!(
            !rendered.contains("you may each ") && !rendered.contains("you may that "),
            "expected {name} not to render a declarative clause directly after may, got {rendered}"
        );
    }
}

#[test]
pub(super) fn serpentine_ambush_regression_renders_color_subtype_and_base_pt() {
    let def = parse_oracle_card_definition("Serpentine Ambush");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("blue serpent with base power and toughness 5/5")
            || rendered.contains("blue serpent creature with base power and toughness 5/5"))
            && rendered.contains("until end of turn"),
        "expected Serpentine Ambush to keep oracle-style base power/toughness wording, got {rendered}"
    );
}

#[test]
pub(super) fn consuming_tide_regression_draws_for_each_opponent_who_is_ahead_on_cards() {
    let def = parse_oracle_card_definition("Consuming Tide");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return all other nonland permanents to their owners' hands")
            && (rendered.contains("more cards in their hand than you")
                || rendered.contains("has more cards in hand than you"))
            && rendered.contains("draw a card"),
        "expected Consuming Tide to keep its per-opponent card-draw rider, got {rendered}"
    );
}

#[test]
pub(super) fn participant_choice_cluster_preserves_the_chooser_and_selected_set() {
    for (name, choice_text, result_text) in [
        (
            "Consuming Tide",
            "each player chooses a nonland permanent they control",
            "return all other nonland permanents to their owners' hands",
        ),
        (
            "Divine Reckoning",
            "each player chooses a creature they control",
            "destroy the rest",
        ),
        (
            "Fatal Grudge",
            "each opponent chooses a permanent they control",
            "and sacrifices it",
        ),
        (
            "Summon: Valefor",
            "each opponent chooses a creature with the greatest mana value among creatures they control",
            "return those creatures to their owners' hands",
        ),
    ] {
        let def = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        let debug = format!("{def:#?}");

        assert!(
            rendered.contains(choice_text),
            "expected {name} choice, got {rendered}"
        );
        assert!(
            rendered.contains(result_text),
            "expected {name} result, got {rendered}"
        );
        assert!(
            debug.contains("ChooseObjectsEffect") && debug.contains("chooser: IteratedPlayer"),
            "expected {name}'s participant to own the choice, got {debug}"
        );
        assert!(
            debug.contains("TaggedObject") || debug.contains("spec: Tagged("),
            "expected {name}'s follow-up to reference the selected set, got {debug}"
        );
    }
}

#[test]
pub(super) fn generated_sacrifice_choice_cluster_compacts_the_selected_set() {
    for (name, expected) in [
        (
            "Nicol Bolas, Planeswalker",
            "sacrifices seven permanents of their choice",
        ),
        (
            "Shimatsu the Bloodcloaked",
            "sacrifice any number of permanents",
        ),
        ("Torrent of Stone", "sacrifice two mountains"),
        ("Wood Elemental", "sacrifice any number of untapped forests"),
    ] {
        let def = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        let debug = format!("{:#?}", def.abilities);

        assert!(
            rendered.contains(expected),
            "expected {name}'s chosen sacrifice set to compact, got {rendered}"
        );
        assert!(
            debug.contains("ChooseObjectsEffect")
                && (debug.contains("SacrificePlayerEffect") || debug.contains("SacrificeEffect"))
                && debug.contains("TaggedObject")
                && (debug.contains("Count(") || debug.contains("count: Fixed(")),
            "expected {name} to retain its tagged chosen-set sacrifice, got {debug}"
        );
    }
}

#[test]
pub(super) fn noncontroller_choice_regressions_keep_resolution_time_ownership() {
    let visions = parse_oracle_card_definition("Visions of Dread");
    let rendered = unprocessed_compiled_lines(&visions)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{visions:#?}");
    assert!(
        rendered.contains(
            "target opponent puts a creature card of their choice from their graveyard onto the battlefield under your control"
        ),
        "expected Visions of Dread to preserve the opponent's choice, got {rendered}"
    );
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("chooser: Target(")
            && debug.contains("Opponent"),
        "expected Visions of Dread to choose before moving the tagged card, got {debug}"
    );

    let obliterator = parse_oracle_card_definition("Phyrexian Obliterator");
    let rendered = unprocessed_compiled_lines(&obliterator)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:#?}", obliterator.abilities);
    assert!(
        rendered.contains("controller sacrifices that many permanents of their choice"),
        "expected Phyrexian Obliterator's damage controller choice, got {rendered}"
    );
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("ControllerOf(")
            && debug.contains("EventValue")
            && debug.contains("TaggedObject"),
        "expected Phyrexian Obliterator to sacrifice only the chosen set, got {debug}"
    );

    let wayfarer = parse_oracle_card_definition("Pale Wayfarer");
    let rendered = unprocessed_compiled_lines(&wayfarer)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:#?}", wayfarer.abilities);
    assert!(
        rendered.contains("protection from the color of its controller's choice until end of turn"),
        "expected Pale Wayfarer's target controller to choose the color, got {rendered}"
    );
    assert!(
        debug.contains("ChooseModeEffect")
            && debug.contains("chooser: Some(")
            && debug.contains("ControllerOf(")
            && debug.contains("Target"),
        "expected Pale Wayfarer's target controller to choose during resolution, got {debug}"
    );
}

#[test]
pub(super) fn thundering_raiju_regression_keeps_each_opponent_damage_target() {
    let def = parse_oracle_card_definition("Thundering Raiju");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:#?}", def.abilities);
    assert!(
        !rendered.contains("damage to you") && rendered.contains("each opponent"),
        "expected Thundering Raiju to keep damage pointed at opponents, got {rendered}"
    );
    assert!(
        debug.contains("DealDamageEffect") && debug.contains("IteratedPlayer"),
        "expected Thundering Raiju damage target to stay bound to the per-opponent loop, got {debug}"
    );
}

#[test]
pub(super) fn strict_parse_vote_regression_cards() {
    for name in [
        "Council's Judgment",
        "Tyrant's Choice",
        "Truth or Consequences",
        "Ballot Broker",
        "Brago's Representative",
        "Tivit, Seller of Secrets",
        "Elrond of the White Council",
        "Travel Through Caradhras",
        "Mob Verdict",
    ] {
        assert_oracle_card_parses_strict(name);
    }
}

#[test]
pub(super) fn strict_parse_meld_regression_cards() {
    for name in [
        "Graf Rats",
        "Gisela, the Broken Blade",
        "Hanweir Battlements",
        "Mishra, Claimed by Gix",
        "Titania, Voice of Gaea",
        "Urza, Lord Protector",
        "Vanille, Cheerful l'Cie",
    ] {
        assert_oracle_card_parses_strict(name);
    }
}

#[test]
pub(super) fn meld_regression_cards_do_not_exile_an_unrelated_hand_card() {
    for name in [
        "Gisela, the Broken Blade",
        "Titania, Voice of Gaea",
        "Vanille, Cheerful l'Cie",
    ] {
        let definition = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&definition)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            !rendered.contains("exile a card in your hand"),
            "{name} retained a synthetic hand-card exile before meld: {rendered}"
        );
    }
}

#[test]
pub(super) fn target_player_or_planeswalker_followups_keep_the_controller_actor() {
    for name in ["Blightning", "Rakdos's Return"] {
        let definition = parse_oracle_card_definition(name);
        let effects = definition
            .spell_effect
            .as_ref()
            .expect("damage-discard card should be a spell")
            .flattened_default_effects();
        let discard = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::DiscardEffect>())
            .expect("damage-discard spell should retain its discard");
        assert_eq!(
            discard.player,
            PlayerFilter::TargetPlayerOrControllerOfTarget,
            "{name} lost the planeswalker-controller branch of its follow-up actor"
        );
        let rendered = canonical_compiled_lines(&definition).join(" ");
        assert!(
            rendered.contains("That player or that planeswalker's controller discards"),
            "{name} did not preserve the typed controller reference in compiled text: {rendered}"
        );
    }
}

#[test]
pub(super) fn strict_parse_exert_regression_cards() {
    for name in [
        "Glory-Bound Initiate",
        "Combat Celebrant",
        "Hope Tender",
        "Vizier of the True",
        "Themberchaud",
    ] {
        assert_oracle_card_parses_strict(name);
    }
}

#[test]
pub(super) fn exert_regression_cards_lower_to_runtime_support_without_fallbacks() {
    let combat_celebrant = parse_oracle_card_definition("Combat Celebrant");
    let combat_celebrant_debug = format!("{:#?}", combat_celebrant.abilities);
    assert!(
        combat_celebrant_debug.contains("ExertAttack"),
        "expected Combat Celebrant to lower to the exert-attack static ability, got {combat_celebrant_debug}"
    );

    let hope_tender = parse_oracle_card_definition("Hope Tender");
    let hope_tender_debug = format!("{:#?}", hope_tender.abilities);
    assert!(
        hope_tender_debug.contains("ExertCostEffect"),
        "expected Hope Tender to lower exert as an activated cost, got {hope_tender_debug}"
    );

    let vizier = parse_oracle_card_definition("Vizier of the True");
    let vizier_debug = format!("{:#?}", vizier.abilities);
    assert!(
        vizier_debug.contains("source_filter: Some"),
        "expected Vizier of the True to keep its creature-only exert trigger filter, got {vizier_debug}"
    );

    let themberchaud = parse_oracle_card_definition("Themberchaud");
    let themberchaud_rendered = unprocessed_compiled_lines(&themberchaud).join(" ");
    let themberchaud_lowered = themberchaud_rendered.to_ascii_lowercase();
    assert!(
        themberchaud_lowered.contains("you may exert themberchaud as he attacks"),
        "expected Themberchaud to preserve its named-source exert wording, got {themberchaud_rendered}"
    );
    assert!(
        !themberchaud_lowered.contains("unsupported"),
        "expected Themberchaud to render without unsupported exert placeholders, got {themberchaud_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn exert_land_activation_cost_lowers_to_exert_runtime_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exert Land Probe")
        .card_types(vec![CardType::Land])
        .parse_text("{R}, {T}, Exert this land: Add {R}{R}.")
        .expect("land exert activation should parse");
    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ExertCostEffect"),
        "expected land exert activation to lower to the shared exert cost effect, got {debug}"
    );
}

#[test]
pub(super) fn vote_regression_truth_or_consequences_keeps_random_choice_before_consequences_loop() {
    let def = parse_oracle_card_definition("Truth or Consequences");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !rendered.contains("Unsupported effect"),
        "expected vote repeat effects to render cleanly, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    let truth_idx = debug
        .find("VoteCount(\"truth\")")
        .expect("truth repeat should be present");
    let choose_idx = debug
        .find("ChoosePlayerEffect")
        .expect("random opponent choice should be present");
    let consequences_idx = debug
        .find("VoteCount(\"consequences\")")
        .expect("consequences repeat should be present");
    assert!(
        truth_idx < choose_idx && choose_idx < consequences_idx,
        "expected random opponent choice to stay between the truth and consequences loops, got {debug}"
    );
}

#[test]
pub(super) fn amplify_regression_glowering_rogon_renders_keyword_without_unsupported_placeholder() {
    let def = parse_oracle_card_definition("Glowering Rogon");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Amplify 1"),
        "expected Glowering Rogon to render amplify keyword, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("unsupported effect"),
        "expected Glowering Rogon to avoid unsupported placeholder, got {rendered}"
    );
}

#[test]
pub(super) fn vote_regression_elrond_preserves_voter_choice_branch_and_owner_attack_restriction() {
    let def = parse_oracle_card_definition("Elrond of the White Council");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !rendered.contains("Unsupported effect"),
        "expected Elrond vote branch to render without unsupported placeholders, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("name: \"fellowship\"")
            && debug.contains("chooser: IteratedPlayer")
            && debug.contains("CantAttackItsOwner")
            && debug.contains("VoteCount(\"aid\")"),
        "expected Elrond to keep the fellowship voter choice branch, the owner-attack restriction, and the aid vote loop, got {debug}"
    );
    assert!(
        rendered.contains(
            "Secret council — When Elrond enters, each player secretly votes for fellowship or aid, then those votes are revealed. For each fellowship vote, the voter chooses a creature they control. You gain control of each creature chosen this way, and they gain \"This creature can't attack its owner.\" Then for each aid vote, put a +1/+1 counter on each creature you control."
        ),
        "expected Elrond's typed vote sequence to render its voter-relative creature set, got {rendered}"
    );
}

#[test]
pub(super) fn travel_through_caradhras_regression_renders_council_dilemma_vote_branches() {
    let def = parse_oracle_card_definition("Travel Through Caradhras");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("council's dilemma")
            && rendered.contains("Starting with you, each player votes for Redhorn Pass or Mines of Moria")
            && rendered.contains("For each Redhorn Pass vote, search your library for a basic land card and put it onto the battlefield tapped")
            && rendered.contains("then shuffle")
            && rendered.contains("For each Mines of Moria vote, return a card from your graveyard to your hand")
            && rendered.contains("Exile Travel Through Caradhras"),
        "expected Travel Through Caradhras to render its council dilemma branches, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("VoteCount")
            && debug.contains("\"redhorn pass\"")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("PutOntoBattlefieldEffect")
            && debug.contains("ShuffleLibraryEffect")
            && debug.contains("\"mines of moria\"")
            && debug.contains("ReturnFromGraveyardToHandEffect"),
        "expected Travel Through Caradhras to keep both vote-count branches structurally, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct TravelVoteDecisionMaker {
    pub(super) votes: Vec<usize>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for TravelVoteDecisionMaker {
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

    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let max = ctx.max.unwrap_or(ctx.candidates.len()).max(ctx.min);
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(max)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn basic_land_for_travel_test(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn graveyard_card_for_travel_test(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Creature])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_travel_through_caradhras_with_votes(
    votes: Vec<usize>,
) -> (
    crate::game_state::GameState,
    Vec<crate::triggers::TriggerEvent>,
    ObjectId,
    Vec<ObjectId>,
    Vec<ObjectId>,
) {
    let def = parse_oracle_card_definition("Travel Through Caradhras");
    let program = def
        .spell_effect
        .as_ref()
        .expect("Travel Through Caradhras should compile to spell effects");
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let forest = basic_land_for_travel_test(91_201, "Travel Forest");
    let island = basic_land_for_travel_test(91_202, "Travel Island");
    let grave_one = graveyard_card_for_travel_test(91_203, "Travel Grave One");
    let grave_two = graveyard_card_for_travel_test(91_204, "Travel Grave Two");
    let land_ids = vec![
        game.create_object_from_definition(&forest, alice, Zone::Library),
        game.create_object_from_definition(&island, alice, Zone::Library),
    ];
    let graveyard_ids = vec![
        game.create_object_from_definition(&grave_one, alice, Zone::Graveyard),
        game.create_object_from_definition(&grave_two, alice, Zone::Graveyard),
    ];
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = TravelVoteDecisionMaker { votes };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let events = crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Travel Through Caradhras should resolve");
    (game, events, source, land_ids, graveyard_ids)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn travel_search_event_count(events: &[crate::triggers::TriggerEvent]) -> usize {
    events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::SearchLibraryEvent>())
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn travel_shuffle_event_count(events: &[crate::triggers::TriggerEvent]) -> usize {
    events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::ShuffleLibraryEvent>())
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn travel_zone_names(game: &crate::game_state::GameState, zone: Zone) -> Vec<String> {
    game.objects_in_zone(zone)
        .into_iter()
        .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn travel_through_caradhras_runtime_redhorn_pass_votes_search_lands_and_exile_source() {
    let (game, events, _source, _land_ids, _graveyard_ids) =
        resolve_travel_through_caradhras_with_votes(vec![0, 0]);

    let battlefield_lands = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|object| object.name.starts_with("Travel ") && object.is_land())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        battlefield_lands.len(),
        2,
        "two Redhorn Pass votes should put two basic lands onto the battlefield; battlefield={:?} library={:?} searches={} shuffles={}",
        travel_zone_names(&game, Zone::Battlefield),
        travel_zone_names(&game, Zone::Library),
        travel_search_event_count(&events),
        travel_shuffle_event_count(&events)
    );
    assert!(
        battlefield_lands.iter().all(|&id| game.is_tapped(id)),
        "Redhorn Pass lands should enter tapped"
    );
    let graveyard_names = travel_zone_names(&game, Zone::Graveyard);
    assert!(
        graveyard_names.contains(&"Travel Grave One".to_string())
            && graveyard_names.contains(&"Travel Grave Two".to_string()),
        "Mines of Moria branch should not run for Redhorn Pass votes"
    );
    assert_eq!(
        travel_search_event_count(&events),
        2,
        "two Redhorn Pass votes should search twice"
    );
    assert!(
        travel_shuffle_event_count(&events) >= 1,
        "searching this way should shuffle the library"
    );
    assert_eq!(
        travel_zone_names(&game, Zone::Exile),
        vec!["Travel Through Caradhras".to_string()],
        "Travel Through Caradhras should exile itself after resolving"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn travel_through_caradhras_runtime_mines_votes_return_graveyard_cards_without_searching()
 {
    let (game, events, _source, _land_ids, _graveyard_ids) =
        resolve_travel_through_caradhras_with_votes(vec![1, 1]);

    let hand_names = travel_zone_names(&game, Zone::Hand);
    assert!(
        hand_names.contains(&"Travel Grave One".to_string())
            && hand_names.contains(&"Travel Grave Two".to_string()),
        "Mines of Moria votes should return both graveyard cards to hand"
    );
    let library_names = travel_zone_names(&game, Zone::Library);
    assert!(
        library_names.contains(&"Travel Forest".to_string())
            && library_names.contains(&"Travel Island".to_string()),
        "Redhorn Pass branch should not search lands for Mines of Moria votes"
    );
    assert_eq!(
        travel_search_event_count(&events),
        0,
        "Mines-only votes should not search"
    );
    assert_eq!(
        travel_shuffle_event_count(&events),
        0,
        "Mines-only votes should not shuffle"
    );
    assert_eq!(
        travel_zone_names(&game, Zone::Exile),
        vec!["Travel Through Caradhras".to_string()],
        "Travel Through Caradhras should exile itself after resolving"
    );
}

#[test]
pub(super) fn mob_verdict_strict_parser_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Mob Verdict");
    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Secret council — Each player secretly votes for another player, then those votes are revealed"
        ) && rendered.contains(
            "For each vote an opponent received, Mob Verdict deals 2 damage to that player and each creature that player controls"
        ) && rendered.contains("For each vote you received, draw a card"),
        "expected Mob Verdict to render secret player votes and received-vote followups, got {rendered}"
    );
    assert!(
        !rendered.contains("Unsupported effect"),
        "Mob Verdict should parse strictly without unsupported placeholders, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("VoteEffect")
            && debug.contains("Players")
            && debug.contains("exclude_voter: true")
            && debug.contains("PlayerVoteCount")
            && debug.contains("ForPlayersEffect")
            && debug.contains("DealDamageEffect")
            && debug.contains("DrawCardsEffect"),
        "expected player-vote counts, opponent damage, controlled-creature damage, and draw structurally, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct MobVerdictDecisionMaker {
    pub(super) votes: Vec<usize>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for MobVerdictDecisionMaker {
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
pub(super) fn mob_test_creature(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn mob_test_card(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Instant])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn mob_owner_zone_count(
    game: &crate::game_state::GameState,
    owner: PlayerId,
    zone: Zone,
) -> usize {
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|&id| game.object(id).is_some_and(|object| object.owner == owner))
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_mob_verdict_with_votes(
    votes: Vec<usize>,
    alice_library_cards: usize,
) -> (
    crate::game_state::GameState,
    ObjectId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let def = parse_oracle_card_definition("Mob Verdict");
    let program = def
        .spell_effect
        .as_ref()
        .expect("Mob Verdict should compile to spell effects");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let alice_creature = mob_test_creature(91_301, "Alice Mob Creature");
    let bob_creature = mob_test_creature(91_302, "Bob Mob Creature");
    let charlie_creature = mob_test_creature(91_303, "Charlie Mob Creature");
    let bob_second_creature = mob_test_creature(91_305, "Second Bob Mob Creature");
    let filler = mob_test_card(91_304, "Mob Draw Filler");
    let alice_creature_id =
        game.create_object_from_definition(&alice_creature, alice, Zone::Battlefield);
    let bob_creature_id = game.create_object_from_definition(&bob_creature, bob, Zone::Battlefield);
    let charlie_creature_id =
        game.create_object_from_definition(&charlie_creature, charlie, Zone::Battlefield);
    let bob_second_creature_id =
        game.create_object_from_definition(&bob_second_creature, bob, Zone::Battlefield);
    for _ in 0..alice_library_cards {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = MobVerdictDecisionMaker { votes };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Mob Verdict should resolve");
    (
        game,
        alice_creature_id,
        bob_creature_id,
        charlie_creature_id,
        bob_second_creature_id,
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mob_verdict_runtime_opponent_votes_damage_players_and_their_creatures_without_you_draw()
 {
    let (game, alice_creature, bob_creature, charlie_creature, bob_second_creature) =
        resolve_mob_verdict_with_votes(vec![0, 1, 1], 0);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "Alice received no votes and should not be damaged"
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        16,
        "Bob received two votes and should take 4 damage"
    );
    assert_eq!(
        game.player(charlie).unwrap().life,
        18,
        "Charlie received one vote and should take 2 damage"
    );
    assert_eq!(
        game.damage_on(alice_creature),
        0,
        "Alice's creature should not be damaged by opponent-only vote followup"
    );
    assert_eq!(
        game.damage_on(bob_creature),
        4,
        "Bob's creature should be damaged once per vote Bob received"
    );
    assert_eq!(
        game.damage_on(bob_second_creature),
        4,
        "every creature Bob controls should be damaged once per vote Bob received"
    );
    assert_eq!(
        game.damage_on(charlie_creature),
        2,
        "Charlie's creature should be damaged once per vote Charlie received"
    );
    assert_eq!(
        mob_owner_zone_count(&game, alice, Zone::Hand),
        0,
        "Alice should not draw when she received no votes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mob_verdict_runtime_votes_you_received_draw_cards_without_opponent_damage_to_you() {
    let (game, alice_creature, bob_creature, charlie_creature, _bob_second_creature) =
        resolve_mob_verdict_with_votes(vec![1, 0, 0], 3);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    assert_eq!(
        mob_owner_zone_count(&game, alice, Zone::Hand),
        2,
        "Alice should draw once for each vote she received"
    );
    assert_eq!(
        mob_owner_zone_count(&game, alice, Zone::Library),
        1,
        "drawing two cards should leave one filler in Alice's library"
    );
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "votes Alice received should draw cards, not damage Alice"
    );
    assert_eq!(
        game.damage_on(alice_creature),
        0,
        "Alice's creature should not be damaged by votes Alice received"
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        20,
        "Bob received no votes and should not be damaged"
    );
    assert_eq!(
        game.damage_on(bob_creature),
        0,
        "Bob's creature should not be damaged without Bob receiving votes"
    );
    assert_eq!(
        game.player(charlie).unwrap().life,
        18,
        "Charlie received Alice's vote and should take 2 damage"
    );
    assert_eq!(
        game.damage_on(charlie_creature),
        2,
        "Charlie's creature should be damaged for Charlie's received vote"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn and_or_union_representative_cards_preserve_surface_and_recipient_domains() {
    let aatchik =
        compiled_text_lines(&parse_oracle_card_definition("Aatchik, Emerald Radian")).join(" ");
    assert!(
        aatchik.contains("artifact and/or creature card in your graveyard"),
        "Aatchik must retain the inclusive artifact/creature graveyard union: {aatchik}"
    );

    let dargo =
        compiled_text_lines(&parse_oracle_card_definition("Dargo, the Shipwrecker")).join(" ");
    assert!(
        dargo.contains("sacrifice any number of artifacts and/or creatures"),
        "Dargo's additional cost must retain both sacrifice object classes: {dargo}"
    );

    let red_terror = compiled_text_lines(&parse_oracle_card_definition("The Red Terror")).join(" ");
    assert!(
        red_terror.contains(
            "a red source you control deals damage to one or more permanents and/or players"
        ),
        "The Red Terror must retain both permanent and player damage recipients: {red_terror}"
    );
}
