#![allow(unused_imports)]
use super::shard_01::*;
use super::shard_02::*;
use super::*;
use crate::ConditionExpr;

#[test]
fn parse_yawgmoths_will_from_text() {
    let text = "Until end of turn, you may play lands and cast spells from your graveyard.\n\
If a card would be put into your graveyard from anywhere this turn, exile that card instead.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Yawgmoth's Will")
        .parse_text(text)
        .expect("parse yawgmoth's will");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(effects.len(), 2);
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<GrantBySpecEffect>().is_some()),
        "should include play-from-graveyard effect"
    );
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<ExileInsteadOfGraveyardEffect>().is_some()),
        "should include exile-instead replacement effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_dauthi_voidwalker_full_text_without_parser_fallback() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
            )
            .expect("Dauthi Voidwalker text should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    let abilities_debug_compact: String = abilities_debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(
        !abilities_debug.contains("UnsupportedParserLine"),
        "expected full Dauthi text to avoid unsupported parser fallbacks, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ExileToCounteredExileInsteadOfGraveyard"),
        "expected Dauthi replacement ability to lower to a real static ability, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ChooseObjectsEffect")
            && abilities_debug_compact.contains("zone:Some(Exile,)")
            && abilities_debug_compact.contains("with_counter:Some(")
            && abilities_debug.contains("Void"),
        "expected Dauthi activation to choose from exile, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("GrantTaggedSpellFreeCastUntilEndOfTurnEffect"),
        "expected Dauthi activation to preserve the free-cast clause, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn each_player_coin_face_followup_round_trips_with_typed_correlation() {
    let oracle = "Whenever this creature or another Goblin enters, each player flips a coin. Each player whose coin comes up tails sacrifices a creature of their choice.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Goblin Coin Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin])
        .parse_text(oracle)
        .expect("each-player coin-face follow-up should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("kind: FaceOnly")
            && debug.contains("ForPlayersEffect")
            && debug.contains("DidNotHappen")
            && debug.contains("IteratedPlayer")
            && debug.contains("SacrificePlayerEffect"),
        "expected a face-only per-player result gate into sacrifice: {debug}"
    );
    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn face_down_top_card_per_opponent_round_trips_as_ordered_exile() {
    let oracle = "When this creature enters, exile a card from the top of your library face down for each opponent you have.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Mourning Wall Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wall])
        .parse_text(oracle)
        .expect("face-down top-card-per-opponent trigger should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ExileTopOfLibraryEffect")
            && debug.contains("CountPlayers")
            && debug.contains("Opponent")
            && debug.contains("ForEach")
            && debug.contains("face_down: true"),
        "expected ordered face-down exile with a typed opponent count: {debug}"
    );
    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn singular_source_exiled_move_round_trips_with_an_exact_choice() {
    let oracle = "At the beginning of your end step, put a card exiled with this creature into its owner's hand.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Source-Linked Return Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("singular source-linked return should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("WithCount")
            && debug.contains("min: 1")
            && debug.contains("max: Some(1)")
            && !debug.contains("target: All("),
        "expected an exact one-card source-linked choice: {debug}"
    );
    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_land_with_quoted_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quoted Aura")
        .parse_text(
            "Enchant land\nEnchanted land has \"{T}: Create a 1/1 green Squirrel creature token.\"",
        )
        .expect("quoted attached activated ability should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        abilities_debug.contains("AttachedAbilityGrant"),
        "expected attached activated ability grant, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("CreateTokenEffect"),
        "expected granted activated ability to keep its token effect, got {abilities_debug}"
    );
    assert!(
        rendered.contains("{T}:"),
        "expected rendered grant to keep the tap symbol, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dauthi_voidwalker_activation_grants_free_exile_cast_action() {
    use crate::ability::AbilityKind;
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let dauthi = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Test")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
            )
            .expect("Dauthi text should parse");
    let dauthi_id = game.create_object_from_definition(&dauthi, alice, Zone::Battlefield);
    game.remove_summoning_sickness(dauthi_id);

    let bears = crate::cards::definitions::grizzly_bears();
    let bears_id = game.create_object_from_definition(&bears, bob, Zone::Battlefield);
    let bears_stable_id = game
        .object(bears_id)
        .expect("grizzly bears should exist")
        .stable_id;

    let mut dm = SelectFirstDecisionMaker;
    let zone_change = crate::events::processing::process_zone_change(
        &mut game,
        bears_id,
        Zone::Battlefield,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_sba(),
        &mut dm,
    );
    assert!(
        matches!(
            zone_change,
            crate::events::processing::ZoneChangeOutcome::Replaced
        ),
        "expected Dauthi replacement to exile the creature, got {zone_change:?}"
    );

    let exiled_bears_id = game
        .find_object_by_stable_id(bears_stable_id)
        .expect("exiled Grizzly Bears should be findable by stable id");
    assert_eq!(
        game.object(exiled_bears_id)
            .expect("exiled bears should exist")
            .zone,
        Zone::Exile,
        "Grizzly Bears should be exiled by Dauthi's replacement effect"
    );
    assert_eq!(
        game.counter_count(exiled_bears_id, CounterType::Void),
        1,
        "exiled Grizzly Bears should have a void counter"
    );

    let actions_before = compute_legal_actions(&game, alice);
    assert!(
        !actions_before.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    ..
                } if *spell_id == exiled_bears_id
            )
        }),
        "card should not be castable from exile before Dauthi's activation resolves"
    );

    let activated = game
        .object(dauthi_id)
        .expect("Dauthi should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Dauthi should have an activated ability");
    let effects_debug = format!("{:#?}", activated.effects);

    let mut ctx = ExecutionContext::new(dauthi_id, alice, &mut dm);
    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Dauthi activation effect should resolve");
    }

    let play_from_grants = game.effect_store.grant_registry.granted_play_from_for_card(
        &game,
        exiled_bears_id,
        Zone::Exile,
        alice,
    );
    let alt_grants = game
        .effect_store
        .grant_registry
        .granted_alternative_casts_for_card(&game, exiled_bears_id, Zone::Exile, alice);
    assert!(
        !play_from_grants.is_empty(),
        "expected a play-from-exile grant after Dauthi activation, effects={effects_debug}, grants={:?}",
        game.effect_store.grant_registry.grants
    );
    assert!(
        !alt_grants.is_empty(),
        "expected a free-cast alternative after Dauthi activation, effects={effects_debug}, grants={:?}",
        game.effect_store.grant_registry.grants
    );

    let actions_after = compute_legal_actions(&game, alice);
    assert!(
        actions_after.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::PlayFrom {
                        zone: Zone::Exile,
                        use_alternative: Some(_),
                        ..
                    },
                } if *spell_id == exiled_bears_id
            )
        }),
        "Dauthi activation should make the exiled void-counter card castable for free, got {actions_after:?}"
    );
}

#[derive(Default)]
struct RecordingObjectChoiceDecisionMaker {
    decide_objects_calls: usize,
    legal_candidates: Vec<crate::ids::ObjectId>,
    preferred_choice: Option<crate::ids::ObjectId>,
    pick_index: usize,
}

