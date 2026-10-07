//! Seven frozen complete bodies. Source scenarios only: execution is deferred.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::continuous::Modification;
use ironsmith::cost::TotalCost;
use ironsmith::decision::{compute_legal_actions, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, ManaPaymentContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor, TurnFaceDownEffect, TurnFaceUpEffect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::special_actions::{turn_face_up_cost_display, TurnFaceUpMethod};
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, CoinFace, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/ordinary_face_down_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    for player in [A, B] {
        for symbol in [ManaSymbol::Colorless, ManaSymbol::Blue, ManaSymbol::Red, ManaSymbol::Green] {
            g.player_mut(player).unwrap().mana_pool.add(symbol, 30);
        }
    }
    g
}
fn object(g: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    g.create_object_from_definition(&compile_to_runtime_definition(name, text, false).unwrap(), owner, zone)
}
fn morph(g: &mut GameState, owner: PlayerId, keyword: &str) -> ObjectId {
    object(g, owner, Zone::Battlefield, "Orientation witness", &format!("Mana cost: {{2}}{{U}}\nType: Creature — Beast\nPower/Toughness: 4/5\n{keyword} {{U}}"))
}
#[derive(Default)]
struct Choices {
    targets: Option<Vec<Target>>,
    forbidden: Vec<Target>,
    accept: bool,
    target_counts: Vec<(usize, Option<usize>)>,
    target_calls: usize,
    pause_objects: bool,
    pending: bool,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, g: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target_calls += 1;
        for req in &ctx.requirements {
            self.target_counts.push((req.min_targets, req.max_targets));
            assert!(self.forbidden.iter().all(|target| !req.legal_targets.contains(target)), "forbidden current-state target: {ctx:?}");
        }
        if let Some(targets) = &self.targets {
            assert!(targets.iter().all(|target| ctx.requirements.iter().any(|req| req.legal_targets.contains(target))), "{ctx:?}");
            targets.clone()
        } else { SelectFirstDecisionMaker.decide_targets(g, ctx) }
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept }
    fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> { vec![0] }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.pending = self.pause_objects;
        ctx.candidates.iter().filter(|c| c.legal).take(ctx.max.unwrap_or(ctx.candidates.len())).map(|c| c.id).collect()
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash }
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn action(g: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    g.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(g.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(g, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(g, &mut queue, dm).unwrap();
}
fn resolve(g: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(g, dm).unwrap();
    put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
}
fn cast(g: &mut GameState, player: PlayerId, spell: ObjectId, method: CastingMethod, dm: &mut Choices) {
    action(g, player, LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand, casting_method: method }, dm);
}
fn activation(definition: &CardDefinition) -> usize {
    definition.abilities.iter().position(|a| matches!(a.kind, AbilityKind::Activated(_))).unwrap()
}
fn turn_down(g: &mut GameState, id: ObjectId) {
    let outcome = TurnFaceDownEffect::new(ChooseSpec::SpecificObject(id)).execute(g, &mut EffectContext::new_default(id, A)).unwrap();
    assert_eq!(outcome.count_or_zero(), 1);
    assert!(outcome.events.is_empty());
}
fn turn_up(g: &mut GameState, id: ObjectId, dm: &mut Choices) {
    action(g, A, LegalAction::TurnFaceUp { creature_id: id, method: TurnFaceUpMethod::TurnFaceUpAbility }, dm);
}
fn modify(g: &mut GameState, id: ObjectId, modification: Modification) {
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(id), modification, Until::EndOfTurn)
        .execute(g, &mut EffectContext::new_default(id, A)).unwrap();
    g.refresh_continuous_state().unwrap();
}
fn morph_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    filter.static_abilities.push(StaticAbilityId::Morph);
    filter
}
fn matches_morph(g: &GameState, id: ObjectId) -> bool {
    filter_matches(&g, A, &morph_filter(), id)
}

