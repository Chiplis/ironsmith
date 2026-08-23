use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

const CLASH_WIN_BRANCH_CARDS: &[&str] = &[
    "Adder-Staff Boggart",
    "Broken Ambitions",
    "Captivating Glance",
    "Fire Juggler",
    "Gilt-Leaf Ambush",
    "Hoarder's Greed",
    "Oaken Brawler",
    "Paperfin Rascal",
    "Pulling Teeth",
    "Recross the Paths",
    "Scattering Stroke",
];

#[test]
pub(super) fn clash_win_followups_keep_the_typed_result_predicate() {
    for name in CLASH_WIN_BRANCH_CARDS {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let compact_debug = format!("{definition:#?}")
            .split_whitespace()
            .collect::<String>();
        assert!(
            compact_debug.contains("ClashEffect")
                && compact_debug.contains("predicate:Value(GreaterThan(0"),
            "{name} must lower `if you win` to the ClashEffect result rather than generic happened semantics: {compact_debug}"
        );

        let compiled = compiled_text_lines(&definition).join("\n");
        let compiled_lower = compiled.to_ascii_lowercase();
        assert!(
            compiled_lower.contains("clash with an opponent")
                && compiled_lower.contains("if you win"),
            "{name} must preserve the clash-win surface: {compiled}"
        );
        assert!(
            !compiled.contains("If you do") && !compiled.contains("If it happened"),
            "{name} must not render a clash win as a generic result condition: {compiled}"
        );
    }
}

#[test]
pub(super) fn clash_loss_and_repeat_branches_render_from_the_same_typed_result() {
    for name in ["Captivating Glance", "Pulling Teeth"] {
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition).join("\n");
        assert!(
            compiled.contains("If you win") && compiled.contains("Otherwise"),
            "{name} must render the complementary clash result as otherwise: {compiled}"
        );
        assert!(
            !compiled.contains("If that doesn't happen"),
            "{name} must not describe losing the clash as a generic failed action: {compiled}"
        );
    }

    let greed = parse_oracle_card_definition("Hoarder's Greed");
    let compiled = compiled_text_lines(&greed).join("\n");
    assert!(
        compiled.contains(
            "You lose 2 life and draw two cards, then clash with an opponent. If you win, repeat this process"
        ),
        "Hoarder's Greed must render its structured clash-controlled loop: {compiled}"
    );
}

#[test]
pub(super) fn clash_and_win_trigger_preserves_the_win_gate_and_optional_action() {
    let definition = parse_oracle_card_definition("Sylvan Echoes");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("WinsClashTrigger") && debug.contains("MayEffect"),
        "the trigger must require a clash win and keep the draw optional: {debug}"
    );
    assert_eq!(
        compiled_text_lines(&definition).join("\n"),
        "Whenever you clash and win, you may draw a card."
    );
}

#[test]
pub(super) fn scattering_stroke_schedules_the_clash_reward_for_the_next_main_phase() {
    let definition = parse_oracle_card_definition("Scattering Stroke");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect")
            && debug.contains("BeginningOfMainPhaseTrigger")
            && debug.contains("player: You"),
        "Scattering Stroke must schedule, rather than immediately add, the clash reward: {debug}"
    );

    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "If you win, at the beginning of your next main phase, you may add an amount of {C} equal to its mana value"
        ),
        "Scattering Stroke must preserve its delayed next-main-phase surface: {compiled}"
    );
}

#[test]
pub(super) fn evolving_adaptive_keeps_both_relative_stat_gates() {
    assert_oracle_card_parses_strict("Evolving Adaptive");
    let definition = parse_oracle_card_definition("Evolving Adaptive");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Evolving Adaptive must have its creature-entry trigger");
    let condition = format!("{:#?}", triggered.intervening_if);
    assert!(
        condition.contains("Or(")
            && condition.matches("GreaterThanExpr").count() == 2
            && condition.contains("SourcePower")
            && condition.contains("SourceToughness"),
        "Evolving Adaptive must compare both triggering stats with the source: {condition}"
    );

    let compiled = compiled_text_lines(&definition)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("if")
            && compiled.contains("power")
            && compiled.contains("toughness")
            && compiled.contains("greater"),
        "Evolving Adaptive must surface its relative-stat gate: {compiled}"
    );
}

