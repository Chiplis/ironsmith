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
            "Gain control of target creature if its toughness is 2 or less. If this spell's madness cost was paid, instead gain control of that creature if its toughness is X or less",
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
            "Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead",
        ],
        "{definition:#?}"
    );
    let debug = format!("{definition:#?}");
    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "{debug}");
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
