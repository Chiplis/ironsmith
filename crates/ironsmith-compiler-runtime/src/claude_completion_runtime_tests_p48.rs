//! Claude package 48 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

const GOBLIN_CHARBELCHER: &str = "Mana cost: {4}\nType: Artifact\n{3}, {T}: Reveal cards from the top of your library until you reveal a land card. This artifact deals damage equal to the number of nonland cards revealed this way to any target. If the revealed land card was a Mountain, this artifact deals double that damage instead. Put the revealed cards on the bottom of your library in any order.";

#[test]
fn goblin_charbelcher_tests_the_stopping_land_and_counts_the_revealed_nonlands() {
    let definition =
        compile_to_runtime_definition("Goblin Charbelcher", GOBLIN_CHARBELCHER, false).unwrap();
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("Mountain"), "{debug}");
    // The Mountain test reads the card that stopped the traversal, never the
    // damage recipient.
    let condition = debug
        .find("TaggedObjectMatches(")
        .expect("the replacement condition tests a tagged object");
    let condition_tag = &debug[condition..debug[condition..].find(')').map_or(debug.len(), |end| condition + end)];
    assert!(condition_tag.contains("consult_match"), "{condition_tag}");
    // Both damage arms count the whole revealed collection (minus lands),
    // not the singular land match.
    let mut damage_arms = 0;
    for (start, _) in debug.match_indices("DealDamageEffect") {
        let arm = &debug[start..];
        let arm = &arm[..arm.find("target:").unwrap_or(arm.len())];
        assert!(arm.contains("_revealed_"), "{arm}");
        assert!(!arm.contains("consult_match"), "{arm}");
        damage_arms += 1;
    }
    assert!(damage_arms >= 2, "default and doubled damage arms: {debug}");
    assert!(debug.contains("Scaled"), "the replacement doubles the default amount: {debug}");
}

#[test]
fn gimli_reflexively_fights_when_the_trigger_resolves_for_the_third_time() {
    let definition = compile_to_runtime_definition(
        "Gimli, Mournful Avenger",
        "Mana cost: {1}{R}{G}\nType: Legendary Creature — Dwarf Warrior\nPower/Toughness: 3/2\nGimli has indestructible as long as two or more creatures died under your control this turn.\nWhenever another creature you control dies, put a +1/+1 counter on Gimli. When this ability resolves for the third time this turn, Gimli fights up to one target creature you don't control.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:#?}");
    // The resolution count is a captured state gate, and the fight is a
    // reflexive trigger reading that gate (its target is chosen only then).
    assert!(debug.contains("ThisAbilityResolvedThisTurnExactly(\n"), "{debug}");
    assert!(debug.contains("capture_condition_result: true"), "{debug}");
    let reflexive = debug
        .find("ReflexiveTriggerEffect")
        .expect("the third resolution creates a reflexive trigger");
    assert!(debug[reflexive..].contains("Fight"), "{debug}");
    assert!(debug.contains("PutCounters"), "{debug}");
}

