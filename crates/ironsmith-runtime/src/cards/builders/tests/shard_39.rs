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
                && compact_debug.contains("predicate:Value(GreaterThan(0))"),
            "{name} must lower `if you win` to the ClashEffect result rather than generic happened semantics: {compact_debug}"
        );

        let compiled = compiled_text_lines(&definition).join("\n");
        assert!(
            compiled.contains("Clash with an opponent") && compiled.contains("If you win"),
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
pub(super) fn scattering_stroke_schedules_the_clash_reward_for_the_next_main_phase() {
    let definition = parse_oracle_card_definition("Scattering Stroke");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect")
            && debug.contains("BeginningOfMainPhase(You)"),
        "Scattering Stroke must schedule, rather than immediately add, the clash reward: {debug}"
    );

    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "If you win, at the beginning of your next main phase, you may add an amount of {C} equal to that spell's mana value"
        ),
        "Scattering Stroke must preserve its delayed next-main-phase surface: {compiled}"
    );
}
