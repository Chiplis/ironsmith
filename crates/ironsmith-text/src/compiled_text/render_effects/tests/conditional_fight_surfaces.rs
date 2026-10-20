use super::*;

fn render_card(name: &str, oracle: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn targeted_conditional_fights_keep_both_target_slots_and_authored_pronouns() {
    let oracle = "Put a +1/+1 counter on target creature you control if it's legendary. Then it fights target creature an opponent controls.";
    assert_eq!(render_card("Ancient Animus", oracle), oracle);
}

#[test]
fn demonstrative_pump_fight_followups_reuse_the_prior_target() {
    let oracle = "Put a +1/+1 counter on target creature you control if its power is 4 or greater. Then that creature gets +1/+1 until end of turn and fights target creature you don't control.";
    assert_eq!(
        render_card("Ent's Fury", oracle),
        oracle.replace(" and fights", ". That creature fights")
    );
}

#[test]
fn explicit_pump_targets_remain_fresh_choices() {
    let oracle = "Put a +1/+1 counter on target creature you control. Then target creature gets +1/+1 until end of turn and fights target creature you don't control.";
    let rendered = render_card("Fresh Fight Targets", oracle);
    assert!(
        rendered.contains("target creature gets +1/+1 until end of turn"),
        "explicit target was rebound to the prior target: {rendered}"
    );
}

#[test]
fn declared_fighters_remain_distinct_through_the_conditional_counter() {
    let oracle = "Choose target creature you control and target creature an opponent controls. Put a +1/+1 counter on the creature you control if it has power 4 or greater. Then those creatures fight each other.";
    let rendered = render_card("Declared fighters", oracle);
    assert!(rendered.starts_with("Choose target creature you control and target creature an opponent controls."), "{rendered}");
    assert!(rendered.contains("Then those creatures fight each other"), "{rendered}");
}

#[test]
fn shared_fighter_collection_omits_redundant_alias_declaration() {
    let oracle = "Choose target creature you control and target creature you don't control. If the creature you control entered this turn, put a +1/+1 counter on it. Then those creatures fight each other.";
    let rendered = render_card("Shared fighters", oracle);
    assert!(rendered.starts_with("Choose target creature you control and target creature you don't control."), "{rendered}");
    assert!(!rendered.contains("Choose it"), "{rendered}");
    assert!(rendered.contains("Then those creatures fight each other"), "{rendered}");
}