#[test]
fn grimdancer_chooses_two_different_keyword_counters_as_it_enters() {
    let definition = compile_to_runtime_definition(
        "Grimdancer",
        "Mana cost: {1}{B}{B}\nType: Creature — Nightmare\nPower/Toughness: 3/3\nThis creature enters with your choice of two different counters on it from among menace, deathtouch, and lifelink.",
        false,
    )
    .unwrap();
    let choice = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => {
                match ability.compiled_model().map(|model| &model.payload) {
                    Some(ironsmith_core::StaticAbilityPayload::EntersWithCounterChoice {
                        counter_types,
                        kinds,
                        condition,
                        ..
                    }) => Some((counter_types.clone(), *kinds, condition.is_none())),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("distinct counter choice static");
    assert_eq!(choice.0.len(), 3, "{:?}", choice.0);
    let mut kinds_listed = choice.0.clone();
    kinds_listed.dedup();
    assert_eq!(kinds_listed.len(), 3, "the three listed kinds are distinct");
    assert_eq!(choice.1, 2, "two different kinds are chosen");
    assert!(choice.2, "unconditional entry choice");
}

#[test]
fn greymond_grants_humans_each_ability_of_the_chosen_pair() {
    let definition = compile_to_runtime_definition(
        "Greymond, Avacyn's Stalwart",
        "Mana cost: {2}{W}{W}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 3/4\nAs Greymond, Avacyn's Stalwart enters, choose two abilities from among first strike, vigilance, and lifelink.\nHumans you control have each of the chosen abilities.\nAs long as you control four or more Humans, Humans you control get +2/+2.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:#?}");
    // Choosing two different abilities is choosing one of the three pairs.
    for pair in ["first strike and vigilance", "first strike and lifelink", "vigilance and lifelink"] {
        assert!(debug.contains(pair), "missing option {pair}: {debug}");
    }
    assert!(debug.contains("ChooseNamedOptionAsEnters"), "{debug}");
    assert!(!debug.contains("stack_kind: Some(Ability)"), "no stack-ability choice: {debug}");
    assert!(
        debug.matches("SourceChosenOption").count() >= 3,
        "one conditional grant per chosen pair: {debug}"
    );
}

#[test]
fn grifters_blade_chooses_its_host_as_it_enters() {
    let definition = compile_to_runtime_definition(
        "Grifter's Blade",
        "Mana cost: {3}\nType: Artifact — Equipment\nFlash\nAs this Equipment enters, choose a creature you control it could be attached to. If you do, it enters attached to that creature.\nEquipped creature gets +1/+1.\nEquip {1}",
        false,
    )
    .unwrap();
    let host_filter = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => {
                match ability.compiled_model().map(|model| &model.payload) {
                    Some(ironsmith_core::StaticAbilityPayload::EntersAttachedToChosenObject {
                        filter,
                        ..
                    }) => Some(filter.clone()),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("entry host choice static");
    assert!(host_filter.card_types.contains(&ironsmith::types::CardType::Creature), "{host_filter:?}");
    assert_eq!(host_filter.controller, Some(ironsmith::target::PlayerFilter::You), "{host_filter:?}");
    let debug = format!("{definition:#?}");
    assert!(!debug.contains("ChooseObjectsEffect"), "no resolving choice program: {debug}");
}

#[test]
fn guile_replaces_its_controllers_spell_counters_with_exile_and_free_play() {
    let definition = compile_to_runtime_definition(
        "Guile",
        "Mana cost: {3}{U}{U}{U}\nType: Creature — Elemental Incarnation\nPower/Toughness: 6/6\nThis creature can't be blocked except by three or more creatures.\nIf a spell or ability you control would counter a spell, instead exile that spell and you may play that card without paying its mana cost.\nWhen Guile is put into a graveyard from anywhere, shuffle it into its owner's library.",
        false,
    )
    .unwrap();
    let permission = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => {
                match ability.compiled_model().map(|model| &model.payload) {
                    Some(ironsmith_core::StaticAbilityPayload::ControllerCounterExilePermission {
                        permission,
                        ..
                    }) => Some(*permission),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("counter replacement static");
    assert_eq!(permission.gate, ironsmith_core::CounterExileGate::AnySpell);
    assert!(permission.allow_land, "\"play\" also permits a land face");
}

#[test]
fn gogo_copies_x_times_and_forbids_copying_its_own_ability() {
    let definition = compile_to_runtime_definition(
        "Gogo, Master of Mimicry",
        "Mana cost: {2}{U}\nType: Legendary Creature — Wizard\nPower/Toughness: 2/4\n{X}{X}, {T}: Copy target activated or triggered ability you control X times. You may choose new targets for the copies. This ability can't be copied and X can't be 0. (Mana abilities can't be targeted.)",
        false,
    )
    .unwrap();
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Gogo's activated ability");
    assert!(activated.ability_cant_be_copied(), "{:?}", activated.additional_restrictions);
    assert_eq!(activated.activation_x_minimum(), 1, "X can't be 0");
    assert!(
        activated.effects.activation_definition.is_some(),
        "copy prohibition needs the authored activation identity"
    );
    let debug = format!("{:?}", activated.effects);
    assert!(debug.contains("CopySpellEffect"), "{debug}");
}
