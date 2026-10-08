//! The additional part of a prevention effect that reads the prevented
//! damage's source (CR 615.5): Channel Harm, Comeuppance, Judgment of
//! Alexander, Samite Ministration; a kicker-dependent divided shield (Pollen
//! Remedy, CR 601.2b before 601.2d); and a source/recipient shared-color
//! shield (Well-Laid Plans). Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const CHANNEL_HARM: &str = "Mana cost: {5}{W}\nType: Instant\nPrevent all damage that would be dealt to you and permanents you control this turn by sources you don't control. If damage is prevented this way, you may have Channel Harm deal that much damage to target creature.";
const COMEUPPANCE: &str = "Mana cost: {3}{W}\nType: Instant\nPrevent all damage that would be dealt to you and planeswalkers you control this turn by sources you don't control. If damage from a creature source is prevented this way, Comeuppance deals that much damage to that creature. If damage from a noncreature source is prevented this way, Comeuppance deals that much damage to the source's controller.";
const JUDGMENT_OF_ALEXANDER: &str = "Mana cost: {2}{W}\nType: Instant\nPrevent all damage that would be dealt to you this turn by sources your opponents control. Whenever damage from a creature is prevented this way, each commander creature you control deals damage equal to its power to that creature.";
const SAMITE_MINISTRATION: &str = "Mana cost: {1}{W}\nType: Instant\nPrevent all damage that would be dealt to you this turn by a source of your choice. Whenever damage from a black or red source is prevented this way this turn, you gain that much life.";
const POLLEN_REMEDY: &str = "Mana cost: {W}\nType: Instant\nKicker—Sacrifice a land. (You may sacrifice a land in addition to any other costs as you cast this spell.)\nPrevent the next 3 damage that would be dealt this turn to any number of targets, divided as you choose. If this spell was kicked, prevent the next 6 damage this way instead.";
const WELL_LAID_PLANS: &str = "Mana cost: {2}{U}\nType: Enchantment\nPrevent all damage that would be dealt to a creature by another creature if they share a color.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

fn spell_debug(definition: &CardDefinition) -> String {
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    format!("{:?}", definition.spell_effect)
}

#[test]
fn channel_harm_declares_its_creature_with_the_spell_and_reflects_each_prevention() {
    for definition in routes("Channel Harm", CHANNEL_HARM) {
        let text = spell_debug(&definition);
        assert!(text.contains("PreventAllDamageEffect"), "{text}");
        assert!(text.contains("YouAndPermanentsMatching"), "{text}");
        assert!(text.contains("controller: Some(NotYou)"), "sources you don't control: {text}");
        assert!(text.contains("TargetOnlyEffect"), "target creature announced on cast: {text}");
        assert!(text.contains("TagTriggeringSourceEffect"), "{text}");
        assert!(text.contains("MayEffect"), "\"you may have\": {text}");
        assert!(text.contains("DealDamageEffect"), "{text}");
        assert!(text.contains("EventValue(Amount)"), "that much: {text}");
    }
}

#[test]
fn comeuppance_branches_on_the_prevented_sources_type() {
    for definition in routes("Comeuppance", COMEUPPANCE) {
        let text = spell_debug(&definition);
        assert!(text.contains("PreventAllDamageEffect"), "{text}");
        assert!(text.contains("Planeswalker"), "you and planeswalkers you control: {text}");
        assert!(text.contains("triggering_source"), "the prevented source is tagged: {text}");
        assert_eq!(text.matches("DealDamageEffect").count(), 2, "{text}");
        assert!(text.contains("excluded_card_types: [Creature]"), "noncreature branch: {text}");
        assert!(text.contains("ControllerOf"), "the source's controller: {text}");
    }
}

#[test]
fn judgment_of_alexander_has_commanders_hit_the_prevented_creature() {
    for definition in routes("Judgment of Alexander", JUDGMENT_OF_ALEXANDER) {
        let text = spell_debug(&definition);
        assert!(text.contains("PreventAllDamageEffect"), "{text}");
        assert!(text.contains("controller: Some(Opponent)"), "{text}");
        assert!(text.contains("DealDamageBySourcesEffect"), "{text}");
        assert!(text.contains("is_commander: true"), "{text}");
        assert!(text.contains("SourcePower"), "{text}");
        assert!(text.contains("triggering_source"), "{text}");
    }
}

#[test]
fn samite_ministration_gains_life_only_for_black_or_red_sources() {
    for definition in routes("Samite Ministration", SAMITE_MINISTRATION) {
        let text = spell_debug(&definition);
        assert!(text.contains("PreventAllDamageEffect"), "{text}");
        assert!(text.contains("source_of_your_choice: true"), "{text}");
        assert!(text.contains("GainLifeEffect"), "{text}");
        assert!(text.contains("TaggedObjectMatches"), "quality gate: {text}");
    }
}

#[test]
fn pollen_remedy_divides_six_when_kicked() {
    for definition in routes("Pollen Remedy", POLLEN_REMEDY) {
        let text = spell_debug(&definition);
        assert!(text.contains("PreventDamageEffect"), "{text}");
        assert!(text.contains("divided: true"), "{text}");
        assert!(text.contains("WasKicked"), "{text}");
        assert!(text.contains("Scaled(WasKicked, 3)"), "3 + 3 if kicked: {text}");
        assert!(!definition.optional_costs.is_empty(), "kicker retained");
    }
}

#[test]
fn well_laid_plans_relates_the_damage_source_and_recipient() {
    for definition in routes("Well-Laid Plans", WELL_LAID_PLANS) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.abilities);
        assert!(text.contains("PreventMatchingDamage"), "{text}");
        assert!(text.contains("SharesColorWithTagged"), "{text}");
        assert!(text.contains("triggering_source"), "{text}");
    }
}
