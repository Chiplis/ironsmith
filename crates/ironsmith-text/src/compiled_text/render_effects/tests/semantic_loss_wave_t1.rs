//! Semantic-loss fix wave (2026-10-21), package T1: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn compile(
    name: &str,
    types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    text: &str,
) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(types)
        .subtypes(subtypes)
        .parse_text(text)
        .unwrap_or_else(|err| panic!("{name} should compile: {err:?}"))
}

fn triggered(definition: &crate::CardDefinition) -> Vec<&crate::ability::TriggeredAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect()
}

fn activated(definition: &crate::CardDefinition) -> Vec<&crate::ability::ActivatedAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .collect()
}

fn statics(
    definition: &crate::CardDefinition,
) -> Vec<&crate::static_abilities::StaticAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(ability) => Some(ability),
            _ => None,
        })
        .collect()
}

fn only_trigger(definition: &crate::CardDefinition) -> &crate::ability::TriggeredAbility {
    let triggers = triggered(definition);
    let [trigger] = triggers.as_slice() else {
        panic!("expected one triggered ability, got {}", triggers.len());
    };
    trigger
}

// --- Finding: intervening-if "it" on an Aura host is a reference, not a
// fresh target (Narcolepsy, Ray of Frost, Volition Reins, Maze's Mantle).

#[test]
fn narcolepsy_taps_the_enchanted_creature_without_targeting_it() {
    let definition = compile(
        "Narcolepsy",
        vec![CardType::Enchantment],
        vec![Subtype::Aura],
        "Enchant creature\nAt the beginning of each upkeep, if enchanted creature is untapped, tap it.",
    );
    let trigger = only_trigger(&definition);
    assert!(
        trigger.choices.is_empty(),
        "tapping the enchanted creature announces no target: {:?}",
        trigger.choices
    );
    let tap = trigger
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::TapEffect>())
        .expect("tap effect");
    assert!(!tap.target.is_target(), "{:?}", tap.target);
}

#[test]
fn volition_reins_untaps_its_host_without_a_target() {
    let definition = compile(
        "Volition Reins",
        vec![CardType::Enchantment],
        vec![Subtype::Aura],
        "Enchant permanent\nWhen this Aura enters, if enchanted permanent is tapped, untap it.\nYou control enchanted permanent.",
    );
    let trigger = only_trigger(&definition);
    assert!(trigger.choices.is_empty(), "{:?}", trigger.choices);
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains(", untap it."), "{text}");
}

// --- Finding: "That player discards that card" reuses the revealed-hand
// target instead of announcing another one (Revealing Eye).

#[test]
fn revealed_hand_discard_is_performed_by_the_already_targeted_player() {
    let definition = compile(
        "Revealing Eye",
        vec![CardType::Creature],
        vec![],
        "When this creature enters, target opponent reveals their hand. You may choose a nonland card from it. If you do, that player discards that card, then draws a card.",
    );
    let trigger = only_trigger(&definition);
    assert_eq!(trigger.choices.len(), 1, "{:?}", trigger.choices);
    let debug = format!("{:?}", trigger.effects);
    let discard = &debug[debug.find("DiscardEffect").expect("discard effect")..];
    let player = &discard[discard.find("player:").expect("discard player")..];
    assert!(
        player.trim_start_matches("player:").trim_start().starts_with("AliasedTarget"),
        "discarding player should alias the declared target: {}",
        &player[..player.len().min(80)]
    );
}

// --- Finding: a non-target blight subject quantifies over players
// (High Perfect Morcant).

#[test]
fn each_opponent_blights_without_choosing_a_target_player() {
    let definition = compile(
        "High Perfect Morcant",
        vec![CardType::Creature],
        vec![Subtype::Elf],
        "Whenever this creature or another Elf you control enters, each opponent blights 1.",
    );
    let trigger = only_trigger(&definition);
    assert!(trigger.choices.is_empty(), "{:?}", trigger.choices);
    let debug = format!("{:?}", trigger.effects);
    assert!(!debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("Blight"), "{debug}");
}

