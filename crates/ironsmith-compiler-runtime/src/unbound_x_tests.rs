//! Regressions for the original candidates and four follow-up X audit defects.
use super::*;
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Value;
use ironsmith::effects::{EffectContext as ExecutionContext, helpers::resolve_value};
use ironsmith::filter::Comparison;
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, PlayerId, Zone};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/unbound_x_regressions.json.fixture"
    ))
    .unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let text = row["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    [direct, restored]
}

fn choices(def: &CardDefinition) -> Vec<ChooseSpec> {
    fn collect(effect: &ironsmith::effect::Effect, out: &mut Vec<ChooseSpec>) {
        if let Some(spec) = effect.0.get_target_spec() {
            out.push(spec.clone());
        }
        effect.visit_child_effects(&mut |child| collect(child, out));
    }
    let mut out = Vec::new();
    for ability in &def.abilities {
        if let AbilityKind::Triggered(trigger) = &ability.kind {
            out.extend(trigger.choices.iter().cloned());
            for effect in trigger.effects.all_effects() {
                collect(effect, &mut out);
            }
        }
    }
    out
}

fn mana_limit(spec: &ChooseSpec) -> Option<&Value> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => mana_limit(spec),
        ChooseSpec::Object(filter) => match filter.mana_value.as_ref() {
            Some(Comparison::LessThanOrEqualExpr(value)) => Some(value),
            _ => None,
        },
        _ => None,
    }
}

#[test]
fn unbound_x_spellstutter_counts_only_your_faeries_and_rechecks_the_count() {
    for definition in definitions("Spellstutter Sprite") {
        let specs = choices(&definition);
        let value = specs.iter().find_map(mana_limit).unwrap();
        assert!(matches!(value.unhinted(), Value::Count(_)), "{value:?}");
        assert!(
            specs
                .iter()
                .filter_map(mana_limit)
                .all(|other| other == value)
        );
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ctx = ExecutionContext::new_default(source, A);
        assert_eq!(resolve_value(&game, value, &ctx).unwrap(), 1);
        game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.create_object_from_definition(&definition, A, Zone::Hand);
        assert_eq!(resolve_value(&game, value, &ctx).unwrap(), 1);
        let second = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(resolve_value(&game, value, &ctx).unwrap(), 2);
        game.move_object_by_game_rule(second, Zone::Graveyard)
            .unwrap();
        assert_eq!(resolve_value(&game, value, &ctx).unwrap(), 1);
    }
}

#[test]
fn unbound_x_warsinger_uses_trigger_damage_for_its_graveyard_target() {
    for definition in definitions("Venerable Warsinger") {
        let specs = choices(&definition);
        let value = specs.iter().find_map(mana_limit).unwrap();
        assert!(
            specs
                .iter()
                .filter_map(mana_limit)
                .all(|other| other == value)
        );
        assert!(
            matches!(value.unhinted(), Value::EventValue(_)),
            "{value:?}"
        );
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for damage in [1, 3, 7] {
            let ctx = ExecutionContext::new_default(source, A).with_event_value_amount(damage);
            assert_eq!(resolve_value(&game, value, &ctx).unwrap(), damage);
        }
    }
}

#[test]
fn unbound_x_marauder_target_count_uses_devotion() {
    for definition in definitions("Mogis's Marauder") {
        let specs = choices(&definition);
        let value = specs
            .iter()
            .find_map(|spec| spec.count_value())
            .expect("typed target count");
        assert!(
            matches!(value.unhinted(), Value::Devotion { .. }),
            "{value:?}"
        );
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ctx = ExecutionContext::new_default(source, A);
        assert_eq!(resolve_value(&game, value, &ctx).unwrap(), 1);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.create_object_from_definition(&definition, B, Zone::Battlefield);
        assert_eq!(resolve_value(&game, value, &ctx).unwrap(), 2);
    }
}

