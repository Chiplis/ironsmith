// Source-authored compatibility cases, UNRUN. Exercise the real private
// public-audit projection; native named JSON is the supported roundtrip here.
use super::{SyncRestrictedManaUnit, sync_restricted_mana};
use ironsmith::ability::{Ability, AbilityKind, RestrictedManaUnit};
use ironsmith::effects::player::GrantNextSpellAbilityEffect;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::triggers::Trigger;
use ironsmith::{Effect, ObjectId, PlayerId};
use ironsmith_compiled_artifact::{WireAbility, WireEffect};
use ironsmith_core::trigger_model::PlayerAttackGrouping;
use ironsmith_core::{
    ActivatedAbilityCostCondition as CostCondition, ActivatedAbilityKeyword as Keyword,
    CombatParticipantCondition, CompilerTrigger, Condition, ManaCost, ManaPaymentPredicate,
    ManaSpendPayload, ManaSymbol, ManaUsageRestriction, ObjectFilter, PlayerFilter,
    ResolutionProgram, StaticAbilityPayload, TotalCost, TriggerKind,
};
use serde_json::Value;

fn unit(
    restriction: ManaPaymentPredicate,
    predicate: ManaPaymentPredicate,
    effects: Vec<Effect>,
) -> RestrictedManaUnit {
    RestrictedManaUnit {
        symbol: ManaSymbol::Green,
        source: ObjectId::from_raw(17),
        source_controller: Some(PlayerId::from_index(0)),
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(restriction),
            on_spend: vec![ManaSpendPayload {
                predicate,
                effects: ResolutionProgram::from_effects(effects),
                choices: vec![],
            }],
        }],
    }
}

fn project(source: RestrictedManaUnit) -> (SyncRestrictedManaUnit, Value) {
    let first = sync_restricted_mana(&[source.clone()]).expect("typed mana payload must project");
    let repeated = sync_restricted_mana(&[source]).expect("repeat must project");
    let bytes = serde_json::to_vec(&first).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&repeated).unwrap());
    let restored: Vec<SyncRestrictedManaUnit> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
    let json = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(first.len(), 1);
    (first.into_iter().next().unwrap(), json)
}

fn distinguish(seen: &mut Vec<Value>, next: Value) {
    for previous in seen.iter() {
        assert_ne!(previous, &next, "different typed programs must remain distinct");
    }
    seen.push(next);
}

fn transaction(
    projected: &SyncRestrictedManaUnit,
) -> (&Option<ManaPaymentPredicate>, &ManaSpendPayload<WireEffect>) {
    assert_eq!(projected.restrictions.len(), 1);
    let ManaUsageRestriction::PaymentTransaction { restriction, on_spend } =
        &projected.restrictions[0]
    else {
        panic!("payment transaction must retain its typed carrier");
    };
    assert_eq!(on_spend.len(), 1);
    (restriction, &on_spend[0])
}

fn granted_effect(ability: Ability) -> Effect {
    Effect::new(GrantNextSpellAbilityEffect::new(
        PlayerFilter::You,
        ObjectFilter::creature(),
        ability,
    ))
}

fn granted_ability(projected: &SyncRestrictedManaUnit) -> WireAbility {
    let (_, payload) = transaction(projected);
    let effects = payload.effects.all_effects();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].kind(), "GrantNextSpellAbilityEffect");
    let grant: ironsmith_core::GrantNextSpellAbilityEffect<WireAbility> =
        serde_json::from_value(effects[0].payload().clone()).unwrap();
    assert_eq!(grant.player, PlayerFilter::You);
    assert_eq!(grant.filter, ObjectFilter::creature());
    grant.ability
}

