use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, can_cast_spell, compute_legal_actions};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    compile_definition_from_payload, default_cards_path, load_card_payloads_by_name,
};

fn teeg() -> ironsmith::cards::CardDefinition {
    let payloads =
        load_card_payloads_by_name(default_cards_path().to_str().unwrap(), "Gaddock Teeg").unwrap();
    compile_definition_from_payload(&payloads[0]).unwrap()
}

#[test]
fn compiled_teeg_preserves_x_symbol() {
    let definition = teeg();
    assert_eq!(
        definition.canonical_text,
        "Noncreature spells with mana value 4 or greater can't be cast.\nNoncreature spells with {X} in their mana costs can't be cast."
    );
}

#[test]
fn teeg_restricts_both_players_and_stops_when_it_leaves() {
    let definition = teeg();
    let alice = PlayerId::from_index(0);
    for caster in [alice, PlayerId::from_index(1)] {
        for (types, generic, x, allowed) in [
            (vec![CardType::Sorcery], 3, false, true),
            (vec![CardType::Sorcery], 4, false, false),
            (vec![CardType::Instant], 5, false, false),
            (vec![CardType::Artifact], 4, false, false),
            (vec![CardType::Enchantment], 4, false, false),
            (vec![CardType::Planeswalker], 4, false, false),
            (vec![CardType::Sorcery], 0, true, false),
            (vec![CardType::Instant], 1, true, false),
            (vec![CardType::Creature], 6, false, true),
            (vec![CardType::Creature], 0, true, true),
            (vec![CardType::Artifact, CardType::Creature], 5, true, true),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.turn.active_player = caster;
            game.turn.priority_player = Some(caster);
            game.player_mut(caster)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let mut cost = ManaCost::new().add_generic(generic);
            if x {
                cost =
                    ManaCost::from_pips([cost.pips().to_vec(), vec![vec![ManaSymbol::X]]].concat());
            }
            let fixture = CardDefinitionBuilder::new(CardId::new(), "Casting fixture")
                .card_types(types.clone())
                .mana_cost(cost)
                .build();
            let spell = game.create_object_from_definition(&fixture, caster, Zone::Hand);
            game.refresh_continuous_state();
            assert_eq!(
                game.effect_store
                    .cant_effects
                    .cast_filters_for_player(caster)
                    .unwrap()
                    .len(),
                2
            );
            let can_cast = |game: &GameState| {
                can_cast_spell(
                    game,
                    caster,
                    game.object(spell).unwrap(),
                    &CastingMethod::Normal,
                )
            };
            assert_eq!(
                can_cast(&game),
                allowed,
                "caster={caster:?}, types={types:?}, mana={generic}, X={x}"
            );
            assert_eq!(compute_legal_actions(&game, caster).iter().any(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)), allowed);
            attempt_cast(
                &game,
                LegalAction::CastSpell {
                    spell_id: spell,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                },
                allowed,
            );
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            game.refresh_continuous_state();
            assert!(can_cast(&game), "restriction must end when Teeg leaves");
        }
    }
}

fn attempt_cast(game: &GameState, action: LegalAction, expected: bool) {
    use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    let mut game = game.clone();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(!game.stack.is_empty(), expected, "cast result: {result:?}");
    if !expected {
        assert!(
            result.is_err(),
            "prohibited cast should be rejected: {result:?}"
        );
    }
}

#[test]
fn teeg_checks_flashback_escape_and_exile_against_printed_cost() {
    use ironsmith::alternative_cast::AlternativeCastingMethod;
    let definition = teeg();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for caster in [alice, bob] {
        for route in ["flashback", "escape", "exile"] {
            // Alternative costs are deliberately cheap: paying less must not
            // evade either the printed mana-value restriction or the X symbol.
            for (creature, generic, x, allowed) in [
                (false, 3, false, true),
                (false, 4, false, false),
                (false, 0, true, false),
                (true, 5, true, true),
            ] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.turn.active_player = caster;
                game.turn.priority_player = Some(caster);
                game.player_mut(caster)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 20);
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let mut pips = vec![vec![ManaSymbol::Generic(generic)]];
                if x {
                    pips.push(vec![ManaSymbol::X]);
                }
                let mut builder =
                    CardDefinitionBuilder::new(CardId::new(), "Alternative casting fixture")
                        .card_types(vec![if creature {
                            CardType::Creature
                        } else {
                            CardType::Sorcery
                        }])
                        .mana_cost(ManaCost::from_pips(pips));
                let alt_cost = ManaCost::new().add_generic(1);
                let origin = if route == "exile" {
                    Zone::Exile
                } else {
                    Zone::Graveyard
                };
                builder = match route {
                    "flashback" => builder.flashback(alt_cost),
                    "escape" => builder.escape(alt_cost, 1),
                    _ => builder.alternative_cast(AlternativeCastingMethod::FromZone {
                        name: "Exile permission".into(),
                        zone: origin,
                        total_cost: ironsmith::cost::TotalCost::mana(alt_cost),
                        condition: None,
                        exiles_after_resolution: false,
                    }),
                };
                let fodder =
                    CardDefinitionBuilder::new(CardId::new(), "Escape payment fixture").build();
                game.create_object_from_definition(&fodder, caster, Zone::Graveyard);
                let spell = game.create_object_from_definition(&builder.build(), caster, origin);
                game.refresh_continuous_state();
                let action = LegalAction::CastSpell {
                    spell_id: spell,
                    from_zone: origin,
                    casting_method: CastingMethod::Alternative(0),
                };
                let offered = compute_legal_actions(&game, caster).iter().any(|action|
                    matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
                assert_eq!(
                    offered, allowed,
                    "{route}, caster={caster:?}, creature={creature}, MV={generic}, X={x}"
                );
                attempt_cast(&game, action.clone(), allowed);
                game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                game.refresh_continuous_state();
                attempt_cast(&game, action, true);
            }
        }
    }
}

#[test]
fn cast_restriction_preview_preserves_explicit_origin_filters() {
    use ironsmith::target::ObjectFilter;
    let alice = PlayerId::from_index(0);
    for origin in [Zone::Hand, Zone::Graveyard, Zone::Exile] {
        for (excluded, announced) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let card = CardDefinitionBuilder::new(CardId::new(), "Origin fixture")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::new())
                .build();
            let mut spell = game.create_object_from_definition(&card, alice, origin);
            if announced {
                let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(
                    game.object(spell).unwrap(),
                    &game,
                );
                spell = game.move_object_by_effect(spell, Zone::Stack).unwrap();
                game.set_cast_origin_snapshot(spell, snapshot);
            }
            let mut filter = ObjectFilter::spell();
            if excluded {
                filter.excluded_cast_origin_zone = Some(Zone::Hand);
            } else {
                filter.zone = Some(Zone::Graveyard);
            }
            game.effect_store
                .cant_effects
                .add_cant_cast_filter(alice, filter);
            // This lower-level query tests restrictions independently of the
            // separately required permission to cast from a non-hand zone.
            let allowed = can_cast_spell(
                &game,
                alice,
                game.object(spell).unwrap(),
                &CastingMethod::Normal,
            );
            assert_eq!(
                allowed,
                if excluded {
                    origin == Zone::Hand
                } else {
                    origin != Zone::Graveyard
                },
                "origin={origin:?}, excluded={excluded}, announced={announced}"
            );
        }
    }
}
