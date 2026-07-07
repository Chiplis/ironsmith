use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::Color;
use crate::continuous::{ContinuousEffect, EffectTarget, Modification, PtSublayer};
use crate::effect::Value;
use crate::filter::ObjectFilter;
use crate::game_state::WorkCounterSnapshot;
use crate::ids::reset_runtime_id_counters;
use crate::static_abilities::{StaticAbility, StaticAbilityId};
use crate::{
    CardId, CardType, ColorSet, GameState, ObjectId, PlayerId, PowerToughness, Subtype, Zone,
};
use std::time::Instant;

pub struct ScaleScenario {
    pub game: GameState,
    pub battlefield: Vec<ObjectId>,
    pub priority_player: PlayerId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectMix {
    None,
    Anthems(usize),
    Lords(usize),
    AnthemsPlusOpalescence(usize),
    TriggerStorm(usize),
    ComplexLayerCake(usize),
}

#[derive(Debug, Clone)]
pub struct ScaleStressReport {
    pub creatures: usize,
    pub effect_sources: usize,
    pub battlefield_objects: usize,
    pub setup_ms: u128,
    pub cold_characteristics_ms: u128,
    pub warm_characteristics_ms: u128,
    pub cold_work_counters: WorkCounterSnapshot,
    pub warm_work_counters: WorkCounterSnapshot,
}

pub fn battlefield_scale(n_tokens: usize, effects: EffectMix) -> ScaleScenario {
    reset_runtime_id_counters();
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let priority_player = PlayerId::from_index(0);

    let token_defs = [
        synthetic_creature("Scale Goblin", Subtype::Goblin),
        synthetic_creature("Scale Elf", Subtype::Elf),
        synthetic_creature("Scale Soldier", Subtype::Soldier),
        synthetic_creature("Scale Zombie", Subtype::Zombie),
    ];
    let mut battlefield = Vec::with_capacity(n_tokens);
    for i in 0..n_tokens {
        let def = &token_defs[i % token_defs.len()];
        battlefield.push(game.create_object_from_definition(
            def,
            priority_player,
            Zone::Battlefield,
        ));
    }

    for index in 0..effect_count(effects) {
        let def = effect_definition(effects, index);
        let source = game.create_object_from_definition(&def, priority_player, Zone::Battlefield);
        battlefield.push(source);
        if matches!(effects, EffectMix::ComplexLayerCake(_)) {
            add_complex_layer_cake_effect(&mut game, source, index, priority_player);
        }
    }

    game.refresh_continuous_state();
    ScaleScenario {
        game,
        battlefield,
        priority_player,
    }
}

fn effect_count(effects: EffectMix) -> usize {
    match effects {
        EffectMix::None => 0,
        EffectMix::Anthems(count)
        | EffectMix::Lords(count)
        | EffectMix::AnthemsPlusOpalescence(count)
        | EffectMix::TriggerStorm(count)
        | EffectMix::ComplexLayerCake(count) => count,
    }
}

pub fn complex_layer_cake_stress_report(
    n_creatures: usize,
    effect_sources: usize,
) -> ScaleStressReport {
    let setup_started = Instant::now();
    let scenario = battlefield_scale(n_creatures, EffectMix::ComplexLayerCake(effect_sources));
    let setup_ms = setup_started.elapsed().as_millis();

    let cold_started = Instant::now();
    for id in &scenario.battlefield {
        let _ = scenario.game.calculated_characteristics(*id);
    }
    let cold_characteristics_ms = cold_started.elapsed().as_millis();
    let cold_work_counters = scenario.game.work_counters();

    let warm_started = Instant::now();
    for id in &scenario.battlefield {
        let _ = scenario.game.calculated_characteristics(*id);
    }
    let warm_characteristics_ms = warm_started.elapsed().as_millis();
    let warm_work_counters = scenario.game.work_counters();

    ScaleStressReport {
        creatures: n_creatures,
        effect_sources,
        battlefield_objects: scenario.battlefield.len(),
        setup_ms,
        cold_characteristics_ms,
        warm_characteristics_ms,
        cold_work_counters,
        warm_work_counters,
    }
}

fn synthetic_creature(name: &str, subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .color_indicator(ColorSet::GREEN)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn effect_definition(effects: EffectMix, index: usize) -> CardDefinition {
    let (name, text) = match effects {
        EffectMix::None => ("Scale Blank", ""),
        EffectMix::Anthems(_) => ("Scale Anthem", "Creatures you control get +1/+1."),
        EffectMix::Lords(_) => ("Scale Lord", "Goblins you control get +1/+1."),
        EffectMix::AnthemsPlusOpalescence(_) if index == 0 => (
            "Scale Opalescence",
            "Each other non-Aura enchantment is a creature in addition to its other types and has base power and base toughness each equal to its mana value.",
        ),
        EffectMix::AnthemsPlusOpalescence(_) => {
            ("Scale Anthem", "Creatures you control get +1/+1.")
        }
        EffectMix::TriggerStorm(_) => (
            "Scale Watcher",
            "Whenever another creature enters the battlefield under your control, you gain 1 life.",
        ),
        EffectMix::ComplexLayerCake(_) => ("Scale Layer Source", ""),
    };

    let builder = CardDefinitionBuilder::new(CardId::new(), format!("{name} {index}"))
        .card_types(vec![CardType::Enchantment]);
    if text.is_empty() {
        return builder.build();
    }
    builder
        .clone()
        .parse_text(text)
        .unwrap_or_else(|_| builder.build())
}

fn add_complex_layer_cake_effect(
    game: &mut GameState,
    source: ObjectId,
    index: usize,
    controller: PlayerId,
) {
    let (target, modification) = match index % 12 {
        0 => (
            EffectTarget::Filter(ObjectFilter::creature().you_control()),
            Modification::AddCardTypes(vec![CardType::Artifact]),
        ),
        1 => (
            EffectTarget::Filter(
                ObjectFilter::creature()
                    .you_control()
                    .with_subtype(Subtype::Goblin),
            ),
            Modification::AddAbility(StaticAbility::flying()),
        ),
        2 => (
            EffectTarget::Filter(
                ObjectFilter::creature()
                    .you_control()
                    .with_subtype(Subtype::Elf),
            ),
            Modification::AddAbility(StaticAbility::vigilance()),
        ),
        3 => (
            EffectTarget::Filter(
                ObjectFilter::creature()
                    .you_control()
                    .with_subtype(Subtype::Soldier),
            ),
            Modification::AddAbility(StaticAbility::haste()),
        ),
        4 => (
            EffectTarget::Filter(with_static_ability(
                ObjectFilter::creature().you_control(),
                StaticAbilityId::Flying,
            )),
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
        ),
        5 => (
            EffectTarget::Filter(
                ObjectFilter::creature()
                    .you_control()
                    .with_colors(ColorSet::WHITE),
            ),
            Modification::ModifyPowerToughness {
                power: 0,
                toughness: 1,
            },
        ),
        6 => (
            EffectTarget::Filter(
                ObjectFilter::creature()
                    .you_control()
                    .with_subtype(Subtype::Zombie),
            ),
            Modification::AddColors(ColorSet::from_color(Color::Black)),
        ),
        7 => (
            EffectTarget::Filter(
                ObjectFilter::creature()
                    .you_control()
                    .with_all_type(CardType::Artifact),
            ),
            Modification::SetPowerToughness {
                power: Value::Fixed(3),
                toughness: Value::Fixed(3),
                sublayer: PtSublayer::Setting,
            },
        ),
        8 => (
            EffectTarget::Filter(with_static_ability(
                ObjectFilter::creature().you_control(),
                StaticAbilityId::Vigilance,
            )),
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 0,
            },
        ),
        9 => (
            EffectTarget::Filter(ObjectFilter::creature().you_control()),
            Modification::ModifyPowerToughnessByColorCount {
                power_multiplier: 1,
                toughness_multiplier: 0,
            },
        ),
        10 => (
            EffectTarget::Filter(
                ObjectFilter::creature()
                    .you_control()
                    .with_subtype(Subtype::Goblin),
            ),
            Modification::AddColors(ColorSet::WHITE.union(ColorSet::RED)),
        ),
        _ => (
            EffectTarget::Filter(with_static_ability(
                ObjectFilter::creature().you_control(),
                StaticAbilityId::Haste,
            )),
            Modification::ModifyPowerToughness {
                power: 0,
                toughness: 2,
            },
        ),
    };

    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            source,
            controller,
            target,
            modification,
        ));
}

