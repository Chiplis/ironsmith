//! Claude package 70 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p70_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

fn compact_debug(definition: &ironsmith::cards::CardDefinition) -> String {
    format!("{definition:#?}").split_whitespace().collect()
}

/// The typed LoyaltyTwice reading must be admitted as a static line.
#[test]
fn oath_of_teferi_loyalty_twice_line_is_admitted_as_static() {
    let definition = p70_compile("Oath of Teferi",
        "Mana cost: {3}{W}{U}\nType: Legendary Enchantment\nWhen Oath of Teferi enters, exile another target permanent you control. Return it to the battlefield under its owner's control at the beginning of the next end step.\nYou may activate the loyalty abilities of planeswalkers you control twice each turn rather than only once.");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::LoyaltyAbilitiesTwiceEachTurn)));
}

/// "You may discard ..." starts with "you", not the rule-name head "may".
#[test]
fn circling_vultures_discard_any_time_line_reaches_its_static_rule() {
    let definition = p70_compile("Circling Vultures",
        "Mana cost: {B}\nType: Creature — Bird\nPower/Toughness: 3/2\nFlying\nYou may discard this card any time you could cast an instant.\nAt the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::MayDiscardSourceAnyTimeYouCouldCastInstant)));
}

/// "Enchanted creature is a copy ..." starts with "enchanted", not "attached".
#[test]
fn metamorphic_alteration_attached_copy_line_reaches_its_static_rule() {
    let definition = p70_compile("Metamorphic Alteration",
        "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant creature\nAs this Aura enters, choose a creature.\nEnchanted creature is a copy of the chosen creature.");
    let debug = compact_debug(&definition);
    assert!(debug.contains("AttachedIsCopyOfChosenObject"), "{debug}");
}

/// Behold a Jace: enters tapped unless a Jace card is in hand or a Jace is
/// controlled (CR 701.66).
#[test]
fn theorists_sanctum_behold_gate_enters_tapped_unless_jace() {
    let definition = p70_compile("Theorist's Sanctum",
        "Type: Land — Island\n({T}: Add {U}.)\nAs this land enters, you may behold a Jace. If you don't, this land enters tapped. (To behold a Jace, choose a Jace you control or reveal a Jace card from your hand.)\n{2}{U}, {T}: Empower Jace 2.");
    let debug = compact_debug(&definition);
    assert!(debug.contains("EntersTappedUnlessCondition"), "{debug}");
    assert!(debug.contains("Jace"), "{debug}");
}

/// "The discovered card" is Discover's object result, not a linked exile.
#[test]
fn hit_the_mother_lode_compares_the_discovered_cards_mana_value() {
    let definition = p70_compile("Hit the Mother Lode",
        "Mana cost: {4}{R}{R}{R}\nType: Sorcery\nDiscover 10. If the discovered card's mana value is less than 10, create a number of tapped Treasure tokens equal to the difference. (To discover 10, exile cards from the top of your library until you exile a nonland card with mana value 10 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)");
    let debug = compact_debug(&definition);
    assert!(debug.contains("DiscoverEffect"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(!debug.contains("__source_exiled__"), "not a source-linked exile: {debug}");
}

/// The leading "During your turn," is the grant's usage limit.
#[test]
fn leonardo_graveyard_cast_permission_is_limited_to_your_turns() {
    let definition = p70_compile("Leonardo, Sewer Samurai",
        "Mana cost: {3}{W}\nType: Legendary Creature — Mutant Ninja Turtle Samurai\nPower/Toughness: 3/3\nSneak {2}{W}{W}\nDouble strike\nDuring your turn, you may cast creature spells with power or toughness 1 or less from your graveyard. If you cast a spell this way, that creature enters with a finality counter on it. (If a creature with a finality counter on it would die, exile it instead.)");
    let debug = compact_debug(&definition);
    assert!(debug.contains("DuringYourTurns"), "{debug}");
    assert!(debug.contains("Finality"), "{debug}");
}

/// A serial type list inside one target phrase is one phase subject.
#[test]
fn reality_ripple_phases_out_a_serial_type_list_target() {
    let definition = p70_compile("Reality Ripple",
        "Mana cost: {1}{U}\nType: Instant\nTarget artifact, creature, or land phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)");
    assert!(definition.spell_effect.is_some());
    let debug = compact_debug(&definition);
    for card_type in ["Artifact", "Creature", "Land"] {
        assert!(debug.contains(card_type), "{card_type}: {debug}");
    }
}

#[test]
fn reverse_the_sands_redistributes_life_totals() {
    let definition = p70_compile("Reverse the Sands",
        "Mana cost: {6}{W}{W}\nType: Sorcery\nRedistribute any number of players' life totals. (Each of those players gets one life total back.)");
    let debug = compact_debug(&definition);
    assert!(debug.contains("RedistributeLifeTotals"), "{debug}");
}

/// Scry, surveil and library search all trigger Spoilers.
#[test]
fn river_song_spoilers_triggers_on_scry_surveil_or_library_search() {
    let definition = p70_compile("River Song",
        "Mana cost: {1}{U}{R}\nType: Legendary Creature — Human Time Lord Rogue\nPower/Toughness: 2/2\nMeet in Reverse — You draw cards from the bottom of your library rather than the top.\nSpoilers — Whenever an opponent scries, surveils, or searches their library, put a +1/+1 counter on River Song. Then River Song deals damage to that player equal to its power.");
    let debug = compact_debug(&definition);
    assert!(debug.contains("Scry"), "{debug}");
    assert!(debug.contains("Surveil"), "{debug}");
    assert!(debug.to_lowercase().contains("search"), "{debug}");
}

/// Others are barred from their own graveyards; you may use theirs.
#[test]
fn shamans_trance_restricts_others_and_grants_you_their_graveyards() {
    let definition = p70_compile("Shaman's Trance",
        "Mana cost: {2}{R}\nType: Instant\nOther players can't play lands or cast spells from their graveyards this turn. You may play lands and cast spells from other players' graveyards this turn as though those cards were in your graveyard.");
    assert!(definition.spell_effect.is_some());
    let debug = compact_debug(&definition);
    assert!(debug.contains("PlayLandsMatching"), "{debug}");
    assert!(debug.contains("CastSpellsMatching"), "{debug}");
    assert!(debug.contains("owner:Some(NotYou)"), "the grant covers other players' graveyards: {debug}");
}

/// The copy lasts only while Vesuvan stays face up.
#[test]
fn vesuvan_copy_lasts_until_turned_face_down() {
    let definition = p70_compile("Vesuvan Shapeshifter",
        "Mana cost: {3}{U}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 0/0\nAs this creature enters or is turned face up, you may choose another creature on the battlefield. If you do, until this creature is turned face down, it becomes a copy of that creature, except it has \"At the beginning of your upkeep, you may turn this creature face down.\"\nMorph {1}{U}");
    let debug = compact_debug(&definition);
    assert!(debug.contains("ObjectFaceUp(Source)"), "{debug}");
    assert!(debug.contains("TurnFaceDownEffect"), "{debug}");
}

/// An iterated token-copy creation owns its quoted grant.
#[test]
fn face_yourself_creates_hasty_copies_with_conditional_end_step_sacrifice() {
    let definition = p70_compile("Face Yourself",
        "Mana cost: {5}{R}{R}\nType: Sorcery\nFor each creature target player controls, create a token that's a copy of that creature, except it has haste and \"At the beginning of the end step, if you don't control a planeswalker, sacrifice this creature.\"");
    assert!(definition.spell_effect.is_some());
    let debug = compact_debug(&definition);
    assert!(debug.contains("Haste"), "{debug}");
    assert!(debug.contains("Planeswalker"), "{debug}");
}

/// Only activated abilities targeting your creatures draw.
#[test]
fn professor_hojo_draws_when_an_activated_ability_targets_your_creature() {
    let definition = p70_compile("Professor Hojo",
        "Mana cost: {1}{G}\nType: Legendary Creature — Human Scientist\nPower/Toughness: 2/2\nThe first activated ability you activate during your turn that targets a creature you control costs {2} less to activate.\nWhenever one or more creatures you control become the target of an activated ability, draw a card. This ability triggers only once each turn.");
    let debug = compact_debug(&definition);
    assert!(debug.contains("ActivatedAbility"), "{debug}");
    assert!(debug.contains("FirstTargetingActivationDuringYourTurn"), "{debug}");
}
