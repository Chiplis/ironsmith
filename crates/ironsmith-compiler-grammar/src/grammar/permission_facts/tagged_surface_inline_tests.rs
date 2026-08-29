use super::*;
use crate::lexer::{TokenWordView, lex_line};

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn permission_leads_and_tagged_targets_are_typed() {
    let tokens = lex("You may cast that spell this turn");
    let lead = parse_permission_lead_tokens(&tokens).unwrap();
    assert_eq!(lead.actor, PermissionActor::You);
    assert_eq!(lead.verb, PermissionVerb::Cast);
    assert!(!lead.verb.allows_land());

    let target = parse_tagged_permission_target_tokens(lead.rest_tokens).unwrap();
    assert_eq!(target.reference, TaggedPermissionReference::LastTagged);
    assert_eq!(target.surface, TaggedPermissionTargetSurface::ThatSpell);
    assert_eq!(
        TokenWordView::new(target.rest_tokens).word_refs(),
        ["this", "turn"]
    );
}

#[test]
fn one_of_tagged_collection_preserves_shared_deferred_limit() {
    let tokens = lex("You may play one of those cards until your next end step");
    let lead = parse_permission_lead_tokens(&tokens).unwrap();
    let target = parse_tagged_permission_target_tokens(lead.rest_tokens).unwrap();
    assert_eq!(target.reference, TaggedPermissionReference::LastTagged);
    assert_eq!(target.max_plays, Some(1));
    assert_eq!(
        TokenWordView::new(target.rest_tokens).word_refs(),
        ["until", "your", "next", "end", "step"]
    );
}

#[test]
fn source_exile_bounded_permission_keeps_target_and_source_surfaces() {
    let cases = [
        (
            "You may play that card until you exile another card with this enchantment",
            TaggedPermissionTargetSurface::ThatCard,
            "this enchantment",
        ),
        (
            "You may play it until you exile another card with this artifact",
            TaggedPermissionTargetSurface::It,
            "this artifact",
        ),
    ];
    for (text, target_surface, source_surface) in cases {
        let tokens = lex(text);
        let parsed = parse_until_source_exiles_another_permission_tokens(&tokens).unwrap();
        assert_eq!(parsed.actor, PermissionActor::You);
        assert_eq!(parsed.verb, PermissionVerb::Play);
        assert_eq!(parsed.reference, TaggedPermissionReference::LastTagged);
        assert_eq!(parsed.target_surface, target_surface);
        assert_eq!(
            TokenWordView::new(parsed.source_reference_tokens).word_refs(),
            source_surface.split(' ').collect::<Vec<_>>()
        );
    }
}

#[test]
fn lifetime_and_tail_facts_preserve_free_cast_and_mana_permissions() {
    let prefix = lex("For as long as those cards remain exiled, you may cast them");
    let prefix = parse_permission_lifetime_prefix_tokens(&prefix).unwrap();
    assert_eq!(prefix.lifetime, PermissionLifetimeFact::ForAsLongAsExiled);

    let tail =
        lex("this turn without paying its mana cost and mana of any type can be spent to cast it");
    let parsed = parse_permission_tail_tokens(&tail, PermissionLifetimeFact::Immediate).unwrap();
    assert_eq!(parsed.lifetime, PermissionLifetimeFact::ThisTurn);
    assert!(parsed.without_paying_mana_cost);
    assert!(parsed.allow_any_color_for_cast);

    let any_type = parse_allow_any_color_for_cast_suffix_tokens(&tail).unwrap();
    assert_eq!(
        any_type.mana_spend_mode,
        ironsmith_core::value_model::ManaSpendMode::AnyType
    );
    assert_eq!(any_type.reference, ManaSpendCastReference::It);

    let any_color =
        lex("this turn, and you may spend mana as though it were mana of any color to cast it");
    let any_color = parse_allow_any_color_for_cast_suffix_tokens(&any_color).unwrap();
    assert_eq!(
        any_color.mana_spend_mode,
        ironsmith_core::value_model::ManaSpendMode::AnyColor
    );
    assert_eq!(any_color.reference, ManaSpendCastReference::It);
}

