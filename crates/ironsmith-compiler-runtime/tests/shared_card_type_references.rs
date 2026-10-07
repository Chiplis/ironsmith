//! Full frozen bodies and real native actions. Restored scenarios, all UNRUN.
//! Three review candidates; the four other frozen family members stay partial.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::Modification;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, ViewCardsContext};
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::ObjectKind;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const CEMETERY: [&str; 2] = ["Cemetery Gatekeeper", "Cemetery Protector"];
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/shared_card_type_references.json.fixture")).unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {p}/{t}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored: CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap(); assert_eq!(artifact, restored);
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    assert!(!loss.is_lossy(), "{name} direct: {}", loss.reasons_text());
    [result.unwrap(), materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    for player in [A, B, C] {
        game.player_mut(player).unwrap().land_plays_per_turn = 8;
        for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(color, 10);
        }
    }
    game
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, kinds: Vec<CardType>) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), name).card_types(kinds)
        .power_toughness(PowerToughness::fixed(2, 2)).build(), owner, zone)
}
#[derive(Default)]
struct Choices {
    chosen: Option<ObjectId>, accept: Option<bool>,
    views: Vec<(PlayerId, bool, Vec<ObjectId>)>,
}
impl DecisionMaker for Choices {
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.chosen {
            assert!(ctx.candidates.iter().any(|candidate| candidate.id == id && candidate.legal));
            vec![id]
        } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool { self.accept.unwrap_or(true) && ctx.can_accept }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId], ctx: &ViewCardsContext) {
        self.views.push((viewer, ctx.public, cards.to_vec()));
    }
}
fn announce(game: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(player);
    assert!(compute_legal_actions(game, player).unwrap().contains(&action));
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(game.players.len());
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("unfinished action: {progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    announce(game, A, LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand, casting_method: CastingMethod::Normal }, dm);
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("unexpected nonterminating trigger chain");
}
fn named(game: &GameState, name: &str) -> ObjectId { *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == name).unwrap() }
fn set_types(game: &mut GameState, object: ObjectId, types: Vec<CardType>) {
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(object), Modification::SetCardTypes(types), Until::EndOfTurn)
        .execute(game, &mut EffectContext::new_default(object, A)).unwrap();
}
fn prepare(definition: &CardDefinition, types: Vec<CardType>) -> (GameState, ObjectId, ObjectId) {
    let mut game = game();
    let donor = card(&mut game, B, Zone::Graveyard, "Linked graveyard card", types);
    let stable = game.object(donor).unwrap().stable_id;
    let mut dm = Choices { chosen: Some(donor), ..Default::default() };
    cast(&mut game, definition, &mut dm); settle(&mut game, &mut dm);
    let source = named(&game, &definition.card.name);
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    assert_eq!(game.get_exiled_with_source_links(source), &[exiled]);
    (game, source, exiled)
}
fn play_land(game: &mut GameState, actor: PlayerId) -> ObjectId {
    game.turn.active_player = actor;
    let land = card(game, actor, Zone::Hand, "Played land", vec![CardType::Land]);
    let stable = game.object(land).unwrap().stable_id;
    announce(game, actor, LegalAction::PlayLand { land_id: land }, &mut Choices::default());
    game.find_object_by_stable_id(stable).unwrap()
}
fn cast_artifact(game: &mut GameState, actor: PlayerId) -> ObjectId {
    game.turn.active_player = actor;
    let def = compile_to_runtime_definition("Played artifact", "Mana cost: {0}\nType: Artifact", false).unwrap();
    let spell = game.create_object_from_definition(&def, actor, Zone::Hand);
    let stable = game.object(spell).unwrap().stable_id;
    announce(game, actor, LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand, casting_method: CastingMethod::Normal }, &mut Choices::default());
    game.find_object_by_stable_id(stable).unwrap()
}
fn humans(game: &GameState) -> usize {
    game.battlefield.iter().filter(|id| game.object(**id).unwrap().kind == ObjectKind::Token && game.current_has_subtype(**id, Subtype::Human)).count()
}
fn reward(game: &GameState, name: &str, count: i32) {
    assert_eq!(game.player(A).unwrap().life, if name == "Cemetery Gatekeeper" { 20 - 2 * count } else { 20 });
    assert_eq!(humans(game), if name == "Cemetery Protector" { count as usize } else { 0 });
}
#[test]
fn three_full_frozen_bodies_keep_semantic_relations_and_secondary_abilities() {
    for name in [CEMETERY[0], CEMETERY[1], "Amareth, the Lustrous"] { for definition in definitions(name) {
        let text = definition.canonical_text.to_lowercase();
        assert!(text.contains("shares a card type"), "{name}: {text}");
        assert!(text.contains(match name { "Cemetery Gatekeeper" => "first strike", "Cemetery Protector" => "flash", _ => "flying" }), "{text}");
        if name.starts_with("Cemetery") { for marker in ["exile", "graveyard", "play", "cast"] { assert!(text.contains(marker), "{text}"); } }
        else { for marker in ["another", "that permanent", "reveal", "hand"] { assert!(text.contains(marker), "{text}"); } }
    }}
}
#[test]
fn gatekeeper_repeated_land_and_spell_notices_damage_each_original_event_actor() {
    for def in definitions(CEMETERY[0]) {
        let (mut game, _, _) = prepare(&def, vec![CardType::Artifact, CardType::Land]);
        for actor in [A, B, C, B] {
            let before = game.player(actor).unwrap().life;
            play_land(&mut game, actor); settle(&mut game, &mut Choices::default());
            cast_artifact(&mut game, actor); settle(&mut game, &mut Choices::default());
            assert_eq!(game.player(actor).unwrap().life, before - 4);
        }
        assert_eq!([game.player(A).unwrap().life, game.player(B).unwrap().life, game.player(C).unwrap().life], [16, 12, 16]);
    }
}
#[test]
fn protector_rewards_only_its_controllers_notices_with_white_one_one_humans() {
    for def in definitions(CEMETERY[1]) {
        let (mut game, _, _) = prepare(&def, vec![CardType::Artifact, CardType::Land]);
        for actor in [A, B, A, C] {
            let before = humans(&game);
            play_land(&mut game, actor); settle(&mut game, &mut Choices::default());
            cast_artifact(&mut game, actor); settle(&mut game, &mut Choices::default());
            assert_eq!(humans(&game), before + if actor == A { 2 } else { 0 });
        }
        for id in &game.battlefield { if game.object(*id).unwrap().kind == ObjectKind::Token {
            assert_eq!(game.current_controller(*id), Some(A));
            assert_eq!((game.current_power(*id), game.current_toughness(*id)), (Some(1), Some(1)));
            assert_eq!(game.current_colors(*id), Some(ironsmith::ColorSet::WHITE));
        }}
    }
}
#[test]
fn cemetery_exact_current_exile_types_gate_admission_and_resolution_and_do_not_follow_reexile() {
    for name in CEMETERY { for def in definitions(name) {
        let (mut game, _, exiled) = prepare(&def, vec![CardType::Artifact]);
        play_land(&mut game, A); settle(&mut game, &mut Choices::default()); reward(&game, name, 0);
        set_types(&mut game, exiled, vec![CardType::Land]);
        play_land(&mut game, A); assert_eq!(game.stack.len(), 1);
        set_types(&mut game, exiled, vec![CardType::Artifact]);
        settle(&mut game, &mut Choices::default()); reward(&game, name, 0);
        let moved = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(moved, Zone::Exile).unwrap(); assert_ne!(returned, exiled);
        cast_artifact(&mut game, A); settle(&mut game, &mut Choices::default()); reward(&game, name, 0);
    }}
}
#[test]
fn cemetery_empty_graveyards_do_not_borrow_another_sources_exiled_card() {
    for name in CEMETERY { for def in definitions(name) {
        let mut game = game();
        let other = card(&mut game, B, Zone::Battlefield, "Other source", vec![CardType::Artifact]);
        let exiled = card(&mut game, B, Zone::Exile, "Other link", vec![CardType::Land]);
        game.add_exiled_with_source_link(other, exiled);
        cast(&mut game, &def, &mut Choices::default()); settle(&mut game, &mut Choices::default());
        assert!(game.get_exiled_with_source_links(named(&game, name)).is_empty());
        play_land(&mut game, A); settle(&mut game, &mut Choices::default()); reward(&game, name, 0);
    }}
}
#[test]
fn cemetery_departure_and_phasing_read_exact_last_types_and_survive_source_loss() {
    for name in CEMETERY { for def in definitions(name) { for changed in [false, true] { for phase in [false, true] {
        let (mut game, source, _) = prepare(&def, vec![CardType::Land]);
        let land = play_land(&mut game, A); assert_eq!(game.stack.len(), 1);
        if changed { set_types(&mut game, land, vec![CardType::Artifact]); }
        if phase { game.phase_out(land); } else {
            let new = game.move_object_by_effect(land, Zone::Graveyard).unwrap();
            assert_eq!(game.current_card_types(new).unwrap(), vec![CardType::Land]);
        }
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        settle(&mut game, &mut Choices::default()); reward(&game, name, i32::from(!changed));
    }}}}
}
#[test]
fn gatekeeper_keeps_cast_actor_after_spell_control_changes() {
    for def in definitions(CEMETERY[0]) {
        let (mut game, source, _) = prepare(&def, vec![CardType::Artifact]);
        let spell = cast_artifact(&mut game, B); assert_eq!(game.stack.len(), 2);
        let stable = game.object(spell).unwrap().stable_id;
        game.set_current_controller(spell, C).unwrap();
        assert_eq!(game.current_controller(spell), Some(C));
        // The native stack-resolution owner refreshes the spell entry's
        // controller from current control; the cast notice retains caster B.
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        settle(&mut game, &mut Choices::default());
        assert_eq!((game.player(B).unwrap().life, game.player(C).unwrap().life), (18, 20));
        assert_eq!(game.current_controller(game.find_object_by_stable_id(stable).unwrap()), Some(C));
    }
}
#[test]
fn cemetery_copied_sources_enter_with_independent_exile_links() {
    for name in CEMETERY { for def in definitions(name) {
        let (mut game, original, original_exile) = prepare(&def, vec![CardType::Artifact]);
        let donor = card(&mut game, B, Zone::Graveyard, "Copy's land", vec![CardType::Land]);
        let stable = game.object(donor).unwrap().stable_id; let before = game.battlefield.clone();
        ironsmith::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(original))
            .execute(&mut game, &mut EffectContext::new_default(original, A)).unwrap();
        let copied = *game.battlefield.iter().find(|id| !before.contains(id) && game.object(**id).unwrap().kind == ObjectKind::Token).unwrap();
        assert!(game.get_exiled_with_source_links(copied).is_empty());
        settle(&mut game, &mut Choices { chosen: Some(donor), ..Default::default() });
        assert_eq!(game.get_exiled_with_source_links(original), &[original_exile]);
        assert_eq!(game.get_exiled_with_source_links(copied), &[game.find_object_by_stable_id(stable).unwrap()]);
        let before_tokens = humans(&game);
        play_land(&mut game, A); settle(&mut game, &mut Choices::default());
        cast_artifact(&mut game, A); settle(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, if name == CEMETERY[0] { 16 } else { 20 });
        assert_eq!(humans(&game), before_tokens + if name == CEMETERY[1] { 2 } else { 0 });
    }}
}
#[test]
fn cemetery_redirected_effect_driven_play_keeps_original_destination_and_actor() {
    for name in CEMETERY { for def in definitions(name) {
        let (mut game, source, _) = prepare(&def, vec![CardType::Land]);
        let land = card(&mut game, A, Zone::Exile, "Redirected land", vec![CardType::Land]);
        let stable = game.object(land).unwrap().stable_id;
        let selected = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(land).unwrap(), &game);
        let replacement = card(&mut game, B, Zone::Battlefield, "Replacement", vec![CardType::Artifact]);
        game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            replacement, B, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ironsmith::ObjectFilter { specific: Some(land), ..Default::default() }, Some(Zone::Exile), Some(Zone::Battlefield)),
            ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Graveyard)));
        let mut ctx = EffectContext::new_default(source, A); ctx.set_tagged_objects("play", vec![selected]);
        ironsmith::effects::CastTaggedEffect::new("play", ironsmith::PlayerFilter::You).allow_land().execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 1);
        settle(&mut game, &mut Choices::default()); reward(&game, name, 1);
        assert_eq!(game.player(B).unwrap().life, 20);
    }}
}
#[test]
fn cemetery_priority_and_direct_land_plays_publish_redirected_completion_without_etb() {
    for name in CEMETERY { for def in definitions(name) { for priority in [false, true] {
        let (mut game, _, _) = prepare(&def, vec![CardType::Land]);
        let land = card(&mut game, A, Zone::Hand, "Redirected ordinary land", vec![CardType::Land]);
        let stable = game.object(land).unwrap().stable_id;
        let replacement = card(&mut game, B, Zone::Battlefield, "Entry replacement", vec![CardType::Artifact]);
        game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            replacement, B, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ironsmith::ObjectFilter { specific: Some(land), ..Default::default() }, Some(Zone::Hand), Some(Zone::Battlefield)),
            ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Graveyard)));
        let entries_before = game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::EnterBattlefield);
        let plays_before = game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::LandPlayed);
        let mut dm = Choices::default();
        if priority {
            announce(&mut game, A, LegalAction::PlayLand { land_id: land }, &mut dm);
        } else {
            game.turn.priority_player = Some(A);
            ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand { card_id: land }, &mut game, A, &mut dm).unwrap();
        }
        let successor = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(successor, land);
        assert_eq!(game.object(successor).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 1);
        assert_eq!(game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::LandPlayed), plays_before + 1);
        assert_eq!(game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::EnterBattlefield), entries_before);
        settle(&mut game, &mut dm); reward(&game, name, 1);
        assert_eq!(game.player(B).unwrap().life, 20);
    }}}
}
fn amareth_entry(game: &mut GameState) -> ObjectId {
    let spell = cast_artifact(game, A); let stable = game.object(spell).unwrap().stable_id;
    resolve_stack_entry_with(game, &mut Choices::default()).unwrap();
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
    assert_eq!(game.stack.len(), 1); game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn amareth_self_and_opponents_entries_do_not_look_and_matching_choice_reveals_only_the_top() {
    for def in definitions("Amareth, the Lustrous") { for (matching, accept) in [(true, true), (true, false), (false, true)] {
        let mut game = game();
        let lower = card(&mut game, A, Zone::Library, "Lower", vec![CardType::Artifact]);
        let top = card(&mut game, A, Zone::Library, "Top", vec![if matching { CardType::Artifact } else { CardType::Land }]);
        let stable = game.object(top).unwrap().stable_id; let mut dm = Choices::default();
        cast(&mut game, &def, &mut dm); settle(&mut game, &mut dm); assert!(dm.views.is_empty());
        cast_artifact(&mut game, B); settle(&mut game, &mut dm); assert!(dm.views.is_empty());
        amareth_entry(&mut game); dm.accept = Some(accept); settle(&mut game, &mut dm);
        assert!(dm.views.iter().any(|(viewer, public, ids)| *viewer == A && !public && ids == &[top]));
        assert!(dm.views.iter().all(|(_, _, ids)| !ids.contains(&lower)));
        assert_eq!(dm.views.iter().filter(|(_, public, _)| *public).count(), if matching && accept { 3 } else { 0 });
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, if matching && accept { Zone::Hand } else { Zone::Library });
    }}
}
#[test]
fn amareth_current_departure_and_phasing_types_are_distinct_from_printed_new_zone_types() {
    for def in definitions("Amareth, the Lustrous") { for transition in [0, 1, 2] { for matching in [false, true] {
        let mut game = game(); cast(&mut game, &def, &mut Choices::default()); settle(&mut game, &mut Choices::default());
        let source = named(&game, "Amareth, the Lustrous");
        let top = card(&mut game, A, Zone::Library, "Compared top", vec![if matching { CardType::Enchantment } else { CardType::Artifact }]);
        let stable = game.object(top).unwrap().stable_id; let entrant = amareth_entry(&mut game);
        set_types(&mut game, entrant, vec![CardType::Enchantment]);
        if transition == 1 { let new = game.move_object_by_effect(entrant, Zone::Graveyard).unwrap(); assert_eq!(game.current_card_types(new).unwrap(), vec![CardType::Artifact]); }
        if transition == 2 { game.phase_out(entrant); }
        game.move_object_by_effect(source, Zone::Graveyard).unwrap(); settle(&mut game, &mut Choices::default());
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, if matching { Zone::Hand } else { Zone::Library });
    }}}
}
#[test]
fn amareth_copied_entry_types_and_empty_library_use_exact_known_evidence() {
    for def in definitions("Amareth, the Lustrous") {
        let mut game = game(); cast(&mut game, &def, &mut Choices::default()); settle(&mut game, &mut Choices::default());
        amareth_entry(&mut game); let mut dm = Choices::default(); settle(&mut game, &mut dm);
        assert!(dm.views.is_empty()); assert!(game.player(A).unwrap().hand.is_empty());
        card(&mut game, A, Zone::Library, "Land top", vec![CardType::Land]);
        let model = card(&mut game, B, Zone::Battlefield, "Land model", vec![CardType::Land]);
        let entrant = amareth_entry(&mut game);
        let values = ironsmith::snapshot::CopiableValues::from_object(game.object(model).unwrap());
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(entrant), Modification::CopyOf {
            target_id: model, copiable_values: Box::new(values), preserve_source_abilities: false,
            name_override: None, name_override_surface: None, add_supertypes: vec![],
        }, Until::EndOfTurn).execute(&mut game, &mut EffectContext::new_default(entrant, A)).unwrap();
        settle(&mut game, &mut Choices::default()); assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}