#[test]
pub(super) fn runaway_steam_kin_keeps_the_upper_counter_bound() {
    assert_oracle_card_parses_strict("Runaway Steam-Kin");
    let definition = parse_oracle_card_definition("Runaway Steam-Kin");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Runaway Steam-Kin must have its spell-cast trigger");
    let condition = format!("{:#?}", triggered.intervening_if);
    assert!(
        condition.contains("ValueComparison")
            && condition.contains("CountersOn")
            && condition.contains("LessThan")
            && condition.contains("Fixed(")
            && condition.contains("3,"),
        "Runaway Steam-Kin must keep the fewer-than-three condition: {condition}"
    );

    let compiled = compiled_text_lines(&definition)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("if this creature has fewer than three +1/+1 counters on it"),
        "Runaway Steam-Kin must surface its counter gate: {compiled}"
    );
}

#[test]
pub(super) fn adaptive_training_post_preserves_counter_gate_cost_and_copy_coordination() {
    assert_oracle_card_parses_strict("Adaptive Training Post");
    let definition = parse_oracle_card_definition("Adaptive Training Post");
    let compiled = compiled_text_lines(&definition).join("\n");

    assert!(
        compiled.contains(
            "if this artifact has fewer than three charge counters on it, put a charge counter on it"
        ),
        "Adaptive Training Post must surface its typed counter gate: {compiled}"
    );
    assert!(
        compiled.contains(
            "Remove three charge counters from this artifact: When you next cast an instant or sorcery spell this turn, copy that spell and you may choose new targets for the copy"
        ),
        "Adaptive Training Post must preserve its cost and coordinated delayed-copy surface: {compiled}"
    );
}

#[test]
pub(super) fn hall_of_mirrors_preserves_visit_and_nonlegendary_copy_surface() {
    assert_oracle_card_parses_strict("Hall of Mirrors");
    let definition = parse_oracle_card_definition("Hall of Mirrors");
    assert!(definition.card.subtypes.contains(&Subtype::Attraction));

    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.starts_with("Visit — Choose target creature you control."),
        "Attractions must retain their Visit ability word: {compiled}"
    );
    assert!(
        compiled.contains(
            "Each other creature you control becomes a copy of that creature until end of turn, except it isn't legendary"
        ) && !compiled.contains("For each"),
        "Hall of Mirrors must render one plural copy clause with its exception: {compiled}"
    );
}

#[test]
pub(super) fn muddle_copy_exception_grants_functional_myriad() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Muddle, the Ever-Changing")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you cast an instant or sorcery spell, this creature becomes a copy of up to one target nonlegendary creature you control until end of turn, except it has myriad.",
        )
        .expect("Muddle's myriad copy exception should parse");
    let debug = format!("{definition:#?}");
    let compiled = compiled_text_lines(&definition).join("\n");

    assert!(debug.contains("CreateTokenCopyEffect"), "{debug}");
    assert!(
        compiled.contains("until end of turn, except it has myriad"),
        "{compiled}"
    );
}

#[test]
pub(super) fn ajani_goldmane_preserves_separate_token_ability_presentation() {
    assert_oracle_card_parses_strict("Ajani Goldmane");
    let definition = parse_oracle_card_definition("Ajani Goldmane");
    let compiled = compiled_text_lines(&definition).join("\n");
    let create = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
                .find_map(|effect| effect.downcast_ref::<crate::effects::CreateTokenEffect>()),
            _ => None,
        })
        .expect("Ajani's ultimate must create its Avatar token");

    assert_eq!(
        create.ability_presentation,
        Some(ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined),
        "Ajani Goldmane must retain the parser's token-ability presentation through runtime conversion"
    );
    assert!(
        compiled.contains("Create a white Avatar creature token. It has")
            && !compiled.contains("Avatar creature token with"),
        "Ajani Goldmane must keep the granted token ability as a separate sentence: {compiled}"
    );
}