/// Match through the public target validator: a non-target object spec is
/// exactly the filter evaluated in the context's filter view.
fn filter_matches(game: &ironsmith::GameState, viewer: ironsmith::PlayerId, filter: &ironsmith::target::ObjectFilter, object: ironsmith::ObjectId) -> bool {
    let ctx = ironsmith::effects::EffectContext::new_default(object, viewer);
    ironsmith::effects::validate_target(game, &ironsmith::effects::ResolvedTarget::Object(object),
        &ironsmith::target::ChooseSpec::Object(filter.clone()), &ctx)
}
#[test]
fn all_complete_bodies_round_trip_with_keywords_costs_and_companion_effects() {
    for (name, fragments) in [
        ("Backslide", vec!["face down", "Cycling {U}"]),
        ("Master of the Veil", vec!["Morph {2}{U}", "may", "face down"]),
        ("Mischievous Quanar", vec!["{3}{U}{U}", "Morph {1}{U}{U}", "copy", "new targets"]),
        ("Obscuring Aether", vec!["{1} less", "{1}{G}", "face down"]),
        ("Skittish Valesk", vec!["upkeep", "coin", "lose", "face down", "Morph {5}{R}"]),
        ("Wall of Deceit", vec!["Defender", "{3}", "Morph {U}", "face down"]),
        ("Weaver of Lies", vec!["Morph {4}{U}", "any number", "morph", "face down"]),
    ] {
        for definition in definitions(name) {
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            for fragment in &fragments { assert!(rendered.to_lowercase().contains(&fragment.to_lowercase()), "{name}: {fragment}: {rendered}"); }
        }
    }
}

#[test]
fn source_activations_preserve_incarnation_counters_attachments_control_tap_and_morph_costs() {
    for (name, down_cost, morph_cost, power, toughness) in [
        ("Wall of Deceit", 3, "{U}", 0, 5),
        ("Mischievous Quanar", 5, "{1}{U}{U}", 3, 3),
    ] {
        for definition in definitions(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, B, Zone::Battlefield);
            g.object_mut(source).unwrap().initial_controller = A;
            g.object_mut(source).unwrap().add_counters(CounterType::PlusOnePlusOne, 1);
            g.tap(source);
            let attachment = object(&mut g, A, Zone::Battlefield, "Orientation Equipment", "Type: Artifact — Equipment");
            assert!(g.attach_object_to_target(attachment, AttachmentTarget::Object(source)));
            let stable = g.object(source).unwrap().stable_id;
            let mut defender = ObjectFilter::creature();
            defender.static_abilities.push(StaticAbilityId::Defender);
            if name == "Wall of Deceit" {
                assert!(filter_matches(&g, A, &defender, source));
            }
            let mana = g.player(A).unwrap().mana_pool.total();
            let mut dm = Choices::default();
            action(&mut g, A, LegalAction::ActivateAbility { source, ability_index: activation(&definition) }, &mut dm);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - down_cost);
            resolve(&mut g, &mut dm);
            assert_eq!(dm.target_calls, 0, "self instruction does not target");
            assert!(g.is_face_down(source));
            assert_eq!(g.object(source).unwrap().stable_id, stable);
            assert_eq!(g.current_controller(source), Some(A));
            assert_eq!(g.object(source).unwrap().owner, B);
            assert!(g.is_tapped(source));
            assert_eq!(g.counter_count(source, CounterType::PlusOnePlusOne), 1);
            assert_eq!(g.current_power(source), Some(3));
            assert_eq!(g.current_toughness(source), Some(3));
            assert!(!filter_matches(&g, A, &defender, source));
            assert_eq!(g.object(attachment).unwrap().attached_to, Some(AttachmentTarget::Object(source)));
            assert!(!g.is_manifested(source));
            assert!(!g.object(source).unwrap().face_down_cast_state.as_ref().unwrap().disguise_ward);
            assert_eq!(turn_face_up_cost_display(&g, source, TurnFaceUpMethod::TurnFaceUpAbility).unwrap().as_deref(), Some(morph_cost));
            assert_eq!(turn_face_up_cost_display(&g, source, TurnFaceUpMethod::PrintedManaCost).unwrap(), None);
            let repeat = TurnFaceDownEffect::new(ChooseSpec::Source).execute(&mut g, &mut EffectContext::new_default(source, A)).unwrap();
            assert_eq!(repeat.count_or_zero(), 0);
            TurnFaceUpEffect::new(ChooseSpec::SpecificObject(source)).execute(&mut g, &mut EffectContext::new_default(source, A)).unwrap();
            assert_eq!(g.current_power(source), Some(power + 1));
            assert_eq!(g.current_toughness(source), Some(toughness + 1));
            if name == "Wall of Deceit" {
                assert!(filter_matches(&g, A, &defender, source));
            }
            assert!(g.is_tapped(source));
            assert_eq!(g.object(source).unwrap().stable_id, stable);
        }
    }
}

