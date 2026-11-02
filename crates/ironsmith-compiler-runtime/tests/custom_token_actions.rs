//! Runtime observables for existing token/counter operations, including artifact transport.
use ironsmith::ability::AbilityKind;
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::types::Subtype;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const ADJUDICATE: &str = "Create a white Aura enchantment token with \"Enchant creature\" and \"Sacrifice this token: Untap enchanted creature.\" attached to target creature.";
const ARM: &str = "Create a colorless Equipment artifact token with \"Equipped creature gets +1/+0.\" and equip {2} attached to this creature.";

struct Accept(bool);
impl DecisionMaker for Accept {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.0 }
    fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        context.candidates.iter().filter(|object| object.legal).map(|object| object.id)
            .take(context.min.max(usize::from(self.0))).collect()
    }
}
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}

#[test]
fn variable_recruit_repeats_complete_process_and_keeps_life_loss_outside() {
    let text = "Mana cost: {X}{B}{B}\nType: Instant\nRepeat the following process X times: Create a 0/0 white Soldier Army creature token; then put a +1/+1 counter on each Army you control. You lose X life.";
    for definition in definitions("Variable army", text) {
        for x in [0, 1, 3] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let army = ironsmith_compiler_runtime::compile_to_runtime_definition("Existing army", "Type: Creature — Army\nPower/Toughness: 2/2", false).unwrap();
            let allied = game.create_object_from_definition(&army, A, Zone::Battlefield);
            let enemy = game.create_object_from_definition(&army, B, Zone::Battlefield);
            let source = game.create_object_from_definition(&definition, A, Zone::Stack);
            let mut context = EffectContext::new_default(source, A).with_x(x);
            for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
                execute_effect(&mut game, effect, &mut context).unwrap();
            }
            assert_eq!(game.player(A).unwrap().life, 20 - x as i32);
            assert_eq!(game.counter_count(allied, CounterType::PlusOnePlusOne), x);
            assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
            let mut counters: Vec<_> = game.battlefield.iter().copied()
                .filter(|id| *id != allied && *id != enemy)
                .map(|id| game.counter_count(id, CounterType::PlusOnePlusOne)).collect();
            counters.sort();
            assert_eq!(counters, (1..=x).collect::<Vec<_>>());
        }
    }
}

#[test]
fn reappropriate_cost_selects_exactly_two_owned_matching_cards_across_zones() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let text = "Type: Artifact\nExile exactly two blue cards from your hand and/or graveyard: You gain 1 life.";
    for definition in definitions("Reusable mixed-zone price", text) {
        for distribution in [0, 1, 2] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.priority_player = Some(A);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let blue = ironsmith_compiler_runtime::compile_to_runtime_definition("Blue material", "Mana cost: {U}\nType: Sorcery", false).unwrap();
            let red = ironsmith_compiler_runtime::compile_to_runtime_definition("Red material", "Mana cost: {R}\nType: Sorcery", false).unwrap();
            let wrong_color = game.create_object_from_definition(&red, A, Zone::Graveyard);
            let enemy = game.create_object_from_definition(&blue, B, Zone::Graveyard);
            let offered = |game: &GameState| compute_legal_actions(game, A).unwrap().iter().any(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source));
            assert!(!offered(&game), "other owner's cards and wrong colors cannot pay");
            let mut materials = Vec::new();
            for index in 0..2 {
                let zone = if index < distribution { Zone::Hand } else { Zone::Graveyard };
                let id = game.create_object_from_definition(&blue, A, zone);
                materials.push(game.object(id).unwrap().stable_id);
                assert_eq!(offered(&game), index == 1, "one card cannot pay an exact two-card price");
            }
            activate(&mut game, source);
            for stable in materials {
                assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
            }
            assert_eq!(game.object(wrong_color).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(enemy).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(A).unwrap().life, 20, "price is paid before the effect resolves");
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept(true)).unwrap();
            assert_eq!(game.player(A).unwrap().life, 21);
        }
    }
}

