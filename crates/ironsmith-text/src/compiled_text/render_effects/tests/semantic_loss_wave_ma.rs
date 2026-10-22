//! Semantic-loss fix wave (2026-10-21), package MA: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

/// Compile real oracle text and return the definition plus a whitespace-free
/// structural dump (`{:#?}` with spaces and trailing commas removed) so the
/// assertions can name exact typed program fragments.
fn compile_shape(
    name: &str,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    text: &str,
) -> (crate::cards::CardDefinition, String) {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .subtypes(subtypes)
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} failed to compile: {error}"));
    let shape = format!("{definition:#?}")
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")")
        .replace(",}", "}")
        .replace(",]", "]");
    (definition, shape)
}

// --- "unless they sacrifice that many permanents" (Archfiend of Spite) ---

#[test]
fn unless_sacrifice_that_many_uses_the_event_amount_not_one_permanent() {
    let (_, shape) = compile_shape(
        "Archfiend of Spite",
        vec![CardType::Creature],
        vec![],
        "Whenever a source an opponent controls deals damage to this creature, that source's controller loses that much life unless they sacrifice that many permanents of their choice.",
    );
    assert!(shape.contains("UnlessActionEffect"), "{shape}");
    assert!(!shape.contains("UnlessPaysEffect"), "fixed one-permanent sacrifice cost: {shape}");
    assert!(
        shape.contains("count_value:Some(EventValue(Amount))"),
        "sacrifice count must be the damage amount: {shape}"
    );
}

// --- "unless they sacrifice that artifact" (Curse Artifact) ---

#[test]
fn unless_sacrifice_that_artifact_names_the_enchanted_artifact() {
    let (_, shape) = compile_shape(
        "Curse Artifact",
        vec![CardType::Enchantment],
        vec![Subtype::Aura],
        "Enchant artifact\nAt the beginning of the upkeep of enchanted artifact's controller, this Aura deals 2 damage to that player unless they sacrifice that artifact.",
    );
    assert!(shape.contains("UnlessActionEffect"), "{shape}");
    assert!(!shape.contains("UnlessPaysEffect"), "any-artifact sacrifice cost: {shape}");
    let alternative = shape
        .split_once("alternative:")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no alternative: {shape}"));
    assert!(
        alternative.contains(
            "TaggedObjectConstraint{tag:TagKey(\"enchanted\"),relation:IsTaggedObject}"
        ),
        "sacrificed object must be the enchanted artifact: {alternative}"
    );
}

// --- "the top ... of <graveyard|library>" put onto the battlefield ---

#[test]
fn put_top_creature_card_of_graveyard_selects_only_the_top_card() {
    let (_, shape) = compile_shape(
        "Bone Dancer",
        vec![CardType::Creature],
        vec![],
        "Whenever this creature attacks and isn't blocked, you may put the top creature card of defending player's graveyard onto the battlefield under your control. If you do, this creature assigns no combat damage this turn.",
    );
    let choice = shape
        .split_once("ChooseObjectsEffect{")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no top-card choice: {shape}"));
    assert!(choice.starts_with("optional_search_zones:false,search_participant_tag:None,filter:ObjectFilter{zone:Some(Graveyard)"), "{choice}");
    assert!(choice.contains("top_only:true"), "any graveyard card was eligible: {choice}");
}

#[test]
fn put_top_two_cards_of_library_selects_only_the_top_cards() {
    let (_, shape) = compile_shape(
        "Cybership",
        vec![CardType::Artifact],
        vec![Subtype::Vehicle],
        "Flying\nWhenever this Vehicle deals combat damage to a player, put the top two cards of that player's library onto the battlefield face down under your control. They're 2/2 Cyberman artifact creatures.\nCrew 4",
    );
    let choice = shape
        .split_once("ChooseObjectsEffect{")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no top-card choice: {shape}"));
    assert!(choice.contains("filter:ObjectFilter{zone:Some(Library)"), "{choice}");
    assert!(choice.contains("top_only:true"), "any two library cards were eligible: {choice}");
}

// --- "Simultaneously untap all tapped creatures and tap all untapped ..." ---