#[test]
fn backslide_uses_live_morph_or_megamorph_and_native_target_rechecking() {
    for definition in definitions("Backslide") {
        for keyword in ["Morph", "Megamorph"] {
            let mut g = game();
            let target = morph(&mut g, B, keyword);
            let disguise = morph(&mut g, B, "Disguise");
            let hidden = morph(&mut g, A, "Morph");
            turn_down(&mut g, hidden);
            let stripped = morph(&mut g, A, "Morph");
            modify(&mut g, stripped, Modification::RemoveAllAbilities);
            let plain = object(&mut g, A, Zone::Battlefield, "Plain", "Type: Creature — Beast\nPower/Toughness: 4/5");
            for excluded in [disguise, hidden, stripped, plain] { assert!(!matches_morph(&g, excluded)); }
            assert!(matches_morph(&g, target));
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices { targets: Some(vec![Target::Object(target)]), forbidden: vec![disguise, hidden, stripped, plain].into_iter().map(Target::Object).collect(), ..Default::default() };
            cast(&mut g, A, spell, CastingMethod::Normal, &mut dm);
            assert_eq!(dm.target_counts, vec![(1, Some(1))]);
            resolve(&mut g, &mut dm);
            assert!(g.is_face_down(target));
            assert!(!matches_morph(&g, target));
            assert!(!g.is_face_down(disguise));
        }
        let mut g = game();
        let target = morph(&mut g, B, "Morph");
        let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices { targets: Some(vec![Target::Object(target)]), ..Default::default() };
        cast(&mut g, A, spell, CastingMethod::Normal, &mut dm);
        modify(&mut g, target, Modification::RemoveAllAbilities);
        resolve(&mut g, &mut dm);
        assert!(!g.is_face_down(target), "losing morph before resolution makes this target illegal");
    }
}

#[test]
fn backslide_cycling_pays_blue_discards_itself_and_draws_exactly_one() {
    for definition in definitions("Backslide") {
        let mut g = game();
        object(&mut g, A, Zone::Library, "Cycling resource", "Type: Land");
        let card = g.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = g.object(card).unwrap().stable_id;
        let mana = g.player(A).unwrap().mana_pool.total();
        let mut dm = Choices::default();
        let offered = compute_legal_actions(&g, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == card)).unwrap();
        action(&mut g, A, offered, &mut dm);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 1);
        assert!(g.player(A).unwrap().hand.is_empty());
        assert!(g.player(A).unwrap().graveyard.iter().any(|id| g.object(*id).unwrap().stable_id == stable));
        resolve(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 1);
        assert_eq!(dm.target_calls, 0);
    }
}

#[test]
fn master_keeps_optional_resolution_and_can_target_itself_or_an_opponent() {
    for definition in definitions("Master of the Veil") {
        for accept in [false, true] {
            for self_target in [false, true] {
                let mut g = game();
                let master = g.create_object_from_definition(&definition, A, Zone::Battlefield);
                let other = morph(&mut g, B, "Megamorph");
                turn_down(&mut g, master);
                let target = if self_target { master } else { other };
                let mut dm = Choices { targets: Some(vec![Target::Object(target)]), accept, ..Default::default() };
                let mana = g.player(A).unwrap().mana_pool.total();
                turn_up(&mut g, master, &mut dm);
                assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 3);
                assert_eq!(g.stack.len(), 1);
                resolve(&mut g, &mut dm);
                assert_eq!(g.is_face_down(target), accept);
                assert_eq!(g.is_face_down(if self_target { other } else { master }), false);
            }
        }
    }
}

