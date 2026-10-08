//! Exact frozen full bodies plus process scenarios. Authored only; UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, NumberContext, SelectObjectsContext};
use ironsmith::effect::{Effect, EffectOutcome};
use ironsmith::effects::{EffectContext as ExecutionContext, EffectExecutor, SequenceEffect};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const FULL_BODIES: [(&str, &str); 8] = [
    ("8095ca78-db19-4724-a6ff-eacc85fa2274", "Another Round"),
    ("18f0cd0b-3e4f-4637-a62e-75dd1b2f3fce", "Claim Jumper"),
    ("1636c4d2-f699-4af7-8508-dbce2f0b7b52", "Countryside Crusher"),
    ("97560adc-814e-450c-9ca7-f9364e910a0b", "Grindstone"),
    ("9df2e909-ed13-456a-9636-7398732009a9", "Professor Onyx"),
    ("99eb50ef-352f-47c4-91e3-32813cbe0649", "Scalpelexis"),
    ("50885640-cdf1-4c62-b3bc-f37db6ab38b5", "Trade Secrets"),
    ("5bc66b18-22ac-4138-b527-fa711116e298", "Zimone and Dina"),
];

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../fixtures/card-failure-campaign/repeat-process-boundaries.json"
    )).unwrap()["cards"].as_array().unwrap().clone()
}
fn definitions(name: &str) -> [CardDefinition; 3] {
    let rows = rows();
    let (oracle_id, _) = FULL_BODIES.iter().find(|(_, expected)| *expected == name)
        .expect("only independently enumerated full bodies enter the positive routes");
    let selected = rows.iter().filter(|row| row["source"]["oracle_id"] == *oracle_id).collect::<Vec<_>>();
    assert_eq!(selected.len(), 1, "one exact frozen Oracle identity for {name}");
    let row = &selected[0]["source"];
    assert_eq!(row["name"], name);
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() { text.push_str(&format!("Loyalty: {loyalty}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(direct.clone()).unwrap();
    let wire = serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap();
    let native = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap();
    [direct, restored, native]
}
fn effects(definition: &CardDefinition, name: &str) -> Vec<Effect> {
    if let Some(program) = &definition.spell_effect { return program.all_effects_owned(); }
    let activated = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Activated(ability) => Some(&ability.effects), _ => None,
    }).collect::<Vec<_>>();
    if !activated.is_empty() {
        return activated[if name == "Professor Onyx" { 2 } else { 0 }].all_effects_owned();
    }
    definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(ability) => Some(ability.effects.all_effects_owned()), _ => None,
    }).unwrap()
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 30) }
fn card(g: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    g.create_object_from_definition(&definition, owner, zone)
}
fn library(g: &mut GameState, player: PlayerId, count: usize) {
    for i in 0..count { card(g, player, Zone::Library, &format!("Filler {i}"), "Type: Sorcery"); }
}
#[derive(Default)]
struct Decisions {
    answers: Vec<bool>, booleans: Vec<PlayerId>, objects: Vec<PlayerId>,
    number: u32, pause: Option<usize>, pending: bool,
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.booleans.push(ctx.player);
        self.pending = self.pause == Some(self.booleans.len());
        self.answers.get(self.booleans.len() - 1).copied().unwrap_or(false)
    }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 { self.number.clamp(ctx.min, ctx.max) }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.objects.push(ctx.player);
        let _ = g;
        ctx.candidates.iter().filter(|candidate| candidate.legal)
            .take(ctx.max.unwrap_or(ctx.candidates.len())).map(|candidate| candidate.id).collect()
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn execute(definition: &CardDefinition, name: &str, g: &mut GameState, source: ObjectId,
    dm: &mut Decisions, x: u32, target: Option<PlayerId>) -> EffectOutcome {
    let mut ctx = ExecutionContext::new(source, A, dm).with_x(x);
    if let Some(player) = target { ctx = ctx.with_targets(vec![ironsmith::ResolvedTarget::Player(player)]); }
    SequenceEffect::new(effects(definition, name)).execute(g, &mut ctx).unwrap()
}

