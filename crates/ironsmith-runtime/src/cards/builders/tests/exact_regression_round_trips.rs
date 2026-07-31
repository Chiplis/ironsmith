#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_exact_round_trip(name: &str, oracle: &str) {
    let definition = parse_oracle_card_definition(name);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle,
        "{definition:#?}"
    );
}

#[test]
fn arcane_artisan_retains_one_typed_player_target_across_the_result_chain() {
    assert_exact_round_trip(
        "Arcane Artisan",
        "{2}{U}, {T}: Target player draws a card, then exiles a card from their hand. If a creature card is exiled this way, that player creates a token that's a copy of that card.\nWhen this creature leaves the battlefield, exile all tokens created with it at the beginning of the next end step.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Arcane Artisan"));
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
}

#[test]
fn bifurcate_folds_the_typed_target_into_the_same_name_search() {
    assert_exact_round_trip(
        "Bifurcate",
        "Search your library for a permanent card with the same name as target nontoken creature, put that card onto the battlefield, then shuffle.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Bifurcate"));
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("SameNameAsTagged"), "{debug}");
}

#[test]
fn welcome_to_the_fold_reuses_one_target_across_both_toughness_thresholds() {
    let definition = parse_oracle_card_definition("Welcome to the Fold");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Gain control of target creature if its toughness is 2 or less. If this spell's madness cost was paid, instead gain control of that creature if its toughness is X or less.",
            "Madness {X}{U}{U}",
        ],
        "{definition:#?}"
    );
    let debug = format!("{definition:#?}");
    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "{debug}");
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("ToughnessOf"), "{debug}");
    assert!(!debug.contains("SourceToughness"), "{debug}");
}

#[test]
fn overload_reuses_one_target_across_both_mana_value_thresholds() {
    let definition = parse_oracle_card_definition("Overload");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Kicker {2}",
            "Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead.",
        ],
        "{definition:#?}"
    );
    let debug = format!("{definition:#?}");
    // Both threshold branches retain their own target-only wrapper, but they
    // deliberately share the same logical target tag.
    assert!(debug.contains("destroyed_0"), "{debug}");
    assert!(!debug.contains("destroyed_1"), "{debug}");
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
}

#[test]
fn talus_paladin_renders_the_optional_group_grant_as_a_causative() {
    assert_exact_round_trip(
        "Talus Paladin",
        "Whenever this creature or another Ally you control enters, you may have Allies you control gain lifelink until end of turn, and you may put a +1/+1 counter on this creature.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Talus Paladin"));
    assert_eq!(debug.matches("MayEffect").count(), 2, "{debug}");
    assert!(debug.contains("ApplyContinuousEffect"), "{debug}");
    assert!(debug.contains("AddAbility"), "{debug}");
}

#[test]
fn soul_swindler_keeps_the_typed_attraction_visit_condition() {
    assert_exact_round_trip(
        "Soul Swindler",
        "As long as you've visited an Attraction this turn, this creature has indestructible.\nWhen this creature enters, open an Attraction.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Soul Swindler"));
    assert!(debug.contains("PlayerVisitedAttractionThisTurn"), "{debug}");
    assert!(debug.contains("Indestructible"), "{debug}");
}