#[test]
fn quanar_paid_morph_copies_a_real_spell_and_keeps_retargeting_permission() {
    for definition in definitions("Mischievous Quanar") {
        let mut g = game();
        let quanar = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices::default();
        action(&mut g, A, LegalAction::ActivateAbility { source: quanar, ability_index: activation(&definition) }, &mut dm);
        resolve(&mut g, &mut dm);
        let original = object(&mut g, B, Zone::Hand, "Copy target", "Mana cost: {U}\nType: Instant\nTarget player gains 2 life.");
        dm.targets = Some(vec![Target::Player(B)]);
        cast(&mut g, B, original, CastingMethod::Normal, &mut dm);
        let spell = g.stack.last().unwrap().object_id;
        dm.targets = Some(vec![Target::Object(spell)]);
        let mana = g.player(A).unwrap().mana_pool.total();
        turn_up(&mut g, quanar, &mut dm);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 3);
        assert_eq!(g.stack.len(), 2);
        dm.accept = true;
        dm.targets = Some(vec![Target::Player(A)]);
        resolve(&mut g, &mut dm);
        assert_eq!(g.stack.len(), 2, "one spell copy above the original");
        assert_eq!(g.stack.last().unwrap().controller, A);
        resolve(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 22);
        assert_eq!(g.player(B).unwrap().life, 20);
        resolve(&mut g, &mut dm);
        assert_eq!(g.player(B).unwrap().life, 22);
        assert_eq!(g.current_power(quanar), Some(3));
    }
}

#[test]
fn weaver_keeps_any_number_target_scope_and_excludes_itself() {
    for definition in definitions("Weaver of Lies") {
        for count in 0..=2 {
            let mut g = game();
            let weaver = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first = morph(&mut g, A, "Morph");
            let second = morph(&mut g, B, "Megamorph");
            let disguise = morph(&mut g, B, "Disguise");
            turn_down(&mut g, weaver);
            let selected = [first, second][..count].to_vec();
            let mut dm = Choices { targets: Some(selected.iter().copied().map(Target::Object).collect()), forbidden: vec![Target::Object(weaver), Target::Object(disguise)], ..Default::default() };
            turn_up(&mut g, weaver, &mut dm);
            assert!(dm.target_counts.iter().all(|count| *count == (0, None)));
            resolve(&mut g, &mut dm);
            for id in [first, second] { assert_eq!(g.is_face_down(id), selected.contains(&id)); }
            assert!(!g.is_face_down(weaver));
            assert!(!g.is_face_down(disguise));
        }
    }
}

#[test]
fn one_instruction_locks_all_morph_targets_before_losing_a_grant_source() {
    for definition in definitions("Weaver of Lies") {
        let mut g = game();
        let cost = TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]));
        let granter_def = CardDefinitionBuilder::new(CardId::new(), "Visible morph grant")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 3))
            .with_ability(Ability::static_ability(StaticAbility::grant_ability(ObjectFilter::creature(), StaticAbility::morph(cost))))
            .build();
        let granter = g.create_object_from_definition(&granter_def, A, Zone::Battlefield);
        let other = object(&mut g, B, Zone::Battlefield, "Granted morph", "Type: Creature — Beast\nPower/Toughness: 4/5");
        let weaver = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        g.refresh_continuous_state().unwrap();
        assert!(matches_morph(&g, granter));
        assert!(matches_morph(&g, other));
        turn_down(&mut g, weaver);
        let mut dm = Choices { targets: Some(vec![Target::Object(granter), Target::Object(other)]), ..Default::default() };
        turn_up(&mut g, weaver, &mut dm);
        resolve(&mut g, &mut dm);
        assert!(g.is_face_down(granter));
        assert!(g.is_face_down(other), "the first transition cannot invalidate a locked second target");
        assert!(!matches_morph(&g, other));
    }
}