#[path = "repeat_process_boundaries/full_bodies.rs"]
mod full_bodies;

#[test]
fn zimone_checks_lands_after_initial_land_and_repeats_the_whole_draw_land_program_once() {
    for definition in definitions("Zimone and Dina") {
        for (lands, accept, draws) in [(6, true, 1), (7, true, 2), (8, false, 2)] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            for i in 0..lands { card(&mut g, A, Zone::Battlefield, &format!("Land {i}"), "Type: Land"); }
            card(&mut g, A, Zone::Hand, "Next Land", "Type: Land");
            library(&mut g, A, 5);
            let mut dm = Decisions { answers: vec![accept, false], ..Default::default() };
            execute(&definition, "Zimone and Dina", &mut g, source, &mut dm, 0, None);
            assert_eq!(g.player(A).unwrap().library.len(), 5 - draws);
            assert_eq!(g.player(A).unwrap().hand.len(), draws + usize::from(!accept));
            assert!(dm.booleans.iter().all(|player| *player == A));
        }
    }
}

#[test]
fn onyx_runs_seven_complete_opponent_rounds_and_never_charges_the_controller() {
    for definition in definitions("Professor Onyx") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Decisions::default();
        execute(&definition, "Professor Onyx", &mut g, source, &mut dm, 0, None);
        assert_eq!(g.player(A).unwrap().life, 30);
        assert_eq!(g.player(B).unwrap().life, 9);
        assert_eq!(g.player(C).unwrap().life, 9);
    }
}

#[test]
fn trade_secrets_target_opponent_alone_chooses_and_zero_additional_rounds_still_draws() {
    for definition in definitions("Trade Secrets") {
        for extra in [0, 2] {
            let mut g = game(); library(&mut g, A, 20); library(&mut g, B, 20); library(&mut g, C, 20);
            let source = g.create_object_from_definition(&definition, A, Zone::Stack);
            let mut answers = vec![true; extra]; answers.push(false);
            let mut dm = Decisions { answers, number: 4, ..Default::default() };
            execute(&definition, "Trade Secrets", &mut g, source, &mut dm, 0, Some(B));
            assert_eq!(g.player(A).unwrap().hand.len(), 4 * (extra + 1));
            assert_eq!(g.player(B).unwrap().hand.len(), 2 * (extra + 1));
            assert!(g.player(C).unwrap().hand.is_empty());
            assert_eq!(dm.booleans, vec![B; extra + 1]);
        }
    }
}

