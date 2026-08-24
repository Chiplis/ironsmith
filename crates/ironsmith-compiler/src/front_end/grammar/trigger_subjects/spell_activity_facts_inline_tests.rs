use super::*;

#[test]
fn typed_spell_activity_facts_preserve_turn_counts_and_origin() {
    let facts = parse_spell_activity_surface_facts(&[
        "whenever", "you", "cast", "your", "third", "spell", "each", "turn",
    ]);
    assert!(facts.has_spell_noun);
    assert_eq!(facts.exact_spells_this_turn, Some(3));
    assert_eq!(facts.min_spells_this_turn, None);

    let outside_hand = parse_spell_activity_surface_facts(&[
        "whenever", "you", "cast", "a", "spell", "from", "anywhere", "other", "than", "your",
        "hand",
    ]);
    assert!(outside_hand.from_not_hand);
}

#[test]
fn passive_nth_spell_of_turn_counts_all_players() {
    let facts = parse_spell_activity_surface_facts(&[
        "whenever", "the", "fourth", "spell", "of", "a", "turn", "is", "cast",
    ]);

    assert_eq!(facts.exact_spells_this_turn, Some(4));
    assert!(facts.count_all_spells_this_turn);
}

#[test]
fn lady_loki_union_qualifier_preserves_first_spell_each_turn() {
    let facts = parse_spell_activity_surface_facts(&[
        "whenever", "you", "cast", "your", "first", "instant", "sorcery", "or", "villain", "spell",
        "each", "turn",
    ]);

    assert_eq!(facts.exact_spells_this_turn, Some(1));
    assert_eq!(facts.min_spells_this_turn, None);
}

#[test]
fn first_spell_during_each_own_turn_preserves_count_and_turn_scope() {
    let facts = parse_spell_activity_surface_facts(&[
        "whenever", "you", "cast", "your", "first", "spell", "during", "each", "of", "your",
        "turns",
    ]);

    assert_eq!(facts.exact_spells_this_turn, Some(1));
    assert_eq!(facts.during_turn, Some(TriggerControllerReference::You));
}

#[test]
fn typed_draw_facts_preserve_draw_step_exception() {
    let facts = parse_draw_turn_surface_facts(&[
        "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of", "their",
        "draw", "steps",
    ]);
    assert!(facts.except_first_in_draw_step);
    assert_eq!(facts.exact_draws_this_turn, Some(2));
    assert!(facts.draw_numbers_this_turn.is_empty());
}

#[test]
fn typed_draw_facts_preserve_numbered_draw_sets() {
    let facts =
        parse_draw_turn_surface_facts(&["your", "first", "or", "second", "card", "each", "turn"]);
    assert_eq!(facts.exact_draws_this_turn, None);
    assert_eq!(facts.draw_numbers_this_turn, vec![1, 2]);
}

#[test]
fn typed_spell_filter_facts_preserve_color_origin_and_owner() {
    let chosen = parse_spell_filter_surface_facts(&["of", "the", "chosen", "color", "spells"]);
    assert!(chosen.chosen_color_qualifier);

    let graveyard = parse_spell_filter_surface_facts(&["a", "spell", "from", "your", "graveyard"]);
    assert_eq!(graveyard.origin, Some(SpellOriginSurface::Graveyard));
    assert_eq!(graveyard.owner, Some(SpellOwnerSurface::SubjectActor));
}

#[test]
fn typed_spell_filter_facts_do_not_promote_nested_comparison_zone() {
    let in_graveyard = parse_spell_filter_surface_facts(&[
        "a",
        "creature",
        "spell",
        "that",
        "doesnt",
        "share",
        "a",
        "creature",
        "type",
        "with",
        "a",
        "creature",
        "you",
        "control",
        "or",
        "a",
        "creature",
        "card",
        "in",
        "your",
        "graveyard",
    ]);
    assert_eq!(in_graveyard.origin, None);
    assert_eq!(in_graveyard.owner, None);

    let from_graveyard = parse_spell_filter_surface_facts(&[
        "a",
        "spell",
        "that",
        "shares",
        "a",
        "type",
        "with",
        "a",
        "card",
        "from",
        "your",
        "graveyard",
    ]);
    assert_eq!(from_graveyard.origin, None);
    assert_eq!(from_graveyard.owner, None);
}

#[test]
fn typed_spell_filter_facts_keep_direct_qualified_origin() {
    let facts = parse_spell_filter_surface_facts(&[
        "a",
        "creature",
        "spell",
        "with",
        "mana",
        "value",
        "four",
        "or",
        "less",
        "from",
        "your",
        "graveyard",
    ]);
    assert_eq!(facts.origin, Some(SpellOriginSurface::Graveyard));
    assert_eq!(facts.owner, Some(SpellOwnerSurface::SubjectActor));

    let actor_relative = parse_spell_filter_surface_facts(&["a", "spell", "from", "their", "hand"]);
    assert_eq!(actor_relative.origin, Some(SpellOriginSurface::Hand));
    assert_eq!(
        actor_relative.owner,
        Some(SpellOwnerSurface::SubjectActorPronoun)
    );
}
