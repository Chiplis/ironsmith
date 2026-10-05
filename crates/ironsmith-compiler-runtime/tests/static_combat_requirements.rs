//! Complete printed bodies and native declaration scenarios, authored without
//! running the compiler, tests, or corpus during the source repair campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState, declare_blockers};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::game_loop::apply_attacker_declarations;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::target::ChooseSpec;
use ironsmith::effect::Until;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!direct_loss.is_lossy(), "{}", direct_loss.reasons_text());
    let (compiled, artifact_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, text, false)
    });
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!artifact_loss.is_lossy(), "{}", artifact_loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let definitions = [direct, materialize_artifact(&restored).unwrap()];
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    definitions
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    g.turn.phase = ironsmith::game_state::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for color in [ironsmith::mana::ManaSymbol::White, ironsmith::mana::ManaSymbol::Blue,
        ironsmith::mana::ManaSymbol::Black, ironsmith::mana::ManaSymbol::Red,
        ironsmith::mana::ManaSymbol::Green, ironsmith::mana::ManaSymbol::Colorless] {
        g.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    g
}
fn creature(g: &mut GameState, controller: PlayerId, text: &str) -> ObjectId {
    let d = compile_to_runtime_definition("Combat participant", format!(
        "Type: Creature — Human\nPower/Toughness: 2/2\n{text}"), false).unwrap();
    let id = g.create_object_from_definition(&d, controller, Zone::Battlefield);
    g.remove_summoning_sickness(id);
    id
}
fn attack(g: &mut GameState, active: PlayerId, entries: &[(ObjectId, PlayerId)])
    -> Result<CombatState, ironsmith::game_loop::GameLoopError>
{
    g.turn.active_player = active;
    for (creature, _) in entries {
        g.untap(*creature);
    }
    g.refresh_continuous_state().unwrap();
    let mut combat = CombatState::default();
    apply_attacker_declarations(g, &mut combat, &mut TriggerQueue::new(),
        &entries.iter().map(|(creature, player)| AttackerDeclaration {
            creature: *creature, target: AttackTarget::Player(*player),
        }).collect::<Vec<_>>())?;
    Ok(combat)
}