impl crate::decision::DecisionMaker for RecordingObjectChoiceDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        self.decide_objects_calls += 1;
        self.legal_candidates = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();

        let choice = self
            .preferred_choice
            .filter(|object_id| self.legal_candidates.contains(object_id))
            .or_else(|| self.legal_candidates.get(self.pick_index).copied())
            .or_else(|| self.legal_candidates.first().copied())
            .expect("choice prompt should contain a legal candidate");
        vec![choice]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dauthi_voidwalker_activation_auto_selects_single_candidate_without_choice_prompt() {
    use crate::ability::AbilityKind;
    use crate::alternative_cast::CastingMethod;
    use crate::decision::LegalAction;
    use crate::decision::compute_legal_actions;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let dauthi = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Test")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
            )
            .expect("Dauthi text should parse");
    let dauthi_id = game.create_object_from_definition(&dauthi, alice, Zone::Battlefield);
    game.remove_summoning_sickness(dauthi_id);

    let exiled_bears_id = game.create_object_from_definition(
        &crate::cards::definitions::grizzly_bears(),
        bob,
        Zone::Exile,
    );
    game.object_mut(exiled_bears_id)
        .expect("exiled bears should exist")
        .counters
        .insert(CounterType::Void, 1);

    let activated = game
        .object(dauthi_id)
        .expect("Dauthi should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Dauthi should have an activated ability");

    let mut dm = RecordingObjectChoiceDecisionMaker::default();
    let mut ctx = ExecutionContext::new(dauthi_id, alice, &mut dm);
    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Dauthi activation effect should resolve");
    }

    assert_eq!(
        dm.decide_objects_calls, 0,
        "single legal exile target should auto-select without surfacing a choose-objects prompt"
    );

    let actions_after = compute_legal_actions(&game, alice);
    assert!(actions_after.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Exile,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == exiled_bears_id
        )
    }));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dauthi_voidwalker_activation_prompts_for_multiple_void_counter_cards_only() {
    use crate::ability::AbilityKind;
    use crate::alternative_cast::CastingMethod;
    use crate::decision::LegalAction;
    use crate::decision::compute_legal_actions;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let dauthi = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Test")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
            )
            .expect("Dauthi text should parse");
    let dauthi_id = game.create_object_from_definition(&dauthi, alice, Zone::Battlefield);
    game.remove_summoning_sickness(dauthi_id);

    let exiled_bears_id = game.create_object_from_definition(
        &crate::cards::definitions::grizzly_bears(),
        bob,
        Zone::Exile,
    );
    let exiled_bolt_id = game.create_object_from_definition(
        &crate::cards::definitions::lightning_bolt(),
        bob,
        Zone::Exile,
    );
    let exiled_without_counter_id = game.create_object_from_definition(
        &crate::cards::definitions::grizzly_bears(),
        bob,
        Zone::Exile,
    );
    for object_id in [exiled_bears_id, exiled_bolt_id] {
        game.object_mut(object_id)
            .expect("exiled card should exist")
            .counters
            .insert(CounterType::Void, 1);
    }

    let activated = game
        .object(dauthi_id)
        .expect("Dauthi should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Dauthi should have an activated ability");

    let mut dm = RecordingObjectChoiceDecisionMaker {
        preferred_choice: Some(exiled_bolt_id),
        ..Default::default()
    };
    let mut ctx = ExecutionContext::new(dauthi_id, alice, &mut dm);
    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Dauthi activation effect should resolve");
    }

    assert_eq!(
        dm.decide_objects_calls, 1,
        "multiple legal exile targets should surface a choose-objects prompt once"
    );
    assert!(
        dm.legal_candidates.contains(&exiled_bears_id),
        "void-counter Grizzly Bears should be a legal Dauthi choice"
    );
    assert!(
        dm.legal_candidates.contains(&exiled_bolt_id),
        "void-counter Lightning Bolt should be a legal Dauthi choice"
    );
    assert!(
        !dm.legal_candidates.contains(&exiled_without_counter_id),
        "cards without a void counter should not be legal Dauthi choices"
    );

    let actions_after = compute_legal_actions(&game, alice);
    assert!(actions_after.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Exile,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == exiled_bolt_id
        )
    }));
    assert!(!actions_after.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                ..
            } if *spell_id == exiled_bears_id || *spell_id == exiled_without_counter_id
        )
    }));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dauthi_voidwalker_zero_cost_spell_only_offers_free_exile_cast_action() {
    use crate::ability::AbilityKind;
    use crate::alternative_cast::CastingMethod;
    use crate::decision::LegalAction;
    use crate::decision::compute_legal_actions;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let dauthi = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Test")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
            )
            .expect("Dauthi text should parse");
    let dauthi_id = game.create_object_from_definition(&dauthi, alice, Zone::Battlefield);
    game.remove_summoning_sickness(dauthi_id);

    let ornithopter_id = game.create_object_from_definition(
        &crate::cards::definitions::ornithopter(),
        bob,
        Zone::Exile,
    );
    game.object_mut(ornithopter_id)
        .expect("exiled Ornithopter should exist")
        .counters
        .insert(CounterType::Void, 1);

    let activated = game
        .object(dauthi_id)
        .expect("Dauthi should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Dauthi should have an activated ability");

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(dauthi_id, alice, &mut dm);
    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Dauthi activation effect should resolve");
    }

    let ornithopter_casts: Vec<_> = compute_legal_actions(&game, alice)
        .into_iter()
        .filter(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    ..
                } if *spell_id == ornithopter_id
            )
        })
        .collect();

    assert_eq!(
        ornithopter_casts.len(),
        1,
        "Dauthi should expose exactly one exile-cast action for Ornithopter, got {ornithopter_casts:?}"
    );
    assert!(
        matches!(
            &ornithopter_casts[0],
            LegalAction::CastSpell {
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Exile,
                    use_alternative: Some(_),
                    ..
                },
                ..
            }
        ),
        "Dauthi should only offer the free cast method for Ornithopter, got {ornithopter_casts:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dauthi_voidwalker_casted_permanent_from_exile_enters_under_casters_control() {
    use crate::ability::AbilityKind;
    use crate::decision::LegalAction;
    use crate::decision::compute_legal_actions;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let dauthi = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Test")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
            )
            .expect("Dauthi text should parse");
    let dauthi_id = game.create_object_from_definition(&dauthi, alice, Zone::Battlefield);
    game.remove_summoning_sickness(dauthi_id);

    let ornithopter_id = game.create_object_from_definition(
        &crate::cards::definitions::ornithopter(),
        bob,
        Zone::Exile,
    );
    game.object_mut(ornithopter_id)
        .expect("exiled Ornithopter should exist")
        .counters
        .insert(CounterType::Void, 1);

    let activated = game
        .object(dauthi_id)
        .expect("Dauthi should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Dauthi should have an activated ability");

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(dauthi_id, alice, &mut dm);
    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Dauthi activation effect should resolve");
    }

    let cast_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    ..
                } if *spell_id == ornithopter_id
            )
        })
        .expect("Dauthi should grant a cast action for Ornithopter");

    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(cast_action.clone()),
        &mut dm,
    )
    .expect("free exile cast should succeed");

    let stack_entry = game
        .stack
        .last()
        .expect("cast Ornithopter should be on the stack");
    assert_eq!(stack_entry.controller, alice);
    assert_eq!(
        game.current_controller(stack_entry.object_id),
        Some(alice),
        "spell on the stack should be controlled by the caster"
    );

    crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm)
        .expect("casted Ornithopter should resolve onto the battlefield");

    let resolved_ornithopter = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .find(|obj| obj.name == "Ornithopter" && obj.owner == bob)
        .expect("resolved Ornithopter should be on the battlefield");
    assert_eq!(
        game.controller_of(resolved_ornithopter),
        alice,
        "a permanent cast through Dauthi should enter under the caster's control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_cant_gain_life_until_eot_from_text() {
    let text = "Until end of turn, players can't gain life.";
    let def = CardDefinitionBuilder::new(CardId::new(), "No Life")
        .parse_text(text)
        .expect("parse cant gain life");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<CantEffect>().is_some()),
        "should include cant effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_source_cant_be_blocked_until_eot_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Horizons Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{2}{U}: This creature can't be blocked this turn.")
        .expect("source cant-be-blocked clause should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let cant = activated
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CantEffect>())
        .expect("expected cant effect");
    assert_eq!(cant.duration, crate::effect::Until::EndOfTurn);
    match &cant.restriction {
        crate::effect::Restriction::BeBlocked(filter) => {
            assert!(
                filter.source,
                "expected source-bound restriction filter, got {filter:?}"
            );
        }
        other => panic!("expected be-blocked restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_cant_be_regenerated_this_turn_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Furnace Brood Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{B}: Target creature can't be regenerated this turn.")
        .expect("target cant-be-regenerated clause should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let cant = activated
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CantEffect>())
        .expect("expected cant effect");
    assert_eq!(cant.duration, crate::effect::Until::EndOfTurn);
    match &cant.restriction {
        crate::effect::Restriction::BeRegenerated(filter) => {
            assert!(
                !filter.tagged_constraints.is_empty(),
                "expected target-bound regeneration restriction filter, got {filter:?}"
            );
        }
        other => panic!("expected be-regenerated restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_source_doesnt_untap_during_next_untap_step_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cloudcrest Lake Variant")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}: Add {W}.\n{T}: Add {U}. This land doesn't untap during your next untap step.",
        )
        .expect("next-untap-step negated untap clause should parse");

    let abilities: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .collect();
    assert!(abilities.len() >= 2, "expected two mana abilities");

    let slow_mana = abilities
        .iter()
        .find(|a| {
            a.effects
                .iter()
                .any(|effect| effect.downcast_ref::<CantEffect>().is_some())
        })
        .expect("expected mana ability with untap restriction");

    let effects = &slow_mana.effects;
    let cant = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CantEffect>())
        .expect("expected untap restriction effect");
    assert_eq!(
        cant.duration,
        crate::effect::Until::ControllersNextUntapStep
    );
    match &cant.restriction {
        crate::effect::Restriction::Untap(filter) => {
            assert!(
                filter.source,
                "expected source-bound untap restriction filter, got {filter:?}"
            );
        }
        other => panic!("expected untap restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_kefnets_last_word_uses_next_untap_step_duration() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kefnet's Last Word Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Gain control of target artifact, creature, or enchantment. Lands you control don't untap during your next untap step.",
            )
            .expect("kefnet untap-skip clause should parse");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let cant = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CantEffect>())
        .expect("expected untap restriction");
    assert_eq!(
        cant.duration,
        crate::effect::Until::ControllersNextUntapStep
    );
    match &cant.restriction {
        crate::effect::Restriction::Untap(filter) => {
            assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
            assert!(filter.card_types.contains(&CardType::Land));
        }
        other => panic!("expected untap restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_targets_dont_untap_during_controller_next_untap_step_uses_controller_duration() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Frost Breath Variant")
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Tap up to two target creatures. Those creatures don't untap during their controller's next untap step.",
            )
            .expect("controller-next-untap-step tap clause should parse");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let cant = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CantEffect>())
        .expect("expected untap restriction");
    assert_eq!(
        cant.duration,
        crate::effect::Until::ControllersNextUntapStep
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_creature_dies_return_under_your_control_uses_move_to_zone() {
    let def = CardDefinitionBuilder::new(CardId::new(), "False Demise Variant")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(
                "Enchant creature\nWhen enchanted creature dies, return that card to the battlefield under your control.",
            )
            .expect("false-demise style trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("MoveToZoneEffect"),
        "expected move-to-zone return effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("battlefield_controller: You"),
        "expected under-your-control return semantics, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "expected compile to avoid target-only graveyard return helper, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_return_to_hand_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Unsummon")
        .parse_text("Return target creature to its owner's hand.")
        .expect("parse return to hand");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effect"));
    assert!(
        debug.contains("ReturnToHandEffect") || debug.contains("MoveToZoneEffect"),
        "should include return-to-hand semantics, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_tap_one_or_two_targets_preserves_choice_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Probe Tap Two")
        .parse_text("Tap one or two target creatures.")
        .expect("parse tap one-or-two targets");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effect"));
    assert!(
        debug.contains("TapEffect"),
        "should include tap effect, got {debug}"
    );
    assert!(
        debug.contains("min: 1") && debug.contains("max: Some(2)"),
        "expected one-or-two choice count in parsed tap effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_tap_all_spirits_compiles_as_non_targeted_all() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Probe Tap All Spirits")
        .parse_text("Tap all Spirits.")
        .expect("parse tap-all clause");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let tap = effects
        .iter()
        .find_map(|e| e.downcast_ref::<TapEffect>())
        .expect("should include tap effect");
    let ChooseSpec::All(filter) = &tap.target else {
        panic!("expected non-targeted tap-all spec, got {:?}", tap.target);
    };
    assert!(
        filter.subtypes.contains(&Subtype::Spirit),
        "expected Spirit subtype filter, got {filter:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exile_any_number_of_target_spells_preserves_choice_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Probe Exile Any")
        .parse_text("Exile any number of target spells.")
        .expect("parse exile any-number targets");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("min: 0") && debug.contains("max: None"),
        "expected any-number target count in runtime effect, got {debug}"
    );

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("any number of target spell"),
        "expected rendered any-number target text, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_return_to_battlefield_from_graveyard_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reanimate Variant")
        .parse_text("Return target creature card from your graveyard to the battlefield tapped.")
        .expect("parse return to battlefield");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        effects.iter().any(|e| e
            .downcast_ref::<ReturnFromGraveyardToBattlefieldEffect>()
            .is_some())
            || debug.contains("ReturnFromGraveyardToBattlefieldEffect")
            || debug.contains("MoveToZoneEffect"),
        "should include return-to-battlefield effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_return_all_from_graveyards_to_battlefield_tapped_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Planar Birth Variant")
            .parse_text(
                "Return all basic land cards from all graveyards to the battlefield tapped under their owners' control.",
            )
            .expect("parse return all cards to battlefield");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ReturnAllToBattlefieldEffect") || debug.contains("MoveToZoneEffect"),
        "should include return-all-to-battlefield effect, got {debug}"
    );
    assert!(
        debug.contains("tapped"),
        "expected tapped return-all effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_control_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Switcheroo")
        .parse_text("Exchange control of two target creatures.")
        .expect("parse exchange control");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<ExchangeControlEffect>().is_some())
            || debug.contains("ExchangeControlEffect"),
        "should include exchange control effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_cultural_exchange_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cultural Exchange")
            .parse_text(
                "Choose any number of creatures target player controls. Choose the same number of creatures another target player controls. Those players exchange control of those creatures.",
            )
            .expect("parse cultural exchange");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("TargetOnlyEffect")
            && debug.contains("ChoosePlayerEffect")
            && debug.contains("ForEachTaggedPlayerEffect")
            && debug.contains("ChangeControllerToPlayer(IteratedPlayer)")
            && debug.contains("count_value: Some("),
        "expected grouped player-choice and control-swap shape, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Choose any number of creatures target player controls.")
            && rendered
                .contains("Choose the same number of creatures another target player controls.")
            && rendered.contains("Those players exchange control of those creatures."),
        "expected oracle-like compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_gruesome_menagerie_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gruesome Menagerie")
            .parse_text(
                "Choose a creature card with mana value 1 in your graveyard, then do the same for creature cards with mana value 2 and 3. Return those cards to the battlefield.",
            )
            .expect("parse gruesome menagerie");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.matches("ChooseObjectsEffect").count() == 3
            && debug.contains("mana_value: Some(Equal(1))")
            && debug.contains("mana_value: Some(Equal(2))")
            && debug.contains("mana_value: Some(Equal(3))")
            && debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "expected three ordered graveyard choices and a shared return, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Choose a creature card with mana value 1 in your graveyard, then do the same for creature cards with mana value 2 and 3. Return those cards to the battlefield.",
        "the authored compact choice surface should survive lowering"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_life_totals_with_target_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Magus Variant")
        .parse_text("Exchange life totals with target opponent.")
        .expect("parse exchange life totals with target");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ExchangeLifeTotalsEffect"),
        "should include exchange life totals effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_life_totals_between_two_targets_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Soul Conduit")
        .parse_text("Two target players exchange life totals.")
        .expect("parse two-player life exchange");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ExchangeLifeTotalsEffect"),
        "should include exchange life totals effect, got {debug}"
    );
    assert!(
        debug.contains("min: 2") || debug.contains("exactly(2)"),
        "expected two-player target selection, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_text_boxes_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exchange of Words")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "When this enchantment enters, choose two target creatures. For as long as this enchantment remains on the battlefield, exchange the text boxes of those creatures.",
            )
            .expect("parse exchange text boxes");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ExchangeTextBoxesEffect"),
        "should include exchange text boxes effect, got {debug}"
    );
    assert!(
        debug.contains("exactly(2)") || debug.contains("min: 2"),
        "expected two-creature exchange selection, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("unsupported effect"),
        "expected compiled text to render exchange text boxes, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_control_heterogeneous_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Avarice Totem")
        .parse_text("Exchange control of this artifact and target nonland permanent.")
        .expect("parse heterogeneous exchange control");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<ExchangeControlEffect>().is_some())
            || debug.contains("ExchangeControlEffect"),
        "should include exchange control effect, got {debug}"
    );
    assert!(
        (debug.contains("permanent1: Source")
            || (debug.contains("permanent1: SurfaceHinted") && debug.contains("spec: Source")))
            && debug.contains("excluded_card_types: [Land]"),
        "expected source-plus-target exchange, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_control_heterogeneous_with_relative_constraint_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Daring Thief")
            .parse_text(
                "Exchange control of target nonland permanent you control and target permanent an opponent controls that shares a card type with it.",
            )
            .expect("parse heterogeneous exchange control with relative constraint");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ExchangeControlEffect") && debug.contains("shared_type: Some(CardType)"),
        "expected shared card-type exchange control effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_hand_and_graveyard_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Harness Infinity")
        .parse_text("Exchange your hand and graveyard.")
        .expect("parse zone exchange");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<ExchangeZonesEffect>().is_some())
            || debug.contains("ExchangeZonesEffect"),
        "should include exchange zones effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_hand_and_library_then_shuffle_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mordenkainen Variant")
        .parse_text("Exchange your hand and library, then shuffle.")
        .expect("parse zone exchange followed by shuffle");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ExchangeZonesEffect") && debug.contains("ShuffleLibraryEffect"),
        "expected exchange zones plus shuffle, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_life_total_with_toughness_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tree of Redemption")
        .parse_text("Exchange your life total with this creature's toughness.")
        .expect("parse life total to toughness exchange");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<ExchangeValuesEffect>().is_some())
            || debug.contains("ExchangeValuesEffect"),
        "should include exchange values effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_named_source_power_exchange_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Evra, Halcyon Witness")
        .card_types(vec![CardType::Creature])
        .parse_text("Lifelink\n{4}: Exchange your life total with Evra's power.")
        .expect("parse named source power exchange");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected exchange activated ability");
    let exchange = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<ExchangeValuesEffect>())
        .expect("ability should lower to ExchangeValuesEffect");

    assert!(matches!(
        exchange.left,
        crate::effects::ExchangeValueOperand::LifeTotal(PlayerFilter::You)
    ));
    assert!(matches!(
        &exchange.right,
        crate::effects::ExchangeValueOperand::Power(target)
            if matches!(target.base(), ChooseSpec::Source)
    ));
    assert_eq!(exchange.duration, crate::effect::Until::Forever);
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Exchange your life total with this creature's power."),
        "life/stat exchange should use the canonical source surface and omit a forever suffix, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_not_your_turn_source_type_identity_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Midnight Mangler")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .parse_text("During turns other than yours, this Vehicle is an artifact creature.\nCrew 2")
        .expect("parse conditioned source type identity");

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("SetCardTypes"), "{debug}");
    assert!(
        debug.contains("Not(") && debug.contains("YourTurn"),
        "{debug}"
    );
    assert!(
        !debug.contains("other: true"),
        "turn-prefix words must not be recovered as an 'other Vehicles' subject: {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !rendered.contains("Other Vehicles are artifact creatures"),
        "conditioned source identity must not render as a global Vehicle rule: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_kicked_source_spell_keyword_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Molten Disaster")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Kicker {R}\nIf this spell was kicked, it has split second.\nMolten Disaster deals X damage to each creature without flying and each player.",
        )
        .expect("parse kicked source spell keyword");

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("SplitSecond"), "{debug}");
    assert!(debug.contains("ThisSpellWasKicked"), "{debug}");
    assert!(
        !debug.contains("other: true"),
        "if-prefix words must not be recovered as a spell filter: {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !rendered.contains("Spells have split second"),
        "conditional source keyword must not render as a global grant: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_contracted_source_animation_with_keyword_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Idol of False Gods")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Eldrazi])
        .parse_text(
            "As long as this artifact has eight or more +1/+1 counters on it, it's a 0/0 creature in addition to its other types and it has annihilator 2.",
        )
        .expect("parse contracted conditioned source animation");

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("AddCardTypes"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
    assert!(debug.contains("Annihilator"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    assert!(
        !debug.contains("other: true"),
        "contracted source pronoun must not become an 'other' filter: {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !rendered.contains("other creatures with power and toughness 0/0"),
        "source animation must not render as a global creature grant: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_copy_exception_stays_in_trigger_resolution() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Vesuvan Drifter")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of each combat, you may reveal the top card of your library. If you reveal a creature card this way, this creature becomes a copy of that card until end of turn, except it has flying.",
        )
        .expect("parse conditional copy exception trigger");

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("CopyOf"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !rendered.contains("All cards have flying"),
        "copy exception must not split into a global static tail: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_carried_copy_exception_stays_in_trigger_resolution() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kaya, Spirits' Justice")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "Whenever one or more creatures you control and/or creature cards in your graveyard are put into exile, you may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has flying.",
        )
        .expect("parse carried copy exception trigger");

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("CopyOf"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !rendered.contains("All permanents have flying"),
        "copy exception must not split into a global static tail: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_power_with_target_power_until_end_of_combat_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Serene Master")
            .parse_text("Exchange its power and the power of target creature it's blocking until end of combat.")
            .expect("parse power exchange until end of combat");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<ExchangeValuesEffect>().is_some())
            || debug.contains("ExchangeValuesEffect"),
        "should include exchange values effect, got {debug}"
    );
    assert!(
        debug.contains("EndOfCombat"),
        "expected end-of-combat duration, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Exchange this creature's power and the power of target blocking creature until end of combat."
        ),
        "stat/stat exchange should retain 'and' plus a spaced finite duration, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_draw_for_each_tapped_creature_target_opponent_controls() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Borrowing Arrows Variant")
        .parse_text("Draw a card for each tapped creature target opponent controls.")
        .expect("draw-for-each clause should parse");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let draw = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<DrawCardsEffect>())
        .expect("expected draw cards effect");
    match draw.count.unhinted() {
        Value::Count(filter) => {
            assert!(
                filter.card_types.contains(&CardType::Creature),
                "expected creature filter, got {:?}",
                filter.card_types
            );
            assert!(filter.tapped, "expected tapped filter");
            assert!(
                filter.controller.is_some(),
                "expected controlled-by-opponent filter"
            );
        }
        other => panic!("expected count-based draw, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_draw_with_unsupported_tail_errors() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Bad Draw Tail")
        .parse_text("Draw a card whenever this is weird.");
    assert!(
        result.is_err(),
        "unknown draw tail should fail instead of silently compiling fixed draw"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_counter_spell_with_graveyard_reference_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Drown in the Loch Variant")
            .parse_text(
                "Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard.",
            )
            .expect("dynamic graveyard comparison in counter target should parse");
    let message = format!("{:#?}", def.spell_effect);
    assert!(
        message.contains("LessThanOrEqualExpr")
            && message.contains("Count")
            && message.contains("Graveyard"),
        "expected dynamic graveyard count comparison in counter target, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_creature_has_base_power_toughness_as_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Illusory Wrappings Variant")
        .parse_text("Enchant creature\nEnchanted creature has base power and toughness 0/2.")
        .expect("base power/toughness Aura line should parse as static ability");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids
            .contains(&crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected static SetBasePowerToughnessForFilter, got {static_ids:?}"
    );

    let spell_has_set_base = def.spell_effect.as_ref().is_some_and(|effects| {
        effects.iter().any(|effect| {
            effect
                .downcast_ref::<SetBasePowerToughnessEffect>()
                .is_some()
        })
    });
    assert!(
        !spell_has_set_base,
        "base P/T for Aura text should not be a spell-effect duration modification"
    );

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let has_base_line = lines
        .iter()
        .find(|line| line.contains("base power and toughness 0/2"))
        .expect("compiled text should include base P/T static wording");
    assert!(
        !has_base_line.contains("until end of turn"),
        "static base P/T line must not be temporary: {has_base_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_creature_loses_abilities_and_transforms_with_base_pt() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ichthyomorphosis Variant")
            .parse_text(
                "Enchant creature\nEnchanted creature loses all abilities and is a blue Fish with base power and toughness 0/1.",
            )
            .expect("transforming lose-all-abilities Aura line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter),
        "expected lose-all-abilities static, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::SetCardTypes),
        "expected set-card-types static, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::SetCreatureSubtypes),
        "expected creature-subtype reset static, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::SetColors),
        "expected set-colors static, got {static_ids:?}"
    );
    assert!(
        static_ids
            .contains(&crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected set-base-power/toughness static, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_bestow_enchanted_creature_loses_abilities_and_transforms_with_base_pt() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Trickster's Elk Variant")
            .parse_text(
                "Bestow {1}{G}\nEnchanted creature loses all abilities and is a green Elk creature with base power and toughness 3/3.",
            )
            .expect("bestow elk transform line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        matches!(
            def.alternative_casts.as_slice(),
            [AlternativeCastingMethod::Bestow { .. }]
        ),
        "expected bestow alternative cast, got {:?}",
        def.alternative_casts
    );
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter),
        "expected lose-all-abilities static, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::SetCardTypes),
        "expected set-card-types static, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::SetCreatureSubtypes),
        "expected creature-subtype reset static, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::SetColors),
        "expected set-colors static, got {static_ids:?}"
    );
    assert!(
        static_ids
            .contains(&crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected set-base-power/toughness static, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_creature_has_base_power_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wak-Wak Variant")
        .parse_text("Target attacking creature has base power 0 until end of turn.")
        .expect("base-power-only clause should parse");

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("base power 0") && spell_line.contains("until end of turn"),
        "compiled text should include temporary base power wording, got {spell_line}"
    );
    assert!(
        !spell_line.contains("Choose target"),
        "base-power-only clause should compile to an effect, not target-only text: {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exile_target_nonland_not_exactly_two_colors_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ravnica Variant")
        .parse_text(
            "Exile target nonland permanent an opponent controls that isn't exactly two colors.",
        )
        .expect("exile target not-exactly-two-colors clause should parse");

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("not exactly two colors"),
        "compiled text should preserve exact-two-colors exclusion, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_base_power_toughness_with_unknown_tail_errors() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Bad Base PT Tail")
        .parse_text("Target creature has base power and toughness 1/1 while enchanted.");
    assert!(
        result.is_err(),
        "unsupported base P/T tail should fail instead of partial target-only parse"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_search_to_battlefield_tapped_preserves_tapped_flag() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Roiling Regrowth Variant")
            .parse_text(
                "Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
            )
            .expect("parse tapped battlefield search");

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("onto the battlefield tapped"),
        "expected tapped battlefield placement in compiled text, got {spell_line}"
    );
    assert!(
        spell_line.contains("Search your library"),
        "expected compact search wording, got {spell_line}"
    );
    assert!(
        !spell_line.contains("chooses up to"),
        "should not leak choose-object internals in search display: {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_double_counters_on_source_preserves_singular_typed_surface() {
    for (card_type, text, expected_surface) in [
        (
            CardType::Creature,
            "{1}: Double the number of +1/+1 counters on this creature.",
            "this creature",
        ),
        (
            CardType::Enchantment,
            "{1}: Double the number of growth counters on this enchantment.",
            "this enchantment",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), "Double Source Variant")
            .card_types(vec![card_type])
            .parse_text(text)
            .expect("parse singular source double-counters clause");
        let AbilityKind::Activated(activated) = &def.abilities[0].kind else {
            panic!("expected activated double-counters ability");
        };
        let double = activated
            .effects
            .flattened_default_effects()
            .iter()
            .find_map(|effect| effect.downcast_ref::<DoubleCountersEffect>())
            .expect("ability should lower to DoubleCountersEffect");

        assert!(matches!(double.target.base(), ChooseSpec::Source));
        assert_eq!(
            double
                .target
                .source_reference_surface()
                .map(|surface| surface.display_text()),
            Some(expected_surface.to_string())
        );
        let rendered = unprocessed_compiled_lines(&def).join(" ");
        assert!(
            rendered.contains(&format!("counters on {expected_surface}")),
            "expected singular source wording, got {rendered}"
        );
        assert!(
            !rendered.contains("on each this"),
            "singular source widened to a filter-wide target: {rendered}"
        );
    }

    let def = CardDefinitionBuilder::new(CardId::new(), "Double It Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever this creature attacks, double the number of +1/+1 counters on it.")
        .expect("parse pronoun source double-counters clause");
    let AbilityKind::Triggered(triggered) = &def.abilities[0].kind else {
        panic!("expected triggered double-counters ability");
    };
    let double = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<DoubleCountersEffect>())
        .expect("ability should lower to DoubleCountersEffect");
    assert!(matches!(double.target.base(), ChooseSpec::Source));
    assert_eq!(
        double
            .target
            .source_reference_surface()
            .map(|surface| surface.display_text()),
        Some("it".to_string())
    );
    assert!(
        unprocessed_compiled_lines(&def)
            .join(" ")
            .contains("counters on it")
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_double_counters_on_each_creature_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kalonian Hydra Variant")
            .parse_text(
                "Whenever this creature attacks, double the number of +1/+1 counters on each creature you control.",
            )
            .expect("parse kalonian hydra attack trigger");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("should have triggered ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected triggered ability");
    };
    let double = triggered
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<DoubleCountersEffect>())
        .expect("triggered ability should compile through DoubleCountersEffect");

    assert_eq!(double.counter_type, Some(CounterType::PlusOnePlusOne));
    let ChooseSpec::All(filter) = &double.target else {
        panic!(
            "expected non-targeted all-creatures spec, got {:?}",
            double.target
        );
    };
    assert!(filter.card_types.contains(&CardType::Creature));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("on each creature you control"),
        "filter-wide double effect lost plural quantification: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_remove_typed_counter_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Power Conduit Variant")
        .parse_text("Remove a +1/+1 counter from target creature.")
        .expect("parse typed counter removal");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects.iter().any(|e| {
            e.downcast_ref::<RemoveCountersEffect>().is_some()
                || format!("{e:?}").contains("RemoveCountersEffect")
                || e.downcast_ref::<RemoveUpToAnyCountersEffect>().is_some()
                || format!("{e:?}").contains("RemoveUpToAnyCountersEffect")
        }),
        "should include remove counters effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_remove_typed_counter_from_text_for_each_card() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Descendant of Masumaro Variant")
        .parse_text(
            "Remove a +1/+1 counter from this creature for each card in target opponent's hand.",
        )
        .expect("parse typed counter removal for each");

    let effects = def.spell_effect.expect("spell effect");
    let for_each = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ForEachObject>())
        .expect("typed counter removal should use for-each wrapper");
    let has_remove_inner = for_each.effects.iter().any(|effect| {
        effect.downcast_ref::<RemoveCountersEffect>().is_some()
            || format!("{effect:?}").contains("RemoveCountersEffect")
            || effect
                .downcast_ref::<RemoveUpToAnyCountersEffect>()
                .is_some()
            || format!("{effect:?}").contains("RemoveUpToAnyCountersEffect")
    });
    assert!(
        has_remove_inner,
        "for-each wrapper should include remove-counters inner effect: {:?}",
        for_each.effects
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_create_token_copy_of_target_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Copy Variant")
        .parse_text("Create a token that's a copy of target artifact.")
        .expect("parse copy token from target");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects.iter().any(|e| {
            e.downcast_ref::<CreateTokenCopyEffect>().is_some()
                || format!("{e:?}").contains("CreateTokenCopyEffect")
        }),
        "should include create-token-copy effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_create_token_copy_keeps_serial_target_type_union() {
    let text = "Create a token that's a copy of target artifact, creature, or land.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Serial Copy Target Variant")
        .parse_text(text)
        .expect("serial copy target should parse");
    let effects = def.spell_effect.as_ref().expect("spell effect");
    let create = effects
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<CreateTokenCopyEffect>().or_else(|| {
                effect
                    .downcast_ref::<TaggedEffect>()
                    .and_then(|tagged| tagged.effect.downcast_ref::<CreateTokenCopyEffect>())
            })
        })
        .unwrap_or_else(|| panic!("expected a typed token copy effect: {effects:#?}"));
    let ChooseSpec::Target(target) = &create.target else {
        panic!("expected a targeted copy source: {:?}", create.target);
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("expected an object target union: {:?}", create.target);
    };
    assert_eq!(
        filter.card_types,
        [CardType::Artifact, CardType::Creature, CardType::Land]
    );
    assert_eq!(crate::compiled_text::compiled_text_lines(&def), vec![text]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_dino_dna_style_copy_modifier_with_trample() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dino DNA Variant")
            .parse_text("Create a token that's a copy of target creature card exiled with this artifact, except it's a 6/6 green Dinosaur creature with trample.")
            .expect("parse dino dna copy clause");

    let effects = def.spell_effect.expect("spell effect");
    let debug = format!("{:#?}", effects);
    assert!(
        debug.contains("CreateTokenCopyEffect"),
        "should include create-token-copy effect: {debug}"
    );
    assert!(
        debug.contains("set_base_power_toughness: Some(") && debug.contains("6,"),
        "expected 6/6 override in copy effect, got {debug}"
    );
    assert!(
        debug.contains("set_colors: Some(") && debug.contains("ColorSet("),
        "expected green color override in copy effect, got {debug}"
    );
    assert!(
        debug.contains("set_card_types: Some(") && debug.contains("Creature"),
        "expected creature card type override in copy effect, got {debug}"
    );
    assert!(
        debug.contains("set_subtypes: Some(") && debug.contains("Dinosaur"),
        "expected Dinosaur subtype override in copy effect, got {debug}"
    );
    assert!(
        debug.contains("Trample"),
        "expected copy effect to grant trample, got {debug}"
    );
    assert!(
        debug.contains("card_types:")
            && debug.contains("Creature")
            && debug.contains("zone: Some(")
            && debug.contains("Exile"),
        "expected creature target filter on copied source, got {debug}"
    );
    assert!(
        !debug.contains("set_card_types: Some([Artifact])")
            && !debug.contains("all_card_types: [Artifact]"),
        "source artifact reference should not become artifact target/type override: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_saw_in_half_style_half_pt_copy_does_not_set_type_override() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Saw in Half Variant")
            .parse_text("Create two tokens that are copies of target creature, except their power is half that creature's power and their toughness is half that creature's toughness. Round up each time.")
            .expect("parse saw in half copy clause");

    let effects = def.spell_effect.expect("spell effect");
    let debug = format!("{:#?}", effects);
    assert!(
        debug.contains("CreateTokenCopyEffect"),
        "should include create-token-copy effect: {debug}"
    );
    assert!(
        debug.contains("set_card_types: None"),
        "half power/toughness wording should not imply a type override: {debug}"
    );
    assert!(
        debug.contains("set_subtypes: None"),
        "half power/toughness wording should not imply a subtype override: {debug}"
    );
    assert!(
        debug.contains("set_colors: None"),
        "half power/toughness wording should not imply a color override: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_shaleskin_bruiser_style_scaling_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shaleskin Bruiser Variant")
            .parse_text(
                "Trample\nWhenever this creature attacks, it gets +3/+0 until end of turn for each other attacking Beast.",
            )
            .expect("parse shaleskin bruiser style text");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let modify = triggered
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ModifyPowerToughnessForEachEffect>())
        .expect("trigger should include ModifyPowerToughnessForEachEffect");
    assert_eq!(modify.power_per, 3);
    assert_eq!(modify.toughness_per, 0);
    let Value::Count(filter) = modify.count.unhinted() else {
        panic!("expected count-based scaling");
    };
    assert!(filter.other, "filter should require other permanents");
    assert!(
        filter.attacking,
        "filter should require attacking permanents"
    );
    assert!(
        filter.subtypes.contains(&Subtype::Beast),
        "filter should require Beast subtype"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn compiled_text_cleans_duplicate_target_mentions() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Torch Fiend Variant")
        .parse_text("{R}, Sacrifice this creature: Destroy target artifact.")
        .expect("parse torch fiend style text");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Destroy target artifact"),
        "compiled text should include destroy target artifact: {joined}"
    );
    assert!(
        !joined.contains("target target"),
        "compiled text should not duplicate target wording: {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_adamant_mana_spent_conditional_compiles_semantically() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Turn into a Pumpkin Variant")
            .parse_text(
                "Return target nonland permanent to its owner's hand. Draw a card.\nAdamant — If at least three blue mana was spent to cast this spell, create a Food token.",
            )
            .expect("adamant spent-to-cast condition should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ManaSpentToCastThisSpellAtLeast") && debug.contains("CreateTokenEffect"),
        "expected adamant condition and token creation in lowered definition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_adamant_mana_spent_conditional_rejects_unparsed_tail() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Broken Adamant Variant")
            .parse_text(
                "Adamant — If at least three blue mana was spent to cast this spell while you control a creature, create a Food token.",
            );
    assert!(
        result.is_err(),
        "unsupported predicate tail should fail parse instead of partial success"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_no_spells_cast_last_turn_conditional_predicate() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Werewolf Transform Variant")
            .parse_text(
                "At the beginning of each upkeep, if no spells were cast last turn, transform this creature.",
            )
            .expect("no-spells-last-turn predicate should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("if no spells were cast last turn"),
        "expected no-spells predicate wording in parsed output, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_daybound_keyword_line_builds_static_keyword() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Daybound Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Daybound")
        .expect("daybound keyword line should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::Daybound
            )
        }),
        "expected daybound to lower into the static daybound keyword, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText")
            && !debug.contains("StaticAbilityId::KeywordFallbackText")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "daybound should not compile via placeholder/marker ability ids: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_ticket_power_toughness_sticker_marker_line_uses_keyword_marker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ticket Sticker Variant")
        .parse_text("{TK}{TK} — 3/3\n{TK}{TK}{TK} — 6/2")
        .expect("ticket sticker p/t lines should parse as keyword markers");

    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("keywordmarker") && !debug.contains("keywordfallbacktext"),
        "expected ticket p/t sticker lines to avoid keyword fallback text, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_ticket_labeled_trigger_does_not_repeat_chosen_option_condition_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ticket Trigger Variant")
        .parse_text(
            "{TK}{TK} — When this permanent leaves the battlefield, create two Food tokens.",
        )
        .expect("ticket-labeled trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("{tk}{tk} — when this permanent leaves the battlefield")
            && !rendered.contains("chosen option is"),
        "expected labeled trigger text without redundant chosen-option clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn daybound_runtime_transforms_source_for_day_night_designation() {
    use crate::ids::PlayerId;

    crate::cards::clear_runtime_custom_cards();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut back_def =
        CardDefinitionBuilder::new(CardId::from_raw(70141), "Nightbound Runtime Probe")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Werewolf])
            .power_toughness(PowerToughness::fixed(4, 4))
            .parse_text("Nightbound\nTrample")
            .expect("night face should parse");
    back_def.card.other_face = Some(CardId::from_raw(70140));
    back_def.card.other_face_name = Some("Daybound Runtime Probe".to_string());
    back_def.card.linked_face_layout = LinkedFaceLayout::TransformLike;
    crate::cards::register_runtime_custom_card(back_def);
    let mut source_def =
        CardDefinitionBuilder::new(CardId::from_raw(70140), "Daybound Runtime Probe")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human, Subtype::Werewolf])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("Daybound")
            .expect("day face should parse");
    source_def.card.other_face = Some(CardId::from_raw(70141));
    source_def.card.other_face_name = Some("Nightbound Runtime Probe".to_string());
    source_def.card.linked_face_layout = LinkedFaceLayout::TransformLike;
    crate::cards::register_runtime_custom_card(source_def.clone());
    let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);

    assert!(
        game.has_day_night(),
        "daybound entering should start day/night"
    );
    assert!(game.is_daytime(), "daybound should start the game at day");
    assert_eq!(
        game.object(source)
            .expect("daybound source should exist")
            .name,
        "Daybound Runtime Probe"
    );

    game.set_daytime(false);
    assert!(
        !game.is_face_down(source),
        "daybound runtime should transform the source to a visible night face"
    );
    assert_eq!(
        game.object(source)
            .expect("transformed source should exist")
            .name,
        "Nightbound Runtime Probe"
    );

    game.set_daytime(true);
    assert!(
        !game.is_face_down(source),
        "nightbound runtime should transform the source back to a visible day face"
    );
    assert_eq!(
        game.object(source)
            .expect("returned source should exist")
            .name,
        "Daybound Runtime Probe"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_nightbound_keyword_line_builds_static_keyword() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nightbound Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Nightbound")
        .expect("nightbound keyword line should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::Nightbound
            )
        }) && !debug.contains("StaticAbilityId::KeywordMarker"),
        "expected nightbound to lower into the static nightbound keyword, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn create_token_render_preserves_cant_attack_or_block_alone_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Toby Token Variant")
            .parse_text(
                "When this creature enters, create a 4/4 white Beast creature token with \"This token can't attack or block alone.\"",
            )
            .expect("token attack-or-block-alone text should parse");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("can't attack or block alone"),
        "compiled token text should preserve attack/block-alone restriction, got: {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_ring_tempts_compiles_and_renders() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ring Variant")
        .parse_text("The Ring tempts you.")
        .expect("ring tempts clause should parse");

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("The Ring tempts you")),
        "expected rendered text to contain ring clause, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn from_text_with_metadata_no_longer_falls_back_on_parse_failure() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Fallback Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .from_text_with_metadata("This line should not parse and used to fallback silently.");
    assert!(
        result.is_err(),
        "metadata parse should return an error instead of silent oracle-only fallback"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn repeated_parse_text_reuses_cached_definition() {
    let builder = CardDefinitionBuilder::new(CardId::from_raw(910_001), "Cache Probe")
        .card_types(vec![CardType::Creature]);
    let cache_key = builder.parse_cache_key("Flying", false);

    let first = builder
        .clone()
        .parse_text("Flying")
        .expect("first parse should succeed");
    assert!(
        lookup_cached_parse(&cache_key).is_some(),
        "first parse should populate the exact cache entry"
    );

    let second = builder
        .parse_text("Flying")
        .expect("second parse should also succeed");
    assert!(
        lookup_cached_parse(&cache_key).is_some(),
        "cached entry should remain present"
    );
    assert_eq!(
        format!("{first:?}"),
        format!("{second:?}"),
        "cached parse should preserve the compiled definition"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_cache_separates_allow_unsupported_mode() {
    let builder = CardDefinitionBuilder::new(CardId::from_raw(910_002), "Cache Unsupported");
    let text = "This line should not parse and must stay unsupported.";
    let strict_key = builder.parse_cache_key(text, false);
    let permissive_key = builder.parse_cache_key(text, true);

    let strict = builder.clone().parse_text(text);
    assert!(
        strict.is_err(),
        "strict parse should fail for unsupported text"
    );
    assert!(
        lookup_cached_parse(&strict_key).is_some(),
        "strict parse result should populate its own cache entry"
    );
    assert!(
        lookup_cached_parse(&permissive_key).is_none(),
        "strict parse should not alias the permissive cache entry"
    );

    let permissive = builder.clone().parse_text_allow_unsupported(text);
    assert!(
        permissive.is_err(),
        "allow_unsupported should also fail when it would emit a placeholder"
    );
    assert!(
        lookup_cached_parse(&strict_key).is_some(),
        "strict parse cache entry should remain intact"
    );
    assert!(
        lookup_cached_parse(&permissive_key).is_some(),
        "allow_unsupported mode should use a distinct cache entry"
    );

    let permissive_repeat = builder.parse_text_allow_unsupported(text);
    assert!(
        lookup_cached_parse(&permissive_key).is_some(),
        "repeating the permissive parse should preserve its cache entry"
    );
    assert_eq!(
        format!("{permissive:?}"),
        format!("{permissive_repeat:?}"),
        "cached permissive parse should preserve the compiled definition"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn metadata_parse_cache_stays_builder_context_aware() {
    let first_source_builder =
        CardDefinitionBuilder::new(CardId::from_raw(910_003), "Metadata Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2));
    let second_source_builder =
        CardDefinitionBuilder::new(CardId::from_raw(910_003), "Metadata Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3));

    let first_text = first_source_builder.build_text_with_metadata("Flying");
    let second_text = second_source_builder.build_text_with_metadata("Flying");
    let mut first_parse_builder = first_source_builder.clone();
    first_parse_builder.additional_cost = TotalCost::free();
    let mut second_parse_builder = second_source_builder.clone();
    second_parse_builder.additional_cost = TotalCost::free();

    let first_key = first_parse_builder.parse_cache_key(&first_text, false);
    let second_key = second_parse_builder.parse_cache_key(&second_text, false);
    assert_ne!(
        first_key, second_key,
        "different builder metadata must produce distinct cache keys"
    );

    let first = first_source_builder
        .from_text_with_metadata("Flying")
        .expect("first metadata parse should succeed");
    assert!(
        lookup_cached_parse(&first_key).is_some(),
        "first metadata parse should populate its cache entry"
    );
    assert!(
        lookup_cached_parse(&second_key).is_none(),
        "second metadata cache entry should not exist before that parse runs"
    );

    let second = second_source_builder
        .from_text_with_metadata("Flying")
        .expect("second metadata parse should succeed");
    assert!(
        lookup_cached_parse(&first_key).is_some() && lookup_cached_parse(&second_key).is_some(),
        "different builder metadata should populate distinct cache entries"
    );
    assert_ne!(
        first.card.power_toughness, second.card.power_toughness,
        "metadata-aware parses should keep their distinct builder characteristics"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_negated_untap_clause_compiles_to_untap_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ty Lee Variant")
            .parse_text("When this creature enters, tap up to one target creature. It doesn't untap during its controller's untap step for as long as you control this creature.");
    let def = def.expect("Ty Lee-style untap restriction should parse");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("CantEffect"),
        "expected restriction effect, got {debug}"
    );
    assert!(
        debug.contains("Untap("),
        "expected untap restriction, got {debug}"
    );
    assert!(
        debug.contains("YouStopControllingThis"),
        "expected source-control duration, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_ty_lee_named_duration_now_errors_instead_of_partial_compile() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ty Lee")
            .parse_text(
                "When Ty Lee enters, tap up to one target creature. It doesn't untap during its controller's untap step for as long as you control Ty Lee.",
            )
            .expect("Ty Lee named-source duration should parse");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("CantEffect"),
        "expected untap restriction effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enters_tapped_unless_two_or_more_other_lands_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shattered Sanctum Variant")
            .parse_text(
                "Shattered Sanctum enters the battlefield tapped unless you control two or more other lands.\n{T}: Add {W}.",
            )
            .expect("should parse conditional ETB line");

    let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands
            )
        });
    assert!(
        has_conditional_etb,
        "expected conditional ETB static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enters_tapped_unless_two_or_fewer_other_lands_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blackcleave Cliffs Variant")
        .parse_text(
            "This land enters tapped unless you control two or fewer other lands.\n{T}: Add {B}.",
        )
        .expect("should parse fast-land conditional ETB line");

    let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands
            )
        });
    assert!(
        has_conditional_etb,
        "expected two-or-fewer-other-lands ETB static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enters_tapped_unless_two_or_more_basic_lands_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Canopy Vista Variant")
        .parse_text(
            "This land enters tapped unless you control two or more basic lands.\n{T}: Add {G}.",
        )
        .expect("should parse battle-land conditional ETB line");

    let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands
            )
        });
    assert!(
        has_conditional_etb,
        "expected two-or-more-basic-lands ETB static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enters_tapped_unless_any_player_has_13_or_less_life_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Abandoned Campground Variant")
        .parse_text("This land enters tapped unless a player has 13 or less life.\n{T}: Add {W}.")
        .expect("should parse life-threshold conditional ETB line");

    let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife
            )
        });
    assert!(
        has_conditional_etb,
        "expected life-threshold ETB static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enters_tapped_unless_two_or_more_opponents_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Vault of Champions Variant")
        .parse_text("This land enters tapped unless you have two or more opponents.\n{T}: Add {W}.")
        .expect("should parse conditional ETB opponents line");

    let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents
            )
        });
    assert!(
        has_conditional_etb,
        "expected conditional-opponents ETB static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enters_tapped_unless_control_mount_or_vehicle_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Country Roads Variant")
        .parse_text("This land enters tapped unless you control a Mount or Vehicle.\n{T}: Add {W}.")
        .expect("should parse generic conditional ETB line");

    let has_conditional_etb = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::EntersTappedUnlessCondition
        )
    });
    assert!(
        has_conditional_etb,
        "expected generic enters-tapped-unless static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_opponents_control_enter_tapped_preserves_controller_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Frozen Aether Variant")
        .parse_text(
            "Artifacts, creatures, and lands your opponents control enter the battlefield tapped.",
        )
        .expect("should parse opponents-control enters tapped line");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("opponent"),
        "expected rendered line to preserve opponents controller filter, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_played_by_your_opponents_enter_tapped_preserves_controller_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Uphill Battle Variant")
        .parse_text("Creatures played by your opponents enter tapped.")
        .expect("should parse played-by-opponents enters tapped line");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered, "Creatures played by your opponents enter tapped.",
        "expected rendered line to preserve the typed played-by-opponents surface"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_pay_life_or_enter_tapped_shockland_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blood Crypt Variant")
            .parse_text(
                "({T}: Add {B} or {R}.)\nAs this land enters, you may pay 2 life. If you don't, it enters tapped.",
            )
            .expect("shockland ETB payment line should parse");

    let has_pay_life_replacement = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::PayLifeOrEnterTappedReplacement
        )
    });
    assert!(
        has_pay_life_replacement,
        "expected pay-life replacement ability, got {:?}",
        def.abilities
    );

    let has_broad_land_tap_replacement = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::EnterTappedForFilter
        )
    });
    assert!(
        !has_broad_land_tap_replacement,
        "shockland text must not compile as broad land tap replacement: {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_pay_life_or_enter_tapped_requires_if_you_dont_tail() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Broken Shockland Variant")
        .parse_text("As this land enters, you may pay 2 life. If you do, it enters tapped.");
    assert!(
        result.is_err(),
        "unsupported trailing clause must error instead of partial parse"
    );
}

