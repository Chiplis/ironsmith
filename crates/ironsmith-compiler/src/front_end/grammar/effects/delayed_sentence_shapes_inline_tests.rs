use super::*;
use crate::lexer::lex_line;

fn tokens(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

#[test]
fn parses_delayed_headers_and_typed_trigger_facts() {
    let next_combat =
        tokens("At the beginning of the next combat phase this turn, target creature attacks.");
    assert!(parse_delayed_next_combat_shape(&next_combat).is_some());

    let end_step = tokens("At the beginning of your next end step, draw a card.");
    assert_eq!(
        parse_delayed_end_step_shape(&end_step).unwrap().player,
        PlayerFilter::You
    );

    let delayed = tokens("This turn, when target creature attacks and isn't blocked, draw a card.");
    let shape = parse_delayed_this_turn_shape(&delayed).unwrap();
    assert_eq!(shape.placement, DelayedThisTurnPlacement::LeadingDuration);
    assert!(parse_delayed_attack_unblocked_subject(shape.trigger_tokens).is_some());
    assert!(!shape.references_previous_creature);

    let prior_creature = tokens("Whenever that creature is dealt damage this turn, draw a card.");
    assert!(
        parse_delayed_this_turn_shape(&prior_creature)
            .unwrap()
            .references_previous_creature
    );

    let unrelated = tokens("Whenever you draw a card this turn, gain 1 life.");
    assert!(
        !parse_delayed_this_turn_shape(&unrelated)
            .unwrap()
            .references_previous_creature
    );

    let target_dies = tokens("When target creature dies this turn, draw a card.");
    let shape = parse_delayed_this_turn_shape(&target_dies).unwrap();
    assert!(parse_delayed_target_dies_subject(shape.trigger_tokens).is_some());

    let target_graveyard =
        tokens("When target creature is put into your graveyard this turn, draw a card.");
    let shape = parse_delayed_this_turn_shape(&target_graveyard).unwrap();
    assert!(parse_delayed_target_put_into_your_graveyard_subject(shape.trigger_tokens).is_some());

    let prior_object = tokens("When it's put into a graveyard this turn, return that card.");
    let shape = parse_delayed_this_turn_shape(&prior_object).unwrap();
    assert!(is_delayed_prior_object_put_into_a_graveyard(
        shape.trigger_tokens
    ));

    let damaged_victim = tokens(
        "Whenever a creature dealt damage by that creature dies this turn, its controller loses 2 life.",
    );
    let shape = parse_delayed_this_turn_shape(&damaged_victim).unwrap();
    let damage_history =
        parse_delayed_dies_after_damage_by_previous_creature_shape(shape.trigger_tokens)
            .expect("damage-history death watcher should retain its victim domain");
    assert_eq!(
        crate::lexer::render_token_slice(damage_history.victim_tokens),
        "a creature"
    );
}

#[test]
fn parses_dies_and_copy_twice_shapes() {
    let dies = tokens("When that creature dies this turn, return it to your hand.");
    assert!(matches!(
        parse_delayed_dies_shape(&dies),
        Some(DelayedDiesShape::ThatReference { .. })
    ));
    let copy = tokens("copy that spell or ability twice you may choose new targets for the copies");
    assert!(
        parse_copy_twice_shape(&copy)
            .unwrap()
            .may_choose_new_targets
    );

    let leaves = tokens(
        "When that creature leaves the battlefield, return this card from exile to the battlefield.",
    );
    let shape = parse_delayed_tagged_leaves_shape(&leaves).unwrap();
    assert_eq!(shape.kind, DelayedLeavesObjectKind::Creature);
    assert!(!shape.effect_tokens.is_empty());
}