#[test]
fn defender_and_vigilance_bodies_enforce_only_feasible_combat_obligations() {
    for (name, text, attacks) in [
        ("Razorgrass Screen", "Mana cost: {1}\nType: Artifact Creature — Wall\nPower/Toughness: 2/1\nDefender (This creature can't attack.)\nThis creature blocks each combat if able.", false),
        ("Iron Golem", "Mana cost: {4}\nType: Artifact Creature — Golem\nPower/Toughness: 5/3\nVigilance\nThis creature attacks or blocks each combat if able.", true),
        ("Relentless Raptor", "Mana cost: {R}{W}\nType: Creature — Dinosaur\nPower/Toughness: 3/3\nVigilance\nThis creature attacks or blocks each combat if able.", true),
    ] {
        for definition in definitions(name, text) {
            let mut g = game();
            let required = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            // Summoning sickness excuses attacking, but never blocking.
            assert!(attack(&mut g, A, &[]).is_ok());
            g.remove_summoning_sickness(required);
            let mut combat = CombatState::default();
            let mut queue = TriggerQueue::new();
            g.refresh_continuous_state().unwrap();
            assert_eq!(apply_attacker_declarations(&mut g, &mut combat, &mut queue, &[]).is_err(), attacks);
            assert!(combat.attackers.is_empty());
            assert!(queue.entries.is_empty());
            assert!(!g.is_tapped(required));
            if attacks {
                assert!(g.object_has_static_ability_id(required, StaticAbilityId::Vigilance));
                assert!(attack(&mut g, A, &[(required, B)]).is_ok());
                assert!(!g.is_tapped(required));
            } else {
                assert!(attack(&mut g, A, &[(required, B)]).is_err());
            }
            let other = creature(&mut g, B, "");
            let mut defending = attack(&mut g, B, &[(other, A)]).unwrap();
            assert!(declare_blockers(&g, &mut defending, vec![]).is_err());
            assert!(defending.blockers.values().all(Vec::is_empty));
            declare_blockers(&g, &mut defending, vec![(required, other)]).unwrap();
            assert_eq!(defending.blockers[&other], vec![required]);
            // An unrelated defending player cannot force this creature to block.
            let mut elsewhere = attack(&mut g, B, &[(other, C)]).unwrap();
            declare_blockers(&g, &mut elsewhere, vec![]).unwrap();
            // A tapped required blocker is unable; an empty legal set is legal.
            g.tap(required);
            let mut unable = attack(&mut g, B, &[(other, A)]).unwrap();
            declare_blockers(&g, &mut unable, vec![]).unwrap();
            g.untap(required);
            let flyer = creature(&mut g, B, "Flying");
            let mut evaded = attack(&mut g, B, &[(flyer, A)]).unwrap();
            declare_blockers(&g, &mut evaded, vec![]).unwrap();
            // Removing the source's own abilities removes its own obligation.
            g.object_mut(required).unwrap().abilities_mut().clear();
            g.refresh_continuous_state().unwrap();
            let mut no_rule = attack(&mut g, B, &[(other, A)]).unwrap();
            declare_blockers(&g, &mut no_rule, vec![]).unwrap();
        }
    }
}

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/static_combat_requirements.json.fixture")).unwrap()
}
fn card(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    definitions(name, row["text"].as_str().unwrap())
}
fn remove_abilities(g: &mut GameState, source: ObjectId, target: ObjectId) {
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(target),
        ironsmith::continuous::Modification::RemoveAllAbilities, Until::EndOfTurn)
        .execute(g, &mut EffectContext::new_default(source, A)).unwrap();
    g.refresh_continuous_state().unwrap();
}
fn can_block(g: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    ironsmith::rules::combat::can_block(g.object(attacker).unwrap(), g.object(blocker).unwrap(), g)
}
fn assert_block(g: &GameState, combat: &CombatState, blocker: ObjectId, attacker: ObjectId, valid: bool) {
    assert_eq!(can_block(g, attacker, blocker), valid);
    let mut declaration = combat.clone();
    assert_eq!(declare_blockers(g, &mut declaration, vec![(blocker, attacker)]).is_ok(), valid);
    if !valid {
        assert!(declaration.blockers.values().all(Vec::is_empty));
    }
}

#[test]
fn all_twelve_full_frozen_bodies_compile_independently_and_round_trip() {
    assert_eq!(rows().len(), 12);
    for row in rows() { card(row["name"].as_str().unwrap()); }
    for body in ["This {R} creature blocks each combat if able.",
        "This creature: blocks each combat if able."] {
        assert!(compile_to_runtime_definition("Malformed obligation",
            format!("Type: Creature — Human\nPower/Toughness: 2/2\n{body}"), false).is_err());
    }
}

#[test]
fn global_flying_filter_and_reach_are_independent_live_rules() {
    for name in ["Dense Canopy", "Chaosphere"] {
        for definition in card(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let air = creature(&mut g, B, "Flying");
            let ground = creature(&mut g, B, "");
            let air_blocker = creature(&mut g, A, "Flying");
            let ground_blocker = creature(&mut g, A, "");
            let air_combat = attack(&mut g, B, &[(air, A)]).unwrap();
            assert_block(&g, &air_combat, air_blocker, air, true);
            assert_block(&g, &air_combat, ground_blocker, air, name == "Chaosphere");
            let ground_combat = attack(&mut g, B, &[(ground, A)]).unwrap();
            assert_block(&g, &ground_combat, air_blocker, ground, false);
            assert_block(&g, &ground_combat, ground_blocker, ground, true);
            // A live flying filter stops selecting a creature whose flying is removed.
            remove_abilities(&mut g, source, air_blocker);
            assert_block(&g, &ground_combat, air_blocker, ground, true);
            // Removing an unrelated creature's abilities cannot remove the rule.
            let second_air_blocker = creature(&mut g, A, "Flying");
            g.refresh_continuous_state().unwrap();
            assert_block(&g, &ground_combat, second_air_blocker, ground, false);
            g.phase_out(source);
            g.refresh_continuous_state().unwrap();
            assert_block(&g, &ground_combat, second_air_blocker, ground, true);
            g.phase_in(source);
            g.refresh_continuous_state().unwrap();
            assert_block(&g, &ground_combat, second_air_blocker, ground, false);
            remove_abilities(&mut g, ground, source);
            assert_block(&g, &ground_combat, second_air_blocker, ground, true);
        }
    }
}

