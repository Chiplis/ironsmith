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

/// Real-card reproduction of the "1000 creatures + 72 anthem sources" puzzle
/// board (250x four creatures + 6x twelve battlefield-wide effect sources,
/// including six legendary Akroma's Memorials so the legend rule fires on the
/// first SBA pass). Times each stage of the start-game path independently.
#[derive(Debug, Clone)]
pub struct PuzzlePremainReport {
    pub battlefield_objects: usize,
    pub setup_ms: u128,
    pub refresh_ms: u128,
    pub cold_characteristics_ms: u128,
    pub sba_ms: u128,
    pub legal_actions_ms: u128,
    pub refresh_work_counters: WorkCounterSnapshot,
    pub cold_work_counters: WorkCounterSnapshot,
    pub sba_work_counters: WorkCounterSnapshot,
    pub legal_actions_work_counters: WorkCounterSnapshot,
}

const PUZZLE_CREATURES: [(&str, &str, &str); 4] = [
    (
        "Goblin Guide",
        "Creature - Goblin Scout",
        "Haste\nWhenever this creature attacks, defending player reveals the top card of their library. If it's a land card, that player puts it into their hand.",
    ),
    ("Llanowar Elves", "Creature - Elf Druid", "{T}: Add {G}."),
    ("Elite Vanguard", "Creature - Human Soldier", ""),
    ("Walking Corpse", "Creature - Zombie", ""),
];

const PUZZLE_EFFECT_SOURCES: [(&str, &str, &str); 12] = [
    (
        "Mycosynth Lattice",
        "Artifact",
        "All permanents are artifacts in addition to their other types.\nAll cards that aren't on the battlefield, spells, and permanents are colorless.\nPlayers may spend mana as though it were mana of any color.",
    ),
    (
        "Akroma's Memorial",
        "Legendary Artifact",
        "Creatures you control have flying, first strike, vigilance, trample, haste, and protection from black and from red.",
    ),
    (
        "Always Watching",
        "Enchantment",
        "Nontoken creatures you control get +1/+1 and have vigilance.",
    ),
    ("Fervor", "Enchantment", "Creatures you control have haste."),
    (
        "Glorious Anthem",
        "Enchantment",
        "Creatures you control get +1/+1.",
    ),
    (
        "Honor of the Pure",
        "Enchantment",
        "White creatures you control get +1/+1.",
    ),
    ("Bad Moon", "Enchantment", "Black creatures get +1/+1."),
    (
        "Favorable Winds",
        "Enchantment",
        "Creatures you control with flying get +1/+1.",
    ),
    (
        "Gaea's Anthem",
        "Enchantment",
        "Creatures you control get +1/+1.",
    ),
    (
        "Intangible Virtue",
        "Enchantment",
        "Creature tokens you control get +1/+1 and have vigilance.",
    ),
    (
        "Spidersilk Armor",
        "Enchantment",
        "Creatures you control get +0/+1 and have reach.",
    ),
    (
        "Dictate of Heliod",
        "Enchantment",
        "Flash\nCreatures you control get +2/+2.",
    ),
];

fn puzzle_definition(name: &str, type_line: &str, text: &str) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name);
    builder = match type_line {
        "Artifact" => builder.card_types(vec![CardType::Artifact]),
        "Legendary Artifact" => builder
            .card_types(vec![CardType::Artifact])
            .supertypes(vec![crate::types::Supertype::Legendary]),
        "Enchantment" => builder.card_types(vec![CardType::Enchantment]),
        _ => builder.card_types(vec![CardType::Creature]),
    };
    if type_line.starts_with("Creature") {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
        let subtype = if name == "Goblin Guide" {
            Subtype::Goblin
        } else if name == "Llanowar Elves" {
            Subtype::Elf
        } else if name == "Elite Vanguard" {
            Subtype::Soldier
        } else {
            Subtype::Zombie
        };
        builder = builder.subtypes(vec![subtype]);
        builder = builder.color_indicator(match subtype {
            Subtype::Goblin => ColorSet::RED,
            Subtype::Elf => ColorSet::GREEN,
            Subtype::Soldier => ColorSet::WHITE,
            _ => ColorSet::BLACK,
        });
    }
    if text.is_empty() {
        return builder.build();
    }
    builder
        .clone()
        .parse_text(text)
        .unwrap_or_else(|err| panic!("failed to parse {name}: {err:?}"))
}

pub fn puzzle_premain_stress_report(copies_per_creature: usize) -> PuzzlePremainReport {
    reset_runtime_id_counters();
    let setup_started = Instant::now();
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let mut battlefield = Vec::new();
    for (name, type_line, text) in PUZZLE_CREATURES {
        let def = puzzle_definition(name, type_line, text);
        for _ in 0..copies_per_creature {
            battlefield.push(game.create_object_from_definition(&def, alice, Zone::Battlefield));
        }
    }
    for (name, type_line, text) in PUZZLE_EFFECT_SOURCES {
        let def = puzzle_definition(name, type_line, text);
        for _ in 0..6 {
            battlefield.push(game.create_object_from_definition(&def, alice, Zone::Battlefield));
        }
    }
    let setup_ms = setup_started.elapsed().as_millis();

    let refresh_started = Instant::now();
    game.refresh_continuous_state();
    let refresh_ms = refresh_started.elapsed().as_millis();
    let refresh_work_counters = game.work_counters();

    let cold_started = Instant::now();
    for id in &battlefield {
        let _ = game.calculated_characteristics(*id);
    }
    let cold_characteristics_ms = cold_started.elapsed().as_millis();
    let cold_work_counters = game.work_counters();

    let sba_started = Instant::now();
    let mut trigger_queue = crate::triggers::TriggerQueue::default();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("sba pass should succeed");
    let sba_ms = sba_started.elapsed().as_millis();
    let sba_work_counters = game.work_counters();

    let legal_started = Instant::now();
    let _ = crate::decision::compute_legal_actions(&game, alice);
    let legal_actions_ms = legal_started.elapsed().as_millis();
    let legal_actions_work_counters = game.work_counters();

    PuzzlePremainReport {
        battlefield_objects: battlefield.len(),
        setup_ms,
        refresh_ms,
        cold_characteristics_ms,
        sba_ms,
        legal_actions_ms,
        refresh_work_counters,
        cold_work_counters,
        sba_work_counters,
        legal_actions_work_counters,
    }
}

