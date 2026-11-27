use super::{A, card, game, put};
use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
use ironsmith::effects::{AttachToEffect, MeldEffect};
use ironsmith::{
    AbilityKind, ConditionExpr, EffectContext, EffectExecutor, GameState, LegalAction, ManaCost,
    ManaSymbol, ObjectId, PlayerId, PriorityLoopState, PriorityResponse, TriggerQueue, Zone,
};

const B: PlayerId = PlayerId(1);

fn designate(g: &mut GameState, route: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = put(g, route, owner, zone);
    g.set_as_commander(id, owner);
    id
}

fn assert_control(g: &GameState, player: PlayerId, source: ObjectId, any: bool, own: bool) {
    assert_eq!(g.player_controls_own_commander(player), own);
    assert_eq!(g.player_controls_a_commander(player), any);
    assert_eq!(
        ironsmith::condition_eval::evaluate_condition_cast_time_checked(
            g,
            &ConditionExpr::YouControlCommander,
            player,
            source
        )
        .unwrap(),
        any
    );
}

#[test]
fn a_commander_under_another_card_remains_a_commander_through_control_changes_and_phasing() {
    for meld in [false, true] {
        let mut g = game();
        let permanent = if meld {
            let bruna = card("bruna-the-fading-light");
            let gisela = card("gisela-the-broken-blade");
            g.register_linked_face_definition(&bruna);
            g.register_linked_face_definition(&gisela);
            let bruna = g.create_object_from_definition(&bruna, B, Zone::Battlefield);
            let gisela = g.create_object_from_definition(&gisela, B, Zone::Battlefield);
            g.set_as_commander(gisela, B);
            g.register_linked_face_definition(&card("brisela-voice-of-nightmares"));
            MeldEffect::new("Brisela, Voice of Nightmares")
                .execute(&mut g, &mut EffectContext::new_default(bruna, B))
                .unwrap()
                .first_output_object()
                .unwrap()
        } else {
            let commander = designate(&mut g, "brokkos-apex-of-forever", B, Zone::Command);
            let spell = g.move_object_by_effect(commander, Zone::Stack).unwrap();
            let host = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
            g.merge_mutating_creature_spell(spell, host, false).unwrap()
        };
        assert_control(&g, B, permanent, true, true);
        assert_control(&g, A, permanent, false, false);
        g.set_current_controller(permanent, A).unwrap();
        assert_control(&g, A, permanent, true, false);
        assert_control(&g, B, permanent, false, false);
        g.phase_out(permanent);
        assert_control(&g, A, permanent, false, false);
        g.phase_in(permanent);
        assert_control(&g, A, permanent, true, false);
    }
}

#[test]
fn designation_after_moving_a_card_persists_across_later_zone_changes() {
    let mut g = game();
    let original = put(&mut g, "isamaru-hound-of-konda", A, Zone::Library);
    let designated = g.move_object_by_effect(original, Zone::Command).unwrap();
    g.set_as_commander(designated, A);
    assert_control(&g, A, designated, false, false);
    let permanent = g
        .move_object_by_effect(designated, Zone::Battlefield)
        .unwrap();
    assert_eq!(g.commander_identity(permanent), Some(original));
    assert_control(&g, A, permanent, true, true);
    g.phase_out(permanent);
    assert_control(&g, A, permanent, false, false);
    g.phase_in(permanent);
    let exiled = g.move_object_by_effect(permanent, Zone::Exile).unwrap();
    assert!(g.is_commander(exiled));
    assert_control(&g, A, exiled, false, false);
}

#[test]
fn path_of_ancestry_checks_creature_types_when_its_mana_is_spent() {
    for (commander, spell, zone, phase_out, scry) in [
        (
            "go-shintai-of-life-s-origin",
            "go-shintai-of-ancient-wars",
            Zone::Command,
            false,
            false,
        ),
        (
            "isamaru-hound-of-konda",
            "alpine-watchdog",
            Zone::Command,
            false,
            true,
        ),
        (
            "isamaru-hound-of-konda",
            "alpine-watchdog",
            Zone::Hand,
            false,
            true,
        ),
        (
            "isamaru-hound-of-konda",
            "grizzly-bears",
            Zone::Command,
            false,
            false,
        ),
        (
            "isamaru-hound-of-konda",
            "alpine-watchdog",
            Zone::Battlefield,
            true,
            false,
        ),
    ] {
        let mut g = game();
        let cmd = designate(&mut g, commander, A, zone);
        let path = put(&mut g, "path-of-ancestry", A, Zone::Battlefield);
        g.untap(path);
        let ability = g.object(path).unwrap().abilities.len() - 1;
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut g,
            A,
            path,
            ability,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        if phase_out {
            g.phase_out(cmd);
        }
        let spell_id = put(&mut g, spell, A, Zone::Stack);
        let generic = ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]);
        assert!(
            g.try_pay_mana_cost_with_reason(
                A,
                Some(spell_id),
                &generic,
                0,
                ironsmith::costs::PaymentReason::CastSpell
            )
            .unwrap()
        );
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut g, &mut queue).unwrap();
        assert_eq!(
            queue.entries.iter().filter(|e| e.source == path).count(),
            usize::from(scry),
            "{commander} in {zone:?}, casting {spell}"
        );
    }
}

