use super::*;

#[test]
fn oath_of_ghouls_uses_the_migrated_graveyard_minority_program() {
    let oracle = "At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Oath of Ghouls")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("graveyard-minority trigger should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("AnOpponentHasFewerThanPlayer"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
    assert!(debug.contains("ReturnFromGraveyardToHandEffect"), "{debug}");
}

#[test]
fn land_majority_search_uses_the_active_player_and_player_target() {
    let oracle = "At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Land comparison fixture")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("land majority search compiles");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("OpponentWithMoreControlledObjectsThan"), "{debug}");
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle, "{debug}");
}

#[test]
fn land_majority_search_target_legality_tracks_active_players_lands() {
    let oracle = "At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Land target fixture")
        .card_types(vec![CardType::Enchantment]).parse_text(oracle).unwrap();
    let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else { panic!("triggered ability"); };
    let target = structural_unwrap_render_wrappers(&triggered.effects.segments[0].default_effects[0])
        .downcast_ref::<crate::effects::TargetOnlyEffect>().unwrap();
    assert_eq!(target.chooser, Some(PlayerFilter::Active));
    for counts in [[5, 2, 3], [0, 2, 2], [4, 3, 1]] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let players: Vec<_> = game.players.iter().map(|p| p.id).collect();
        let source = game.create_object_from_definition(&definition, players[0], Zone::Battlefield);
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land fixture").card_types(vec![CardType::Land]).build();
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Noise creature").card_types(vec![CardType::Creature]).build();
        for (idx, player) in players.iter().enumerate() {
            for _ in 0..counts[idx] { game.create_object_from_card(&land, *player, Zone::Battlefield); }
            for _ in 0..6 { game.create_object_from_card(&land, *player, Zone::Hand); game.create_object_from_card(&creature, *player, Zone::Battlefield); }
        }
        for active in 0..3 {
            game.turn.active_player = players[active];
            game.refresh_continuous_state();
            let legal = crate::game_loop::compute_legal_targets(&game, &target.target, players[0], Some(source));
            for candidate in 0..3 {
                assert_eq!(legal.contains(&crate::game_state::Target::Player(players[candidate])), candidate != active && counts[candidate] > counts[active], "counts={counts:?}, active={active}, candidate={candidate}");
            }
        }
    }
}

struct LandSearchDecision {
    player: crate::ids::PlayerId,
    land: crate::ids::ObjectId,
    accept: bool,
    boolean_calls: usize,
}
impl crate::decision::DecisionMaker for LandSearchDecision {
    fn decide_boolean(&mut self, _: &crate::game_state::GameState, ctx: &crate::decisions::context::BooleanContext) -> bool {
        assert_eq!(ctx.player, self.player);
        self.boolean_calls += 1;
        self.accept
    }
    fn decide_objects(&mut self, _: &crate::game_state::GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
        assert_eq!(ctx.player, self.player);
        let legal: Vec<_> = ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).collect();
        assert_eq!(legal, vec![self.land]);
        vec![self.land]
    }
}

#[test]
fn land_majority_search_resolves_for_active_player_and_can_be_declined() {
    let oracle = "At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search resolution fixture")
        .card_types(vec![CardType::Enchantment]).parse_text(oracle).unwrap();
    let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else { panic!("trigger"); };
    for accept in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id; let carol = game.players[2].id;
        game.turn.active_player = bob;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let basic = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Basic fixture")
            .card_types(vec![CardType::Land]).supertypes(vec![crate::types::Supertype::Basic]).build();
        let nonbasic = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Nonbasic fixture").card_types(vec![CardType::Land]).build();
        game.create_object_from_card(&basic, carol, Zone::Battlefield);
        let candidate = game.create_object_from_card(&basic, bob, Zone::Library);
        let stable = game.object(candidate).unwrap().stable_id;
        let own = game.create_object_from_card(&basic, alice, Zone::Library);
        let opponents = game.create_object_from_card(&basic, carol, Zone::Library);
        let ineligible = game.create_object_from_card(&nonbasic, bob, Zone::Library);
        let mut dm = LandSearchDecision { player: bob, land: candidate, accept, boolean_calls: 0 };
        {
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_targets(vec![crate::effects::ResolvedTarget::Player(carol)]).with_decision_maker(&mut dm);
            for segment in &triggered.effects.segments {
                for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            }
        }
        assert_eq!(dm.boolean_calls, 1);
        let current = game.find_object_by_stable_id(stable).unwrap();
        let object = game.object(current).unwrap();
        assert_eq!(object.zone, if accept { Zone::Battlefield } else { Zone::Library });
        assert_eq!(game.controller_of(object), bob);
        assert!(!game.is_tapped(current));
        for id in [own, opponents, ineligible] { assert_eq!(game.object(id).unwrap().zone, Zone::Library); }
    }
}

#[test]
fn land_majority_search_surface_rejects_conflicting_selection_zone() {
    let oracle = "At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search zone fixture")
        .card_types(vec![CardType::Enchantment]).parse_text(oracle).unwrap();
    let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else { panic!("trigger"); };
    let effects = &triggered.effects.segments[0].default_effects;
    assert!(describe_relative_player_target_then_optional_search(effects).is_some());
    let mut may = structural_unwrap_render_wrappers(&effects[1]).downcast_ref::<crate::effects::MayEffect>().unwrap().clone();
    let mut sequence = structural_unwrap_render_wrappers(&may.effects[0]).downcast_ref::<crate::effects::SequenceEffect>().unwrap().clone();
    let mut choose = sequence.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap().clone();
    choose.filter.zone = Some(Zone::Battlefield);
    sequence.effects[0] = Effect::new(choose);
    may.effects[0] = Effect::new(sequence);
    assert!(describe_relative_player_target_then_optional_search(&[effects[0].clone(), Effect::new(may)]).is_none());
}