fn with_static_ability(mut filter: ObjectFilter, ability: StaticAbilityId) -> ObjectFilter {
    filter.static_abilities.push(ability);
    filter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "manual scale stress probe"]
    fn complex_layer_cake_300_by_12_report() {
        let report = complex_layer_cake_stress_report(300, 12);
        eprintln!("{report:#?}");
        assert_eq!(report.creatures, 300);
        assert_eq!(report.effect_sources, 12);
        assert_eq!(report.battlefield_objects, 312);
    }

    #[test]
    #[ignore = "manual scale stress probe"]
    fn complex_layer_cake_300_by_36_report() {
        let report = complex_layer_cake_stress_report(300, 36);
        eprintln!("{report:#?}");
        assert_eq!(report.creatures, 300);
        assert_eq!(report.effect_sources, 36);
        assert_eq!(report.battlefield_objects, 336);
    }

    #[test]
    #[ignore = "manual scale stress probe"]
    fn complex_layer_cake_500_by_72_report() {
        let report = complex_layer_cake_stress_report(500, 72);
        eprintln!("{report:#?}");
        assert_eq!(report.creatures, 500);
        assert_eq!(report.effect_sources, 72);
        assert_eq!(report.battlefield_objects, 572);
    }

    #[test]
    #[ignore = "manual scale stress probe"]
    fn complex_layer_cake_1000_by_72_expected_characteristics() {
        let scenario = battlefield_scale(1000, EffectMix::ComplexLayerCake(72));
        let goblin = scenario.battlefield[0];
        let elf = scenario.battlefield[1];
        let soldier = scenario.battlefield[2];
        let zombie = scenario.battlefield[3];

        assert_token_characteristics(
            &scenario.game,
            goblin,
            Subtype::Goblin,
            27,
            15,
            ColorSet::GREEN.union(ColorSet::WHITE).union(ColorSet::RED),
            &[StaticAbilityId::Flying],
        );
        assert_token_characteristics(
            &scenario.game,
            elf,
            Subtype::Elf,
            15,
            3,
            ColorSet::GREEN,
            &[StaticAbilityId::Vigilance],
        );
        assert_token_characteristics(
            &scenario.game,
            soldier,
            Subtype::Soldier,
            9,
            15,
            ColorSet::GREEN,
            &[StaticAbilityId::Haste],
        );
        assert_token_characteristics(
            &scenario.game,
            zombie,
            Subtype::Zombie,
            15,
            3,
            ColorSet::GREEN.union(ColorSet::from_color(Color::Black)),
            &[],
        );
    }

    fn assert_token_characteristics(
        game: &GameState,
        id: ObjectId,
        subtype: Subtype,
        power: i32,
        toughness: i32,
        colors: ColorSet,
        static_ability_ids: &[StaticAbilityId],
    ) {
        let chars = game
            .calculated_characteristics(id)
            .expect("token should have calculated characteristics");
        assert_eq!(chars.power, Some(power));
        assert_eq!(chars.toughness, Some(toughness));
        assert!(chars.card_types.contains(&CardType::Creature));
        assert!(chars.card_types.contains(&CardType::Artifact));
        assert!(chars.subtypes.contains(&subtype));
        assert_eq!(chars.colors, colors);
        for ability_id in static_ability_ids {
            assert!(
                chars
                    .static_abilities
                    .iter()
                    .any(|ability| ability.id() == *ability_id),
                "expected token #{:?} to have static ability {:?}, got {:?}",
                id,
                ability_id,
                chars.static_abilities
            );
        }
    }
}