#[test]
fn war_room_requires_a_defined_color_identity_to_offer_and_pay_its_draw_cost() {
    for (commanders, life, price) in [
        (vec![], 20, None),
        (vec![("isamaru-hound-of-konda", B)], 20, None),
        (vec![("traxos-scourge-of-kroog", A)], 1, Some(0)),
        (vec![("isamaru-hound-of-konda", A)], 2, Some(1)),
        (vec![("brokkos-apex-of-forever", A)], 2, None),
        (vec![("brokkos-apex-of-forever", A)], 4, Some(3)),
        (
            vec![("thrasios-triton-hero", A), ("tymna-the-weaver", A)],
            5,
            Some(4),
        ),
    ] {
        let mut g = game();
        for (name, owner) in commanders {
            designate(&mut g, name, owner, Zone::Command);
        }
        let room = put(&mut g, "war-room", A, Zone::Battlefield);
        put(&mut g, "grizzly-bears", A, Zone::Library);
        let player = g.player_mut(A).unwrap();
        player.life = life;
        player.mana_pool.add(ManaSymbol::Colorless, 3);
        let action = LegalAction::ActivateAbility {
            source: room,
            ability_index: 1,
        };
        assert_eq!(
            ironsmith::compute_legal_actions(&g, A)
                .unwrap()
                .contains(&action),
            price.is_some()
        );
        if let Some(price) = price {
            let mut queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(g.players_in_game());
            let mut dm = SelectFirstDecisionMaker;
            let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut g,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            while g.stack.is_empty() {
                let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                    panic!("activation stopped before payment");
                };
                progress = ironsmith::game_loop::apply_decision_context_with_dm(
                    &mut g, &mut queue, &mut state, &ctx, &mut dm,
                )
                .unwrap();
            }
            assert_eq!(g.player(A).unwrap().life, life - price);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            assert!(g.is_tapped(room));
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(g.player(A).unwrap().hand.len(), 1);
        }
    }
}

#[test]
fn commanders_plate_uses_its_controllers_defined_color_identity() {
    for (commanders, count, protection_from_red) in [
        (vec![], 0, false),
        (vec![("isamaru-hound-of-konda", B)], 0, false),
        (vec![("traxos-scourge-of-kroog", A)], 5, true),
        (vec![("brokkos-apex-of-forever", A)], 2, true),
        (vec![("go-shintai-of-life-s-origin", A)], 0, false),
    ] {
        let mut g = game();
        for (name, owner) in commanders {
            designate(&mut g, name, owner, Zone::Command);
        }
        let bear = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
        let plate = put(&mut g, "commander-s-plate", A, Zone::Battlefield);
        AttachToEffect::new(ironsmith::target::ChooseSpec::SpecificObject(bear))
            .execute(&mut g, &mut EffectContext::new_default(plate, A))
            .unwrap();
        g.set_current_controller(bear, B).unwrap();
        let bolt = put(&mut g, "lightning-bolt", B, Zone::Stack);
        assert_eq!(g.current_power(bear), Some(5));
        let abilities = g.current_abilities(bear).unwrap();
        assert_eq!(
            abilities
                .iter()
                .filter(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_protection()))
                .count(),
            count
        );
        assert_eq!(
            ironsmith::targeting::has_protection_from_source(&g, bear, bolt),
            protection_from_red
        );
    }
}

fn animate_cactus(g: &mut GameState) -> ObjectId {
    let land = put(g, "cactus-preserve", A, Zone::Battlefield);
    let AbilityKind::Activated(ability) = &g.object(land).unwrap().abilities.last().unwrap().kind
    else {
        panic!("expected the animation ability");
    };
    g.push_to_stack(ironsmith::StackEntry::ability(
        land,
        A,
        ability.effects.clone(),
    ));
    ironsmith::game_loop::resolve_stack_entry_with(g, &mut SelectFirstDecisionMaker).unwrap();
    land
}