#[test]
fn payment_transaction_retains_each_keyword_in_both_predicate_positions() {
    let mut seen = Vec::new();
    for keyword in [
        Keyword::Equip, Keyword::PowerUp, Keyword::ClassLevel(2), Keyword::ClassLevel(3),
        Keyword::Cycling, Keyword::Ninjutsu, Keyword::Boast, Keyword::Exhaust,
    ] {
        let exact = ManaPaymentPredicate::ActivatedAbilityKeyword(keyword);
        for (restriction, predicate) in [
            (exact.clone(), ManaPaymentPredicate::Any),
            (ManaPaymentPredicate::Any, exact.clone()),
            (exact.clone(), exact),
        ] {
            let (projected, encoded) = project(unit(
                restriction.clone(), predicate.clone(), vec![Effect::gain_life(2)],
            ));
            let (actual_restriction, actual_payload) = transaction(&projected);
            assert_eq!(actual_restriction.as_ref(), Some(&restriction));
            assert_eq!(actual_payload.predicate, predicate);
            let effects = actual_payload.effects.all_effects();
            assert_eq!(effects.len(), 1);
            assert_eq!(effects[0].kind(), "GainLifeEffect");
            distinguish(&mut seen, encoded);
        }
    }
}

#[test]
fn granted_typed_static_models_retain_new_activation_cost_selectors() {
    let selectors = [
        CostCondition::Keyword(Keyword::ClassLevel(2)),
        CostCondition::Keyword(Keyword::ClassLevel(3)),
        CostCondition::Keyword(Keyword::Cycling),
        CostCondition::Keyword(Keyword::Ninjutsu),
        CostCondition::Keyword(Keyword::Boast),
        CostCondition::Keyword(Keyword::Exhaust),
        CostCondition::NonManaAbility,
        CostCondition::LoyaltyAbility,
        CostCondition::Activator(PlayerFilter::You),
        CostCondition::Activator(PlayerFilter::Opponent),
        CostCondition::All(vec![
            CostCondition::NonManaAbility, CostCondition::Activator(PlayerFilter::Opponent),
        ]),
    ];
    let mut seen = Vec::new();
    for selector in selectors {
        for increase in [false, true] {
            let filter = ObjectFilter::creature();
            let model = if increase {
                ironsmith_core::StaticAbility::increase_activated_ability_costs(
                    filter,
                    TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
                )
            } else {
                ironsmith_core::StaticAbility::reduce_activated_ability_costs(filter, 2, Some(1))
            }.with_activated_ability_cost_condition(selector.clone());
            // Native generic cost/restriction executors need not own a
            // canonical model. Carry the typed model into the grant explicitly.
            let ability = StaticAbility::from_model(model).into();
            let (projected, encoded) = project(unit(
                ManaPaymentPredicate::Any,
                ManaPaymentPredicate::Any,
                vec![granted_effect(ability)],
            ));
            let ironsmith_core::AbilityKind::Static(ability) = granted_ability(&projected).kind else {
                panic!("static cost grant must remain a static ability");
            };
            match ability.payload {
                StaticAbilityPayload::ActivatedAbilityCostReduction {
                    condition, reduction, minimum_total_mana, ..
                } => {
                    assert!(!increase);
                    assert_eq!(condition, Some(selector.clone()));
                    assert_eq!(reduction, 2);
                    assert_eq!(minimum_total_mana, Some(1));
                }
                StaticAbilityPayload::ActivatedAbilityCostIncrease { ability_condition, .. } => {
                    assert!(increase);
                    assert_eq!(ability_condition, Some(selector.clone()));
                }
                _ => panic!("projection must retain the cost-modifier model"),
            }
            distinguish(&mut seen, encoded);
        }
    }
}