#[test]
fn multi_target_arm_creates_one_equipment_per_announced_legal_creature() {
    let text = "Type: Sorcery\nFor each of up to two target creatures you control, create a colorless Equipment artifact token with \"Equipped creature gets +1/+0.\" and equip {2} attached to that creature.";
    for definition in definitions("Distributed equipment", text) {
        for count in [0, 1, 2] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let targets: Vec<_> = (0..count).map(|_| game.create_object_from_definition(&body(), A, Zone::Battlefield)).collect();
            let unrelated = game.create_object_from_definition(&body(), A, Zone::Battlefield);
            let source = game.create_object_from_definition(&definition, A, Zone::Stack);
            let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
            apply(&mut game, source, &effects, true, targets.iter().copied().map(ResolvedTarget::Object).collect());
            let equipment: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id))
                .filter(|object| object.subtypes.contains(&Subtype::Equipment)).collect();
            assert_eq!(equipment.len(), count);
            for target in targets {
                assert_eq!(equipment.iter().filter(|object| object.attached_to == Some(AttachmentTarget::Object(target))).count(), 1);
            }
            assert!(!equipment.iter().any(|object| object.attached_to == Some(AttachmentTarget::Object(unrelated))));
        }
    }
}

#[test]
fn optional_exact_cross_zone_operation_cannot_partially_exile_one_card() {
    let text = "Type: Artifact\n{0}: You may exile exactly two cards from your hand and/or graveyard. If you do, you gain 3 life. Otherwise, you gain 1 life.";
    for definition in definitions("Optional exact selection", text) {
        for count in [0, 1, 2] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut materials = Vec::new();
            for index in 0..count {
                let zone = if index == 0 { Zone::Hand } else { Zone::Graveyard };
                let id = game.create_object_from_definition(&body(), A, zone);
                materials.push((game.object(id).unwrap().stable_id, zone));
            }
            apply(&mut game, source, &activated(&definition), true, vec![]);
            assert_eq!(game.player(A).unwrap().life, if count == 2 { 23 } else { 21 });
            for (stable, original) in materials {
                let zone = game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone;
                assert_eq!(zone, if count == 2 { Zone::Exile } else { original });
            }
        }
    }
}

#[test]
fn required_exact_exile_is_indivisible_but_ordinary_count_exiles_what_is_available() {
    for exact in [false, true] {
        let text = format!("Type: Sorcery\nExile {}two cards from your hand and/or graveyard.", if exact { "exactly " } else { "" });
        for definition in definitions("Exact operation", &text) {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let material = game.create_object_from_definition(&body(), A, Zone::Hand);
            let stable = game.object(material).unwrap().stable_id;
            let source = game.create_object_from_definition(&definition, A, Zone::Stack);
            apply(&mut game, source, &definition.spell_effect.as_ref().unwrap().all_effects_owned(), true, vec![]);
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, if exact { Zone::Hand } else { Zone::Exile });
        }
    }
}

#[test]
fn scaled_exile_additional_cost_announces_x_and_pays_two_cards_per_unit() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct XChoice(u32);
    impl DecisionMaker for XChoice {
        fn decide_number(&mut self, _: &GameState, context: &ironsmith::decisions::context::NumberContext) -> u32 {
            assert!(context.is_x_value);
            assert_eq!(context.max, 2, "four eligible cards support at most two repetitions");
            self.0
        }
        fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            assert_eq!(context.min, (2 * self.0) as usize);
            context.candidates.iter().filter(|object| object.legal).take(context.min).map(|object| object.id).collect()
        }
    }
    let text = "Mana cost: {0}\nType: Sorcery\nAs an additional cost to cast this spell, exile exactly two times X cards from your hand and/or graveyard.\nYou gain X life.";
    for definition in definitions("Repeated exile price", text) {
        for x in [0, 1, 2] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let materials: Vec<_> = (0..4).map(|index| {
                let id = game.create_object_from_definition(&body(), A, if index < 2 { Zone::Hand } else { Zone::Graveyard });
                game.object(id).unwrap().stable_id
            }).collect();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).expect("legal repeated price");
            let mut dm = XChoice(x);
            let mut state = PriorityLoopState::new(2);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
            for _ in 0..64 {
                if !state.has_pending_action() { break; }
                let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
                progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
            }
            assert!(!state.has_pending_action());
            assert_eq!(materials.iter().filter(|stable| game.object(game.find_object_by_stable_id(**stable).unwrap()).unwrap().zone == Zone::Exile).count(), (2 * x) as usize);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(A).unwrap().life, 20 + x as i32);
        }
    }
}
fn body() -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition("Body", "Type: Creature\nPower/Toughness: 2/2", false).unwrap()
}
fn activated(definition: &CardDefinition) -> Vec<Effect> {
    definition.abilities.iter().find_map(|a| match &a.kind {
        AbilityKind::Activated(a) => Some(a.effects.all_effects_owned()), _ => None,
    }).expect("activated action")
}
fn apply(game: &mut GameState, source: ObjectId, effects: &[Effect], yes: bool, targets: Vec<ResolvedTarget>) {
    let mut dm = Accept(yes);
    let mut context = EffectContext::new_default(source, A).with_targets(targets).with_decision_maker(&mut dm);
    let sequence = Effect::new(ironsmith::effects::SequenceEffect::new(effects.to_vec()));
    execute_effect(game, &sequence, &mut context).unwrap();
}