#[test]
fn target_opponent_blights_still_announces_its_target() {
    let definition = compile(
        "Blight Probe",
        vec![CardType::Creature],
        vec![],
        "When this creature enters, target opponent blights 2.",
    );
    assert_eq!(only_trigger(&definition).choices.len(), 1);
}

// --- Finding: "the amassed Army's power" measures the Army; it does not
// restrict the target to the Army (Grishnákh, Brash Instigator).

#[test]
fn amassed_army_power_comparison_does_not_bind_the_target_to_the_army() {
    let definition = compile(
        "Grishnákh, Brash Instigator",
        vec![CardType::Creature],
        vec![Subtype::Goblin],
        "When Grishnákh enters, amass Orcs 2. When you do, until end of turn, gain control of target nonlegendary creature an opponent controls with power less than or equal to the amassed Army's power. Untap that creature. It gains haste until end of turn.",
    );
    let debug = format!("{:?}", only_trigger(&definition).effects);
    // The only tagged constraint on the reflexive target must not be the
    // amass result; the amass tag survives only inside the power comparison.
    let target = &debug[debug.find("target_spec").expect("control target")..];
    let target = &target[..target.find("runtime_modifications").unwrap_or(target.len())];
    assert!(target.contains("PowerOf"), "{target}");
    assert!(
        !target.contains("relation: IsTaggedObject"),
        "the gained creature must not be the amassed Army: {target}"
    );
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains("the amassed Army's power"), "{text}");
}

// --- Finding: a compound delayed trigger binds its bare "it" to the
// declared object (Hunter's Insight).

#[test]
fn delayed_compound_damage_trigger_watches_the_chosen_creature() {
    let definition = compile(
        "Hunter's Insight",
        vec![CardType::Instant],
        vec![],
        "Choose target creature you control. Whenever that creature deals combat damage to a player or planeswalker this turn, draw that many cards.",
    );
    let debug = format!("{:?}", definition.spell_effect.as_ref().expect("spell"));
    let delayed = &debug[debug.find("ScheduleDelayedTriggerEffect").expect("delayed")..];
    assert!(!delayed.contains("\"__it__\""), "unresolved it tag: {delayed}");
    assert!(delayed.contains("\"targeted_0\""), "{delayed}");
}

// --- Finding: "target creature you cast this turn" keeps the turn bound
// and its caster resolves on the battlefield (Cycle of Life).

#[test]
fn cycle_of_life_target_requires_a_creature_you_cast_this_turn() {
    let definition = compile(
        "Cycle of Life",
        vec![CardType::Enchantment],
        vec![],
        "Return this enchantment to its owner's hand: Target creature you cast this turn has base power and toughness 0/1 until your next upkeep. At the beginning of your next upkeep, put a +1/+1 counter on that creature.",
    );
    let ability = activated(&definition)[0];
    let target = ability.choices.first().expect("target");
    let ChooseSpec::Object(filter) = target.base() else {
        panic!("object target: {target:?}");
    };
    assert!(filter.cast_this_turn);
    assert_eq!(filter.cast_by, Some(PlayerFilter::You));
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains("Target creature you cast this turn"), "{text}");
}

// --- Finding: "cast target ... card" is a targeted cast (Vadrok).

#[test]
fn vadrok_casts_an_announced_target_card() {
    let definition = compile(
        "Vadrok, Apex of Thunder",
        vec![CardType::Creature],
        vec![Subtype::Elemental],
        "Whenever this creature mutates, you may cast target noncreature card with mana value 3 or less from your graveyard without paying its mana cost.",
    );
    let trigger = only_trigger(&definition);
    assert_eq!(trigger.choices.len(), 1, "{:?}", trigger.choices);
    let debug = format!("{:?}", trigger.effects);
    assert!(
        !debug.contains("MayCastMatchingSpellWithoutPayingManaCostEffect"),
        "{debug}"
    );
}

// --- Finding: coordinated subjects keep their target (Deadly Dancer) and
// "you or another permanent" keeps "you" (Valkmira).