#[test]
fn granted_triggered_models_keep_combat_grouping_and_condition_timing_distinct() {
    let mut seen = Vec::new();
    for participant in [
        CombatParticipantCondition::YouAreDefendingPlayer,
        CombatParticipantCondition::AttackingPlayerAttackedYouOrYourPlaneswalker,
        CombatParticipantCondition::AttackingPlayerIsNotAttackingYou,
        CombatParticipantCondition::AnyAttackedPlayerIsPoisoned,
        CombatParticipantCondition::TriggeringCreatureAttacksMostLifePlayer,
    ] {
        for grouping in [
            PlayerAttackGrouping::Attacker, PlayerAttackGrouping::Defender,
            PlayerAttackGrouping::Pair, PlayerAttackGrouping::AttackerAnyTarget,
        ] {
            for event_time in [false, true] {
                let condition = Condition::CombatParticipant(participant);
                let declaration = CompilerTrigger::player_attack_declaration(
                    PlayerFilter::Opponent, PlayerFilter::Any, grouping,
                );
                let trigger = if event_time {
                    CompilerTrigger::condition_qualified(
                        declaration, condition.clone(), "while the combat condition holds",
                    )
                } else {
                    declaration
                };
                let intervening_if = (!event_time).then_some(condition);
                let mut ability = Ability::triggered(
                    Trigger::from_model(trigger.clone()).expect("typed trigger materializes"),
                    vec![Effect::gain_life(2)],
                );
                let AbilityKind::Triggered(body) = &mut ability.kind else { unreachable!() };
                body.intervening_if = intervening_if.clone();
                let (projected, encoded) = project(unit(
                    ManaPaymentPredicate::Any,
                    ManaPaymentPredicate::Any,
                    vec![granted_effect(ability)],
                ));
                let ironsmith_core::AbilityKind::Triggered(body) = granted_ability(&projected).kind else {
                    panic!("trigger grant must remain a triggered ability");
                };
                assert_eq!(body.trigger, trigger);
                assert_eq!(body.intervening_if, intervening_if);
                let effects = body.effects.all_effects();
                assert_eq!(effects.len(), 1);
                assert_eq!(effects[0].kind(), "GainLifeEffect");
                distinguish(&mut seen, encoded);
            }
        }
    }
}

#[test]
fn granted_combat_damage_models_keep_controller_and_recipient_groupings_distinct() {
    let mut seen = Vec::new();
    for (one_or_more, each_damaged_player, per_source_controller) in [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (true, false, true),
        (true, true, true),
    ] {
        let mut trigger = CompilerTrigger::deals_combat_damage_to_player(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent), PlayerFilter::You,
        );
        let TriggerKind::DealsCombatDamageToPlayer {
            one_or_more: actual_one_or_more,
            each_damaged_player: actual_each_player,
            per_source_controller: actual_per_controller,
            ..
        } = &mut trigger.kind else { unreachable!() };
        // Keep identical labels and filters: only the typed grouping flags
        // may distinguish these otherwise identical public programs.
        *actual_one_or_more = one_or_more;
        *actual_each_player = each_damaged_player;
        *actual_per_controller = per_source_controller;
        let ability = Ability::triggered(
            Trigger::from_model(trigger.clone()).unwrap(), vec![Effect::gain_life(2)],
        );
        let (projected, encoded) = project(unit(
            ManaPaymentPredicate::Any,
            ManaPaymentPredicate::Any,
            vec![granted_effect(ability)],
        ));
        let ironsmith_core::AbilityKind::Triggered(body) = granted_ability(&projected).kind else {
            panic!("combat-damage grant must retain its triggered model");
        };
        assert_eq!(body.trigger, trigger);
        assert!(body.intervening_if.is_none());
        distinguish(&mut seen, encoded);
    }
}