#[test]
fn simultaneous_untap_and_tap_snapshot_both_sets_before_acting() {
    let (_, shape) = compile_shape(
        "Breaking Wave",
        vec![CardType::Sorcery],
        vec![],
        "Simultaneously untap all tapped creatures and tap all untapped creatures.",
    );
    let snapshots: Vec<_> = shape.match_indices("TagMatchingObjectsEffect").map(|(i, _)| i).collect();
    let untap = shape.find("UntapEffect").unwrap_or_else(|| panic!("{shape}"));
    let tap = shape.find("TapEffect{").unwrap_or_else(|| panic!("{shape}"));
    assert!(snapshots.len() >= 2, "both sets must be snapshotted: {shape}");
    assert!(
        snapshots[1] < untap && snapshots[1] < tap,
        "the untapped set was read after untapping (taps everything): {shape}"
    );
    assert!(shape.contains("simultaneous_1"), "{shape}");
}

// --- "Put each other card exiled this way on the bottom" (Codie) ---

#[test]
fn each_other_exiled_card_bottom_keeps_the_found_card_in_exile() {
    let (_, shape) = compile_shape(
        "Codie, Vociferous Codex",
        vec![CardType::Artifact, CardType::Creature],
        vec![],
        "You can't cast permanent spells.\n{4}, {T}: Add {W}{U}{B}{R}{G}. When you next cast a spell this turn, exile cards from the top of your library until you exile an instant or sorcery card with lesser mana value. Until end of turn, you may cast that card without paying its mana cost. Put each other card exiled this way on the bottom of your library in a random order.",
    );
    let remainder = shape
        .split_once("PutTaggedRemainderOnLibraryBottomEffect{")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no remainder: {shape}"));
    assert!(
        remainder.contains("keep_tagged:Some(TagKey(\"__sentence_helper_consult_match"),
        "the castable match was bottomed too: {remainder}"
    );
}

// --- conditional "exile it, then return it to the battlefield" (Ifrit) ---

#[test]
fn conditional_exile_then_return_blinks_the_exiled_object() {
    let (_, shape) = compile_shape(
        "Ifrit, Warden of Inferno",
        vec![CardType::Enchantment, CardType::Creature],
        vec![Subtype::Saga],
        "I — Ifrit fights up to one other target creature.\nII, III — Add {R}{R}{R}{R}. If Ifrit has three or more lore counters on it, exile it, then return it to the battlefield.",
    );
    assert!(
        !shape.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "return looked in the graveyard and never found the exiled card: {shape}"
    );
    assert!(
        shape.contains("target:Tagged(TagKey(\"__source_exiled__\")),zone:Battlefield"),
        "return must follow the exiled object: {}",
        shape.split_once("chapters:[2,3]").map(|(_, rest)| rest).unwrap_or(&shape)
    );
}

// --- "creature card ... that's an artifact or that has mana value 3 or less" ---

#[test]
fn creature_card_that_is_artifact_or_cheap_requires_creature() {
    let (definition, _) = compile_shape(
        "Desdemona, Freedom's Edge",
        vec![CardType::Creature],
        vec![],
        "Vigilance\nWhenever Desdemona attacks, target creature card in your graveyard that's an artifact or that has mana value 3 or less gains escape until end of turn. The escape cost is equal to its mana cost plus exile two other cards from your graveyard.",
    );
    let shape = format!("{definition:?}");
    let filter = shape
        .split_once("Target(Object(ObjectFilter {")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no target: {shape}"));
    let filter = filter.split_once("functional_zones").map(|(head, _)| head).unwrap_or(filter);
    assert!(filter.contains("card_types: [Creature]"), "creature head lost: {filter}");
    assert!(
        !filter.contains("card_types: [Creature, Artifact]"),
        "creature-or-artifact union instead of creature with alternatives: {filter}"
    );
    assert!(filter.contains("card_types: [Artifact]"), "artifact alternative: {filter}");
    assert!(filter.contains("LessThanOrEqual(3)"), "mana value alternative: {filter}");
}

// --- "you may cast an instant or sorcery spell from among those exiled cards" ---

#[test]
fn single_spell_from_exiled_pool_allows_one_cast() {
    let (_, shape) = compile_shape(
        "Chandra, Hope's Beacon",
        vec![CardType::Planeswalker],
        vec![Subtype::Chandra],
        "+1: Exile the top five cards of your library. Until the end of your next turn, you may cast an instant or sorcery spell from among those exiled cards.",
    );
    let grant = shape
        .split_once("GrantPlayTaggedEffect{")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no grant: {shape}"));
    assert!(grant.contains("max_plays:Some(1)"), "every exiled spell was castable: {grant}");
}