#[test]
fn global_attack_and_block_requirements_survive_recipient_loss_and_maximize_feasible_sets() {
    for definition in card("Grand Melee") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(attack(&mut g, A, &[]).is_ok());
        let first = creature(&mut g, A, "");
        let second = creature(&mut g, A, "");
        let unable = creature(&mut g, A, "Defender");
        remove_abilities(&mut g, source, first);
        assert!(attack(&mut g, A, &[(first, B)]).is_err());
        assert!(!g.is_tapped(first));
        assert!(attack(&mut g, A, &[(first, B), (second, C)]).is_ok());
        let attacker = creature(&mut g, B, "Menace");
        g.untap(first); g.untap(second);
        let combat = attack(&mut g, B, &[(attacker, A)]).unwrap();
        let mut omitted = combat.clone();
        assert!(declare_blockers(&g, &mut omitted, vec![(first, attacker), (second, attacker)]).is_err());
        assert!(omitted.blockers.values().all(Vec::is_empty));
        let mut all = combat.clone();
        declare_blockers(&g, &mut all, vec![(first, attacker), (second, attacker), (unable, attacker)]).unwrap();
        // If only one blocker can block menace, zero satisfies the legal maximum.
        g.tap(second); g.tap(unable);
        let mut infeasible = combat.clone();
        declare_blockers(&g, &mut infeasible, vec![]).unwrap();
        g.phase_out(source); g.refresh_continuous_state().unwrap();
        let mut phased = combat.clone();
        declare_blockers(&g, &mut phased, vec![]).unwrap();
        g.phase_in(source); g.untap(second); g.refresh_continuous_state().unwrap();
        let mut restored = combat.clone();
        assert!(declare_blockers(&g, &mut restored, vec![]).is_err());
        remove_abilities(&mut g, first, source);
        let mut gone = combat.clone();
        declare_blockers(&g, &mut gone, vec![]).unwrap();
    }
}

#[test]
fn coordinated_keyword_grants_keep_distinct_controller_scopes_and_rule_ownership() {
    for (name, keyword, global) in [
        ("Avatar of Slaughter", StaticAbilityId::DoubleStrike, true),
        ("Hellraiser Goblin", StaticAbilityId::Haste, false),
    ] {
        for definition in card(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let ours = creature(&mut g, A, "");
            let theirs = creature(&mut g, B, "");
            g.remove_summoning_sickness(source);
            g.refresh_continuous_state().unwrap();
            assert!(g.object_has_static_ability_id(ours, keyword));
            assert_eq!(g.object_has_static_ability_id(theirs, keyword), global);
            assert!(attack(&mut g, A, &[(source, B)]).is_err());
            remove_abilities(&mut g, source, ours);
            assert!(!g.object_has_static_ability_id(ours, keyword));
            assert!(attack(&mut g, A, &[(source, B)]).is_err());
            assert!(attack(&mut g, A, &[(source, B), (ours, C)]).is_ok());
            assert_eq!(attack(&mut g, B, &[]).is_err(), global);
            g.set_current_controller(source, C).unwrap();
            g.refresh_continuous_state().unwrap();
            assert_eq!(attack(&mut g, B, &[]).is_err(), global);
            g.untap(ours);
            assert_eq!(attack(&mut g, A, &[]).is_err(), global);
            g.phase_out(source); g.refresh_continuous_state().unwrap();
            assert!(attack(&mut g, A, &[]).is_ok());
        }
    }
}