fn activate(game: &mut GameState, source: ObjectId) {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).expect("activation must be legal");
    let mut dm = Accept(true);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if state.pending_activation.is_none() && !game.stack.is_empty() { return; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    panic!("activation did not complete");
}

#[test]
fn nourish_preserves_both_optional_branches_and_number() {
    for count in [1, 2, 3] {
        let text = format!("Type: Creature\nPower/Toughness: 2/2\n{{0}}: You may put {count} +1/+1 counters on this creature. If you don't, create {count} 1/1 white Citizen creature tokens.");
        for definition in definitions("Nourishing source", &text) {
            for accept in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                apply(&mut game, source, &activated(&definition), accept, vec![]);
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), if accept {count} else {0});
                let citizens: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|o| o.subtypes.contains(&Subtype::Citizen)).collect();
                assert_eq!(citizens.len(), if accept {0} else {count as usize});
                for token in citizens { assert_eq!(game.controller_of(token), A); assert_eq!(token.power(), Some(1)); assert_eq!(token.toughness(), Some(1)); }
            }
        }
    }
}

#[test]
fn germinate_is_a_choice_not_a_target_and_can_be_declined_without_a_creature() {
    let text = "Type: Enchantment\n{0}: You may put a +1/+1 counter on a creature. If you don't, create a 1/1 green Saproling creature token.";
    for definition in definitions("Germination", text) {
        for (accept, has_creature) in [(true, true), (false, true), (false, false)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            // Original rule says "a creature", not "a creature you control".
            let chosen = has_creature.then(|| game.create_object_from_definition(&body(), B, Zone::Battlefield));
            apply(&mut game, source, &activated(&definition), accept, vec![]);
            if let Some(id) = chosen { assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), u32::from(accept)); }
            assert_eq!(game.battlefield.iter().filter(|id| game.object(**id).unwrap().subtypes.contains(&Subtype::Saproling)).count(), usize::from(!accept));
        }
    }
}

#[test]
fn arm_creates_equipment_attaches_and_changes_only_equipped_power() {
    for definition in definitions("Arming source", &format!("Type: Creature\nPower/Toughness: 2/2\n{{0}}: {ARM}")) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&body(), A, Zone::Battlefield);
        apply(&mut game, source, &activated(&definition), true, vec![]);
        let gear = *game.battlefield.iter().find(|id| game.object(**id).unwrap().subtypes.contains(&Subtype::Equipment)).expect("Equipment created");
        assert_eq!(game.object(gear).unwrap().attached_to, Some(AttachmentTarget::Object(source)));
        assert_eq!(game.current_power(source), Some(3));
        assert_eq!(game.current_toughness(source), Some(2));
        assert_eq!(game.current_power(other), Some(2));
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        let equip_is_legal = |game: &GameState| ironsmith::decision::compute_legal_actions(game, A).unwrap().iter().any(|action|
            matches!(action, ironsmith::decision::LegalAction::ActivateAbility { source: id, .. } if *id == gear));
        assert!(!equip_is_legal(&game), "equip {{2}} cannot activate without mana");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        assert!(equip_is_legal(&game), "equip {{2}} is offered after funding the full cost");
    }
}