#[test]
fn trade_secrets_pending_later_continuation_rolls_back_prior_rounds_and_receipts() {
    for definition in definitions("Trade Secrets") {
        let mut g = game(); library(&mut g, A, 20); library(&mut g, B, 20);
        let source = g.create_object_from_definition(&definition, A, Zone::Stack);
        let mut dm = Decisions { answers: vec![true, false], number: 4, pause: Some(2), ..Default::default() };
        let mut ctx = ExecutionContext::new(source, A, &mut dm)
            .with_targets(vec![ironsmith::ResolvedTarget::Player(B)]);
        let receipt_id = ironsmith::effect::EffectId(9001);
        ctx.store_outcome(receipt_id, EffectOutcome::count(73));
        let sentinel = ironsmith::snapshot::ObjectSnapshot::from_object(g.object(source).unwrap(), &g);
        ctx.tag_object(ironsmith::TagKey::from("outer-repeat-sentinel"), sentinel);
        let saved_receipts = ctx.effect_outcomes.clone();
        let saved_tags = ctx.tagged_objects.clone();
        let saved_libraries = [g.player(A).unwrap().library.clone(), g.player(B).unwrap().library.clone()];
        let outcome = SequenceEffect::new(effects(&definition, "Trade Secrets")).execute(&mut g, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(outcome.events.is_empty(), "suspended rounds publish no completed events");
        assert_eq!(ctx.effect_outcomes, saved_receipts, "restore exact preexisting and absent result IDs");
        assert_eq!(ctx.tagged_objects, saved_tags, "restore the caller's exact reference memory");
        drop(ctx);
        assert_eq!([g.player(A).unwrap().library.clone(), g.player(B).unwrap().library.clone()], saved_libraries,
            "rollback restores library identity and order, not merely its length");
        assert_eq!(g.player(A).unwrap().library.len(), 20);
        assert_eq!(g.player(B).unwrap().library.len(), 20);
        assert!(g.player(A).unwrap().hand.is_empty());
        assert!(g.player(B).unwrap().hand.is_empty());
        dm.pending = false; dm.pause = None; dm.booleans.clear();
        execute(&definition, "Trade Secrets", &mut g, source, &mut dm, 0, Some(B));
        assert_eq!(g.player(A).unwrap().hand.len(), 8);
        assert_eq!(g.player(B).unwrap().hand.len(), 4);
    }
}

#[test]
fn claim_jumper_shuffles_once_after_either_or_both_searches_and_never_without_search() {
    for definition in definitions("Claim Jumper") {
        for (own_lands, enemy_lands, answers, expected_searches) in [
            (1, 2, vec![true], 1), (1, 3, vec![true, true], 2),
            (1, 3, vec![true, false], 1), (1, 3, vec![false, true], 1),
            (1, 3, vec![false, false], 0),
        ] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            for i in 0..own_lands { card(&mut g, A, Zone::Battlefield, &format!("Own {i}"), "Type: Land"); }
            for i in 0..enemy_lands { card(&mut g, B, Zone::Battlefield, &format!("Enemy {i}"), "Type: Land"); }
            for i in 0..3 { card(&mut g, A, Zone::Library, &format!("Plains {i}"), "Type: Basic Land — Plains"); }
            let mut dm = Decisions { answers, ..Default::default() };
            let outcome = execute(&definition, "Claim Jumper", &mut g, source, &mut dm, 0, None);
            assert_eq!(outcome.events_of_type::<ironsmith::events::SearchLibraryEvent>().count(), expected_searches);
            assert_eq!(outcome.events_of_type::<ironsmith::events::ShuffleLibraryEvent>().count(), usize::from(expected_searches > 0));
            assert_eq!(g.player(A).unwrap().library.len(), 3 - expected_searches);
        }
    }
}

#[test]
fn claim_jumper_failed_searches_still_authorize_exactly_one_final_shuffle() {
    for definition in definitions("Claim Jumper") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        card(&mut g, B, Zone::Battlefield, "Opponent land", "Type: Land");
        library(&mut g, A, 3);
        let mut dm = Decisions { answers: vec![true, true], ..Default::default() };
        let outcome = execute(&definition, "Claim Jumper", &mut g, source, &mut dm, 0, None);
        assert_eq!(g.player(A).unwrap().library.len(), 3);
        assert_eq!(outcome.events_of_type::<ironsmith::events::SearchLibraryEvent>().count(), 2,
            "both accepted searches occurred even though neither found a Plains");
        assert_eq!(outcome.events_of_type::<ironsmith::events::ShuffleLibraryEvent>().count(), 1);
    }
}

#[test]
fn countryside_consumes_each_land_and_stops_on_nonland_or_empty_latest_reveal() {
    for definition in definitions("Countryside Crusher") {
        for stop_nonland in [false, true] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            if stop_nonland { card(&mut g, A, Zone::Library, "Stop", "Type: Sorcery"); }
            for i in 0..3 { card(&mut g, A, Zone::Library, &format!("Land {i}"), "Type: Land"); }
            let mut dm = Decisions::default();
            let outcome = execute(&definition, "Countryside Crusher", &mut g, source, &mut dm, 0, None);
            assert_eq!(g.player(A).unwrap().graveyard.len(), 3);
            assert_eq!(g.player(A).unwrap().library.len(), usize::from(stop_nonland));
            assert_eq!(outcome.events_of_type::<ironsmith::events::ZoneChangeEvent>()
                .filter(|event| event.from == Zone::Library && event.to == Zone::Graveyard)
                .map(|event| event.objects.len()).sum::<usize>(), 3);
        }
    }
}