#[test]
fn all_three_resolution_failures_roll_back_and_recover_the_same_stacked_receipt() {
    for name in [CEMETERY[0], CEMETERY[1], "Amareth, the Lustrous"] { for def in definitions(name) {
        let mut game = if name.starts_with("Cemetery") { prepare(&def, vec![CardType::Land]).0 } else {
            let mut game = game(); cast(&mut game, &def, &mut Choices::default()); settle(&mut game, &mut Choices::default());
            card(&mut game, A, Zone::Library, "Recovery top", vec![CardType::Artifact]); game
        };
        if name.starts_with("Cemetery") { play_land(&mut game, A); } else { amareth_entry(&mut game); }
        let stack: Vec<_> = game.stack.iter().map(|entry| entry.object_id).collect();
        let battlefield = game.battlefield.clone(); let library = game.player(A).unwrap().library.clone();
        let pool = game.player(B).unwrap().mana_pool.clone(); game.player_mut(B).unwrap().mana_pool.blue = u32::MAX;
        let mut dm = Choices::default(); assert!(resolve_stack_entry_with(&mut game, &mut dm).is_err());
        assert_eq!(game.stack.iter().map(|entry| entry.object_id).collect::<Vec<_>>(), stack);
        assert_eq!(game.battlefield, battlefield); assert_eq!(game.player(A).unwrap().library, library);
        assert_eq!(game.player(A).unwrap().life, 20); assert!(dm.views.is_empty());
        game.player_mut(B).unwrap().mana_pool = pool; settle(&mut game, &mut dm);
        if name.starts_with("Cemetery") { reward(&game, name, 1); } else { assert_eq!(game.player(A).unwrap().hand.len(), 1); }
    }}
}
#[test]
#[ignore = "four full bodies remain partial until their disclosed owners close"]
fn remaining_frozen_bodies_must_retain_relations_without_parse_loss() {
    for name in ["Creeping Dread", "Holistic Wisdom", "Reality Scramble", "Wild Magic Surge"] {
        for definition in definitions(name) { assert!(definition.canonical_text.to_lowercase().contains("shares a card type")); }
    }
}