#[test]
fn adjudicate_creates_attached_aura_and_illegal_host_creates_nothing() {
    for definition in definitions("Adjudicator", &format!("Type: Creature\nPower/Toughness: 2/2\n{{0}}: {ADJUDICATE}")) {
        for valid_host in [true, false] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let host_definition = if valid_host { body() } else { ironsmith_compiler_runtime::compile_to_runtime_definition("Land", "Type: Land", false).unwrap() };
            let host = game.create_object_from_definition(&host_definition, B, Zone::Battlefield);
            game.tap(host);
            apply(&mut game, source, &activated(&definition), true, vec![ResolvedTarget::Object(host)]);
            let auras: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().subtypes.contains(&Subtype::Aura)).collect();
            assert_eq!(auras.len(), usize::from(valid_host), "illegal Aura attachment must prevent token creation, not merely detach it");
            if let Some(aura) = auras.first().copied() {
                assert_eq!(game.object(aura).unwrap().attached_to, Some(AttachmentTarget::Object(host)));
                game.turn.active_player = A;
                game.turn.priority_player = Some(A);
                activate(&mut game, aura);
                assert!(!game.battlefield.contains(&aura), "sacrificing the Aura is a required activation cost");
                assert!(game.is_tapped(host), "untap waits until the ability resolves");
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept(true)).unwrap();
                assert!(!game.is_tapped(host), "sacrificed token's enchanted reference must retain its attached host");
            }
        }
    }
}

#[test]
fn later_unread_quoted_token_rule_cannot_be_silently_dropped() {
    let rules = "Type: Sorcery\nCreate a white Aura enchantment token with \"Enchant creature\" and \"Sacrifice this token: Untap enchanted creature.\" and \"This token frobniculates the moon.\" attached to target creature.";
    let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
        ironsmith_compiler_runtime::compile_to_runtime_definition("Unread token clause", rules, false)
    });
    assert!(result.is_err(), "a recognized first token rule must not hide a later unrecognized rule");
}

#[test]
fn same_quote_known_rule_cannot_hide_an_unread_later_clause() {
    for body in [
        "Whenever an opponent casts a creature spell, this token isn't a creature until end of turn. This token frobniculates the moon.",
        "Sacrifice this token: Add one mana of any color. Frobnicate the moon.",
    ] {
        let text = format!("Type: Sorcery\nCreate a 1/1 colorless creature token with \"{body}\".");
        let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_runtime_definition("Unread same-quote rule", &text, false));
        assert!(result.is_err(), "known earlier rule must not hide unknown clause: {body}");
    }
}

#[test]
fn equipment_atomic_entry_keeps_prior_target_and_illegal_host_does_not_cancel_creation() {
    let rules = "Target creature gets +1/+1 until end of turn. Create a colorless Equipment artifact token with \"Equipped creature gets +1/+0.\" and equip {2} attached to it.";
    for definition in definitions("Prior target", &format!("Type: Creature\nPower/Toughness: 2/2\n{{0}}: {rules}")) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let host = game.create_object_from_definition(&body(), A, Zone::Battlefield);
        apply(&mut game, source, &activated(&definition), true, vec![ResolvedTarget::Object(host)]);
        let gear = *game.battlefield.iter().find(|id| game.object(**id).unwrap().subtypes.contains(&Subtype::Equipment)).unwrap();
        assert_eq!(game.object(gear).unwrap().attached_to, Some(AttachmentTarget::Object(host)));
        assert_eq!(game.current_power(host), Some(4));
        assert_eq!(game.current_power(source), Some(2));
    }
    let rules = "Create a colorless Equipment artifact token with \"Equipped creature gets +1/+0.\" and equip {2} attached to target creature.";
    for definition in definitions("Illegal Equipment host", &format!("Type: Creature\nPower/Toughness: 2/2\n{{0}}: {rules}")) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let land = ironsmith_compiler_runtime::compile_to_runtime_definition("Land", "Type: Land", false).unwrap();
        let host = game.create_object_from_definition(&land, A, Zone::Battlefield);
        apply(&mut game, source, &activated(&definition), true, vec![ResolvedTarget::Object(host)]);
        let gear = *game.battlefield.iter().find(|id| game.object(**id).unwrap().subtypes.contains(&Subtype::Equipment)).expect("illegal Equipment host does not suppress token creation");
        assert_eq!(game.object(gear).unwrap().attached_to, None);
    }
}