#[cfg(ironsmith_runtime_removed_parser_helper_unit_tests)]
#[test]
fn tokenize_line_keeps_hybrid_slash_inside_mana_braces() {
    let tokens = tokenize_line("{U/R}, {T}: Add {C}.", 0);
    let words = words(&tokens);
    assert!(
        words.contains(&"u/r"),
        "hybrid mana symbol should preserve slash in token stream: {words:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mana_vault_upkeep_pay_clause_includes_pay_mana_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mana Vault Trigger Variant")
        .parse_text("At the beginning of your upkeep, you may pay {4}. If you do, untap this.")
        .expect("mana vault upkeep line should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("PayManaEffect"),
        "expected pay mana effect, got {debug}"
    );
    assert!(
        debug.contains("UntapEffect"),
        "expected untap effect in if-you-do branch, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_energy_pay_clause_includes_pay_energy_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Energy Pay Trigger Variant")
            .parse_text(
                "Whenever this creature attacks, you may pay {E}. If you do, put a +1/+1 counter on this creature.",
            )
            .expect("energy pay trigger line should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("PayEnergyEffect"),
        "expected pay energy effect, got {debug}"
    );
    assert!(
        debug.contains("PutCountersEffect"),
        "expected +1/+1 counter effect in if-you-do branch, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exchange_then_energy_and_sacrifice_unless_payment_keeps_final_action_scoped() {
    let oracle = "Flying, hexproof from activated and triggered abilities\nWhen this creature enters, exchange control of this creature and target creature an opponent controls. If you do, you get {E}{E}{E}{E}, then sacrifice that creature unless you pay an amount of {E} equal to its mana value.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Exchange Energy Variant")
        .parse_text(oracle)
        .expect("exchange followed by energy and sacrifice-unless should parse");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected enter trigger");
    let debug = format!("{:#?}", triggered.effects);
    let energy = debug
        .find("EnergyCountersEffect")
        .expect("fixed energy gain must survive");
    let unless = debug
        .find("UnlessPaysEffect")
        .expect("payment choice must survive");
    let sacrifice = debug[unless..]
        .find("SacrificeTargetEffect")
        .map(|offset| unless + offset)
        .expect("only the sacrifice belongs inside UnlessPays");
    assert!(energy < unless && unless < sacrifice, "{debug}");
    assert!(debug[unless..].contains("ManaValueOf"), "{debug}");
    assert!(
        debug.matches("exchanged_0").count() >= 2,
        "the sacrifice and dynamic energy payment must share the exchanged-creature tag: {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("hexproof from activated and triggered abilities"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "you get {E}{E}{E}{E}, then sacrifice that creature unless you pay an amount of {E} equal to its mana value"
        ),
        "{rendered}"
    );
    assert_eq!(rendered, oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn goad_all_then_restrict_the_exact_result_set_keeps_plural_back_reference() {
    let oracle = "Goad all creatures your opponents control. Until your next turn, those creatures can't block.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Goaded Set Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("linked goad and blocking restriction should parse");
    let program = def.spell_effect.as_ref().expect("spell effect");
    let [goad_segment, restriction_segment] = program.segments.as_slice() else {
        panic!("expected two linked source segments, got {program:#?}");
    };
    let [goad_effect] = goad_segment.default_effects.as_slice() else {
        panic!("expected one tagged goad effect, got {goad_segment:#?}");
    };
    let tagged = goad_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("goaded set should retain a result tag");
    let goad = tagged
        .effect
        .downcast_ref::<crate::effects::GoadEffect>()
        .expect("tag should wrap the goad effect");
    let ChooseSpec::All(goad_filter) = &goad.target else {
        panic!("goad should affect all matching creatures: {goad:#?}");
    };
    assert_eq!(goad_filter.controller, Some(PlayerFilter::Opponent));
    assert!(goad_filter.card_types.contains(&CardType::Creature));

    let [restriction_effect] = restriction_segment.default_effects.as_slice() else {
        panic!("expected one blocking restriction, got {restriction_segment:#?}");
    };
    let cant = restriction_effect
        .downcast_ref::<crate::effects::CantEffect>()
        .expect("followup should be an executable restriction");
    let crate::effect::Restriction::Block(block_filter) = &cant.restriction else {
        panic!("expected a blocking restriction, got {cant:#?}");
    };
    assert!(block_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == tagged.tag
    }));
    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn repeatable_instant_timing_prevention_payment_keeps_duration_target_and_surface() {
    let oracle = "Prevent the next X damage that would be dealt to any target this turn. Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Repeatable Prevention Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("repeatable prevention permission should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("GrantRepeatableManaPaymentActionUntilEndOfTurnEffect"),
        "{debug}"
    );
    assert_eq!(unprocessed_compiled_lines(&def).join(" "), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn target_player_choice_chain_and_cipher_keep_exact_structure_and_source_lines() {
    let oracle = "Target player loses 1 life, discards a card, then sacrifices a permanent of their choice.\nCipher";
    let def = CardDefinitionBuilder::new(CardId::new(), "Target Player Choice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("target-player choice chain should parse");
    let debug = format!("{def:#?}");

    for expected in [
        "SequenceEffect",
        "LoseLifeEffect",
        "DiscardEffect",
        "ChooseObjectsEffect",
        "SacrificePlayerEffect",
        "IsTaggedObject",
        "CipherEffect",
    ] {
        assert!(debug.contains(expected), "missing {expected}: {debug}");
    }
    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn three_action_counter_trigger_keeps_typed_children_and_oracle_conjunction() {
    let oracle = "Whenever a creature you control enters, scry 1 and put a plan counter on this enchantment.\nWhen the fourth plan counter is put on this enchantment, sacrifice it, draw a card, and put a +1/+1 counter on each creature you control.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Counter Plan Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("counter-plan triggers should parse");
    let debug = format!("{def:#?}");
    let compact_debug = debug
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();

    for expected in [
        "CounterPutOnTrigger",
        "SacrificeTargetEffect",
        "DrawCardsEffect",
        "ForEachObject",
        "PlusOnePlusOne",
        "Iterated",
    ] {
        assert!(debug.contains(expected), "missing {expected}: {debug}");
    }
    assert!(
        compact_debug.contains("counter_number:Some(4)"),
        "fourth-counter threshold was not preserved: {debug}"
    );
    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn revealed_hand_choice_discard_keeps_linked_card_and_independent_followup() {
    let oracle = "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. Destroy up to one target Attraction that player controls.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Hand Inspection Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("revealed-hand choice followed by an independent action should parse");
    let debug = format!("{def:#?}");

    for expected in [
        "LookAtHandEffect",
        "ChooseObjectsEffect",
        "DiscardEffect",
        "DestroyEffect",
        "Attraction",
        "__revealed_this_way__",
        "__it__",
    ] {
        assert!(debug.contains(expected), "missing {expected}: {debug}");
    }
    assert_eq!(unprocessed_compiled_lines(&def).join(" "), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_one_or_more_energy_pay_clause_includes_pay_any_energy_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Flexible Energy Pay Trigger Variant")
            .parse_text(
                "Whenever this creature attacks, you may pay one or more {E}. If you do, put a +1/+1 counter on this creature.",
            )
            .expect("one-or-more energy pay trigger line should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("PayAnyEnergyEffect"),
        "expected pay any energy effect, got {debug}"
    );
    assert!(
        debug.contains("PutCountersEffect"),
        "expected +1/+1 counter effect in if-you-do branch, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_get_energy_equal_to_tagged_spell_mana_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Electrosiphon Variant")
            .parse_text("Counter target spell. You get an amount of {E} (energy counters) equal to its mana value.")
            .expect("mana-value-scaled energy clause should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let energy = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<EnergyCountersEffect>())
        .expect("expected EnergyCountersEffect");

    match &energy.count {
        Value::ManaValueOf(spec) => match spec.base() {
            ChooseSpec::Tagged(tag) => assert_eq!(tag.as_str(), IT_TAG),
            other => panic!("expected tagged mana-value reference, got {other:?}"),
        },
        other => panic!("expected mana-value scaling for energy counters, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_black_for_each_creature_in_graveyard_compiles_scaled_mana() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Crypt Probe")
        .parse_text("Add {B} for each creature card in your graveyard.")
        .expect("dynamic add-mana line should parse");
    let effects = def.spell_effect.as_ref().expect("spell effects");
    assert_eq!(effects.len(), 1, "expected exactly one spell effect");

    let add_scaled = effects[0]
        .downcast_ref::<AddScaledManaEffect>()
        .expect("expected AddScaledManaEffect");
    assert_eq!(add_scaled.mana, vec![ManaSymbol::Black]);
    assert_eq!(add_scaled.player, PlayerFilter::You);

    match add_scaled.amount.unhinted() {
        Value::Count(filter) => {
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert_eq!(filter.owner, Some(PlayerFilter::You));
            assert!(
                filter.card_types.contains(&CardType::Creature),
                "expected creature type filter, got {:?}",
                filter.card_types
            );
        }
        other => panic!("expected graveyard creature count, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_activated_add_for_each_creature_compiles_scaled_mana() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gaea Probe")
        .parse_text("{T}: Add {G} for each creature you control.")
        .expect("for-each mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");

    assert!(
        mana_ability.mana_symbols().is_empty(),
        "scaled mana should compile via effects, got direct mana {:?}",
        mana_ability.mana_symbols()
    );
    let effects = &mana_ability.effects;
    let add_scaled = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
        .expect("expected AddScaledManaEffect");
    assert_eq!(add_scaled.mana, vec![ManaSymbol::Green]);
    assert_eq!(add_scaled.player, PlayerFilter::You);
    match add_scaled.amount.unhinted() {
        Value::Count(filter) => {
            assert!(
                filter.card_types.contains(&CardType::Creature),
                "expected creature filter, got {:?}",
                filter.card_types
            );
            assert_eq!(filter.controller, Some(PlayerFilter::You));
        }
        other => panic!("expected count-based scaling, got {other:?}"),
    }

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("for each"),
        "compiled text should preserve for-each semantics: {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_triggered_add_mana_for_creatures_sharing_type_with_it() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mana Echoes Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "Whenever a creature enters, you may add an amount of {C} equal to the number of creatures you control that share a creature type with it.",
            )
            .expect("triggered shared-type scaled mana should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("AddScaledManaEffect") && debug.contains("SharesSubtypeWithTagged"),
        "expected scaled mana count to keep tagged shared-creature-type constraint, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
            rendered_lower.contains(
                "add an amount of {c} equal to the number of creatures you control that share a creature type with it"
            ),
            "compiled text should preserve shared creature type count, got {rendered}"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_activated_add_for_each_swamp_compiles_scaled_mana() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Coffers Probe")
        .parse_text("{2}, {T}: Add {B} for each Swamp you control.")
        .expect("for-each swamp mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");

    let effects = &mana_ability.effects;
    let add_scaled = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
        .expect("expected AddScaledManaEffect");
    assert_eq!(add_scaled.mana, vec![ManaSymbol::Black]);
    match add_scaled.amount.unhinted() {
        Value::Count(filter) => {
            assert!(
                filter.subtypes.contains(&Subtype::Swamp),
                "expected swamp subtype filter, got {:?}",
                filter.subtypes
            );
            assert_eq!(filter.controller, Some(PlayerFilter::You));
        }
        other => panic!("expected count-based scaling, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_activated_add_equal_to_devotion_compiles_scaled_mana() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Karametra Probe")
        .parse_text("{T}: Add an amount of {G} equal to your devotion to green.")
        .expect("devotion mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");

    assert!(
        mana_ability.mana_symbols().is_empty(),
        "devotion-scaled mana should compile via effects"
    );
    let effects = &mana_ability.effects;
    let add_scaled = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
        .expect("expected AddScaledManaEffect");
    assert_eq!(add_scaled.mana, vec![ManaSymbol::Green]);
    assert_eq!(
        add_scaled.amount,
        Value::Devotion {
            player: PlayerFilter::You,
            color: crate::color::Color::Green,
        }
    );

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("devotion to green"),
        "compiled text should preserve devotion semantics: {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_add_equal_to_devotion_compiles_scaled_mana() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Devotion Ritual Probe")
        .parse_text("Add an amount of {R} equal to your devotion to red.")
        .expect("devotion ritual line should parse");
    let effects = def.spell_effect.as_ref().expect("spell effects");
    assert_eq!(effects.len(), 1, "expected exactly one spell effect");
    let add_scaled = effects[0]
        .downcast_ref::<AddScaledManaEffect>()
        .expect("expected AddScaledManaEffect");
    assert_eq!(add_scaled.mana, vec![ManaSymbol::Red]);
    assert_eq!(
        add_scaled.amount,
        Value::Devotion {
            player: PlayerFilter::You,
            color: crate::color::Color::Red,
        }
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_equal_to_source_power_compiles_scaled_mana() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Viridian Joiner Variant")
        .parse_text("{T}: Add an amount of {G} equal to this creature's power.")
        .expect("power-scaled mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let effects = &mana_ability.effects;
    let add_scaled = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
        .expect("expected AddScaledManaEffect");
    assert_eq!(add_scaled.mana, vec![ManaSymbol::Green]);
    assert_eq!(
        add_scaled.amount,
        Value::PowerOf(Box::new(ChooseSpec::Source))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_equal_to_sacrificed_creature_mana_value_uses_sacrifice_cost_tag() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Szeras Variant")
            .parse_text(
                "{T}, Sacrifice another creature: Add an amount of {B} equal to the sacrificed creature's mana value.",
            )
            .expect("sacrifice-scaled mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let effects = &mana_ability.effects;
    let add_scaled = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
        .expect("expected AddScaledManaEffect");
    assert_eq!(add_scaled.mana, vec![ManaSymbol::Black]);
    match add_scaled.amount.unhinted() {
        Value::ManaValueOf(spec) => match spec.as_ref() {
            ChooseSpec::Tagged(tag) => assert_eq!(tag.as_str(), "sacrifice_cost_0"),
            other => panic!("expected sacrifice-cost tag reference, got {other:?}"),
        },
        other => panic!("expected mana-value scaling, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_same_mana_value_as_sacrificed_creature_uses_sacrifice_cost_tag() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sanguine Praetor Variant")
            .parse_text(
                "{B}, Sacrifice a creature: Destroy each creature with the same mana value as the sacrificed creature.",
            )
            .expect("same-mana-value destroy ability should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability),
            _ => None,
        })
        .expect("expected activated ability");
    let destroy = activated
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
        .expect("expected destroy effect");

    let ChooseSpec::All(filter) = &destroy.spec else {
        panic!("expected destroy-all filter");
    };

    let tag_constraint = filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::SameManaValueAsTagged
            )
        })
        .expect("expected same-mana-value tagged constraint");
    assert_eq!(tag_constraint.tag.as_str(), "sacrifice_cost_0");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_that_much_colorless_uses_previous_effect_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mana Seism Variant")
        .parse_text("Sacrifice any number of lands, then add that much {C}.")
        .expect("that-much mana spell should parse");

    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let mut scaled_mana = Vec::new();
    visit_nested_effects::<AddScaledManaEffect>(effects, |effect| {
        scaled_mana.push((effect.mana.clone(), effect.amount.clone()));
    });
    let [(mana, amount)] = scaled_mana.as_slice() else {
        panic!("expected one nested AddScaledManaEffect, got {scaled_mana:#?}");
    };
    assert_eq!(mana.as_slice(), &[ManaSymbol::Colorless]);
    assert!(
        matches!(
            amount,
            Value::EffectValue(_) | Value::EffectValueOffset(_, _) | Value::EventValue(_)
        ),
        "expected dynamic backreference amount, got {:?}",
        amount
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_x_any_one_color_where_count_keeps_dynamic_amount() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Harabaz Druid Variant")
        .parse_text(
            "{T}: Add X mana of any one color, where X is the number of Allies you control.",
        )
        .expect("dynamic any-one-color mana line should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let effects = &mana_ability.effects;
    let add_any_one = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfAnyOneColorEffect>())
        .expect("expected AddManaOfAnyOneColorEffect");
    match add_any_one.amount.unhinted() {
        Value::Count(filter) => {
            assert_eq!(filter.controller, Some(PlayerFilter::You));
            assert!(
                filter.subtypes.contains(&Subtype::Ally),
                "expected ally subtype filter, got {:?}",
                filter.subtypes
            );
        }
        other => panic!("expected count-based amount, got {other:?}"),
    }

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("any one color"),
        "compiled text should preserve any-one-color semantics: {mana_line}"
    );
    assert!(
        !mana_line.contains("{X}{X}"),
        "compiled text should not duplicate X as mana symbols: {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_combination_of_two_colors_keeps_amount_and_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lumberjack Variant")
        .parse_text("{T}, Sacrifice a Forest: Add three mana in any combination of {R} and/or {G}.")
        .expect("restricted any-combination mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let effects = &mana_ability.effects;
    let add_any = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfAnyColorEffect>())
        .expect("expected AddManaOfAnyColorEffect");
    assert_eq!(add_any.amount.unhinted(), &Value::Fixed(3));
    let colors = add_any
        .available_colors
        .as_ref()
        .expect("expected restricted colors");
    assert!(
        colors.contains(&crate::color::Color::Red)
            && colors.contains(&crate::color::Color::Green)
            && colors.len() == 2,
        "expected red/green restriction, got {colors:?}"
    );

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("in any combination of {R} and/or {G}"),
        "compiled text should preserve restricted color combination, got: {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_or_mana_colors_compiles_single_restricted_choice_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dual Land Variant")
        .parse_text("{T}: Add {W} or {B}.")
        .expect("restricted color choice mana ability should parse");

    assert_eq!(def.abilities.len(), 1, "expected a single mana ability");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let add_any = mana_ability
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfAnyColorEffect>())
        .expect("expected AddManaOfAnyColorEffect");
    assert_eq!(add_any.amount.unhinted(), &Value::Fixed(1));

    let colors = add_any
        .available_colors
        .as_ref()
        .expect("expected restricted colors");
    assert_eq!(colors.len(), 2, "expected two restricted colors");
    assert!(colors.contains(&crate::color::Color::White));
    assert!(colors.contains(&crate::color::Color::Black));

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("Add {W} or {B}"),
        "compiled text should preserve color choice wording, got: {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_combination_of_colors_expands_to_five_colors() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Terrarion Variant")
        .parse_text("{T}, Sacrifice this artifact: Add two mana in any combination of colors.")
        .expect("any-combination-of-colors mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let add_any = mana_ability
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfAnyColorEffect>())
        .expect("expected AddManaOfAnyColorEffect");
    assert_eq!(add_any.amount.unhinted(), &Value::Fixed(2));
    let colors = add_any
        .available_colors
        .as_ref()
        .expect("expected explicit five-color restriction");
    assert_eq!(colors.len(), 5, "expected WUBRG, got {colors:?}");
    assert!(colors.contains(&crate::color::Color::White));
    assert!(colors.contains(&crate::color::Color::Blue));
    assert!(colors.contains(&crate::color::Color::Black));
    assert!(colors.contains(&crate::color::Color::Red));
    assert!(colors.contains(&crate::color::Color::Green));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_combination_with_where_tail_keeps_color_choices() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Vivi Variant")
            .parse_text("{T}: Add X mana in any combination of {G} and/or {U}, where X is this creature's power.")
            .expect("any-combination clause with where-tail should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let add_any = mana_ability
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfAnyColorEffect>())
        .expect("expected AddManaOfAnyColorEffect");
    let colors = add_any
        .available_colors
        .as_ref()
        .expect("expected restricted colors");
    assert_eq!(
        colors.len(),
        2,
        "expected two-color restriction, got {colors:?}"
    );
    assert_eq!(add_any.amount.unhinted(), &Value::SourcePower);
    assert!(add_any.amount.has_surface_hint(ValueSurfaceHint::WhereXIs));
    assert!(colors.contains(&crate::color::Color::Green));
    assert!(colors.contains(&crate::color::Color::Blue));

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.to_ascii_lowercase().contains("power"),
        "compiled text should describe the X value, got: {mana_line}"
    );
    assert!(
        mana_line.contains("where X is"),
        "compiled text should retain the binding for X, got: {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_combination_with_unbound_x_without_definition_fails() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Broken Vivi Variant")
        .parse_text("{T}: Add X mana in any combination of {G} and/or {U}.")
        .expect_err("bare X mana ability should fail without a where clause or X cost");
    let message = format!("{err:?}");
    assert!(
        message.contains("unresolved X in mana ability"),
        "expected unresolved-X parse error, got: {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_combination_with_named_self_where_tail_keeps_source_power() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Vivi Ornitier")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{0}: Add X mana in any combination of {U} and/or {R}, where X is Vivi Ornitier's power. Activate only during your turn and only once each turn.",
            )
            .expect("named self-reference where-tail should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let add_any = mana_ability
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfAnyColorEffect>())
        .expect("expected AddManaOfAnyColorEffect");
    let Value::PowerOf(source) = add_any.amount.unhinted() else {
        panic!("expected source-power amount, got {:?}", add_any.amount);
    };
    assert!(
        matches!(source.base(), ChooseSpec::Source),
        "expected the named self reference to resolve to source, got {source:?}"
    );
    assert!(add_any.amount.has_surface_hint(ValueSurfaceHint::WhereXIs));
    let colors = add_any
        .available_colors
        .as_ref()
        .expect("expected restricted colors");
    assert!(colors.contains(&crate::color::Color::Blue));
    assert!(colors.contains(&crate::color::Color::Red));
    // Mana-ability restrictions are kept together in `activation_condition`
    // because the mana special-action path evaluates that condition directly.
    // Keeping both clauses there preserves the conjunction without asking the
    // single-valued `timing` field to represent two independent restrictions.
    assert_eq!(mana_ability.timing, ActivationTiming::AnyTime);
    assert_eq!(
        mana_ability.activation_condition,
        Some(ConditionExpr::And(
            Box::new(ConditionExpr::ActivationTiming(
                ActivationTiming::DuringYourTurn
            )),
            Box::new(ConditionExpr::ActivationTiming(
                ActivationTiming::OncePerTurn
            )),
        ))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_quoted_token_rule_stays_inside_trigger_resolution() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Digsite Engineer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you cast an artifact spell, you may pay {2}. If you do, create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
        )
        .expect("conditional quoted token creation should parse");

    assert_eq!(
        def.abilities.len(),
        1,
        "the token's quoted rule must not become a second source static ability: {:#?}",
        def.abilities
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("If you do, create a 0/0 colorless Construct artifact creature token")
            && rendered.contains("\"This token gets +1/+1 for each artifact you control.\""),
        "expected the conditional create and quoted token rule to remain together, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_source_spell_cast_trigger_stays_as_stack_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Malicious Affliction Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Morbid — When you cast this spell, if a creature died this turn, you may copy this spell and may choose a new target for the copy.\nDestroy target nonblack creature.",
        )
        .expect("source spell-cast trigger should parse as a stack ability");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("YouCastThisSpellTrigger")
            && abilities_debug.contains("CopySpellEffect")
            && abilities_debug.contains("ChooseNewTargetsEffect"),
        "expected an executable source-cast copy trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Died"),
        "the morbid intervening condition must remain structural: {abilities_debug}"
    );
    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("DestroyEffect"),
        "the ordinary spell resolution must remain separate: {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_color_that_opponent_land_could_produce_compiles_restricted_mana_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exotic Orchard Variant")
        .parse_text(
            "{T}: Add one mana of any color that a land an opponent controls could produce.",
        )
        .expect("land-could-produce mana line should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let effects = &mana_ability.effects;
    let restricted = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfLandProducedTypesEffect>())
        .expect("expected AddManaOfLandProducedTypesEffect");
    assert_eq!(restricted.amount.unhinted(), &Value::Fixed(1));
    assert_eq!(restricted.player, PlayerFilter::You);
    assert!(
        !restricted.allow_colorless,
        "any color clause must not allow colorless"
    );
    assert!(
        !restricted.same_type,
        "any color clause should allow independent color choices"
    );
    assert_eq!(
        restricted.mana_type_source,
        crate::effects::ManaTypeSource::MatchingLandsCouldProduce,
        "could-produce clauses must inspect prospective mana abilities"
    );
    assert!(
        restricted.land_filter.card_types.contains(&CardType::Land),
        "expected land filter, got {:?}",
        restricted.land_filter
    );
    assert_eq!(
        restricted.land_filter.controller,
        Some(PlayerFilter::Opponent),
        "expected opponent-controlled land filter"
    );

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("could produce"),
        "compiled text should preserve could-produce clause, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_type_that_gate_you_control_could_produce_keeps_type_semantics() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gond Gate Variant")
        .parse_text("{T}: Add one mana of any type that a Gate you control could produce.")
        .expect("gate could-produce mana line should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a),
            _ => None,
        })
        .expect("expected mana ability");
    let effects = &mana_ability.effects;
    let restricted = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfLandProducedTypesEffect>())
        .expect("expected AddManaOfLandProducedTypesEffect");
    assert!(
        restricted.allow_colorless,
        "any type clause must allow colorless"
    );
    assert_eq!(
        restricted.land_filter.controller,
        Some(PlayerFilter::You),
        "expected you-control filter for gates"
    );
    assert!(
        restricted.land_filter.subtypes.contains(&Subtype::Gate),
        "expected gate subtype filter, got {:?}",
        restricted.land_filter
    );
    assert_eq!(
        restricted.mana_type_source,
        crate::effects::ManaTypeSource::MatchingLandsCouldProduce
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_triggering_land_produced_types_uses_actual_mana_event() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Heartbeat Variant")
        .parse_text(
            "Whenever a player taps a land for mana, that player adds one mana of any type that land produced.",
        )
        .expect("actual land-produced mana trigger should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let restricted = triggered
        .effects
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<AddManaOfLandProducedTypesEffect>())
        .expect("expected produced-types mana effect");
    assert_eq!(restricted.amount.unhinted(), &Value::Fixed(1));
    assert_eq!(restricted.player, PlayerFilter::IteratedPlayer);
    assert!(restricted.allow_colorless);
    assert_eq!(
        restricted.mana_type_source,
        crate::effects::ManaTypeSource::TriggeringEventProduced,
        "past-tense 'produced' must consume the actual triggering mana event"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mana_ability_activate_only_if_you_control_an_artifact() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spire Variant")
            .parse_text(
                "{T}: Add {C}.\n{T}, Pay 1 life: Add one mana of any color. Activate only if you control an artifact.",
            )
            .expect("artifact-gated mana ability should parse");

    let lines = unprocessed_compiled_lines(&def);
    let gated = lines
        .iter()
        .find(|line| {
            line.contains("Pay 1 life")
                && line.contains("Add one mana of any color")
                && line.contains("Activate only if you control one or more artifacts")
        })
        .unwrap_or_else(|| panic!("expected gated mana rendering, got lines: {lines:?}"));
    assert!(
        gated.contains("Add one mana of any color"),
        "expected gated rainbow mana text, got: {gated}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_add_any_color_with_unsupported_trailing_clause_fails() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Broken Orchard Variant")
            .parse_text(
                "{T}: Add one mana of any color that a land an opponent controls could produce unless it's your turn.",
            )
            .expect_err("unsupported could-produce tail should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing mana clause"),
        "expected strict-tail parse error, got: {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_cost_increase_per_target_beyond_first_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fireball Variant")
        .parse_text("This spell costs {1} more to cast for each target beyond the first.")
        .expect("fireball cost line should parse");

    let has_target_cost_increase = def.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        static_ability.id()
            == crate::static_abilities::StaticAbilityId::CostIncreaseManaCostPerAdditionalTarget
            && static_ability
                .cost_increase_mana_cost_per_additional_target()
                .is_some_and(|cost| cost.to_oracle() == "{1}")
    });
    assert!(
        has_target_cost_increase,
        "expected additional-target cost increase ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_life_cost_per_target_preserves_nonmana_payment_and_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Purge Variant")
        .parse_text(
            "This spell costs 3 life more to cast for each target.\nDestroy any number of target creatures.",
        )
        .expect("per-target life cost should parse");

    let life_cost = def.abilities.iter().find_map(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        static_ability.additional_life_cost_per_target()
    });
    assert_eq!(life_cost, Some(3), "{:?}", def.abilities);
    assert_eq!(
        unprocessed_compiled_lines(&def).join("\n"),
        "This spell costs 3 life more to cast for each target.\nDestroy any number of target creatures."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_tax_except_during_controller_turn_preserves_semantics_and_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Defense Grid Variant")
        .parse_text("Each spell costs {3} more to cast except during its controller's turn.")
        .expect("controller-turn spell tax should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Each spell costs {3} more to cast except during its controller's turn."
    );
    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("Excluding") && debug.contains("excluded: Active"),
        "expected the tax to exclude spells cast by the active player, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_target_blocked_creature_keeps_targeting_legality() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Smite Variant")
        .parse_text("Destroy target blocked creature.")
        .expect("blocked-creature removal should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(rendered, "Destroy target blocked creature.");
    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("blocked: true") && !debug.contains("TargetIsBlocked"),
        "expected blocked to remain part of target legality, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn radiant_solar_keeps_venture_and_life_gain_in_the_discard_ability() {
    let oracle = "Flying, lifelink\n\
Whenever this creature or another nontoken creature you control enters, venture into the dungeon.\n\
{W}, Discard this card: Venture into the dungeon and you gain 3 life.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Radiant Solar")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Radiant Solar text should parse");

    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle);
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("discard ability");
    let debug = format!("{:#?}", activated.effects);
    assert!(debug.contains("VentureIntoDungeonEffect"), "{debug}");
    assert!(debug.contains("GainLifeEffect"), "{debug}");
}