#[test]
fn temporary_permission_references_preserve_distinct_collection_surfaces() {
    let cases = [
        ("them", TaggedPermissionTargetSurface::Them),
        ("those cards", TaggedPermissionTargetSurface::ThoseCards),
        (
            "spells from among those cards",
            TaggedPermissionTargetSurface::SpellsFromAmongThoseCards,
        ),
        (
            "spells from among those exiled cards",
            TaggedPermissionTargetSurface::SpellsFromAmongThoseExiledCards,
        ),
        (
            "spells from among them",
            TaggedPermissionTargetSurface::Other,
        ),
    ];
    for (text, expected) in cases {
        let tokens = lex(text);
        let parsed = parse_tagged_permission_target_tokens(&tokens).unwrap();
        assert_eq!(parsed.surface, expected, "{text}");
    }

    let suffix = lex("and mana of any type can be spent to cast those spells");
    assert_eq!(
        parse_allow_any_color_for_cast_suffix_tokens(&suffix)
            .unwrap()
            .reference,
        ManaSpendCastReference::ThoseSpells
    );
}

#[test]
fn tagged_look_revealed_top_and_permanent_pool_facts_keep_boundaries() {
    let singular = lex(
        "For as long as it remains exiled, you may look at that card and you may cast it if it's a creature spell",
    );
    let singular = parse_for_as_long_as_look_at_tagged_tokens(&singular).unwrap();
    assert_eq!(singular.reference, TaggedLookReference::ThatCard);
    assert_eq!(
        TokenWordView::new(singular.permission_tokens).word_refs(),
        [
            "you", "may", "cast", "it", "if", "its", "a", "creature", "spell"
        ]
    );

    let look = lex(
        "For as long as those cards remain exiled, you may look at them, and you may cast permanent spells from among them",
    );
    let look = parse_for_as_long_as_look_at_tagged_tokens(&look).unwrap();
    assert_eq!(look.reference, TaggedLookReference::Them);
    let permanent = parse_permission_lead_tokens(look.permission_tokens).unwrap();
    let permanent = parse_spells_from_tagged_tokens(permanent.rest_tokens).unwrap();
    assert_eq!(
        TokenWordView::new(permanent.subject_tokens).word_refs(),
        ["permanent", "spells"]
    );
    assert!(permanent.tail_tokens.is_empty());

    let revealed = lex(
        "Until end of turn, for as long as that revealed card remains on top of your library, play with the top card of your library revealed and you may play that card",
    );
    let revealed = parse_revealed_top_library_permission_tokens(&revealed).unwrap();
    assert_eq!(
        TokenWordView::new(revealed.permission_tokens).word_refs(),
        ["you", "may", "play", "that", "card"]
    );
}

#[test]
fn qualified_spell_pool_fact_preserves_subject_surface_and_tail() {
    let tokens = lex("noncreature spells from among those cards without paying their mana costs");
    let parsed = parse_spells_from_tagged_tokens(&tokens).unwrap();
    assert_eq!(parsed.reference, TaggedPermissionReference::LastTagged);
    assert_eq!(
        TokenWordView::new(parsed.subject_tokens).word_refs(),
        ["noncreature", "spells"]
    );
    assert_eq!(
        parsed.surface,
        TaggedPermissionTargetSurface::SpellsFromAmongThoseCards
    );
    assert_eq!(
        TokenWordView::new(parsed.tail_tokens).word_refs(),
        ["without", "paying", "their", "mana", "costs"]
    );

    let permanent = lex("permanent spells from among them this turn");
    let permanent = parse_spells_from_tagged_tokens(&permanent).unwrap();
    assert_eq!(
        TokenWordView::new(permanent.subject_tokens).word_refs(),
        ["permanent", "spells"]
    );
    assert_eq!(permanent.surface, TaggedPermissionTargetSurface::Other);
    assert_eq!(
        TokenWordView::new(permanent.tail_tokens).word_refs(),
        ["this", "turn"]
    );
}

#[test]
fn additional_land_and_conditional_free_cast_facts_are_semantic() {
    let land_tokens = lex("Play an additional land this turn");
    let land = parse_additional_land_play_tokens(&land_tokens).unwrap();
    assert_eq!(land.count, Value::Fixed(1));

    let tail = lex("without paying its mana cost if its mana value is 3 or less");
    let tail = parse_conditional_tagged_free_cast_tail_tokens(&tail).unwrap();
    assert_eq!(tail.lifetime, PermissionLifetimeFact::Immediate);
    let condition = parse_tagged_mana_value_condition_tokens(tail.condition_tokens).unwrap();
    assert_eq!(condition.operator, ValueComparisonOperator::LessThanOrEqual);
    assert_eq!(condition.right, Value::Fixed(3));
}

#[test]
fn unsupported_permission_shapes_are_typed() {
    assert_eq!(
        parse_unsupported_permission_tokens(&lex("Play any number of lands on each of your turns")),
        Some(UnsupportedPermissionFact::AdditionalLandEachTurn)
    );
}