#[test]
fn attachment_in_count_or_value_filter_is_not_a_token_entry_modifier() {
    for rules in [
        "Create a 1/1 white Soldier creature token for each Aura attached to target creature.",
        "Create X 1/1 white Soldier creature tokens, where X is the number of Auras attached to target creature.",
    ] {
        for action in definitions("Count attached Auras", &format!("Type: Sorcery\n{rules}")) {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let aura = ironsmith_compiler_runtime::compile_to_runtime_definition("Aura", "Type: Enchantment — Aura\nEnchant creature", false).unwrap();
            let host = game.create_object_from_definition(&body(), A, Zone::Battlefield);
            let other = game.create_object_from_definition(&body(), A, Zone::Battlefield);
            for recipient in [host, other] {
                let enchantment = game.create_object_from_definition(&aura, A, Zone::Battlefield);
                assert!(game.attach_object_to_target(enchantment, AttachmentTarget::Object(recipient)));
            }
            let source = game.create_object_from_definition(&action, A, Zone::Stack);
            let mut context = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(host)]);
            for effect in action.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
            let soldiers: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|o| o.subtypes.contains(&Subtype::Soldier)).collect();
            assert_eq!(soldiers.len(), 1, "count only the Aura on the selected creature: {rules}; compiled: {}", ironsmith_text::compiled_text_lines(&action).join(" | "));
            assert_eq!(soldiers[0].attached_to, None, "the nested attachment predicate never attaches the created Soldier");
        }
    }
}

#[test]
fn attachment_in_copy_source_filter_does_not_attach_created_copy() {
    for action in definitions("Copy attached Equipment", "Type: Sorcery\nCreate a token that's a copy of target Equipment attached to a creature.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let equipment = ironsmith_compiler_runtime::compile_to_runtime_definition("Equipment", "Type: Artifact — Equipment", false).unwrap();
        let host = game.create_object_from_definition(&body(), A, Zone::Battlefield);
        let original = game.create_object_from_definition(&equipment, A, Zone::Battlefield);
        assert!(game.attach_object_to_target(original, AttachmentTarget::Object(host)));
        let before = game.battlefield.clone();
        let source = game.create_object_from_definition(&action, A, Zone::Stack);
        let mut context = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(original)]);
        for effect in action.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
        let created: Vec<_> = game.battlefield.iter().filter(|id| !before.contains(id)).copied().collect();
        assert_eq!(created.len(), 1);
        assert!(game.object(created[0]).unwrap().subtypes.contains(&Subtype::Equipment));
        assert_eq!(game.object(created[0]).unwrap().attached_to, None);
        assert_eq!(game.object(original).unwrap().attached_to, Some(AttachmentTarget::Object(host)));
    }
}

#[test]
fn object_target_inside_zero_token_count_is_announced_and_respects_shroud() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    for action in definitions("Count-bound target", "Mana cost: {0}\nType: Sorcery\nCreate a 1/1 white Soldier creature token for each Aura attached to target creature.") {
        for shrouded in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let creature = ironsmith_compiler_runtime::compile_to_runtime_definition("Count recipient", if shrouded {"Type: Creature\nPower/Toughness: 2/2\nShroud"} else {"Type: Creature\nPower/Toughness: 2/2"}, false).unwrap();
            let host = game.create_object_from_definition(&creature, A, Zone::Battlefield);
            let spell = game.create_object_from_definition(&action, A, Zone::Hand);
            let offered = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
            if shrouded {
                assert!(offered.is_none(), "zero matching Auras does not remove the required creature target");
                continue;
            }
            let mut dm = Accept(true);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut state = PriorityLoopState::new(2);
            let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(offered.expect("a legal creature target makes the zero-token spell castable")), &mut dm).unwrap();
            for _ in 0..64 {
                if !state.has_pending_action() { break; }
                let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
                progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
            }
            assert!(!state.has_pending_action());
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.stack[0].targets, vec![ironsmith::game_state::Target::Object(host)]);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.battlefield.len(), 1, "the target is real even when no Aura means no token");
        }
    }
}