#[test]
fn valesk_upkeep_turns_down_only_after_losing_and_preserves_morph() {
    for definition in definitions("Skittish Valesk") {
        for (face, down) in [(CoinFace::Heads, false), (CoinFace::Tails, true)] {
            let mut g = game();
            let valesk = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            g.force_next_coin_flip(face);
            let mut dm = Choices::default();
            g.turn.phase = ironsmith::Phase::Beginning;
            g.turn.step = Some(ironsmith::game_state::Step::Upkeep);
            let mut queue = TriggerQueue::new();
            g.turn.active_player = B;
            generate_and_queue_step_triggers(&mut g, &mut queue);
            put_triggers_on_stack_with_dm(&mut g, &mut queue, &mut dm).unwrap();
            assert!(g.stack.is_empty(), "opponent upkeep must not trigger Valesk");
            assert!(!g.is_face_down(valesk));
            g.turn.active_player = A;
            generate_and_queue_step_triggers(&mut g, &mut queue);
            put_triggers_on_stack_with_dm(&mut g, &mut queue, &mut dm).unwrap();
            assert_eq!(g.stack.len(), 1);
            resolve(&mut g, &mut dm);
            assert_eq!(g.is_face_down(valesk), down);
            assert_eq!(g.current_power(valesk), Some(if down { 2 } else { 5 }));
            if down {
                assert_eq!(turn_face_up_cost_display(&g, valesk, TurnFaceUpMethod::TurnFaceUpAbility).unwrap(), Some("{5}{R}".into()));
                turn_up(&mut g, valesk, &mut dm);
                assert_eq!(g.current_power(valesk), Some(5));
            }
        }
    }
}

#[test]
fn aether_discount_applies_to_own_face_down_casts_and_disappears_after_its_activation() {
    for definition in definitions("Obscuring Aether") {
        for turn_aether_down in [false, true] {
            for caster in [A, B] {
                let mut g = game();
                let aether = g.create_object_from_definition(&definition, A, Zone::Battlefield);
                let mut dm = Choices::default();
                if turn_aether_down {
                    let mana = g.player(A).unwrap().mana_pool.total();
                    action(&mut g, A, LegalAction::ActivateAbility { source: aether, ability_index: activation(&definition) }, &mut dm);
                    assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 2);
                    resolve(&mut g, &mut dm);
                    assert!(g.is_face_down(aether));
                    assert_eq!(g.current_power(aether), Some(2));
                    assert_eq!(turn_face_up_cost_display(&g, aether, TurnFaceUpMethod::TurnFaceUpAbility).unwrap(), None);
                    assert_eq!(turn_face_up_cost_display(&g, aether, TurnFaceUpMethod::PrintedManaCost).unwrap(), None);
                }
                let card = object(&mut g, caster, Zone::Hand, "Discounted morph", "Mana cost: {4}{U}\nType: Creature — Beast\nPower/Toughness: 5/5\nMorph {U}");
                g.player_mut(caster).unwrap().mana_pool = Default::default();
                g.turn.active_player = caster;
                g.turn.priority_player = Some(caster);
                let expected_cost = if caster == A && !turn_aether_down { 2 } else { 3 };
                g.player_mut(caster).unwrap().mana_pool.add(ManaSymbol::Colorless, expected_cost);
                assert!(compute_legal_actions(&g, caster).unwrap().iter().any(|action|
                    matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::FaceDown, .. } if *spell_id == card)),
                    "the discounted spell must be offered with exactly its discounted payment");
                cast(&mut g, caster, card, CastingMethod::FaceDown, &mut dm);
                assert_eq!(g.player(caster).unwrap().mana_pool.total(), 0);
                resolve(&mut g, &mut dm);
            }
        }
    }
}