#[test]
fn unbound_x_drake_and_zethi_target_counts_use_paid_multikicker() {
    for name in ["Voyager Drake", "Zethi, Arcane Blademaster"] {
        for definition in definitions(name) {
            let specs = choices(&definition);
            let value = specs
                .iter()
                .find_map(|spec| spec.count_value())
                .unwrap_or_else(|| panic!("{name}: missing typed target count: {specs:?}"));
            assert!(
                matches!(value.unhinted(), Value::KickCount),
                "{name}: {value:?}"
            );
            assert!(
                specs
                    .iter()
                    .filter_map(|spec| spec.count_value())
                    .all(|other| other == value)
            );
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for paid in [0, 1, 3] {
                let mut ctx = ExecutionContext::new_default(source, A);
                ctx.optional_costs_paid =
                    ironsmith_core::OptionalCostsPaid::from_costs(&definition.optional_costs);
                ctx.optional_costs_paid.costs[0].1 = paid;
                assert_eq!(
                    resolve_value(&game, value, &ctx).unwrap(),
                    paid as i32,
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn unbound_x_shadow_of_mortality_is_already_bound_by_cost_condition() {
    for definition in definitions("Shadow of Mortality") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        for (life, expected) in [
            (25, "{13}{B}{B}"),
            (20, "{13}{B}{B}"),
            (12, "{5}{B}{B}"),
            (1, "{B}{B}"),
        ] {
            game.player_mut(A).unwrap().life = life;
            let spell = game.object(source).unwrap();
            let cost = ironsmith::decision::calculate_effective_mana_cost(
                &game,
                A,
                spell,
                spell.mana_cost.as_ref().unwrap(),
            );
            assert_eq!(cost.to_oracle(), expected);
        }
    }
}

fn triggered(def: &CardDefinition, index: usize) -> &ironsmith::ability::TriggeredAbility {
    def.abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(ability) => Some(ability),
            _ => None,
        })
        .nth(index)
        .unwrap()
}

fn creature(game: &mut GameState, player: PlayerId, power: i32) -> ironsmith::ObjectId {
    let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Test creature")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(power, 30))
        .build();
    game.create_object_from_card(&card, player, Zone::Battlefield)
}

struct AuditChoices {
    wanted: usize,
    allowed: Vec<ironsmith::ObjectId>,
    players: Vec<PlayerId>,
    limits: Vec<(usize, Option<usize>)>,
}
impl ironsmith::decision::DecisionMaker for AuditChoices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
    fn decide_objects(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ironsmith::ObjectId> {
        self.players.push(ctx.player);
        self.limits.push((ctx.min, ctx.max));
        ctx.candidates
            .iter()
            .filter(|c| c.legal && self.allowed.contains(&c.id))
            .take(self.wanted.min(ctx.max.unwrap_or(usize::MAX)))
            .map(|c| c.id)
            .collect()
    }
}

#[test]
fn unbound_x_myr_payment_chooses_zero_one_or_several_and_uses_actual_taps() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for definition in definitions("Myr Battlesphere") {
        let ability = triggered(&definition, 1);
        assert!(
            ability.choices.is_empty(),
            "the tap payment does not target"
        );
        for wanted in [0, 1, 3] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.tap(source);
            let myr = compile_to_runtime_definition(
                "Test Myr",
                "Type: Artifact Creature — Myr\nPower/Toughness: 1/1",
                false,
            )
            .unwrap();
            let mine: Vec<_> = (0..3)
                .map(|_| game.create_object_from_definition(&myr, A, Zone::Battlefield))
                .collect();
            let enemy = game.create_object_from_definition(&myr, B, Zone::Battlefield);
            let already_tapped = game.create_object_from_definition(&myr, A, Zone::Battlefield);
            game.tap(already_tapped);
            let mut combat = CombatState::default();
            combat.attackers.push(AttackerInfo {
                creature: source,
                target: AttackTarget::Player(B),
            });
            game.combat = Some(combat);
            let mut dm = AuditChoices {
                wanted,
                allowed: mine.clone(),
                players: vec![],
                limits: vec![],
            };
            let event = ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::combat::CreatureAttackedEvent::new(
                    source, ironsmith::events::combat::AttackEventTarget::Player(B)),
                ironsmith::provenance::ProvNodeId::default(),
            );
            let mut ctx = ExecutionContext::new(source, A, &mut dm).with_triggering_event(event);
            assert_eq!(ctx.x_value, None);
            for effect in ability.effects.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            drop(ctx);
            assert_eq!(
                mine.iter().filter(|id| game.is_tapped(**id)).count(),
                wanted
            );
            assert!(!game.is_tapped(enemy));
            assert_eq!(game.player(B).unwrap().life, 20 - wanted as i32);
            assert_eq!(game.current_power(source), Some(4 + wanted as i32));
        }
    }
}

