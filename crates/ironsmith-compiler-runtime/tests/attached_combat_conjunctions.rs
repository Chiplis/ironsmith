//! UNVALIDATED Attached stat/keyword/combat conjunction scenarios; authored, unrun.
#![allow(dead_code)]
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/attached_combat_conjunctions.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}

use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::combat_state::{AttackTarget, CombatState, declare_attackers};
use ironsmith::effects::{AttachToEffect, EffectExecutor};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::{CardId, CardType, Subtype};
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(id) = self.target {
            assert_eq!(ctx.requirements.len(), 1);
            assert!(
                ctx.requirements[0]
                    .legal_targets
                    .contains(&Target::Object(id))
            );
            vec![Target::Object(id)]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}
fn creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn enter_aura(game: &mut GameState, definition: &CardDefinition, host: ObjectId) -> ObjectId {
    let mut dm = Choices { target: Some(host) };
    cast(game, definition, CastingMethod::Normal, &mut dm);
    resolve_stack_entry_with(game, &mut dm).unwrap();
    let aura = game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).unwrap().name == definition.card.name)
        .unwrap();
    assert_eq!(
        game.object(aura).unwrap().attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(host))
    );
    game.refresh_continuous_state().unwrap();
    aura
}
fn legal_targets(game: &GameState, creature: ObjectId) -> Vec<AttackTarget> {
    ironsmith::decision::compute_legal_attackers(game, &CombatState::default())
        .into_iter()
        .find(|o| o.creature == creature)
        .map(|o| o.valid_targets)
        .unwrap_or_default()
}
fn assert_attack(game: &GameState, attacker: ObjectId, target: AttackTarget, expected: bool) {
    assert_eq!(
        legal_targets(game, attacker).contains(&target),
        expected,
        "preview {target:?}"
    );
    let mut copy = game.clone();
    assert_eq!(
        declare_attackers(
            &mut copy,
            &mut CombatState::default(),
            vec![(attacker, target.clone())]
        )
        .is_ok(),
        expected,
        "declaration {target:?}"
    );
}
fn forbidden_a(game: &GameState, host: ObjectId) {
    assert_attack(game, host, AttackTarget::Player(A), false);
    assert_attack(game, host, AttackTarget::Player(C), true);
}
#[test]
fn all_six_paid_auras_keep_stats_keyword_and_granters_attack_rule() {
    assert_eq!(fixtures().len(), 6);
    for (name, boost, keyword) in [
        ("Vow of Duty", 2, StaticAbility::vigilance()),
        ("Vow of Flight", 2, StaticAbility::flying()),
        ("Vow of Lightning", 2, StaticAbility::first_strike()),
        ("Vow of Malice", 2, StaticAbility::intimidate()),
        ("Vow of Torment", 2, StaticAbility::menace()),
        ("Vow of Wildness", 3, StaticAbility::trample()),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let host = creature(&mut game, B, "Enchanted attacker");
            let other = creature(&mut game, B, "Unenchanted attacker");
            let aura = enter_aura(&mut game, &definition, host);
            game.turn.active_player = B;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
            assert_eq!(game.current_power(host), Some(2 + boost));
            assert_eq!(game.current_toughness(host), Some(2 + boost));
            assert!(game.object_has_ability(host, &keyword));
            forbidden_a(&game, host);
            assert_attack(&game, other, AttackTarget::Player(A), true);
            game.set_current_controller(aura, C).unwrap();
            game.refresh_continuous_state().unwrap();
            assert_attack(&game, host, AttackTarget::Player(A), true);
            assert_attack(&game, host, AttackTarget::Player(C), false);
            game.set_current_controller(aura, A).unwrap();
            game.refresh_continuous_state().unwrap();
            AttachToEffect::new(ChooseSpec::SpecificObject(other))
                .execute(&mut game, &mut EffectContext::new_default(aura, A))
                .unwrap();
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_power(host), Some(2));
            assert_eq!(game.current_power(other), Some(2 + boost));
            assert_attack(&game, host, AttackTarget::Player(A), true);
            forbidden_a(&game, other);
            game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_power(other), Some(2));
            assert!(!game.object_has_ability(other, &keyword));
            assert_attack(&game, other, AttackTarget::Player(A), true);
        }
    }
}
#[test]
fn player_planeswalker_and_battle_are_distinct_in_preview_and_declaration() {
    for definition in definitions("Vow of Flight") {
        let mut game = game();
        let host = creature(&mut game, B, "Attacker");
        enter_aura(&mut game, &definition, host);
        let pw_card = CardBuilder::new(CardId::new(), "Alice walker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .build();
        let walker = game.create_object_from_card(&pw_card, A, Zone::Battlefield);
        let battle_card = CardBuilder::new(CardId::new(), "Bob Siege")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![Subtype::Siege])
            .defense(5)
            .build();
        let battle = game.create_object_from_card(&battle_card, B, Zone::Battlefield);
        assert!(game.set_battle_protector(battle, A));
        game.turn.active_player = B;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        game.refresh_continuous_state().unwrap();
        assert_attack(&game, host, AttackTarget::Player(A), false);
        assert_attack(&game, host, AttackTarget::Planeswalker(walker), false);
        assert_attack(&game, host, AttackTarget::Battle(battle), true);
    }
}
#[test]
fn creature_ability_loss_does_not_erase_unquoted_aura_rule_but_aura_loss_does() {
    for definition in definitions("Vow of Duty") {
        let mut game = game();
        let host = creature(&mut game, B, "Attacker");
        let aura = enter_aura(&mut game, &definition, host);
        game.turn.active_player = B;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        ironsmith::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(host),
            ironsmith::continuous::Modification::RemoveAllAbilities,
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(aura, A))
        .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!game.object_has_ability(host, &StaticAbility::vigilance()));
        forbidden_a(&game, host);
        ironsmith::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(aura),
            ironsmith::continuous::Modification::RemoveAllAbilities,
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(host, B))
        .unwrap();
        game.refresh_continuous_state().unwrap();
        assert_attack(&game, host, AttackTarget::Player(A), true);
        assert_eq!(game.current_power(host), Some(2));
    }
}
#[test]
fn phased_attachment_supplies_no_new_grants_and_resumes_on_phase_in() {
    for definition in definitions("Vow of Torment") {
        let mut game = game();
        let host = creature(&mut game, B, "Attacker");
        let aura = enter_aura(&mut game, &definition, host);
        game.turn.active_player = B;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        game.phase_out(aura);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(host), Some(2));
        assert_attack(&game, host, AttackTarget::Player(A), true);
        game.phase_in(aura);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(host), Some(4));
        forbidden_a(&game, host);
    }
}

fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}