#[test]
fn source_and_another_target_creature_each_get_the_pump() {
    let definition = compile(
        "Deadly Dancer",
        vec![CardType::Creature],
        vec![],
        "{R}{R}: This creature and another target creature each get +1/+0 until end of turn.",
    );
    let ability = activated(&definition)[0];
    let [target] = ability.choices.as_slice() else {
        panic!("one target: {:?}", ability.choices);
    };
    let ChooseSpec::Object(filter) = target.base() else {
        panic!("object target: {target:?}");
    };
    assert!(filter.other && !filter.source, "{filter:?}");
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains("This creature and another target creature each get +1/+0"), "{text}");
}

#[test]
fn you_or_another_permanent_targeting_trigger_includes_the_player() {
    let definition = compile(
        "Valkmira, Protector's Shield",
        vec![CardType::Artifact],
        vec![],
        "Whenever you or another permanent you control becomes the target of a spell or ability an opponent controls, counter that spell or ability unless its controller pays {1}.",
    );
    let debug = format!("{:?}", only_trigger(&definition).trigger);
    assert!(debug.contains("PlayerOrObjectBecomesTargetedBySourceController"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
}

// --- Finding: "Spells you cast that target enchanted player" names the
// enchanted player (Infectious Curse).

#[test]
fn player_aura_cost_reduction_targets_the_enchanted_player() {
    let definition = compile(
        "Infectious Curse",
        vec![CardType::Enchantment],
        vec![Subtype::Aura, Subtype::Curse],
        "Enchant player\nSpells you cast that target enchanted player cost {1} less to cast.",
    );
    let payloads = statics(&definition)
        .into_iter()
        .filter_map(|ability| ability.compiled_model().map(|model| format!("{:?}", model.payload)))
        .collect::<Vec<_>>();
    let reduction = payloads
        .iter()
        .find(|payload| payload.contains("CostReduction"))
        .unwrap_or_else(|| panic!("cost reduction: {payloads:?}"));
    assert!(reduction.contains("targets_player: Some(TaggedPlayer(TagKey(\"enchanted\")))"), "{reduction}");
    assert!(reduction.contains("targets_object: None"), "{reduction}");
}

// --- Finding: "choose an opponent and a card name" keeps the player choice
// and remembers it for "the chosen player" (Booby Trap).

#[test]
fn booby_trap_chooses_and_remembers_an_opponent_as_it_enters() {
    let definition = compile(
        "Booby Trap",
        vec![CardType::Artifact],
        vec![],
        "As this artifact enters, choose an opponent and a card name other than a basic land card name.\nThe chosen player reveals each card they draw.\nWhen the chosen player draws a card with the chosen name, sacrifice this artifact. If you do, this artifact deals 10 damage to that player.",
    );
    let payload = statics(&definition)
        .into_iter()
        .filter_map(|ability| ability.compiled_model().map(|model| format!("{:?}", model.payload)))
        .find(|payload| payload.contains("AsEntersEffectProgram"))
        .expect("as-enters program");
    let player = payload.find("ChoosePlayerEffect").expect("player choice");
    let name = payload.find("ChooseCardNameEffect").expect("name choice");
    assert!(player < name, "{payload}");
    let choice = &payload[player..name];
    assert!(choice.contains("filter: Opponent"), "{choice}");
    assert!(choice.contains("remember_as_chosen_player: true"), "{choice}");
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        text.contains("choose an opponent and a card name other than a basic land card name"),
        "{text}"
    );
}

// --- Finding: a comma inside the face name is not the trigger/effect
// boundary (Sygg).

#[test]
fn comma_bearing_face_name_keeps_the_trigger_target() {
    let definition = compile(
        "Sygg, Wanderbrine Shield",
        vec![CardType::Creature],
        vec![Subtype::Merfolk],
        "Whenever this creature transforms into Sygg, Wanderbrine Shield, target creature you control gains protection from each color until your next turn.",
    );
    let trigger = only_trigger(&definition);
    assert_eq!(trigger.choices.len(), 1, "{:?}", trigger.choices);
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains("target creature you control gains protection"), "{text}");
}