#[derive(Default)]
struct TargetChoice {
    target: Option<ObjectId>,
    forbidden: Option<ObjectId>,
    seen: bool,
}
impl ironsmith::decision::DecisionMaker for TargetChoice {
    fn decide_targets(&mut self, game: &GameState, context: &ironsmith::decisions::context::TargetsContext)
        -> Vec<ironsmith::Target>
    {
        use ironsmith::decision::DecisionMaker;
        let Some(target) = self.target else {
            return ironsmith::decision::SelectFirstDecisionMaker.decide_targets(game, context);
        };
        assert_eq!(context.requirements.len(), 1);
        let legal = &context.requirements[0].legal_targets;
        assert!(legal.contains(&ironsmith::Target::Object(target)));
        if let Some(forbidden) = self.forbidden {
            assert!(!legal.contains(&ironsmith::Target::Object(forbidden)));
        }
        self.seen = true;
        vec![ironsmith::Target::Object(target)]
    }
}
fn cast_attached(g: &mut GameState, definition: &CardDefinition, host: ObjectId,
    method: ironsmith::alternative_cast::CastingMethod) -> ObjectId
{
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse,
        apply_priority_response_with_dm, apply_decision_context_with_dm,
        put_triggers_on_stack_with_dm, resolve_stack_entry_with};
    let land = compile_to_runtime_definition("Invalid attachment target", "Type: Land", false).unwrap();
    let land = g.create_object_from_definition(&land, A, Zone::Battlefield);
    let spell = g.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand, casting_method: method };
    assert!(compute_legal_actions(g, A).unwrap().contains(&action));
    let mut choices = TargetChoice { target: Some(host), forbidden: Some(land), seen: false };
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(g, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut choices).unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_mana_ability.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &context, &mut choices).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_mana_ability.is_none());
    assert!(choices.seen);
    for _ in 0..24 {
        put_triggers_on_stack_with_dm(g, &mut queue, &mut choices).unwrap();
        if g.stack_is_empty() { break; }
        resolve_stack_entry_with(g, &mut choices).unwrap();
    }
    assert!(g.stack_is_empty());
    let aura = *g.battlefield.iter().find(|id| g.object(**id).unwrap().name == definition.card.name).unwrap();
    assert_eq!(g.object(aura).unwrap().attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(host)));
    g.refresh_continuous_state().unwrap();
    aura
}

#[test]
fn auras_cast_draw_and_keep_the_flying_limit_on_the_attachment_source() {
    for name in ["Air Bladder", "Stratus Walk"] {
        for definition in card(name) {
            let mut g = game();
            let host = creature(&mut g, A, "");
            let other = creature(&mut g, A, "Flying");
            let ground = creature(&mut g, B, "");
            let air = creature(&mut g, B, "Flying");
            let draw = compile_to_runtime_definition("Draw witness", "Type: Land", false).unwrap();
            g.create_object_from_definition(&draw, A, Zone::Library);
            let aura = cast_attached(&mut g, &definition, host,
                ironsmith::alternative_cast::CastingMethod::Normal);
            assert_eq!(g.player(A).unwrap().hand.iter().any(|id|
                g.object(*id).unwrap().name == "Draw witness"), name == "Stratus Walk");
            assert!(g.object_has_static_ability_id(host, StaticAbilityId::Flying));
            let ground_combat = attack(&mut g, B, &[(ground, A)]).unwrap();
            assert_block(&g, &ground_combat, host, ground, false);
            assert_block(&g, &ground_combat, other, ground, true);
            let air_combat = attack(&mut g, B, &[(air, A)]).unwrap();
            assert_block(&g, &air_combat, host, air, true);
            remove_abilities(&mut g, aura, host);
            assert!(!g.object_has_static_ability_id(host, StaticAbilityId::Flying));
            assert_block(&g, &ground_combat, host, ground, false);
            g.phase_out(aura); g.refresh_continuous_state().unwrap();
            assert_block(&g, &ground_combat, host, ground, true);
            g.phase_in(aura); g.refresh_continuous_state().unwrap();
            assert_block(&g, &ground_combat, host, ground, false);
            // Move the attachment and the source rule follows its new host.
            ironsmith::effects::AttachToEffect::new(ChooseSpec::SpecificObject(other))
                .execute(&mut g, &mut EffectContext::new_default(aura, A)).unwrap();
            g.refresh_continuous_state().unwrap();
            assert_block(&g, &ground_combat, host, ground, true);
            assert_block(&g, &ground_combat, other, ground, false);
            remove_abilities(&mut g, host, aura);
            assert_block(&g, &ground_combat, other, ground, true);
        }
    }
}