#[test]
fn cactus_preserve_finds_the_greatest_commander_mana_value_across_zones() {
    for zone in [
        Zone::Command,
        Zone::Library,
        Zone::Hand,
        Zone::Exile,
        Zone::Graveyard,
    ] {
        let mut g = game();
        designate(&mut g, "tymna-the-weaver", A, zone);
        designate(&mut g, "thrasios-triton-hero", A, Zone::Battlefield);
        designate(&mut g, "brokkos-apex-of-forever", B, Zone::Command);
        let land = animate_cactus(&mut g);
        assert_eq!(g.current_power(land), Some(3), "{zone:?}");
        assert_eq!(g.current_toughness(land), Some(3), "{zone:?}");
    }
}

#[test]
fn commander_mana_value_aggregates_include_layer_one_copy_effects() {
    let mut g = game();
    let commander = designate(&mut g, "tymna-the-weaver", A, Zone::Battlefield);
    let bear = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let dinosaur = put(&mut g, "colossal-dreadmaw", B, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    super::resolve_spell(
        &mut g,
        "mirrorweave",
        vec![ironsmith::Target::Object(dinosaur)],
        &mut dm,
    );
    assert_eq!(g.current_power(commander), Some(6));

    let mut armor_board = g.clone();
    let armor = put(&mut armor_board, "tangleweave-armor", A, Zone::Battlefield);
    AttachToEffect::new(ironsmith::target::ChooseSpec::SpecificObject(bear))
        .execute(&mut armor_board, &mut EffectContext::new_default(armor, A))
        .unwrap();
    assert_eq!(armor_board.current_power(bear), Some(12));

    let mut land_board = g.clone();
    let land = animate_cactus(&mut land_board);
    assert_eq!(land_board.current_power(land), Some(6));

    super::resolve_spell(&mut g, "cloudkill", vec![], &mut dm);
    assert_eq!(g.current_power(bear), Some(0));
}

#[test]
fn copied_artifact_mana_values_feed_flashback_costs_draws_and_static_bonuses() {
    let mut g = game();
    designate(&mut g, "tymna-the-weaver", A, Zone::Battlefield);
    let sphere = put(&mut g, "myr-battlesphere", B, Zone::Battlefield);
    super::resolve_spell(
        &mut g,
        "mirrorweave",
        vec![ironsmith::Target::Object(sphere)],
        &mut SelectFirstDecisionMaker,
    );

    let mut flashback = g.clone();
    let spell = put(&mut flashback, "visions-of-glory", A, Zone::Graveyard);
    flashback
        .player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::White, 3);
    assert_eq!(
        super::expanded_interactions::spend(
            &mut flashback,
            spell,
            ironsmith::CastingMethod::Alternative(0)
        ),
        3
    );

    for _ in 0..8 {
        put(&mut g, "grizzly-bears", A, Zone::Library);
    }
    super::resolve_spell(
        &mut g,
        "rush-of-knowledge",
        vec![],
        &mut SelectFirstDecisionMaker,
    );
    assert_eq!(g.player(A).unwrap().hand.len(), 7);
    let raker = put(&mut g, "glint-raker", A, Zone::Battlefield);
    assert_eq!(g.current_power(raker), Some(8));
}

#[test]
fn a_life_cost_with_an_undefined_operand_cannot_be_paid() {
    use ironsmith::effect::{OutcomeStatus, Value};
    use ironsmith::effects::{CostExecutableEffect, PayLifeEffect};
    let mut g = game();
    let source = put(&mut g, "war-room", A, Zone::Battlefield);
    let amount = Value::Add(
        Box::new(Value::Fixed(2)),
        Box::new(Value::CommanderColorIdentityColors(
            ironsmith::PlayerFilter::You,
        )),
    );
    let cost = PayLifeEffect::you(amount);
    let mut ctx = EffectContext::new_default(source, A);
    assert!(CostExecutableEffect::can_execute_as_cost(&cost, &g, source, A).is_err());
    assert_eq!(
        cost.execute(&mut g, &mut ctx).unwrap().status,
        OutcomeStatus::Impossible
    );
    assert_eq!(g.player(A).unwrap().life, 20);
    designate(&mut g, "traxos-scourge-of-kroog", A, Zone::Command);
    assert!(CostExecutableEffect::can_execute_as_cost(&cost, &g, source, A).is_ok());
    assert_eq!(cost.execute(&mut g, &mut ctx).unwrap().as_count(), Some(2));
    assert_eq!(g.player(A).unwrap().life, 18);
}