#[test]
fn countryside_missing_authenticated_identity_rolls_back_prior_work_and_exact_reference_memory() {
    for definition in definitions("Countryside Crusher") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hidden = g.create_hidden_card_placeholder(A, Zone::Library, 0,
            "test-only-unopened-repeat-identity".into());
        let library_before = g.player(A).unwrap().library.clone();
        let audit_before = g.crypto_audit_checkpoint();
        let mut dm = Decisions::default();
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        ctx.store_outcome(ironsmith::effect::EffectId(9002), EffectOutcome::count(81));
        ctx.tag_object(ironsmith::TagKey::from("outer-error-sentinel"),
            ironsmith::snapshot::ObjectSnapshot::from_object(g.object(source).unwrap(), &g));
        let receipts_before = ctx.effect_outcomes.clone();
        let tags_before = ctx.tagged_objects.clone();
        let mut program = vec![Effect::gain_life(7)];
        program.extend(effects(&definition, "Countryside Crusher"));
        assert!(matches!(SequenceEffect::new(program).execute(&mut g, &mut ctx),
            Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))),
            "a placeholder cannot supply the land gate's public identity");
        assert_eq!(ctx.effect_outcomes, receipts_before);
        assert_eq!(ctx.tagged_objects, tags_before);
        assert_eq!(g.player(A).unwrap().life, 30);
        assert_eq!(g.player(A).unwrap().library, library_before);
        assert!(g.player(A).unwrap().graveyard.is_empty());
        assert!(g.is_hidden_card_placeholder(hidden));
        assert!(!g.is_publicly_revealed_hidden_card(hidden));
        assert_eq!(g.crypto_audit_checkpoint(), audit_before);
    }
}

#[test]
fn scalpelexis_repeats_for_any_pair_but_does_not_union_separate_batches() {
    for definition in definitions("Scalpelexis") {
        for last_batch_count in [0, 1, 4] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            for i in 0..last_batch_count { let name = if i == 0 { "Pair".to_string() } else { format!("Last {i}") }; card(&mut g, B, Zone::Library, &name, "Type: Sorcery"); }
            for name in ["Pair", "Pair", "Other", "Different"] {
                card(&mut g, B, Zone::Library, name, "Type: Sorcery");
            }
            let mut dm = Decisions::default();
            let mut ctx = ExecutionContext::new(source, A, &mut dm);
            ctx.iteration.iterated_player = Some(B);
            let outcome = SequenceEffect::new(effects(&definition, "Scalpelexis")).execute(&mut g, &mut ctx).unwrap();
            assert_eq!(outcome.as_count(), Some(1), "only the first batch authorizes another execution");
            assert!(g.player(B).unwrap().library.is_empty());
            assert_eq!(g.exile.iter().filter(|id| g.object(**id).is_some_and(|object| object.owner == B)).count(), 4 + last_batch_count);
        }
    }
}

#[test]
fn another_round_blinks_initial_and_x_additional_passes_with_fresh_choices_and_owner_control() {
    for definition in definitions("Another Round") {
        for x in [0, 2] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Stack);
            let own = card(&mut g, A, Zone::Battlefield, "Own creature", "Type: Creature\nPower/Toughness: 1/1");
            let borrowed = card(&mut g, B, Zone::Battlefield, "Borrowed creature", "Type: Creature\nPower/Toughness: 2/2");
            let own_stable = g.object(own).unwrap().stable_id;
            let borrowed_stable = g.object(borrowed).unwrap().stable_id;
            g.object_mut(borrowed).unwrap().initial_controller = A;
            let mut dm = Decisions::default();
            let outcome = execute(&definition, "Another Round", &mut g, source, &mut dm, x, None);
            let current_own = g.find_object_by_stable_id(own_stable).unwrap();
            let current_borrowed = g.find_object_by_stable_id(borrowed_stable).unwrap();
            assert_ne!(current_own, own);
            assert_ne!(current_borrowed, borrowed);
            assert_eq!(g.object(current_own).unwrap().zone, Zone::Battlefield);
            assert_eq!(g.current_controller(current_own), Some(A));
            assert_eq!(g.current_controller(current_borrowed), Some(B));
            assert_eq!(outcome.events_of_type::<ironsmith::events::ZoneChangeEvent>()
                .filter(|event| event.from == Zone::Exile && event.to == Zone::Battlefield)
                .map(|event| event.objects.len()).sum::<usize>(), x as usize + 2,
                "borrowed creature returns to its owner and is unavailable for later choices");
            assert_eq!(dm.objects, vec![A; x as usize + 1]);
        }
    }
}

