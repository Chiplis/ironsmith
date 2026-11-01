//! MSEM701.78 expressed with source-relative value and optional hand cast readers.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const BODY: &str = "You may cast a spell from your hand with mana value less than this card's mana value without paying its mana cost.";
fn materialized(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact(name, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn definitions() -> [CardDefinition; 2] {
    materialized("Abound witness", &format!("Mana cost: {{2}}{{U}}{{U}}\nType: Creature — Wizard\nPower/Toughness: 3/3\nWhenever this creature attacks, {BODY}"))
}

struct Choice(bool);
impl DecisionMaker for Choice {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {self.0}
    fn decide_options(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> { SelectFirstDecisionMaker.decide_options(game, ctx) }
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> { SelectFirstDecisionMaker.decide_objects(game, ctx) }
}
fn spell(game: &mut GameState, name: &str, cost: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, format!("Mana cost: {cost}\nType: Instant\nYou gain 2 life."), false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
#[test]
fn optional_hand_cast_uses_strict_source_threshold_and_departed_source_lki() {
    for definition in definitions() { for departed in [false, true] { for accept in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let low = spell(&mut game, "Below threshold", "{2}", A, Zone::Hand);
        let low_stable = game.object(low).unwrap().stable_id;
        let equal = spell(&mut game, "Equal threshold", "{4}", A, Zone::Hand);
        let high = spell(&mut game, "Above threshold", "{5}", A, Zone::Hand);
        let grave = spell(&mut game, "Wrong zone", "{1}", A, Zone::Graveyard);
        let opponent = spell(&mut game, "Wrong player", "{1}", B, Zone::Hand);
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(source, ironsmith::events::AttackEventTarget::Player(B)), Default::default());
        game.queue_trigger_event(Default::default(), event);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = Choice(accept);
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        if departed {
            execute_effect(&mut game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Graveyard, false)), &mut EffectContext::new_default(source, A)).unwrap();
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.stack.len(), usize::from(accept));
        let selected = game.find_object_by_stable_id(low_stable).unwrap();
        assert_eq!(game.object(selected).unwrap().zone, if accept {Zone::Stack} else {Zone::Hand});
        for (id, zone) in [(equal, Zone::Hand), (high, Zone::Hand), (grave, Zone::Graveyard), (opponent, Zone::Hand)] {assert_eq!(game.object(id).unwrap().zone, zone);}
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        if accept {
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(A).unwrap().life, 22);
        }
    } } }
}
#[test]
fn zero_and_absent_source_offer_no_spell_even_with_zero_mana_value_in_hand() {
    let program = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_runtime_definition("Freecast program", format!("Type: Sorcery\n{BODY}"), false)).unwrap();
    for missing in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = if missing {ObjectId::from_raw(900000)} else {game.create_object_from_definition(&program, A, Zone::Stack)};
        let zero = spell(&mut game, "Zero value", "{0}", A, Zone::Hand);
        let mut dm = Choice(true);
        let effects = program.spell_effect.as_ref().unwrap().all_effects_owned();
        let effect = Effect::new(ironsmith::effects::SequenceEffect::new(effects));
        execute_effect(&mut game, &effect, &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm)).unwrap();
        assert_eq!(game.object(zero).unwrap().zone, Zone::Hand);
        assert!(game.stack.iter().all(|entry| entry.object_id == source));
    }
}

#[test]
fn equal_and_higher_mana_values_do_not_offer_a_free_cast() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let equal = spell(&mut game, "Exactly four", "{4}", A, Zone::Hand);
        let higher = spell(&mut game, "Exactly five", "{5}", A, Zone::Hand);
        game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
            ironsmith::events::CreatureAttackedEvent::new(source, ironsmith::events::AttackEventTarget::Player(B)), Default::default()));
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = Choice(true);
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.is_empty(), "strictly lesser cannot accept equal mana value");
        assert_eq!(game.object(equal).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(higher).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn granted_abound_uses_attacking_recipient_mana_value_instead_of_grantor() {
    let text = format!("Mana cost: {{U}}{{R}}\nType: Creature — Merfolk Wizard\nPower/Toughness: 2/2\nEach Merfolk and Wizard you control has \"Whenever this creature attacks, {BODY}\"");
    for grantor_definition in materialized("Tidecaller grantor", &text) {
        for (matching, controller) in [(true, A), (false, A), (true, B)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.create_object_from_definition(&grantor_definition, A, Zone::Battlefield);
            let recipient = ironsmith_compiler_runtime::compile_to_runtime_definition("Granted attacker",
                format!("Mana cost: {{5}}\nType: Creature — {}\nPower/Toughness: 4/4", if matching {"Wizard"} else {"Beast"}), false).unwrap();
            let attacker = game.create_object_from_definition(&recipient, controller, Zone::Battlefield);
            let offered = spell(&mut game, "Four mana offered", "{4}", A, Zone::Hand);
            let stable = game.object(offered).unwrap().stable_id;
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::CreatureAttackedEvent::new(attacker, ironsmith::events::AttackEventTarget::Player(if controller == A {B} else {A})), Default::default()));
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut dm = Choice(true);
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            let applies = matching && controller == A;
            assert_eq!(game.stack.len(), usize::from(applies));
            if applies {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.stack.len(), 1, "MV4 is lower than recipient5, not grantor2");
                let cast = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(cast).unwrap().zone, Zone::Stack);
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.player(A).unwrap().life, 22);
            } else {
                assert_eq!(game.object(offered).unwrap().zone, Zone::Hand);
            }
        }
    }
}

#[test]
fn source_mana_value_operand_is_independent_of_cast_permission_word_order() {
    let tokens = ironsmith_compiler::lexer::lex_line("this card's mana value", 0).unwrap();
    let parsed = ironsmith_compiler::grammar::values::parse_value_prefix_lexed(&tokens);
    assert!(parsed.is_some(), "source value tokens: {tokens:?}");
    let (_, used) = parsed.unwrap();
    assert_eq!(used, tokens.len());
    for rest in [
        "a spell from your hand with mana value less than this card's mana value without paying its mana cost",
        "a spell with mana value less than this card's mana value from your hand without paying its mana cost",
    ] {
        let tokens = ironsmith_compiler::lexer::lex_line(rest, 0).unwrap();
        let fact = ironsmith_compiler::grammar::permission_facts::zone_free_cast::parse_free_cast_from_zone_tokens(&tokens).unwrap();
        let comparison = fact.mana_value.unwrap();
        let (_, rhs) = ironsmith_compiler::grammar::values::parse_value_comparison_tokens(comparison.tokens).unwrap();
        assert!(ironsmith_compiler::grammar::values::parse_value_prefix_lexed(rhs).is_some(), "rhs: {rhs:?}");
    }
}
