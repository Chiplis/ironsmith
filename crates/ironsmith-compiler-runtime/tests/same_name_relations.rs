//! Exact frozen bodies, direct and artifact paths, and native scenarios. UNRUN.
use std::collections::VecDeque;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::Modification;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, PartitionContext, SelectObjectsContext, TargetsContext, ViewCardsContext};
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::ObjectKind;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, CounterType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Supertype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/same_name_relations.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = result.unwrap();
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for player in [A, B, C] {
        game.player_mut(player).unwrap().land_plays_per_turn = 10;
        for mana in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(mana, 10);
        }
    }
    game
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, kind: CardType) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), name).card_types(vec![kind])
        .power_toughness(PowerToughness::fixed(2, 2)).build(), owner, zone)
}
fn land(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, basic: bool) -> ObjectId {
    let definition = compile_to_runtime_definition(name,
        &format!("Type: {}Land\n{{T}}: Add {{G}}.", if basic { "Basic " } else { "" }), false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn set_name(game: &mut GameState, id: ObjectId, name: &str) {
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(id), Modification::SetName(name.into()), Until::EndOfTurn)
        .execute(game, &mut EffectContext::new_default(id, A)).unwrap();
}
#[derive(Default)]
struct Choices {
    target: Option<Target>,
    excluded_targets: Vec<Target>,
    objects: VecDeque<Vec<ObjectId>>,
    accept: Option<bool>,
    partitions: Vec<(PlayerId, usize)>,
    views: Vec<(PlayerId, bool, Vec<ObjectId>)>,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for excluded in &self.excluded_targets {
            assert!(ctx.requirements.iter().all(|requirement| !requirement.legal_targets.contains(excluded)));
        }
        if let Some(target) = self.target {
            assert!(ctx.requirements.iter().all(|requirement| requirement.legal_targets.contains(&target)), "requested target is illegal: {ctx:?}");
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, ctx) }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(chosen) = self.objects.pop_front() {
            for id in &chosen {
                assert!(ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal), "{ctx:?}");
            }
            chosen
        } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool { self.accept.unwrap_or(true) && ctx.can_accept }
    fn decide_partition(&mut self, _: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
        self.partitions.push((ctx.player, ctx.cards.len()));
        Vec::new()
    }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId], ctx: &ViewCardsContext) {
        self.views.push((viewer, ctx.public, cards.to_vec()));
    }
}
fn announce(game: &mut GameState, actor: PlayerId, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(actor);
    assert!(compute_legal_actions(game, actor).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players.len());
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("unfinished action: {progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn cast(game: &mut GameState, actor: PlayerId, definition: &CardDefinition, dm: &mut Choices) -> ironsmith::ids::StableId {
    let spell = game.create_object_from_definition(definition, actor, Zone::Hand);
    let stable = game.object(spell).unwrap().stable_id;
    announce(game, actor, LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand, casting_method: CastingMethod::Normal }, dm);
    stable
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("nonterminating trigger chain");
}
fn activation(game: &GameState, actor: PlayerId, source: ObjectId) -> LegalAction {
    compute_legal_actions(game, actor).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, .. } | LegalAction::ActivateManaAbility { source: id, .. } if *id == source)).unwrap()
}
fn activate(game: &mut GameState, actor: PlayerId, source: ObjectId, dm: &mut Choices) {
    game.turn.priority_player = Some(actor);
    let action = activation(game, actor, source);
    announce(game, actor, action, dm);
}
fn queue_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) -> usize {
    let entries = check_triggers(game, &event);
    let count = entries.len();
    let mut queue = TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
}
fn current(game: &GameState, stable: ironsmith::ids::StableId) -> ObjectId { game.find_object_by_stable_id(stable).unwrap() }
fn tokens(game: &GameState) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token).collect()
}
/// Match through the public target validator: a non-target object spec is
/// exactly the filter evaluated in the context's filter view.
fn filter_matches(game: &GameState, ctx: &EffectContext, filter: &ironsmith::target::ObjectFilter, object: ironsmith::ObjectId) -> bool {
    ironsmith::effects::validate_target(game, &ironsmith::effects::ResolvedTarget::Object(object),
        &ChooseSpec::Object(filter.clone()), ctx)
}
#[test]
fn every_frozen_body_retains_name_relations_and_its_secondary_rules() {
    for (name, markers) in [
        ("Canoptek Wraith", vec!["can't be blocked", "combat damage", "sacrifice", "basic land", "tapped", "shuffle"]),
        ("Cylian Sunsinger", vec!["+3/+3", "end of turn"]),
        ("Extraplanar Lens", vec!["exile", "mana", "produced"]),
        ("Grim Reminder", vec!["reveal", "6 life", "shuffle", "graveyard", "upkeep"]),
        ("Invader Parasite", vec!["exile", "opponent", "2 damage"]),
        ("Strata Scythe", vec!["search", "exile", "shuffle", "equipped", "equip"]),
        ("The Apprentice's Folly", vec!["i", "ii", "iii", "reflection", "haste", "legendary", "sacrifice"]),
        ("Winnow", vec!["destroy", "draw a card"]),
        ("Yenna, Redtooth Regent", vec!["legendary", "aura", "untap", "scry 2", "sorcery"]),
    ] {
        for definition in definitions(name) {
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n").to_lowercase();
            assert!(rendered.contains("same name"), "{name}: {rendered}");
            for marker in &markers { assert!(rendered.contains(marker), "{name}: missing {marker}: {rendered}"); }
        }
    }
}
#[test]
fn sunsinger_pays_all_colors_and_uses_current_or_departed_source_name_once() {
    for definition in definitions("Cylian Sunsinger") {
        for depart in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let printed = object(&mut game, B, Zone::Battlefield, "Cylian Sunsinger", CardType::Creature);
            let current_name = object(&mut game, C, Zone::Battlefield, "Copied name", CardType::Creature);
            let noncreature = object(&mut game, A, Zone::Battlefield, "Copied name", CardType::Artifact);
            let before = game.player(A).unwrap().mana_pool.clone();
            activate(&mut game, A, source, &mut Choices::default());
            let after = &game.player(A).unwrap().mana_pool;
            assert_eq!((before.red - after.red, before.green - after.green, before.white - after.white), (1, 1, 1));
            set_name(&mut game, source, "Copied name");
            if depart { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            settle(&mut game, &mut Choices::default());
            if !depart { assert_eq!(game.current_power(source), Some(5), "source receives one bonus"); }
            assert_eq!(game.current_power(current_name), Some(5));
            assert_eq!(game.current_power(printed), Some(2));
            assert_eq!(game.current_power(noncreature), Some(2));
            let late = object(&mut game, A, Zone::Battlefield, "Copied name", CardType::Creature);
            assert_eq!(game.current_power(late), Some(2), "recipient set is fixed on resolution");
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_power(current_name), Some(2));
        }
    }
}
#[test]
fn winnow_checks_another_current_name_on_resolution_and_draws_even_when_false() {
    for definition in definitions("Winnow") {
        for case in 0..6 {
            let mut game = game();
            let target = object(&mut game, B, Zone::Battlefield, if case == 3 { "" } else { "Victim" }, CardType::Artifact);
            let target_stable = game.object(target).unwrap().stable_id;
            let donor = object(&mut game, C, Zone::Battlefield, if case == 3 { "" } else if case == 5 { "First // Victim" } else { "Other" }, CardType::Land);
            if case == 1 || case == 2 { set_name(&mut game, donor, "Victim"); }
            object(&mut game, A, Zone::Library, "Drawn", CardType::Instant);
            let mut dm = Choices { target: Some(Target::Object(target)), ..Default::default() };
            cast(&mut game, A, &definition, &mut dm);
            if case == 2 { game.move_object_by_effect(donor, Zone::Hand).unwrap(); }
            if case == 4 { game.move_object_by_effect(target, Zone::Hand).unwrap(); }
            settle(&mut game, &mut dm);
            assert_eq!(game.object(current(&game, target_stable)).unwrap().zone,
                if case == 1 || case == 5 { Zone::Graveyard } else if case == 4 { Zone::Hand } else { Zone::Battlefield });
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(case != 4));
        }
    }
}
#[test]
fn lens_optional_imprint_is_exact_and_bonus_mana_belongs_to_the_tapping_controller() {
    for definition in definitions("Extraplanar Lens") {
        for accept in [false, true] {
            let mut game = game();
            let imprint = land(&mut game, A, Zone::Battlefield, "Imprinted name", false);
            let stable = game.object(imprint).unwrap().stable_id;
            let own = land(&mut game, A, Zone::Battlefield, "Imprinted name", false);
            let enemy = land(&mut game, B, Zone::Battlefield, "Other printed name", false);
            set_name(&mut game, enemy, "Imprinted name");
            let mut dm = Choices { target: Some(Target::Object(imprint)), accept: Some(accept), ..Default::default() };
            let source = cast(&mut game, A, &definition, &mut dm);
            settle(&mut game, &mut dm);
            let source = current(&game, source);
            assert_eq!(game.get_exiled_with_source_links(source).len(), usize::from(accept));
            assert_eq!(game.object(current(&game, stable)).unwrap().zone, if accept { Zone::Exile } else { Zone::Battlefield });
            for (actor, land) in [(A, own), (B, enemy)] {
                let before = game.player(actor).unwrap().mana_pool.green;
                activate(&mut game, actor, land, &mut Choices::default());
                settle(&mut game, &mut Choices::default());
                assert_eq!(game.player(actor).unwrap().mana_pool.green - before, if accept { 2 } else { 1 });
            }
            if accept {
                let old = current(&game, stable);
                let hand = game.move_object_by_effect(old, Zone::Hand).unwrap();
                game.move_object_by_effect(hand, Zone::Exile).unwrap();
                let later = land(&mut game, C, Zone::Battlefield, "Imprinted name", false);
                let before = game.player(C).unwrap().mana_pool.green;
                activate(&mut game, C, later, &mut Choices::default());
                assert_eq!(game.player(C).unwrap().mana_pool.green - before, 1, "a later unlinked incarnation grants nothing");
            }
        }
    }
}
#[test]
fn parasite_imprint_damages_only_matching_opponent_land_controller_and_keeps_queued_trigger() {
    for definition in definitions("Invader Parasite") {
        let mut game = game();
        let imprint = land(&mut game, C, Zone::Battlefield, "Imprinted name", false);
        let mut dm = Choices { target: Some(Target::Object(imprint)), ..Default::default() };
        let stable = cast(&mut game, A, &definition, &mut dm);
        settle(&mut game, &mut dm);
        let source = current(&game, stable);
        for (actor, name, expected) in [(A, "Imprinted name", 20), (B, "Other", 20), (C, "Imprinted name", 18)] {
            game.turn.active_player = actor;
            let land = land(&mut game, actor, Zone::Hand, name, false);
            announce(&mut game, actor, LegalAction::PlayLand { land_id: land }, &mut Choices::default());
            if actor == C { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            settle(&mut game, &mut Choices::default());
            assert_eq!(game.player(actor).unwrap().life, expected);
        }
        let departed = current(&game, stable);
        let successor = game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
        assert!(game.get_exiled_with_source_links(successor).is_empty());
    }
}
#[test]
fn scythe_search_equips_and_counts_all_players_current_matching_land_names() {
    for definition in definitions("Strata Scythe") {
        let mut game = game();
        let chosen = land(&mut game, A, Zone::Library, "Imprinted name", false);
        let chosen_stable = game.object(chosen).unwrap().stable_id;
        let host = object(&mut game, A, Zone::Battlefield, "Host", CardType::Creature);
        let first = land(&mut game, B, Zone::Battlefield, "Imprinted name", false);
        let second = land(&mut game, C, Zone::Battlefield, "Other", false);
        object(&mut game, B, Zone::Battlefield, "Imprinted name", CardType::Artifact);
        let mut dm = Choices { objects: VecDeque::from([vec![chosen]]), ..Default::default() };
        let stable = cast(&mut game, A, &definition, &mut dm);
        settle(&mut game, &mut dm);
        let source = current(&game, stable);
        assert_eq!(game.get_exiled_with_source_links(source), &[current(&game, chosen_stable)]);
        dm.target = Some(Target::Object(host));
        let before = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, A, source, &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 3);
        assert_eq!(game.current_power(host), Some(3));
        set_name(&mut game, second, "Imprinted name");
        assert_eq!(game.current_power(host), Some(4));
        game.phase_out(first);
        assert_eq!(game.current_power(host), Some(3));
        let exiled = current(&game, chosen_stable);
        game.move_object_by_effect(exiled, Zone::Hand).unwrap();
        assert_eq!(game.current_power(host), Some(2));
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(game.current_power(host), Some(2));
    }
}
#[test]
fn canoptek_combat_trigger_pays_and_sacrifices_before_one_choice_and_bounded_basic_search() {
    for definition in definitions("Canoptek Wraith") {
        for accept in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let stable = game.object(source).unwrap().stable_id;
            assert!(!game.can_be_blocked(source));
            let chosen = land(&mut game, A, Zone::Battlefield, "Chosen name", false);
            let first = land(&mut game, A, Zone::Library, "Chosen name", true);
            let second = land(&mut game, A, Zone::Library, "Chosen name", true);
            let first_stable = game.object(first).unwrap().stable_id;
            let second_stable = game.object(second).unwrap().stable_id;
            let nonbasic = land(&mut game, A, Zone::Library, "Chosen name", false);
            let wrong = land(&mut game, A, Zone::Library, "Wrong name", true);
            let mut dm = Choices { accept: Some(accept), objects: VecDeque::from([vec![chosen], vec![first, second]]), ..Default::default() };
            let event = |combat| TriggerEvent::new(ironsmith::events::DamageEvent::with_cause(
                source, ironsmith::events::DamageTarget::Player(B), 2, combat,
                ironsmith::events::EventCause::from_effect(source, A)), Default::default());
            game.take_pending_trigger_events();
            assert_eq!(queue_event(&mut game, event(false), &mut dm), 0);
            let before = game.player(A).unwrap().mana_pool.total();
            assert_eq!(queue_event(&mut game, event(true), &mut dm), 1);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - if accept { 3 } else { 0 });
            assert_eq!(game.object(current(&game, stable)).unwrap().zone, if accept { Zone::Graveyard } else { Zone::Battlefield });
            for stable in [first_stable, second_stable] {
                let id = current(&game, stable);
                assert_eq!(game.object(id).unwrap().zone, if accept { Zone::Battlefield } else { Zone::Library });
                if accept { assert!(game.is_tapped(id)); }
            }
            assert_eq!(game.object(nonbasic).unwrap().zone, Zone::Library);
            assert_eq!(game.object(wrong).unwrap().zone, Zone::Library);
            assert_eq!(dm.objects.len(), if accept { 0 } else { 2 }, "chosen land is reused, never chosen again inside search");
        }
    }
}
#[test]
fn grim_reminder_uses_per_opponent_cast_snapshots_and_returns_only_during_its_controllers_upkeep() {
    for definition in definitions("Grim Reminder") {
        let mut game = game();
        for (actor, name) in [(A, "Revealed name"), (B, "Revealed name"), (C, "Other name")] {
            let spell = compile_to_runtime_definition(name, "Mana cost: {0}\nType: Instant\nYou gain 1 life.", false).unwrap();
            cast(&mut game, actor, &spell, &mut Choices::default());
            settle(&mut game, &mut Choices::default());
        }
        let revealed = object(&mut game, A, Zone::Library, "Revealed name", CardType::Creature);
        let mut dm = Choices { objects: VecDeque::from([vec![revealed]]), ..Default::default() };
        let stable = cast(&mut game, A, &definition, &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!((game.player(A).unwrap().life, game.player(B).unwrap().life, game.player(C).unwrap().life), (21, 15, 21));
        assert_eq!(game.object(revealed).unwrap().zone, Zone::Library, "search only reveals and then shuffles");
        assert!(dm.views.iter().any(|(_, public, cards)| *public && cards.contains(&revealed)));
        let grave = current(&game, stable);
        let return_is_legal = |game: &GameState| compute_legal_actions(game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source, .. } if *source == grave));
        assert!(!return_is_legal(&game));
        game.turn.phase = ironsmith::Phase::Beginning;
        game.turn.step = Some(ironsmith::Step::Upkeep);
        game.turn.active_player = B;
        assert!(!return_is_legal(&game));
        game.turn.active_player = A;
        assert!(return_is_legal(&game));
        let black = game.player(A).unwrap().mana_pool.black;
        activate(&mut game, A, grave, &mut Choices::default());
        settle(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().mana_pool.black, black - 2);
        assert_eq!(game.object(current(&game, stable)).unwrap().zone, Zone::Hand);
    }
}
#[test]
fn folly_two_chapters_restrict_names_copy_exceptions_and_sacrifice_only_controlled_reflections() {
    for definition in definitions("The Apprentice's Folly") {
        let mut game = game();
        let donor_definition = compile_to_runtime_definition("Legendary donor", "Type: Legendary Creature — Elf\nPower/Toughness: 3/3", false).unwrap();
        let first = game.create_object_from_definition(&donor_definition, A, Zone::Battlefield);
        object(&mut game, A, Zone::Battlefield, "Legendary donor", CardType::Artifact);
        let second = object(&mut game, A, Zone::Battlefield, "Second donor", CardType::Creature);
        let enemy = object(&mut game, B, Zone::Battlefield, "Enemy Reflection", CardType::Creature);
        game.object_mut(enemy).unwrap().subtypes.push(Subtype::Reflection);
        let natural = object(&mut game, A, Zone::Battlefield, "Natural Reflection", CardType::Creature);
        game.object_mut(natural).unwrap().subtypes.push(Subtype::Reflection);
        let mut dm = Choices { target: Some(Target::Object(first)), ..Default::default() };
        let stable = cast(&mut game, A, &definition, &mut dm);
        settle(&mut game, &mut dm);
        let source = current(&game, stable);
        let first_copy = tokens(&game);
        assert_eq!(first_copy.len(), 1);
        let chars = game.current_characteristics(first_copy[0]).unwrap();
        assert_eq!(chars.name.as_str(), "Legendary donor");
        assert!(!chars.supertypes.contains(&Supertype::Legendary));
        assert!(chars.subtypes.contains(&Subtype::Elf) && chars.subtypes.contains(&Subtype::Reflection));
        assert!(game.current_has_static_ability_id(first_copy[0], ironsmith::static_abilities::StaticAbilityId::Haste));
        assert_eq!(game.current_power(first_copy[0]), Some(3));
        dm.target = Some(Target::Object(second));
        dm.excluded_targets = vec![Target::Object(first), Target::Object(first_copy[0]), Target::Object(enemy)];
        let chapter_two = game.add_counters(source, CounterType::Lore, 1).unwrap();
        assert_eq!(queue_event(&mut game, chapter_two, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert_eq!(tokens(&game).len(), 2);
        dm.target = None;
        dm.excluded_targets.clear();
        let chapter_three = game.add_counters(source, CounterType::Lore, 1).unwrap();
        assert_eq!(queue_event(&mut game, chapter_three, &mut dm), 1);
        settle(&mut game, &mut dm);
        assert!(tokens(&game).is_empty());
        assert!(game.battlefield.contains(&first) && game.battlefield.contains(&second));
        assert!(game.battlefield.contains(&enemy));
        assert!(!game.battlefield.contains(&natural));
        assert_eq!(game.object(current(&game, stable)).unwrap().zone, Zone::Graveyard);
    }
}
#[test]
fn yenna_rechecks_unique_name_and_copies_aura_with_untap_and_scry_only_for_that_result() {
    for definition in definitions("Yenna, Redtooth Regent") {
        for case in 0..3 {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            object(&mut game, A, Zone::Library, "First scry card", CardType::Instant);
            object(&mut game, A, Zone::Library, "Second scry card", CardType::Instant);
            let host = object(&mut game, A, Zone::Battlefield, "Aura host", CardType::Creature);
            let donor = if case == 1 {
                let aura = compile_to_runtime_definition("Aura donor", "Type: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+1.", false).unwrap();
                let id = game.create_object_from_definition(&aura, A, Zone::Battlefield);
                game.attach_object_to_target(id, ironsmith::object::AttachmentTarget::Object(host));
                id
            } else {
                let donor = compile_to_runtime_definition("Legendary enchantment", "Type: Legendary Enchantment", false).unwrap();
                game.create_object_from_definition(&donor, A, Zone::Battlefield)
            };
            let mut dm = Choices { target: Some(Target::Object(donor)), ..Default::default() };
            let before = game.player(A).unwrap().mana_pool.total();
            activate(&mut game, A, source, &mut dm);
            assert!(game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 2);
            if case == 2 { object(&mut game, A, Zone::Battlefield, "Legendary enchantment", CardType::Artifact); }
            dm.target = None;
            settle(&mut game, &mut dm);
            let copies = tokens(&game);
            assert_eq!(copies.len(), usize::from(case != 2));
            if case != 2 {
                assert!(!game.current_characteristics(copies[0]).unwrap().supertypes.contains(&Supertype::Legendary));
            }
            if case == 1 {
                assert!(game.object(copies[0]).unwrap().attached_to.is_some());
                assert!(!game.is_tapped(source));
                assert_eq!(dm.partitions, [(A, 2)]);
            } else {
                assert!(game.is_tapped(source));
                assert!(dm.partitions.is_empty());
            }
            game.turn.active_player = B;
            assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
                matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)), "sorcery timing remains enforced");
        }
    }
}
#[test]
fn live_name_sets_and_frozen_tags_distinguish_split_nameless_copy_and_departure_names() {
    use ironsmith::target::{ObjectFilter, TaggedOpbjectRelation};
    use ironsmith_core::{ObjectCharacteristic, ObjectCharacteristicRelation};
    let mut game = game();
    let source = object(&mut game, A, Zone::Battlefield, "Original name", CardType::Creature);
    let candidate = object(&mut game, B, Zone::Battlefield, "Copied name", CardType::Creature);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    let mut ctx = EffectContext::new_default(source, A);
    ctx.tagged_objects.insert("captured".into(), vec![snapshot]);
    let live = ObjectFilter::creature().match_tagged(ironsmith_core::SOURCE_OBJECT_TAG, TaggedOpbjectRelation::SameNameAsTagged);
    let captured = ObjectFilter::creature().match_tagged("captured", TaggedOpbjectRelation::SameNameAsTagged);
    assert!(!filter_matches(&game, &ctx, &live, candidate));
    let model = object(&mut game, C, Zone::Battlefield, "Copied name", CardType::Creature);
    let copiable = ironsmith::snapshot::CopiableValues::from_object(game.object(model).unwrap());
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(source), Modification::CopyOf {
        target_id: model, copiable_values: Box::new(copiable), preserve_source_abilities: false,
        name_override: None, name_override_surface: None, add_supertypes: Vec::new(),
    }, Until::EndOfTurn).execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
    assert!(filter_matches(&game, &ctx, &live, candidate));
    assert!(!filter_matches(&game, &ctx, &captured, candidate));
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert!(filter_matches(&game, &ctx, &live, candidate), "exact source LKI retains the copied name");
    let split = object(&mut game, A, Zone::Graveyard, "First half // Second half", CardType::Instant);
    let half = object(&mut game, B, Zone::Hand, "Second half", CardType::Instant);
    let mut relation = ObjectCharacteristicRelation::shares(vec![ObjectCharacteristic::Name], ObjectFilter::default().in_zone(Zone::Graveyard));
    relation.exclude_candidate = true;
    let mut filter = ObjectFilter::default();
    filter.characteristic_relations.push(relation);
    assert!(filter_matches(&game, &ctx, &filter, half));
    game.move_object_by_effect(split, Zone::Exile).unwrap();
    assert!(!filter_matches(&game, &ctx, &filter, half));
    let nameless = object(&mut game, A, Zone::Graveyard, "", CardType::Instant);
    let no_name = object(&mut game, B, Zone::Hand, "", CardType::Instant);
    assert!(!filter_matches(&game, &ctx, &filter, no_name));
    assert!(game.object(nameless).is_some());
}
#[test]
fn unavailable_name_evidence_cannot_turn_negative_comparison_into_a_legal_target() {
    for definition in definitions("Yenna, Redtooth Regent") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let donor = object(&mut game, A, Zone::Battlefield, "Unique name", CardType::Enchantment);
        let mut dm = Choices { target: Some(Target::Object(donor)), ..Default::default() };
        activate(&mut game, A, source, &mut dm);
        let stack = game.stack.iter().map(|entry| entry.object_id).collect::<Vec<_>>();
        let battlefield = game.battlefield.clone();
        let pool = game.player(B).unwrap().mana_pool.clone();
        game.player_mut(B).unwrap().mana_pool.blue = u32::MAX;
        assert!(resolve_stack_entry_with(&mut game, &mut dm).is_err());
        assert_eq!(game.stack.iter().map(|entry| entry.object_id).collect::<Vec<_>>(), stack);
        assert_eq!(game.battlefield, battlefield);
        assert!(tokens(&game).is_empty());
        game.player_mut(B).unwrap().mana_pool = pool;
        settle(&mut game, &mut dm);
        assert_eq!(tokens(&game).len(), 1);
    }
}
#[test]
fn yenna_comparison_is_other_controlled_permanents_and_allows_nameless_enchantments() {
    for definition in definitions("Yenna, Redtooth Regent") {
        for nameless in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let candidate = object(&mut game, A, Zone::Battlefield, if nameless { "" } else { "Unique" }, CardType::Enchantment);
            let illegal = object(&mut game, A, Zone::Battlefield, "Duplicated", CardType::Enchantment);
            object(&mut game, A, Zone::Battlefield, "Duplicated", CardType::Creature);
            object(&mut game, B, Zone::Battlefield, "Unique", CardType::Artifact);
            if nameless { object(&mut game, A, Zone::Battlefield, "", CardType::Artifact); }
            let mut dm = Choices {
                target: Some(Target::Object(candidate)),
                excluded_targets: vec![Target::Object(illegal)],
                ..Default::default()
            };
            activate(&mut game, A, source, &mut dm);
            settle(&mut game, &mut dm);
            assert_eq!(tokens(&game).len(), 1);
        }
    }
}
#[test]
fn lens_adds_the_replacement_produced_type_to_the_original_recipient() {
    for definition in definitions("Extraplanar Lens") {
        let mut game = game();
        let imprint = land(&mut game, A, Zone::Battlefield, "Imprinted", false);
        let producer = land(&mut game, B, Zone::Battlefield, "Imprinted", false);
        let mut dm = Choices { target: Some(Target::Object(imprint)), ..Default::default() };
        cast(&mut game, A, &definition, &mut dm);
        settle(&mut game, &mut dm);
        let rule = ironsmith_core::ManaOutputRewrite {
            source_filter: ironsmith::ObjectFilter::land(), controller: None, tapped_for_mana: true,
            input: ironsmith_core::ManaRewriteInput::Any,
            output: ironsmith_core::ManaRewriteOutput::Symbol(ManaSymbol::Blue),
            quantity: ironsmith_core::ManaRewriteQuantity::Preserve,
        };
        let host = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Production replacement")
            .card_types(vec![CardType::Enchantment])
            .with_ability(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::mana_production_rewrite(
                rule, "If a land is tapped for mana, it produces {U} instead of any other type."))).build();
        game.create_object_from_definition(&host, A, Zone::Battlefield);
        let before = game.player(B).unwrap().mana_pool.clone();
        activate(&mut game, B, producer, &mut Choices::default());
        settle(&mut game, &mut Choices::default());
        let after = &game.player(B).unwrap().mana_pool;
        assert_eq!((after.blue - before.blue, after.green - before.green), (2, 0));
    }
}
#[test]
fn checked_history_uses_cast_actor_and_rejects_missing_or_mismatched_characteristics() {
    use ironsmith::effect::{Effect, Value};
    use ironsmith::effects::{execute_effect, ExecutionError};
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    for case in 0..5 {
        let mut game = game();
        let source = object(&mut game, A, Zone::Battlefield, "Ability source", CardType::Artifact);
        let cast_spell = object(&mut game, C, Zone::Stack, "Matched cast", CardType::Instant);
        if case != 0 {
            let mut snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(cast_spell).unwrap(), &game);
            if case == 3 { snapshot.object_id = source; }
            if case == 4 { snapshot.zone = Zone::Hand; }
            let mut event = ironsmith::events::SpellCastEvent::new(cast_spell, B, Zone::Hand);
            if case != 2 { event.snapshot = Some(snapshot); }
            let event = TriggerEvent::new(event, Default::default());
            game.turn_store.turn_history.record_event(&event, None, None);
        }
        let query = |actor| Value::SpellsCastThisTurnMatching {
            player: PlayerFilter::Specific(actor), filter: ObjectFilter::spell().named("Matched cast"), exclude_source: false,
        };
        let mut ctx = EffectContext::new_default(source, A);
        let result = execute_effect(&mut game, &Effect::gain_life(query(B)), &mut ctx);
        if case >= 2 {
            assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))));
            assert_eq!(game.player(A).unwrap().life, 20);
        } else {
            result.unwrap();
            assert_eq!(game.player(A).unwrap().life, if case == 0 { 20 } else { 21 });
            execute_effect(&mut game, &Effect::gain_life(query(C)), &mut ctx).unwrap();
            assert_eq!(game.player(A).unwrap().life, if case == 0 { 20 } else { 21 }, "the captured spell controller is not the caster");
        }
    }
}
#[test]
fn grim_failed_find_is_a_known_empty_name_set_and_still_shuffles() {
    for definition in definitions("Grim Reminder") {
        let mut game = game();
        let spell = compile_to_runtime_definition("Possible name", "Mana cost: {0}\nType: Instant\nYou gain 1 life.", false).unwrap();
        cast(&mut game, B, &spell, &mut Choices::default());
        settle(&mut game, &mut Choices::default());
        object(&mut game, A, Zone::Library, "Possible name", CardType::Creature);
        let observer = compile_to_runtime_definition("Shuffle witness", "Type: Enchantment\nWhenever you shuffle your library, you gain 1 life.", false).unwrap();
        game.create_object_from_definition(&observer, A, Zone::Battlefield);
        let mut dm = Choices { objects: VecDeque::from([Vec::new()]), ..Default::default() };
        cast(&mut game, A, &definition, &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!((game.player(A).unwrap().life, game.player(B).unwrap().life, game.player(C).unwrap().life), (21, 21, 20));
        assert!(dm.views.iter().all(|(_, public, _)| !public));
        assert!(dm.objects.is_empty());
    }
}
#[test]
fn renamed_or_copied_split_reference_cannot_keep_its_printed_alternate_name() {
    use ironsmith::target::{ObjectFilter, TaggedOpbjectRelation};
    use ironsmith_core::{ObjectCharacteristic, ObjectCharacteristicRelation};
    for copy in [false, true] {
        let mut game = game();
        let source = object(&mut game, A, Zone::Battlefield, "Source", CardType::Artifact);
        let split = object(&mut game, A, Zone::Exile, "First half", CardType::Instant);
        game.object_mut(split).unwrap().linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
        game.object_mut(split).unwrap().other_face_name = Some("Old second half".into());
        game.add_exiled_with_source_link(source, split);
        let candidate = object(&mut game, A, Zone::Hand, "Old second half", CardType::Instant);
        let ctx = EffectContext::new_default(source, A);
        let linked = ObjectFilter::default().match_tagged(ironsmith_core::SOURCE_EXILED_TAG, TaggedOpbjectRelation::SameNameAsTagged);
        let mut live = ObjectFilter::default();
        live.characteristic_relations.push(ObjectCharacteristicRelation::shares(vec![ObjectCharacteristic::Name], ObjectFilter::default().in_zone(Zone::Exile)));
        assert!(filter_matches(&game, &ctx, &linked, candidate));
        assert!(filter_matches(&game, &ctx, &live, candidate));
        if copy {
            // The primary name is deliberately unchanged: name equality is
            // insufficient evidence that printed alternate names survived.
            let model = object(&mut game, B, Zone::Battlefield, "First half", CardType::Artifact);
            let values = ironsmith::snapshot::CopiableValues::from_object(game.object(model).unwrap());
            ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(split), Modification::CopyOf {
                target_id: model, copiable_values: Box::new(values), preserve_source_abilities: false,
                name_override: None, name_override_surface: None, add_supertypes: Vec::new(),
            }, Until::EndOfTurn).execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
        } else { set_name(&mut game, split, "Changed name"); }
        assert!(!filter_matches(&game, &ctx, &linked, candidate));
        assert!(!filter_matches(&game, &ctx, &live, candidate));
        let retained = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(split).unwrap(), &game);
        assert_eq!(retained.split_other_half_name(), None);
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state().unwrap();
        assert!(filter_matches(&game, &ctx, &linked, candidate));
    }
}
#[test]
fn yenna_zero_and_departed_created_results_never_fall_back_to_the_aura_donor() {
    for definition in definitions("Yenna, Redtooth Regent") {
        for departure in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let host = object(&mut game, A, Zone::Battlefield, "Host", CardType::Creature);
            let aura = compile_to_runtime_definition("Aura donor", "Type: Enchantment — Aura\nEnchant creature", false).unwrap();
            let donor = game.create_object_from_definition(&aura, A, Zone::Battlefield);
            game.attach_object_to_target(donor, ironsmith::object::AttachmentTarget::Object(host));
            object(&mut game, A, Zone::Library, "First", CardType::Instant);
            object(&mut game, A, Zone::Library, "Second", CardType::Instant);
            let replacement = if departure {
                ironsmith::replacement::ReplacementAction::Additionally(vec![ironsmith::Effect::exile(
                    ChooseSpec::All(ironsmith::ObjectFilter::default().token()))])
            } else { ironsmith::replacement::ReplacementAction::Prevent };
            game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
                source, A, ironsmith::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(ironsmith::PlayerFilter::You), replacement));
            let mut dm = Choices { target: Some(Target::Object(donor)), ..Default::default() };
            activate(&mut game, A, source, &mut dm);
            dm.target = None;
            settle(&mut game, &mut dm);
            assert!(tokens(&game).is_empty());
            assert_eq!(game.is_tapped(source), !departure);
            assert_eq!(dm.partitions.len(), usize::from(departure));
            if departure { assert_eq!(dm.partitions, [(A, 2)]); }
            assert!(game.battlefield.contains(&donor));
        }
    }
}
#[test]
fn yenna_does_not_untap_or_scry_when_the_aura_copy_has_no_legal_attachment() {
    use ironsmith::ability::{Ability, ProtectionFrom};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith::target::ObjectFilter;
    for definition in definitions("Yenna, Redtooth Regent") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let host_definition = CardDefinitionBuilder::new(CardId::new(), "Only host")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::protection(ProtectionFrom::Permanents(ObjectFilter::default().token())))).build();
        let host = game.create_object_from_definition(&host_definition, A, Zone::Battlefield);
        let aura = CardDefinitionBuilder::new(CardId::new(), "Restricted Aura").card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura]).enchants(ObjectFilter::creature().named("Only host")).build();
        let donor = game.create_object_from_definition(&aura, A, Zone::Battlefield);
        game.attach_object_to_target(donor, ironsmith::object::AttachmentTarget::Object(host));
        let mut dm = Choices { target: Some(Target::Object(donor)), ..Default::default() };
        activate(&mut game, A, source, &mut dm);
        dm.target = None;
        settle(&mut game, &mut dm);
        assert!(tokens(&game).is_empty());
        assert!(game.is_tapped(source));
        assert!(dm.partitions.is_empty());
        assert!(game.battlefield.contains(&donor));
    }
}
#[test]
fn created_token_characteristic_predicate_uses_departure_not_a_new_zone_record() {
    let mut game = game();
    let source = object(&mut game, A, Zone::Battlefield, "Ability source", CardType::Artifact);
    let token = object(&mut game, A, Zone::Battlefield, "Created Aura", CardType::Enchantment);
    game.object_mut(token).unwrap().kind = ObjectKind::Token;
    game.object_mut(token).unwrap().subtypes.push(Subtype::Aura);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(token).unwrap(), &game);
    let mut ctx = EffectContext::new_default(source, A);
    ctx.set_tagged_objects("created", vec![snapshot]);
    let successor = game.move_object_by_effect(token, Zone::Exile).unwrap();
    game.object_mut(successor).unwrap().subtypes.clear();
    let condition = ironsmith::ConditionExpr::TaggedObjectMatches("created".into(), ironsmith::ObjectFilter::default().with_subtype(Subtype::Aura));
    assert!(ironsmith::condition_eval::evaluate_condition_resolution(&game, &condition, &ctx).unwrap());
}