#[test]
fn grindstone_uses_common_color_actual_public_mills_and_fresh_short_or_empty_collections() {
    for definition in definitions("Grindstone") {
        for exile_replacement in [false, true] {
            for (top_costs, tail_count, expected_mills) in [
                (vec!["{R}", "{B}{R}"], 2, 4),
                (vec!["{R}", "{B}{R}"], 1, 3),
                (vec!["{R}", "{U}"], 2, 2),
                (vec!["{1}", "{2}"], 2, 2),
                (vec!["{R}"], 0, 1), (vec![], 0, 0),
            ] {
                let mut g = game();
                let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
                for i in 0..tail_count { card(&mut g, B, Zone::Library, &format!("Stop {i}"),
                    if tail_count == 1 { "Mana cost: {R}\nType: Sorcery" } else { "Mana cost: {1}\nType: Sorcery" }); }
                for (i, mana) in top_costs.iter().enumerate() {
                    card(&mut g, B, Zone::Library, &format!("Top {i}"), &format!("Mana cost: {mana}\nType: Sorcery"));
                }
                if exile_replacement {
                    ironsmith::effects::ExileInsteadOfGraveyardEffect::you()
                        .execute(&mut g, &mut ExecutionContext::new_default(source, B)).unwrap();
                }
                let mut dm = Decisions::default();
                let outcome = execute(&definition, "Grindstone", &mut g, source, &mut dm, 0, Some(B));
                if expected_mills > 2 {
                    assert_eq!(outcome.as_count(), Some(1), "the red singleton cannot authorize another pass");
                }
                assert_eq!(g.player(B).unwrap().library.len(), top_costs.len() + tail_count - expected_mills);
                if exile_replacement { assert_eq!(g.exile.len(), expected_mills); }
                else { assert_eq!(g.player(B).unwrap().graveyard.len(), expected_mills); }
                assert!(g.player(A).unwrap().graveyard.is_empty());
                assert!(g.player(C).unwrap().graveyard.is_empty());
            }
        }
    }
}

#[test]
fn claim_jumper_search_followup_names_the_complete_runtime_process_receipt() {
    fn collect(effect: &Effect, nodes: &mut Vec<Effect>) {
        nodes.push(effect.clone());
        effect.visit_child_effects(&mut |child| collect(child, nodes));
    }
    for definition in definitions("Claim Jumper") {
        let mut nodes = Vec::new();
        for effect in effects(&definition, "Claim Jumper") { collect(&effect, &mut nodes); }
        let followup = nodes.iter().find_map(|effect| {
            effect.downcast_ref::<ironsmith::effects::IfEffect>().filter(|gate|
                gate.predicate == ironsmith::effect::EffectPredicate::SearchedLibrary)
        }).expect("the final shuffle must retain its searched-library result gate");
        let producer = nodes.iter().find_map(|effect| {
            effect.downcast_ref::<ironsmith::effects::WithIdEffect>().filter(|producer|
                producer.id == followup.condition)
        }).expect("the shuffle's precise producer must be retained");
        let process = producer.effect.downcast_ref::<SequenceEffect>()
            .expect("initial and conditional searches must have one runtime receipt owner");
        assert!(process.effects.len() >= 2,
            "a flattened initial search cannot share the second search's result ID");
        assert!(process.effects.iter().any(|effect|
            effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().is_some()
                || effect.transparent_child_effect().is_some_and(|inner|
                    inner.downcast_ref::<ironsmith::effects::ConditionalEffect>().is_some())));
    }
}