#[test]
fn archelos_keeps_complementary_conditions_on_other_permanent_entry_rules() {
    assert_exact_round_trip(
        "Archelos, Lagoon Mystic",
        "As long as Archelos is tapped, other permanents enter tapped.\nAs long as Archelos is untapped, other permanents enter untapped.",
    );
    let debug = format!(
        "{:#?}",
        parse_oracle_card_definition("Archelos, Lagoon Mystic")
    );
    assert!(debug.contains("EntersTappedForFilter"), "{debug}");
    assert!(debug.contains("EntersUntappedForFilter"), "{debug}");
    assert!(debug.contains("SourceIsTapped"), "{debug}");
    assert!(debug.contains("SourceIsUntapped"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
    assert!(
        !debug.contains("GrantAbility"),
        "entry rules must keep native conditional replacement semantics: {debug}"
    );
}

#[test]
fn fumble_preserves_the_former_attachment_set_and_one_new_recipient() {
    assert_exact_round_trip(
        "Fumble",
        "Return target creature to its owner's hand. Gain control of all Auras and Equipment that were attached to it, then attach them to another creature.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Fumble"));
    assert!(
        debug.contains("WasAttachedToTaggedObject"),
        "the controlled set must come from the bounced creature's last-known attachments: {debug}"
    );
    assert!(
        debug.contains("ChangeControllerToEffectController"),
        "{debug}"
    );
    assert!(debug.contains("AttachObjectsEffect"), "{debug}");
    assert!(debug.contains("objects: All("), "{debug}");
    assert_eq!(
        debug.matches("ChooseObjectsEffect").count(),
        1,
        "all former attachments must share one chosen new recipient: {debug}"
    );
    assert!(
        !debug.contains("relation: AttachedToTaggedObject"),
        "past-tense attachment provenance must not widen to current attachment state: {debug}"
    );
}

#[test]
fn gutter_grime_keeps_one_creator_bound_token_cda() {
    assert_exact_round_trip(
        "Gutter Grime",
        "Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with \"This token's power and toughness are each equal to the number of slime counters on Gutter Grime.\"",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Gutter Grime"));
    assert!(
        debug.contains("CharacteristicDefiningPt"),
        "the Ooze must carry an intrinsic dynamic P/T ability: {debug}"
    );
    assert!(
        debug.contains("FullName(") && debug.contains("\"Gutter Grime\""),
        "the CDA must retain the creating permanent's typed name reference: {debug}"
    );
    assert!(
        !debug.contains("SetBasePowerToughnessEffect"),
        "the creator-bound CDA must not also lower to an X/X fallback: {debug}"
    );
}

// Byte-exact oracle round trips for cards whose former tests pinned internal
// AST shapes / pre-merge render lines. The round trip is the durable contract:
// it fixes the decompiled text against real oracle, while the intermediate
// representation stays free to change.
#[test]
fn commander_liara_portyr_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Commander Liara Portyr",
        "Whenever you attack, spells you cast from exile this turn cost {X} less to cast, where X is the number of players being attacked. Exile the top X cards of your library. Until end of turn, you may cast spells from among those exiled cards.",
    );
}

#[test]
fn communal_brewing_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Communal Brewing",
        "When this enchantment enters, any number of target opponents each draw a card. Put an ingredient counter on this enchantment, then put an ingredient counter on it for each card drawn this way.\nWhenever you cast a creature spell, that creature enters with X additional +1/+1 counters on it, where X is the number of ingredient counters on this enchantment.",
    );
}

#[test]
fn dina_essence_brewer_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Dina, Essence Brewer",
        "Whenever you sacrifice a creature, draw a card. This ability triggers only once each turn.\n{2}, {T}, Sacrifice another creature: You gain X life and put X +1/+1 counters on target creature you control, where X is the sacrificed creature's power.",
    );
}

#[test]
fn forge_boss_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Forge Boss",
        "Whenever you sacrifice one or more other creatures, this creature deals 2 damage to each opponent. This ability triggers only once each turn.",
    );
}

#[test]
fn irresistible_prey_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Irresistible Prey",
        "Target creature must be blocked this turn if able.\nDraw a card.",
    );
}

#[test]
fn kang_prime_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Kang Prime",
        "Flying\nWhenever Kang Prime enters or attacks, exile cards from the top of your library until you exile a nonland card. Put two time counters on that card. If it doesn't have suspend, it gains suspend.",
    );
}

#[test]
fn lucid_dreams_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Lucid Dreams",
        "Draw X cards, where X is the number of card types among cards in your graveyard.",
    );
}

#[test]
fn maskwood_nexus_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Maskwood Nexus",
        "Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\n{3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling.",
    );
}

#[test]
fn rakdos_the_muscle_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Rakdos, the Muscle",
        "Flying, trample\nWhenever you sacrifice another creature, exile cards equal to its mana value from the top of target player's library. You may play those cards until your next end step, and mana of any type can be spent to cast them.\nSacrifice another creature: Rakdos gains indestructible until end of turn. Tap it. Activate only once each turn.",
    );
}

#[test]
fn sigarda_s_splendor_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Sigarda's Splendor",
        "As this enchantment enters, note your life total.\nAt the beginning of your upkeep, draw a card if your life total is greater than or equal to the last noted life total for this enchantment. Then note your life total.\nWhenever you cast a white spell, you gain 1 life.",
    );
}

#[test]
fn soul_partition_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Soul Partition",
        "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast.",
    );
}

#[test]
fn well_of_lost_dreams_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Well of Lost Dreams",
        "Whenever you gain life, you may pay {X}, where X is less than or equal to the amount of life you gained. If you do, draw X cards.",
    );
}

#[test]
fn wonderscape_sage_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Wonderscape Sage",
        "Flying\n{T}, Return a land you control to its owner's hand: Draw a card. Then discard a card unless that land had a nonbasic land type.",
    );
}
