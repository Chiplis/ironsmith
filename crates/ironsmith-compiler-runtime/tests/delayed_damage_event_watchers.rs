//! cf8 p10: delayed "deals damage" / "is dealt damage by" / "attacks alone" /
//! "this deals combat damage to a player" registrations.
//! Source-authored, deliberately UNRUN (implementation-first campaign).
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::effects::ScheduleDelayedTriggerEffect;
use ironsmith::triggers::{
    AttacksAloneTrigger, DealsCombatDamageToPlayerTrigger, DealsDamageToTrigger, DealsDamageTrigger,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const SPIRITUALIZE: &str = "Mana cost: {2}{W}\nType: Instant\nUntil end of turn, whenever target creature deals damage, you gain that much life.\nDraw a card.";
const PALADIN_OF_PRAHV: &str = "Mana cost: {4}{W}{W}\nType: Creature — Human Knight\nPower/Toughness: 3/4\nWhenever this creature deals damage, you gain that much life.\nForecast — {1}{W}, Reveal this card from your hand: Whenever target creature deals damage this turn, you gain that much life. (Activate only during your upkeep and only once each turn.)";
const GLYPH_OF_LIFE: &str = "Mana cost: {W}\nType: Instant\nChoose target Wall creature. Whenever that creature is dealt damage by an attacking creature this turn, you gain that much life.";
const LYRA: &str = "Mana cost: {1}{U}{U}\nType: Legendary Creature — Angel Wizard\nPower/Toughness: 3/3\nFlying\nAt the beginning of each end step, if you've drawn three or more cards this turn, create a 3/3 blue Angel creature token with flying.\n{3}{U}{U}: Until end of turn, whenever Lyra deals combat damage to a player, draw two cards.";
const LAST_RONIN: &str = "Mana cost: {4}{B}{G}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — Destroy all creatures.\nII — Mill four cards. When you do, return target creature card from your graveyard to your hand.\nIII — Whenever a creature you control attacks alone this turn, put three +1/+1 counters on it. It gains trample, lifelink, and indestructible until end of turn.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    let (artifact, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

fn all_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    if let Some(spell) = &definition.spell_effect {
        for effect in spell.all_effects() {
            collect(effect, &mut all);
        }
    }
    for ability in &definition.abilities {
        let program = match &ability.kind {
            AbilityKind::Activated(ability) => &ability.effects,
            AbilityKind::Triggered(ability) => &ability.effects,
            _ => continue,
        };
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    all
}

fn schedules(definition: &CardDefinition) -> Vec<ScheduleDelayedTriggerEffect> {
    all_effects(definition)
        .iter()
        .filter_map(|effect| effect.downcast_ref::<ScheduleDelayedTriggerEffect>().cloned())
        .collect()
}

#[test]
fn spiritualize_watches_its_declared_target_for_any_damage() {
    for definition in definitions("Spiritualize", SPIRITUALIZE) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let [schedule] = schedules(&definition).try_into().expect("one registration");
        let matcher = schedule
            .trigger
            .downcast_ref::<DealsDamageTrigger>()
            .expect("noncombat-inclusive damage matcher");
        assert!(!matcher.combat_only && !matcher.noncombat_only);
        assert!(matcher.filter.source, "the watched target is the matcher's source");
        assert!(schedule.target_tag.is_some(), "the target creature is watched, not any creature");
        assert!(schedule.until_end_of_turn);
    }
}

#[test]
fn paladin_forecast_watches_its_declared_target_this_turn() {
    for definition in definitions("Paladin of Prahv", PALADIN_OF_PRAHV) {
        let [schedule] = schedules(&definition).try_into().expect("forecast registration");
        let matcher = schedule.trigger.downcast_ref::<DealsDamageTrigger>().unwrap();
        assert!(!matcher.combat_only);
        assert!(matcher.filter.source);
        assert!(schedule.target_tag.is_some());
        assert!(schedule.until_end_of_turn);
    }
}

#[test]
fn glyph_of_life_watches_the_wall_as_damage_recipient() {
    for definition in definitions("Glyph of Life", GLYPH_OF_LIFE) {
        let [schedule] = schedules(&definition).try_into().expect("one registration");
        let matcher = schedule.trigger.downcast_ref::<DealsDamageToTrigger>().unwrap();
        assert!(!matcher.combat_only);
        assert!(matcher.target_filter.source, "the watched Wall is the recipient");
        assert!(matcher.source_filter.attacking, "only attacking creatures' damage counts");
        assert!(schedule.target_tag.is_some());
    }
}

#[test]
fn lyra_registers_its_own_combat_damage_to_a_player() {
    for definition in definitions("Lyra, Tolarian Archangel", LYRA) {
        let [schedule] = schedules(&definition).try_into().expect("one registration");
        let matcher = schedule
            .trigger
            .downcast_ref::<DealsCombatDamageToPlayerTrigger>()
            .unwrap();
        assert!(matcher.filter.source, "Lyra itself, the ability source");
        assert!(schedule.until_end_of_turn);
    }
}

#[test]
fn last_ronin_chapter_three_registers_attacks_alone() {
    for definition in definitions("The Last Ronin", LAST_RONIN) {
        let [schedule] = schedules(&definition).try_into().expect("chapter III registration");
        let matcher = schedule.trigger.downcast_ref::<AttacksAloneTrigger>().unwrap();
        assert_eq!(matcher.filter.controller, Some(ironsmith::target::PlayerFilter::You));
        assert!(schedule.until_end_of_turn);
    }
}

#[test]
fn new_delayed_specs_round_trip_and_interpret() {
    use ironsmith_core::DelayedTriggerSpec;
    let creature = ironsmith::target::ObjectFilter::creature();
    for spec in [
        DelayedTriggerSpec::DealsDamage { source: ironsmith::target::ObjectFilter::source() },
        DelayedTriggerSpec::DealsDamageTo {
            source: creature.clone(),
            target: ironsmith::target::ObjectFilter::source(),
        },
        DelayedTriggerSpec::AttacksAlone(creature.clone()),
    ] {
        let json = serde_json::to_string(&spec).unwrap();
        let back: DelayedTriggerSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back, spec);
        let _ = ironsmith::triggers::Trigger::from_delayed_trigger_spec(spec);
    }
}