/// Diagnostic probe: attach a parsed "doesn't untap unless monarch" aura and
/// report where the granted restriction disappears.
pub fn aura_grant_probe() {
    reset_runtime_id_counters();
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let creature_def = synthetic_creature("Probe Bear", Subtype::Elf);
    let creature = game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);

    let aura_def = CardDefinitionBuilder::new(CardId::new(), "Probe Fall from Favor")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\nWhen this Aura enters, tap enchanted creature and you become the monarch.\nEnchanted creature doesn't untap during its controller's untap step unless that player is the monarch.",
        )
        .expect("probe aura should parse");
    let aura = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);

    if let Some(obj) = game.object_mut(aura) {
        obj.attached_to = Some(crate::object::AttachmentTarget::Object(creature));
    }
    if let Some(obj) = game.object_mut(creature) {
        obj.attachments.push(aura);
    }

    game.refresh_continuous_state();

    let aura_obj = game.object(aura).expect("aura exists");
    eprintln!("aura abilities ({}):", aura_obj.abilities.len());
    for ability in aura_obj.abilities.iter() {
        eprintln!("  kind={:?}", std::mem::discriminant(&ability.kind));
        if let crate::ability::AbilityKind::Static(s) = &ability.kind {
            eprintln!("    static id={:?}", s.id());
        }
    }

    let effects = game.all_continuous_effects();
    eprintln!("continuous effects ({}):", effects.len());
    for effect in effects.iter() {
        eprintln!(
            "  source={:?} target={:?} modification_layer={:?} condition={} duration={:?}",
            effect.source,
            effect.applies_to,
            effect.modification.layer(),
            effect.condition.is_some(),
            effect.duration,
        );
    }

    let chars = game
        .calculated_characteristics(creature)
        .expect("creature chars");
    eprintln!(
        "creature static abilities: {:?}",
        chars
            .static_abilities
            .iter()
            .map(|ability| ability.id())
            .collect::<Vec<_>>()
    );

    // Cursed Role shape: base P/T setting through an attached aura.
    let bear2 = game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);
    let role_def = CardDefinitionBuilder::new(CardId::new(), "Probe Cursed Role")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .parse_text("Enchanted creature loses all abilities and has base power and toughness 1/1.")
        .expect("probe role should parse");
    let role = game.create_object_from_definition(&role_def, alice, Zone::Battlefield);
    if let Some(obj) = game.object_mut(role) {
        obj.attached_to = Some(crate::object::AttachmentTarget::Object(bear2));
    }
    if let Some(obj) = game.object_mut(bear2) {
        obj.attachments.push(role);
    }
    game.refresh_continuous_state();
    let chars2 = game.calculated_characteristics(bear2).expect("bear2 chars");
    eprintln!(
        "role-enchanted creature P/T: {:?}/{:?} (expected 1/1)",
        chars2.power, chars2.toughness
    );
    let effects = game.all_continuous_effects();
    eprintln!("effects after role attach ({}):", effects.len());
    for effect in effects.iter() {
        eprintln!(
            "  source={:?} target={:?} layer={:?}",
            effect.source,
            effect.applies_to,
            effect.modification.layer(),
        );
    }

    // Fall from Favor shape: opponent-controlled creature, monarch set,
    // untap step driven directly (mirrors the failing card test).
    let bob = PlayerId::from_index(1);
    let bob_creature = game.create_object_from_definition(&creature_def, bob, Zone::Battlefield);
    let ffav = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
    assert!(
        game.attach_object_to_target(ffav, crate::object::AttachmentTarget::Object(bob_creature))
    );
    game.set_monarch(Some(alice));
    game.tap(bob_creature);
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Untap);

    let chars3 = game
        .calculated_characteristics(bob_creature)
        .expect("bob creature chars");
    eprintln!(
        "pre-untap: bob creature statics={:?} tapped={}",
        chars3
            .static_abilities
            .iter()
            .map(|ability| ability.id())
            .collect::<Vec<_>>(),
        game.is_tapped(bob_creature)
    );
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::turn::execute_untap_step_with(&mut game, &mut dm);
    eprintln!(
        "post-untap: tapped={} (expected true)",
        game.is_tapped(bob_creature)
    );
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
    #[ignore = "manual diagnostic probe"]
    fn aura_grant_probe_report() {
        aura_grant_probe();
    }

    #[test]
    #[ignore = "manual scale stress probe"]
    fn puzzle_premain_1000_by_72_report() {
        let report = puzzle_premain_stress_report(250);
        eprintln!("{report:#?}");
        assert_eq!(report.battlefield_objects, 1072);
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