// --- "look at the top card of that player's library" (Blightwing Bandit) ---

#[test]
fn opponent_turn_cast_trigger_that_player_is_the_active_player() {
    let (_, shape) = compile_shape(
        "Blightwing Bandit",
        vec![CardType::Creature],
        vec![],
        "Flying, deathtouch\nWhenever you cast your first spell during each opponent's turn, look at the top card of that player's library, then exile it face down. You may play that card for as long as it remains exiled, and mana of any type can be spent to cast it.",
    );
    assert!(
        shape.contains("LookAtTopCardsEffect{player:Active"),
        "looked at your own library: {shape}"
    );
}

// --- "If you cast a spell this way, mana of any type can be spent to cast it." ---

#[test]
fn any_type_rider_folds_into_the_until_next_turn_permission() {
    let (_, shape) = compile_shape(
        "Bloodsoaked Insight",
        vec![CardType::Sorcery],
        vec![],
        "Target opponent exiles the top three cards of their library. Until the end of your next turn, you may play those cards. If you cast a spell this way, mana of any type can be spent to cast it.",
    );
    assert_eq!(
        shape.matches("GrantPlayTaggedEffect{").count(),
        1,
        "rider became a second permission: {shape}"
    );
    assert!(!shape.contains("ForAsLongAsExiled"), "permanent cast permission: {shape}");
    assert!(shape.contains("mana_spend_mode:AnyType"), "{shape}");
}

// --- "Alandra and Drakes you control each get +X/+X" ---

#[test]
fn source_and_filter_pair_both_get_the_pump() {
    let (_, shape) = compile_shape(
        "Alandra, Sky Dreamer",
        vec![CardType::Creature],
        vec![],
        "Whenever you draw your fifth card each turn, Alandra and Drakes you control each get +X/+X until end of turn, where X is the number of cards in your hand.",
    );
    assert_eq!(
        shape.matches("ModifyPowerToughness").count(),
        2,
        "only the source was pumped: {shape}"
    );
    assert!(shape.contains("subtypes:[Drake]"), "Drakes dropped: {shape}");
}

// --- "for each {R}{R} spent to cast it, it gets +1/+0 and gains ..." ---

#[test]
fn mana_spent_multiplier_repeats_the_pump_and_grant() {
    let (_, shape) = compile_shape(
        "Bladecoil Serpent",
        vec![CardType::Artifact, CardType::Creature],
        vec![],
        "When this creature enters, for each {R}{R} spent to cast it, it gets +1/+0 and gains trample and haste until end of turn.",
    );
    let repeat = shape
        .split_once("RepeatEffectsEffect{")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("multiplier dropped: {shape}"));
    assert!(repeat.contains("ManaSymbolSpentToCastThisSpell{symbol:Red"), "{repeat}");
    assert!(repeat.contains("ModifyPowerToughness"), "pump outside the repeat: {repeat}");
}

// --- "If you do, another target creature ... gets +1/+0 and gains haste" ---

#[test]
fn result_prefixed_another_target_pump_keeps_its_grant() {
    let (_, shape) = compile_shape(
        "Conduit Goblin",
        vec![CardType::Creature],
        vec![],
        "At the beginning of combat on your turn, you may pay {E}. If you do, another target creature you control gets +1/+0 and gains haste until end of turn.",
    );
    assert!(shape.contains("ModifyPowerToughness"), "{shape}");
    assert!(shape.contains("id:Some(Haste)"), "haste grant dropped: {shape}");
}

// --- "If you put a noncreature, nonland card into your graveyard this way" ---

#[test]
fn surveil_followup_checks_only_the_card_put_into_the_graveyard() {
    let (_, shape) = compile_shape(
        "Chandra, Chill of Compliance",
        vec![CardType::Planeswalker],
        vec![Subtype::Chandra],
        "+1: Surveil 1. If you put a noncreature, nonland card into your graveyard this way, put that card into your hand.",
    );
    assert!(
        shape.contains("TaggedObjectMatches(TagKey(\"__surveil_put_into_graveyard__\")"),
        "a card kept on top of the library could be put into hand: {shape}"
    );
}