#[test]
fn unbound_x_shriekwood_counts_only_the_triggering_attack_group() {
    use ironsmith::events::combat::{AttackEventTarget, CreatureAttackedEvent};
    for definition in definitions("Shriekwood Devourer") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = creature(&mut game, A, 2);
        let largest = creature(&mut game, A, 4);
        creature(&mut game, A, 15);
        creature(&mut game, B, 20);
        let snapshots = [first, largest]
            .into_iter()
            .map(|id| {
                ironsmith::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), &game)
            })
            .collect();
        let event = ironsmith::triggers::TriggerEvent::new(
            CreatureAttackedEvent::with_total_attackers(first, AttackEventTarget::Player(B), 2),
            ironsmith::provenance::ProvNodeId::default(),
        );
        let mut ctx = ExecutionContext::new_default(source, A).with_triggering_event(event);
        ctx.set_tagged_objects(ironsmith_core::ATTACKING_GROUP_TAG, snapshots);
        for effect in triggered(&definition, 0).effects.all_effects() {
            if effect
                .downcast_ref::<ironsmith::effects::TagTriggeringObjectEffect>()
                .is_some()
            {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        let effects = triggered(&definition, 0).effects.all_effects();
        let value = effects.iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::UntapEffect>())
            .and_then(|effect| effect.target.count_value())
            .expect("bound untap count");
        assert_eq!(
            resolve_value(&game, value, &ctx).unwrap(),
            4,
            "ignore creatures outside the declaration"
        );
        game.move_object_by_game_rule(largest, Zone::Graveyard)
            .unwrap();
        assert_eq!(
            resolve_value(&game, value, &ctx).unwrap(),
            4,
            "use last-known power for a departed attacker"
        );
    }
}

#[test]
fn unbound_x_obliterator_makes_the_damage_source_controller_sacrifice() {
    use ironsmith::events::{DamageEvent, DamageTarget, EventCause};
    for definition in definitions("Phyrexian Obliterator") {
        for source_leaves in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let damager = creature(&mut game, B, 2);
            let mine = creature(&mut game, A, 1);
            let theirs: Vec<_> = (0..3).map(|_| creature(&mut game, B, 1)).collect();
            let event = ironsmith::triggers::TriggerEvent::new(
                DamageEvent::with_cause(
                    damager,
                    DamageTarget::Object(source),
                    2,
                    false,
                    EventCause::from_effect(damager, B),
                ),
                ironsmith::provenance::ProvNodeId::default(),
            );
            if source_leaves {
                game.move_object_by_game_rule(damager, Zone::Graveyard)
                    .unwrap();
            }
            let mut dm = AuditChoices {
                wanted: 2,
                allowed: theirs.clone(),
                players: vec![],
                limits: vec![],
            };
            let mut ctx = ExecutionContext::new(source, A, &mut dm)
                .with_triggering_event(event)
                .with_event_value_amount(2);
            for effect in triggered(&definition, 0).effects.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            drop(ctx);
            assert!(!dm.players.is_empty());
            assert!(
                dm.players.iter().all(|p| *p == B),
                "damage source controller chooses: {:?}",
                dm.players
            );
            assert_eq!(game.object(mine).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert_eq!(
                theirs
                    .iter()
                    .filter(|id| game
                        .object(**id)
                        .is_some_and(|o| o.zone == Zone::Battlefield))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn unbound_x_heroic_feast_announces_targets_and_counters_only_those_targets() {
    use ironsmith::effects::ResolvedTarget;
    for definition in definitions("Heroic Feast") {
        let ability = triggered(&definition, 1);
        assert_eq!(
            ability.choices.len(),
            1,
            "the trigger must announce its targets"
        );
        let spec = &ability.choices[0];
        assert!(spec.is_target());
        assert_eq!(spec.count().min, 0);
        let value = spec.count_value().expect("life-gain amount limits targets");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let selected = creature(&mut game, A, 1);
        let unselected = creature(&mut game, A, 1);
        let enemy = creature(&mut game, B, 1);
        let mut ctx = ExecutionContext::new_default(source, A)
            .with_event_value_amount(3)
            .with_targets(vec![ResolvedTarget::Object(selected)]);
        assert_eq!(resolve_value(&game, value, &ctx).unwrap(), 3);
        for effect in ability.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(
            game.counter_count(selected, ironsmith::CounterType::PlusOnePlusOne),
            1
        );
        assert_eq!(
            game.counter_count(unselected, ironsmith::CounterType::PlusOnePlusOne),
            0
        );
        assert_eq!(
            game.counter_count(enemy, ironsmith::CounterType::PlusOnePlusOne),
            0
        );
    }
}