#[test]
fn spirespine_bestow_retains_stats_and_separate_self_and_host_requirements() {
    for definition in card("Spirespine") {
        let mut g = game();
        let host = creature(&mut g, A, "");
        let attacker = creature(&mut g, B, "");
        let before = g.player(A).unwrap().mana_pool.total();
        let source = cast_attached(&mut g, &definition, host,
            ironsmith::alternative_cast::CastingMethod::Alternative(0));
        assert_eq!(g.player(A).unwrap().mana_pool.total(), before - 5);
        assert!(!g.calculated_card_types(source).contains(&ironsmith::CardType::Creature));
        assert_eq!((g.current_power(host), g.current_toughness(host)), (Some(6), Some(3)));
        let mut combat = attack(&mut g, B, &[(attacker, A)]).unwrap();
        assert!(declare_blockers(&g, &mut combat, vec![]).is_err());
        declare_blockers(&g, &mut combat, vec![(host, attacker)]).unwrap();
        remove_abilities(&mut g, source, host);
        let mut still_required = attack(&mut g, B, &[(attacker, A)]).unwrap();
        assert!(declare_blockers(&g, &mut still_required, vec![]).is_err());
        g.move_object_by_effect(host, Zone::Graveyard).unwrap();
        ironsmith::game_loop::check_and_apply_sbas(&mut g, &mut TriggerQueue::new()).unwrap();
        g.refresh_continuous_state().unwrap();
        assert!(g.calculated_card_types(source).contains(&ironsmith::CardType::Creature));
        assert_eq!((g.current_power(source), g.current_toughness(source)), (Some(4), Some(1)));
        let mut detached = attack(&mut g, B, &[(attacker, A)]).unwrap();
        assert!(declare_blockers(&g, &mut detached, vec![]).is_err());
        declare_blockers(&g, &mut detached, vec![(source, attacker)]).unwrap();
        remove_abilities(&mut g, attacker, source);
        let mut removed = attack(&mut g, B, &[(attacker, A)]).unwrap();
        declare_blockers(&g, &mut removed, vec![]).unwrap();
    }
}

#[test]
fn watchdog_untapped_condition_and_attack_destination_remain_executable() {
    for definition in card("Watchdog") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let at_us = creature(&mut g, B, "");
        let at_them = creature(&mut g, B, "");
        let mut combat = attack(&mut g, B, &[(at_us, A), (at_them, C)]).unwrap();
        // The live combat state supplies the attack-destination filter.
        g.combat = Some(combat.clone());
        g.refresh_continuous_state().unwrap();
        assert_eq!(g.current_power(at_us), Some(1));
        assert_eq!(g.current_power(at_them), Some(2));
        assert!(declare_blockers(&g, &mut combat, vec![]).is_err());
        declare_blockers(&g, &mut combat, vec![(source, at_us)]).unwrap();
        g.tap(source); g.refresh_continuous_state().unwrap();
        assert_eq!(g.current_power(at_us), Some(2));
        g.untap(source); g.refresh_continuous_state().unwrap();
        assert_eq!(g.current_power(at_us), Some(1));
        g.set_current_controller(source, C).unwrap(); g.refresh_continuous_state().unwrap();
        assert_eq!(g.current_power(at_us), Some(2));
        assert_eq!(g.current_power(at_them), Some(1));
        g.phase_out(source); g.refresh_continuous_state().unwrap();
        assert_eq!(g.current_power(at_them), Some(2));
    }
}

#[test]
fn quoted_requirement_belongs_to_the_recipient_and_can_be_removed_there() {
    for definition in definitions("Quoted requirement granter",
        "Type: Enchantment\nCreatures you control have \"This creature blocks each combat if able.\"")
    {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let host = creature(&mut g, A, "");
        let attacker = creature(&mut g, B, "");
        let mut combat = attack(&mut g, B, &[(attacker, A)]).unwrap();
        assert!(declare_blockers(&g, &mut combat, vec![]).is_err());
        assert!(g.object_has_static_ability_id(host, StaticAbilityId::MustBlock));
        remove_abilities(&mut g, source, host);
        let mut removed = attack(&mut g, B, &[(attacker, A)]).unwrap();
        declare_blockers(&g, &mut removed, vec![]).unwrap();
    }
}