#[test]
fn delayed_on_spend_models_retain_controller_grouping_and_its_tagged_player_body() {
    use ironsmith_core::{DelayedTriggerSpec, DrawCardsEffect, ScheduleDelayedTriggerEffect};
    use ironsmith_core::tag::DAMAGE_SOURCE_CONTROLLER_TAG;
    use ironsmith_runtime_catalog::artifact_materializer::materialize_effect;

    let mut seen = Vec::new();
    for each_damaged_player in [false, true] {
        for per_source_controller in [false, true] {
            let trigger = DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                source: ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
                player: PlayerFilter::You,
                each_damaged_player,
                per_source_controller,
            };
            let draw = DrawCardsEffect::new(
                1, PlayerFilter::TaggedPlayer(DAMAGE_SOURCE_CONTROLLER_TAG.into()),
            );
            let model = ScheduleDelayedTriggerEffect::new(
                trigger.clone(),
                vec![WireEffect::new("DrawCardsEffect", serde_json::to_value(&draw).unwrap())],
                false,
                vec![],
                PlayerFilter::You,
            );
            let wire = WireEffect::new(
                "ScheduleDelayedTriggerEffect", serde_json::to_value(&model).unwrap(),
            );
            // Enter through the real materializer so the runtime effect owns
            // a retained canonical model before the private public projection.
            let effect = materialize_effect(wire.clone()).expect("typed delayed program materializes");
            assert!(effect.serialized_model().is_some());
            let (projected, encoded) = project(unit(
                ManaPaymentPredicate::Any,
                ManaPaymentPredicate::Any,
                vec![effect],
            ));
            let (_, payload) = transaction(&projected);
            let effects = payload.effects.all_effects();
            assert_eq!(effects.len(), 1);
            assert_eq!(effects[0], &wire);
            let restored: ScheduleDelayedTriggerEffect<WireEffect> =
                serde_json::from_value(effects[0].payload().clone()).unwrap();
            assert_eq!(restored, model);
            assert_eq!(restored.trigger, trigger);
            assert_eq!(restored.effects.len(), 1);
            assert_eq!(restored.effects[0].kind(), "DrawCardsEffect");
            let restored_draw: DrawCardsEffect =
                serde_json::from_value(restored.effects[0].payload().clone()).unwrap();
            assert_eq!(restored_draw, draw);
            assert!(encoded.to_string().contains(DAMAGE_SOURCE_CONTROLLER_TAG));
            distinguish(&mut seen, encoded);
        }
    }
}

#[derive(Debug, Clone)]
struct UnencodedProgram;

impl ironsmith::effects::EffectExecutor for UnencodedProgram {
    fn execute(
        &self,
        _game: &mut ironsmith::GameState,
        _ctx: &mut ironsmith::effects::EffectContext,
    ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        panic!("audit encoding must never execute an opaque payload");
    }
}

#[test]
fn new_payment_predicates_do_not_hide_unencodable_direct_or_nested_bodies() {
    let trigger = CompilerTrigger::player_attack_declaration(
        PlayerFilter::Opponent, PlayerFilter::Any, PlayerAttackGrouping::AttackerAnyTarget,
    );
    let mut triggered = Ability::triggered(
        Trigger::from_model(trigger).unwrap(), vec![Effect::new(UnencodedProgram)],
    );
    let AbilityKind::Triggered(body) = &mut triggered.kind else { unreachable!() };
    body.intervening_if = Some(Condition::CombatParticipant(
        CombatParticipantCondition::YouAreDefendingPlayer,
    ));
    // A native generic restriction without a retained canonical model must
    // continue to fail instead of being reduced to its display label.
    let native_restriction = StaticAbility::restriction(
        ironsmith_core::Restriction::PlayerHexproofFrom(
            PlayerFilter::You, ObjectFilter::creature(),
        ),
        "targeting restriction".into(),
    );
    assert!(native_restriction.canonical_model().is_none());
    for effect in [
        Effect::new(UnencodedProgram),
        granted_effect(triggered),
        granted_effect(native_restriction.into()),
    ] {
        let source = unit(
            ManaPaymentPredicate::ActivatedAbilityKeyword(Keyword::Cycling),
            ManaPaymentPredicate::ActivatedAbilityKeyword(Keyword::Exhaust),
            vec![effect],
        );
        assert!(sync_restricted_mana(&[source.clone()]).is_err());
        assert!(sync_restricted_mana(&[source]).is_err());
    }
}