fn enter_and_stack_token_trigger(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    let move_effect = Effect::new(ironsmith::effects::MoveToZoneEffect::new(
        ironsmith::target::ChooseSpec::SpecificObject(hand), Zone::Battlefield, false,
    ));
    let outcome = execute_effect(game, &move_effect, &mut EffectContext::new_default(hand, A)).unwrap();
    for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
    let source = game.find_object_by_stable_id(stable).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
    assert_eq!(queue.entries.len(), 1, "the actual entry must produce one source-backed trigger");
    ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
    source
}

#[test]
fn nourish_variable_counts_creatures_at_trigger_resolution_for_both_branches() {
    let text = "Type: Creature — Elemental\nPower/Toughness: 1/1\nWhen this creature enters, you may put X +1/+1 counters on this creature, where X is the number of creatures you control. If you don't, create X 1/1 white Citizen creature tokens.";
    for definition in definitions("Autumn resolution", text) {
        for initial_others in [0, 2] {
            for accept in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                for _ in 0..initial_others { game.create_object_from_definition(&body(), A, Zone::Battlefield); }
                // Opposing creatures must not contribute to X.
                game.create_object_from_definition(&body(), B, Zone::Battlefield);
                let source = enter_and_stack_token_trigger(&mut game, &definition);
                // State changes after triggering: X is evaluated at resolution.
                game.create_object_from_definition(&body(), A, Zone::Battlefield);
                let expected = initial_others + 2;
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept(accept)).unwrap();
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), if accept {expected} else {0});
                let citizens = game.battlefield.iter().filter(|id| game.object(**id).unwrap().subtypes.contains(&Subtype::Citizen)).count();
                assert_eq!(citizens, if accept {0} else {expected as usize});
            }
        }
    }
}

#[test]
fn mixed_nourish_and_germinate_keep_two_independent_optional_decisions() {
    struct MixedChoices { answers: [bool; 2], next: usize, recipient: ObjectId }
    impl DecisionMaker for MixedChoices {
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            assert!(self.next < self.answers.len(), "the composed program has exactly two optional actions");
            let answer = self.answers[self.next];
            self.next += 1;
            answer
        }
        fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            assert!(context.candidates.iter().any(|candidate| candidate.id == self.recipient && candidate.legal), "authoritative germinate allows an opponent's creature");
            vec![self.recipient]
        }
    }
    let text = "Type: Creature — Fungus Advisor\nPower/Toughness: 1/1\nWhen this creature enters, you may put a +1/+1 counter on this creature. If you don't, create a 1/1 white Citizen creature token. Then you may put a +1/+1 counter on a creature. If you don't, create a 1/1 green Saproling creature token.";
    for definition in definitions("Envoy decisions", text) {
        for nourish in [false, true] {
            for germinate in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let recipient = game.create_object_from_definition(&body(), B, Zone::Battlefield);
                let source = enter_and_stack_token_trigger(&mut game, &definition);
                let mut choices = MixedChoices { answers: [nourish, germinate], next: 0, recipient };
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert_eq!(choices.next, 2);
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), u32::from(nourish));
                assert_eq!(game.counter_count(recipient, CounterType::PlusOnePlusOne), u32::from(germinate));
                for (subtype, declined) in [(Subtype::Citizen, !nourish), (Subtype::Saproling, !germinate)] {
                    let count = game.battlefield.iter().filter(|id| game.object(**id).unwrap().subtypes.contains(&subtype)).count();
                    assert_eq!(count, usize::from(declined));
                }
            }
        }
    }
}