#[test]
fn any_legal_attack_target_satisfies_a_general_requirement_but_watchdog_only_counts_its_player() {
    use ironsmith::card::CardBuilder;
    use ironsmith::{CardId, CardType};
    for definition in card("Iron Golem") {
        let mut g = game();
        let required = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        g.remove_summoning_sickness(required);
        let walker = CardBuilder::new(CardId::new(), "Defending walker")
            .card_types(vec![CardType::Planeswalker]).loyalty(5).build();
        let walker = g.create_object_from_card(&walker, B, Zone::Battlefield);
        let mut combat = CombatState::default();
        g.refresh_continuous_state().unwrap();
        apply_attacker_declarations(&mut g, &mut combat, &mut TriggerQueue::new(),
            &[AttackerDeclaration { creature: required, target: AttackTarget::Planeswalker(walker) }]).unwrap();
        assert_eq!(combat.attackers.len(), 1);
        assert!(!g.is_tapped(required));
    }
    for definition in card("Watchdog") {
        let mut g = game();
        let watchdog = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = creature(&mut g, B, "");
        let walker = CardBuilder::new(CardId::new(), "Protected walker")
            .card_types(vec![CardType::Planeswalker]).loyalty(5).build();
        let walker = g.create_object_from_card(&walker, A, Zone::Battlefield);
        g.turn.active_player = B;
        let mut combat = CombatState::default();
        g.refresh_continuous_state().unwrap();
        apply_attacker_declarations(&mut g, &mut combat, &mut TriggerQueue::new(),
            &[AttackerDeclaration { creature: attacker, target: AttackTarget::Planeswalker(walker) }]).unwrap();
        g.combat = Some(combat.clone()); g.refresh_continuous_state().unwrap();
        assert_eq!(g.current_power(attacker), Some(2));
        // Controlling the attacked planeswalker makes A a defending player.
        assert!(declare_blockers(&g, &mut combat, vec![]).is_err());
        declare_blockers(&g, &mut combat, vec![(watchdog, attacker)]).unwrap();
    }
}

#[test]
fn unattached_spirespine_never_obliges_an_unrelated_enchanted_creature() {
    for definition in card("Spirespine") {
        for detached in [false, true] {
            let mut g = game();
            let unrelated = creature(&mut g, A, "Flying");
            let other_aura = compile_to_runtime_definition("Unrelated Aura",
                "Type: Enchantment — Aura\nEnchant creature", false).unwrap();
            let other_aura = g.create_object_from_definition(&other_aura, A, Zone::Battlefield);
            ironsmith::effects::AttachToEffect::new(ChooseSpec::SpecificObject(unrelated))
                .execute(&mut g, &mut EffectContext::new_default(other_aura, A)).unwrap();
            let source = if detached {
                let former_host = creature(&mut g, A, "");
                let source = cast_attached(&mut g, &definition, former_host,
                    ironsmith::alternative_cast::CastingMethod::Alternative(0));
                g.move_object_by_effect(former_host, Zone::Graveyard).unwrap();
                ironsmith::game_loop::check_and_apply_sbas(&mut g, &mut TriggerQueue::new()).unwrap();
                source
            } else {
                g.create_object_from_definition(&definition, A, Zone::Battlefield)
            };
            assert!(g.object(source).unwrap().attached_to.is_none());
            g.refresh_continuous_state().unwrap();
            let attacker = creature(&mut g, B, "Flying");
            let mut combat = attack(&mut g, B, &[(attacker, A)]).unwrap();
            assert!(!can_block(&g, attacker, source));
            assert!(can_block(&g, attacker, unrelated));
            assert!(!ironsmith::rules::combat::must_block_with_game(g.object(unrelated).unwrap(), &g));
            declare_blockers(&g, &mut combat, vec![]).unwrap();
            // Its own printed requirement still applies whenever it can block.
            remove_abilities(&mut g, source, attacker);
            let mut grounded = attack(&mut g, B, &[(attacker, A)]).unwrap();
            assert!(declare_blockers(&g, &mut grounded, vec![]).is_err());
            declare_blockers(&g, &mut grounded, vec![(source, attacker)]).unwrap();
        }
    }
}