#[test]
fn ordinary_turn_down_rejects_double_faced_cards_but_does_not_change_face_down_entry() {
    for transforming in [false, true] {
        let mut g = game();
        let card = ironsmith::card::CardBuilder::new(CardId::new(), "DFC orientation control")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4))
            .other_face(CardId::new()).other_face_name("Other face")
            .linked_face_layout(LinkedFaceLayout::TransformLike).transforming_dfc(transforming).build();
        let definition = CardDefinition::with_abilities(card, Vec::new());
        let dfc = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = g.object(dfc).unwrap().stable_id;
        let result = TurnFaceDownEffect::new(ChooseSpec::SpecificObject(dfc)).execute(&mut g, &mut EffectContext::new_default(dfc, A)).unwrap();
        assert_eq!(result.count_or_zero(), 0);
        assert!(!g.is_face_down(dfc));
        assert_eq!(g.object(dfc).unwrap().stable_id, stable);
        // Entry/casting uses the low-level status setter; this patch must not
        // prohibit a DFC entering face down through Manifest or Cloak.
        assert!(g.set_face_down(dfc));
        assert!(g.is_face_down(dfc));
    }
}

#[test]
fn pending_selection_makes_no_partial_orientation_change_and_can_resume() {
    let mut g = game();
    let target = morph(&mut g, A, "Morph");
    let other = morph(&mut g, B, "Morph");
    let effect = TurnFaceDownEffect::new(ChooseSpec::Object(ObjectFilter::creature()));
    let mut dm = Choices { pause_objects: true, ..Default::default() };
    let mut ctx = EffectContext::new(target, A, &mut dm);
    let result = effect.execute(&mut g, &mut ctx).unwrap();
    assert_eq!(result.count_or_zero(), 0);
    assert!(!g.is_face_down(target));
    assert!(!g.is_face_down(other));
    drop(ctx);
    dm.pause_objects = false;
    dm.pending = false;
    effect.execute(&mut g, &mut EffectContext::new(target, A, &mut dm)).unwrap();
    assert_eq!([target, other].into_iter().filter(|id| g.is_face_down(*id)).count(), 1);
}

#[test]
fn merged_orientation_checks_buried_dfc_and_turns_all_single_face_components_together() {
    for buried_dfc in [false, true] {
        let mut g = game();
        let mut base = ironsmith::card::CardBuilder::new(CardId::new(), "Merged orientation base")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 3));
        if buried_dfc {
            base = base.other_face(CardId::new()).other_face_name("Buried other face")
                .linked_face_layout(LinkedFaceLayout::TransformLike).transforming_dfc(true);
        }
        let base = CardDefinition::with_abilities(base.build(), Vec::new());
        let host = g.create_object_from_definition(&base, A, Zone::Battlefield);
        let top = CardDefinitionBuilder::new(CardId::new(), "Merged visible single face")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4)).build();
        let top = g.create_object_from_definition(&top, A, Zone::Stack);
        g.merge_mutating_creature_spell(top, host, true).unwrap();
        let stable = g.object(host).unwrap().stable_id;
        assert_eq!(g.object(host).unwrap().linked_face_layout, LinkedFaceLayout::None,
            "the buried component, rather than the visible top, supplies DFC eligibility");
        g.take_pending_trigger_events();
        let result = TurnFaceDownEffect::new(ChooseSpec::SpecificObject(host))
            .execute(&mut g, &mut EffectContext::new_default(host, A)).unwrap();
        assert_eq!(result.count_or_zero(), if buried_dfc { 0 } else { 1 });
        assert_eq!(g.is_face_down(host), !buried_dfc);
        assert_eq!(g.object(host).unwrap().stable_id, stable);
        let merged = g.merged_permanent(stable).unwrap();
        assert_eq!(merged.components.len(), 2);
        assert!(merged.components.iter().all(|component| component.face_down == !buried_dfc));
        assert!(result.events.is_empty());
        assert!(g.take_pending_trigger_events().is_empty());
    }
}
