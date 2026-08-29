#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_06::*;
use super::*;

#[test]
pub(super) fn rewrite_lowered_for_each_player_choose_uses_controller_as_chooser()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Ghouls' Night Out Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "For each player, choose a creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're black Zombies in addition to their other colors and types and they gain decayed."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug.contains("SequenceEffect") || debug.contains("ForPlayersEffect"),
        "{debug}"
    );
    assert!(compact.contains("owner:Some(IteratedPlayer"), "{debug}");
    assert!(compact.contains("chooser:You"), "{debug}");
    assert!(compact.contains("target_spec:Some(Tagged("), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_for_each_player_counted_target_stays_a_cast_target()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Afterlife from the Loam Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "For each player, choose up to one target creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're Zombies in addition to their other types."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(!debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(compact.contains("owner:Some(IteratedPlayer"), "{debug}");
    assert!(compact.contains("min:0"), "{debug}");
    assert!(compact.contains("max:Some(1"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_radiance_shared_color_fanout() {
    let text = "Radiance — Target creature and each other creature that shares a color with it gain haste until end of turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify labeled radiance fanout sentence");

    let stripped = crate::cards::builders::parse_effect_sentence_lexed(
        lexed
            .split(|token| {
                matches!(
                    token.kind,
                    super::super::lexer::TokenKind::Dash | super::super::lexer::TokenKind::EmDash
                )
            })
            .nth(1)
            .expect("labeled sentence should contain body after dash"),
    )
    .expect("rewrite effect sentence parser should support radiance fanout");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should support radiance fanout");
    let direct = crate::cards::builders::parse_shared_color_target_fanout_sentence(
        lexed
            .split(|token| {
                matches!(
                    token.kind,
                    super::super::lexer::TokenKind::Dash | super::super::lexer::TokenKind::EmDash
                )
            })
            .nth(1)
            .expect("labeled sentence should contain body after dash"),
    )
    .expect("shared-color primitive should not error");
    let mut lowered_body = lexed
        .split(|token| {
            matches!(
                token.kind,
                super::super::lexer::TokenKind::Dash | super::super::lexer::TokenKind::EmDash
            )
        })
        .nth(1)
        .expect("labeled sentence should contain body after dash")
        .to_vec();
    for token in &mut lowered_body {
        token.lowercase_word();
    }
    let lowered_direct =
        crate::cards::builders::parse_shared_color_target_fanout_sentence(&lowered_body)
            .expect("lowered shared-color primitive should not error");
    let debug = format!("{parsed:?}");
    let direct_debug = format!("{direct:?}");
    let lowered_direct_debug = format!("{lowered_direct:?}");
    let stripped_debug = format!("{stripped:?}");

    assert!(
        direct_debug.contains("GrantAbilitiesAll"),
        "expected direct shared-color primitive to build fanout grant effect, got {direct_debug}"
    );
    assert!(
        direct_debug.contains("SharesColorWithTagged"),
        "expected direct shared-color primitive to keep shared-color tagged constraint, got {direct_debug}"
    );
    assert!(
        lowered_direct_debug.contains("GrantAbilitiesAll"),
        "expected lowered shared-color primitive to build fanout grant effect, got {lowered_direct_debug}"
    );
    assert!(
        stripped_debug.contains("GrantAbilitiesAll"),
        "expected stripped sentence parser to preserve fanout grant effect, got {stripped_debug}"
    );
    assert!(
        debug.contains("GrantAbilitiesAll"),
        "expected labeled sentence parser to preserve fanout grant effect, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_destroy_radiance_keeps_the_shared_color_target_set() {
    let text = "Radiance — Destroy target enchantment and each other enchantment that shares a color with it.";
    let lexed = lex_line(text, 0).expect("destroy radiance clause should lex");
    let parsed = parse_effect_sentence_lexed(&lexed).expect("destroy radiance clause should parse");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("Destroy")
            && debug.contains("DestroyAll")
            && debug.contains("SharesColorWithTagged")
            && debug.contains("IsNotTaggedObject"),
        "expected target destroy plus linked shared-color fanout, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_radiance_duration_prefix_quoted_ability_fanout()
 {
    let text = "Radiance — Until end of turn, target creature and each other creature that shares a color with it gain \"This creature can't block.\"";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify radiance quoted fanout sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should support quoted radiance fanout");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantAbilitiesAll"),
        "expected fanout grant effect, got {debug}"
    );
    assert!(
        debug.contains("SharesColorWithTagged"),
        "expected shared-color tagged constraint, got {debug}"
    );
    assert!(
        debug.contains("CantBlock"),
        "expected CantBlock grant, got {debug}"
    );
    assert!(
        debug.contains("EndOfTurn"),
        "expected leading duration to apply to the grant, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_compound_damage_to_target_and_controller_objects()
 {
    let text = "Chandra Nalaar deals 10 damage to target player or planeswalker and each creature that player or that planeswalker's controller controls.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify compound damage fanout sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should split compound damage fanout");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("DealDamage") && debug.contains("DealDamageEach"),
        "expected target damage plus object fanout damage, got {debug}"
    );
    assert!(
        debug.contains("TargetPlayerOrControllerOfTarget"),
        "expected controller fanout to track target player or planeswalker controller, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_compound_damage_to_object_groups() {
    let text = "This deals 4 damage to each non-Giant creature and each planeswalker.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify compound object damage sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should split compound object damage");
    let debug = format!("{parsed:?}");

    assert_eq!(
        debug.matches("DealDamageEach").count(),
        2,
        "expected two object fanout damage effects, got {debug}"
    );
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("Planeswalker"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_equal_to_compound_damage_to_objects_and_players()
 {
    let text = "This artifact deals damage equal to the number of time counters on it to each creature and each player.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify equal-to compound damage sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should split equal-to compound damage");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("CountersOnSource"), "{debug}");
    assert!(debug.contains("DealDamageEach"), "{debug}");
    assert!(debug.contains("ForEachPlayer"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_equal_to_damage_to_controller_phrase_target() {
    let text = "It deals damage equal to that creature's power to the creature's controller.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify equal-to damage with controller phrase target");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should accept controller phrase target");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("PowerOf(Tagged(TagKey(\"__it__\")))"),
        "{debug}"
    );
    assert!(
        debug.contains("target: Player(ControllerOf(Tagged(TagKey(\"__it__\"))), None)"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_equal_to_damage_to_any_target_with_source_counters()
 {
    let text = "It deals damage equal to the number of pressure counters on it to any target.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify equal-to source-counter damage sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should accept equal-to any-target damage");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("CountersOnSource(Named(\"pressure\"))"),
        "{debug}"
    );
    assert!(
        debug.contains("target: AnyTarget") || debug.contains("target: Any"),
        "expected any-target damage lowering, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_draw_for_each_counter_on_source() {
    let text = "Draw a card for each lore counter on this enchantment.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify source-counter draw sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should accept source-counter draw");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("CountersOnSource(Lore)"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_draw_for_each_counter_on_this_aura() {
    let text = "Draw a card for each page counter on this Aura.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify aura-counter draw sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should accept aura-counter draw");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Draw"), "{debug}");
    assert!(
        debug.contains("CountersOnSource(Named(\"page\"))"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_draw_for_each_spell_cast_this_turn() {
    let text = "Draw a card for each other instant and sorcery spell you've cast this turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify spell-cast-count draw sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should accept spell-cast-count draw");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Draw"), "{debug}");
    assert!(
        debug.contains("TurnHistoryCount") && debug.contains("SpellsCast"),
        "{debug}"
    );
    assert!(debug.contains("exclude_source: true"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_target_gain_then_get_where_x() {
    let text = "Target creature gains trample and gets +X/+0 until end of turn, where X is the number of creatures you control.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify gain-then-get sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should split gain-then-get target pump");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantAbilitiesToTarget") && debug.contains("Pump"),
        "expected target ability grant plus pump effect, got {debug}"
    );
    assert!(
        debug.contains("Trample") && debug.matches("EndOfTurn").count() >= 2,
        "expected trample and shared end-of-turn duration, got {debug}"
    );
    assert!(
        debug.matches("WhereXIs").count() >= 1,
        "expected the pump values to preserve the where-X surface, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_filter_gain_then_get_where_x() {
    let text = "Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creatures you control.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify gain-then-get sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should split gain-then-get anthem pump");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantAbilitiesAll") && debug.contains("PumpAll"),
        "expected filter ability grant plus pump-all effect, got {debug}"
    );
    assert!(
        debug.contains("Trample") && debug.matches("EndOfTurn").count() >= 2,
        "expected trample and shared end-of-turn duration, got {debug}"
    );
    assert!(
        debug.matches("WhereXIs").count() >= 2,
        "expected the pump-all values to preserve the where-X surface, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_partial_hand_reveal_preserves_direct_and_computed_x_counts() {
    for (text, expected) in [
        (
            "Target player reveals X cards from their hand.",
            &["RevealCardsFromHand", "dynamic_x: true"][..],
        ),
        (
            "Target player reveals X cards from their hand, where X is the number of Faeries you control.",
            &[
                "RevealCardsFromHand",
                "dynamic_x: true",
                "WhereXIs",
                "Faerie",
            ][..],
        ),
        (
            "Target player reveals a number of cards from their hand equal to one plus the number of creature cards in your graveyard.",
            &[
                "RevealCardsFromHand",
                "dynamic_x: true",
                "Add",
                "Fixed(1)",
                "Graveyard",
            ][..],
        ),
    ] {
        let lexed = lex_line(text, 0).expect("partial hand-reveal sentence should lex");
        let parsed = parse_effect_sentence_lexed(&lexed)
            .expect("partial hand-reveal sentence should preserve its dynamic count");
        let debug = format!("{parsed:?}");
        for needle in expected {
            assert!(debug.contains(needle), "expected {needle:?} in {debug}");
        }
    }
}

#[test]
pub(super) fn rewrite_full_production_thieving_sprite_preserves_where_x_hand_reveal()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Thieving Sprite")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Faerie, Subtype::Rogue])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1));
    let text = "Flying\nWhen this creature enters, target player reveals X cards from their hand, where X is the number of Faeries you control. You choose one of those cards. That player discards that card.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Thieving Sprite should have a triggered ability");
    let effects = &triggered.effects.segments[0].default_effects;
    let reveal = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .find(|choose| choose.reveal && choose.count.dynamic_x)
        .expect("Thieving Sprite should dynamically reveal cards from the target player's hand");
    let count_value = reveal
        .count_value
        .as_ref()
        .expect("the full production path should carry the where-X value onto the hand reveal");
    assert!(
        count_value.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs),
        "the hand reveal should retain its where-X surface: {count_value:#?}"
    );
    let Value::Count(faerie_filter) = count_value.unhinted() else {
        panic!("expected where-X to count controlled Faeries, got {count_value:#?}");
    };
    assert!(faerie_filter.subtypes.contains(&Subtype::Faerie));
    assert_eq!(
        faerie_filter.controller,
        Some(crate::target::PlayerFilter::You)
    );

    Ok(())
}

#[test]
pub(super) fn rewrite_full_production_hollow_specter_keeps_dependent_discard_in_if_branch()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Hollow Specter")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Specter])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2));
    let text = "Flying\nWhenever this creature deals combat damage to a player, you may pay {X}. If you do, that player reveals X cards from their hand and you choose one of them. That player discards that card.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Hollow Specter should have a triggered ability");
    let effects = &triggered.effects.segments[0].default_effects;
    assert!(
        effects.iter().all(|effect| effect
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_none()),
        "the dependent discard must not escape the if-you-do branch: {effects:#?}"
    );
    let if_effect = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
        .expect("Hollow Specter should lower its if-you-do clause");
    let [reveal_effect, choose_effect, discard_effect] = if_effect.then.as_slice() else {
        panic!(
            "expected reveal, choose-one, and dependent discard in the if branch: {:#?}",
            if_effect.then
        );
    };

    let reveal = reveal_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("the branch should start by revealing cards from hand");
    assert!(reveal.reveal && reveal.count.dynamic_x);
    assert_eq!(reveal.chooser, crate::target::PlayerFilter::DamagedPlayer);
    assert_eq!(
        reveal.filter.owner,
        Some(crate::target::PlayerFilter::DamagedPlayer)
    );

    let choose = choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("the branch should choose one of the revealed cards");
    assert_eq!(choose.count.min, 1);
    assert_eq!(choose.count.max, Some(1));
    assert_eq!(choose.chooser, crate::target::PlayerFilter::You);
    assert!(choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == reveal.tag
    }));

    let discard = discard_effect
        .downcast_ref::<crate::effects::DiscardEffect>()
        .expect("the branch should end by discarding the chosen card");
    assert_eq!(discard.player, crate::target::PlayerFilter::DamagedPlayer);
    let discard_filter = discard
        .card_filter
        .as_ref()
        .expect("the discard should be restricted to the chosen card");
    assert_eq!(
        discard_filter.owner,
        Some(crate::target::PlayerFilter::DamagedPlayer)
    );
    assert!(discard_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == choose.tag
    }));

    Ok(())
}

#[test]
pub(super) fn rewrite_full_production_leshracs_sigil_keeps_looked_card_discard_in_if_branch()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Leshrac's Sigil")
        .card_types(vec![CardType::Enchantment]);
    let text = "Whenever an opponent casts a green spell, you may pay {B}{B}. If you do, look at that player's hand and choose a card from it. The player discards that card.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Leshrac's Sigil should have a triggered ability");
    let effects = &triggered.effects.segments[0].default_effects;
    assert!(
        effects.iter().all(|effect| effect
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_none()),
        "the chosen-card discard must not escape the if-you-do branch: {effects:#?}"
    );
    let if_effect = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
        .expect("Leshrac's Sigil should lower its if-you-do clause");
    let sequence = if_effect
        .then
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::SequenceEffect>())
        .expect("the successful branch should retain the authored look-and-choose coordination");
    assert!(sequence.effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::LookAtHandEffect>()
            .is_some()
    }));
    let choose = sequence
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .expect("the successful branch should choose a card from the looked-at hand");
    let discard = if_effect
        .then
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DiscardEffect>())
        .expect("the successful branch should discard the chosen card");
    let discard_filter = discard
        .card_filter
        .as_ref()
        .expect("the discard should be restricted to the chosen card");
    assert!(discard_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == choose.tag
    }));

    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_gain_then_get_for_each_cards_drawn() {
    let text = "Until end of turn, target creature gains trample and gets +1/+0 for each card you've drawn this turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify gain-then-get for-each sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should split gain-then-get for-each pump");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantAbilitiesToTarget") && debug.contains("PumpForEach"),
        "expected target ability grant plus for-each pump effect, got {debug}"
    );
    assert!(
        debug.contains("MaxCardsDrawnThisTurn") && debug.contains("Trample"),
        "expected cards-drawn count and trample grant, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_kydele_mana_scaling_clause() {
    let text = "Add {C} for each card you've drawn this turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify Kydele mana-scaling sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should support cards-drawn mana scaling");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("AddMana") && debug.contains("MaxCardsDrawnThisTurn"),
        "expected add-mana effect scaled by cards drawn this turn, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_compound_damage_to_you_and_your_objects() {
    let text = "This deals 2 damage to you and each creature you control.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify player plus object damage sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should split player plus object damage");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Player(You"), "{debug}");
    assert!(debug.contains("DealDamageEach"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_split_labeled_effect_prefix_supports_two_word_labels() {
    let lexed = lex_line("Spell mastery — Draw a card.", 0)
        .expect("rewrite lexer should classify spell mastery sentence");

    let stripped = crate::grammar::effects::split_labeled_effect_prefix_lexed(&lexed)
        .expect("spell mastery label should be stripped by grammar helper");

    assert_eq!(
        crate::lexer::token_word_refs(stripped)
            .into_iter()
            .map(|word| word.to_ascii_lowercase())
            .collect::<Vec<_>>(),
        vec!["draw".to_string(), "a".to_string(), "card".to_string()]
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_does_not_strip_unknown_labeled_prefix() {
    let lexed = lex_line("Mystery — Draw a card.", 0)
        .expect("rewrite lexer should classify unknown labeled sentence");

    let parsed = parse_effect_sentence_lexed(&lexed);
    assert!(
        parsed.is_err(),
        "unknown labeled prefix should not be stripped as a known effect label"
    );
}

#[test]
pub(super) fn typed_labeled_line_regressions_cover_saga_choice_cycling_and_quoted_rules() {
    let saga = CardDefinitionBuilder::new(CardId::new(), "Labeled Saga")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "I, II, III — Pain — You draw a card and you lose 1 life.\nIV — Oblivion — Each opponent sacrifices a creature of their choice and loses 3 life.",
        )
        .expect("typed Saga chapter presentation labels should parse");
    let saga_debug = format!("{saga:#?}");
    assert!(saga_debug.contains("SagaChapter"), "{saga_debug}");
    assert!(saga_debug.contains("Draw"), "{saga_debug}");
    assert!(saga_debug.contains("Sacrifice"), "{saga_debug}");

    let villainous = CardDefinitionBuilder::new(CardId::new(), "Villainous Choice Trigger")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your end step, draw a card. Then each opponent faces a villainous choice — That player discards a card, or you may put a Construct, Robot, or Vehicle card from your hand onto the battlefield.",
        )
        .expect("typed each-opponent villainous-choice separator should parse");
    let villainous_debug = format!("{villainous:#?}");
    assert!(
        villainous_debug.contains("VillainousChoiceEffect")
            && villainous_debug.contains("IteratedPlayer"),
        "{villainous_debug}"
    );

    let cycling = CardDefinitionBuilder::new(CardId::new(), "Nonmana Cycling")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Cycling—Sacrifice a land. (Sacrifice a land, Discard this card: Draw a card.)")
        .expect("typed nonmana cycling cost should parse");
    let cycling_debug = format!("{cycling:#?}");
    assert!(cycling_debug.contains("SacrificeEffect"), "{cycling_debug}");
    assert!(cycling_debug.contains("Cycle"), "{cycling_debug}");

    let quoted_token_rule =
        CardDefinitionBuilder::new(CardId::new(), "Quoted Labeled Token Rule")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, create Zabu, a legendary 2/2 green Cat creature token with \"Landfall — Whenever a land you control enters, put a +1/+1 counter on Zabu.\"",
        )
        .expect("a dash inside quoted token rules must not be classified as a leading label");
    let quoted_token_debug = format!("{quoted_token_rule:#?}");
    assert!(
        quoted_token_debug.contains("PutCountersEffect")
            && quoted_token_debug.contains("PlusOnePlusOne")
            && quoted_token_debug.contains("Land"),
        "the typed token rule must lower its land trigger and counter effect: {quoted_token_debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_preserves_non_vampire_sacrifice_filter() {
    let text = "Each player sacrifices a non-Vampire creature of their choice.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify non-Vampire sacrifice sentence");
    let effects = parse_effect_sentence_lexed(&lexed)
        .expect("lexed non-Vampire sacrifice sentence should parse");
    let debug = format!("{effects:?}");

    assert!(
        debug.contains("card_types: [Creature]"),
        "expected creature filter in parsed effect, got {debug}"
    );
    assert!(
        debug.contains("excluded_subtypes: [Vampire]"),
        "expected excluded Vampire subtype in parsed effect, got {debug}"
    );
}

#[test]
pub(super) fn parse_target_opponent_puts_from_their_graveyard_compiles() {
    let built = CardDefinitionBuilder::new(CardId::from_raw(98_501), "Target Opponent Put")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target opponent puts a creature card of their choice from their graveyard onto the battlefield under your control.",
        )
        .expect("target-opponent put-from-graveyard sentence should compile");

    let debug = format!("{built:?}");
    assert!(
        debug.contains("Target(Opponent)") || debug.contains("target_opponent"),
        "expected target-opponent ownership context in compiled card, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_supports_create_for_each_creatures_died() {
    let text = "Create a Treasure token for each creature that died this turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify create-for-each effect");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed create-for-each parser should succeed");

    let debug = format!("{native:?}");
    assert!(
        debug.contains("CreaturesDiedThisTurn"),
        "expected dynamic creature-died count in create clause, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_supports_investigate_for_each_creatures_died() {
    let text = "Investigate for each creature that died this turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify investigate-for-each effect");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed investigate-for-each parser should succeed");

    let debug = format!("{native:?}");
    assert!(
        debug.contains("CreaturesDiedThisTurn"),
        "expected dynamic creature-died count in investigate clause, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_supports_investigate_once_for_each_attacking_creature()
 {
    let text = "Investigate once for each nontoken attacking creature.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify investigate-once-for-each effect");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed investigate-once-for-each parser should succeed");

    let debug = format!("{native:?}");
    assert!(
        debug.contains("Investigate")
            && debug.contains("Count")
            && debug.contains("attacking: true")
            && debug.contains("nontoken: true"),
        "expected dynamic nontoken attacking creature count in investigate clause, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_cost_reduction_line_rejects_unmodeled_activate_if_condition() {
    let tokens = lex_line(
        "this ability costs 1 less to activate if you control an artifact.",
        0,
    )
    .expect("rewrite lexer should classify activated cost reduction");
    let err = parse_cost_reduction_line(&tokens)
        .expect_err("unmodeled activated cost reduction condition should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported activated-ability cost reduction condition"),
        "expected explicit unsupported cost reduction condition, got {message}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_keeps_permission_may_as_grant() {
    let text = "You may play it this turn without paying its mana cost.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify permission sentence");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed permission sentence parser should succeed");

    let native_debug = format!("{native:?}");
    assert!(
        !native_debug.contains("May"),
        "permission-granting may clause should not be wrapped as a May effect: {native_debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_keeps_additional_land_play_as_permission() {
    let text = "You may play an additional land this turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify land-play permission");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed land-play permission parser should succeed");

    let native_debug = format!("{native:?}");
    assert!(
        native_debug.contains("AdditionalLandPlays"),
        "expected additional land-play effect, got {native_debug}"
    );
    assert!(
        native_debug.contains("MayByPlayer") || !native_debug.contains("May"),
        "land-play permission clause should remain a typed permission: {native_debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_splits_untap_and_additional_combat_phase() {
    let text = "Untap all other creatures you control and after this phase, there is an additional combat phase.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify combat-celebrant effect");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed combat-celebrant effect should parse");

    let native_debug = format!("{native:?}");
    assert!(
        native_debug.contains("Untap") && native_debug.contains("AdditionalPhases"),
        "expected untap and additional combat effects, got {native_debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_parses_two_additional_combat_phases() {
    let text = "After this main phase, there are two additional combat phases.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify Full Throttle clause");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed Full Throttle clause should parse");

    let native_debug = format!("{native:?}");
    assert!(
        native_debug.contains("AdditionalPhases") && native_debug.matches("Combat").count() >= 2,
        "expected two additional combat phases, got {native_debug}"
    );
}

#[test]
pub(super) fn rewrite_mana_symbol_group_parser_handles_hybrid_symbols() {
    let symbols =
        parse_mana_symbol_group_rewrite("{W/U}").expect("parser should parse hybrid mana group");
    assert_eq!(symbols, vec![ManaSymbol::White, ManaSymbol::Blue]);
}

#[test]
pub(super) fn rewrite_mana_symbol_group_parser_handles_multiple_slash_separators() {
    let symbols = parse_mana_symbol_group_rewrite("{W/U/B}")
        .expect("parser should parse repeated slash-delimited mana symbols");
    assert_eq!(
        symbols,
        vec![ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black]
    );
}

#[test]
pub(super) fn rewrite_parser_root_values_entrypoints_match_grammar_outputs() {
    let root_symbols = parse_mana_symbol_group_rewrite("{W/U/B}")
        .expect("parser-root mana-group entrypoint should succeed");
    let grammar_symbols = super::super::grammar::values::parse_mana_symbol_group("{W/U/B}")
        .expect("grammar mana-group parser should succeed");
    assert_eq!(root_symbols, grammar_symbols);

    let root_mana_cost = parse_mana_cost_rewrite("{2}{W/U}{B}")
        .expect("parser-root mana-cost entrypoint should succeed");
    let grammar_mana_cost = super::super::grammar::values::parse_mana_cost_rewrite("{2}{W/U}{B}")
        .expect("grammar mana-cost parser should succeed");
    assert_eq!(root_mana_cost, grammar_mana_cost);

    let root_type_line = parse_type_line_rewrite("Legendary Creature — Elf Druid")
        .expect("parser-root type-line entrypoint should succeed");
    let grammar_type_line = super::super::grammar::values::parse_type_line_with(
        "Legendary Creature — Elf Druid",
        |word| match word {
            "Legendary" => Some(Supertype::Legendary),
            _ => None,
        },
        |word| match word {
            "Creature" => Some(CardType::Creature),
            _ => None,
        },
        |word| match word {
            "Elf" => Some(Subtype::Elf),
            "Druid" => Some(Subtype::Druid),
            _ => None,
        },
    )
    .expect("grammar type-line parser should succeed");
    assert_eq!(root_type_line.supertypes, grammar_type_line.0);
    assert_eq!(root_type_line.card_types, grammar_type_line.1);
    assert_eq!(root_type_line.subtypes, grammar_type_line.2);
}

#[test]
pub(super) fn rewrite_shared_mana_cost_parser_keeps_scryfall_and_rewrite_entrypoints_in_sync() {
    let rewrite = parse_mana_cost_rewrite("{2}{W/U}{B}")
        .expect("rewrite mana-cost entrypoint should succeed");
    let scryfall = super::super::util::parse_scryfall_mana_cost("{2}{W/U}{B}")
        .expect("scryfall mana-cost entrypoint should succeed");

    assert_eq!(rewrite, scryfall);
    assert_eq!(
        super::super::util::parse_scryfall_mana_cost("")
            .expect("blank scryfall mana cost is empty"),
        crate::mana::ManaCost::new()
    );

    let error = parse_error_message(parse_mana_cost_rewrite("—"));
    assert!(
        error.contains("mana-cost"),
        "expected rewrite mana-cost parser context, got {error}"
    );
}

#[test]
pub(super) fn rewrite_type_line_parser_handles_supertypes_types_and_subtypes() {
    let parsed = parse_type_line_rewrite("Legendary Creature — Elf Druid")
        .expect("rewrite type-line parser should succeed");
    assert_eq!(parsed.supertypes, vec![Supertype::Legendary]);
    assert_eq!(parsed.card_types, vec![CardType::Creature]);
    assert_eq!(parsed.subtypes, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
pub(super) fn rewrite_type_line_parser_recognizes_spacecraft_as_an_artifact_subtype() {
    let parsed = parse_type_line_rewrite("Artifact — Spacecraft")
        .expect("Spacecraft should be registered as an artifact subtype");
    assert_eq!(parsed.card_types, vec![CardType::Artifact]);
    assert_eq!(parsed.subtypes, vec![Subtype::Spacecraft]);
    assert!(Subtype::Spacecraft.is_artifact_subtype());
    assert!(!Subtype::Spacecraft.is_creature_type());
}

#[test]
pub(super) fn rewrite_values_type_line_parser_keeps_front_face_only() {
    let parsed = super::super::grammar::values::parse_type_line_with(
        "Legendary Creature — Elf Druid // Sorcery",
        |word| match word {
            "Legendary" => Some(Supertype::Legendary),
            _ => None,
        },
        |word| match word {
            "Creature" => Some(CardType::Creature),
            _ => None,
        },
        |word| match word {
            "Elf" => Some(Subtype::Elf),
            "Druid" => Some(Subtype::Druid),
            _ => None,
        },
    )
    .expect("direct values type-line parser should keep the front face");

    assert_eq!(parsed.0, vec![Supertype::Legendary]);
    assert_eq!(parsed.1, vec![CardType::Creature]);
    assert_eq!(parsed.2, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
pub(super) fn rewrite_shared_type_line_parser_keeps_conditionals_entrypoint_in_sync() {
    let parsed = super::super::effect_sentences::conditionals::parse_type_line(
        "Legendary Creature — Elf Druid",
    )
    .expect("shared type-line parser should support conditionals entrypoint");
    assert_eq!(parsed.0, vec![Supertype::Legendary]);
    assert_eq!(parsed.1, vec![CardType::Creature]);
    assert_eq!(parsed.2, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
pub(super) fn rewrite_shared_scryfall_mana_cost_parser_handles_grouped_and_empty_costs() {
    let parsed = super::super::util::parse_scryfall_mana_cost("{2}{W/U}{B}")
        .expect("shared mana-cost parser should parse grouped mana costs");

    assert_eq!(
        parsed.pips(),
        vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White, ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]
    );
    assert_eq!(
        super::super::util::parse_scryfall_mana_cost("—").expect("emdash should mean no mana cost"),
        crate::mana::ManaCost::new()
    );
}

#[test]
pub(super) fn rewrite_values_parse_value_prefix_trims_edge_punctuation() {
    let tokens = lex_line("\"three,\"", 0)
        .expect("rewrite lexer should classify punctuation-wrapped values");
    let (value, used) = super::super::grammar::values::parse_value_prefix_lexed(&tokens)
        .expect("direct values parser should trim edge punctuation");

    assert_eq!(value, crate::effect::Value::Fixed(3));
    assert_eq!(used, 1);
}

#[test]
pub(super) fn rewrite_mana_symbol_group_error_mentions_mana_symbol() {
    let error = parse_error_message(parse_mana_symbol_group_rewrite("{Q}"));
    assert!(
        error.contains("mana-group"),
        "expected grouped mana parser context, got {error}"
    );
    assert!(
        error.contains("mana symbol"),
        "expected mana symbol context, got {error}"
    );
}

#[test]
pub(super) fn rewrite_modal_header_parse_all_reports_cut_for_partial_choose_range() {
    use super::super::grammar::primitives::parse_all;

    let tokens = lex_line("Choose up to", 0)
        .expect("rewrite lexer should classify partial modal choose range");
    let error = parse_error_message(parse_all(
        &tokens,
        super::super::grammar::structure::parse_modal_header_choose_spec,
        "modal-header",
    ));

    assert!(
        error.contains("modal-header"),
        "expected modal-header adapter context, got {error}"
    );
    assert!(
        error.contains("modal choice range"),
        "expected choose-range cut context, got {error}"
    );
    assert!(
        error.contains("up") || error.contains("end of input"),
        "expected committed failure location, got {error}"
    );
}

#[test]
pub(super) fn rewrite_modal_header_parse_all_accepts_choose_any_number_clause() {
    use super::super::grammar::primitives::parse_all;

    let tokens = lex_line("Choose any number", 0)
        .expect("rewrite lexer should classify choose-any-number modal header");
    let parsed = parse_all(
        &tokens,
        super::super::grammar::structure::parse_modal_header_choose_spec,
        "modal-header",
    )
    .expect("choose-any-number modal header should parse");

    let choose_spec = parsed.expect("choose-any-number header should produce a choose spec");
    assert_eq!(choose_spec.min, crate::effect::Value::Fixed(0));
    assert_eq!(choose_spec.max, None);
}

#[test]
pub(super) fn rewrite_type_line_error_mentions_type_line_subtypes_after_dash() {
    let error = parse_error_message(parse_type_line_rewrite("Legendary Creature — !"));
    assert!(
        error.contains("type-line"),
        "expected type-line context, got {error}"
    );
    assert!(
        error.contains("subtype"),
        "expected subtype context after em dash, got {error}"
    );
}

#[test]
pub(super) fn rewrite_type_line_error_reports_cut_at_end_after_em_dash() {
    let error = parse_error_message(parse_type_line_rewrite("Legendary Creature —"));
    assert!(
        error.contains("type-line"),
        "expected type-line context, got {error}"
    );
    assert!(
        error.contains("subtype"),
        "expected subtype cut context after em dash, got {error}"
    );
    assert!(
        error.contains("end of input") || error.contains("line 1"),
        "expected committed end-of-input location, got {error}"
    );
}

#[test]
pub(super) fn rewrite_activation_cost_parses_sacrifice_segments() {
    let cst = parse_activation_cost_rewrite("Sacrifice a creature")
        .expect("rewrite activation-cost parser should parse sacrifice segments");
    let lowered = lower_activation_cost_cst(&cst)
        .expect("rewrite sacrifice segment should lower to TotalCost");
    assert!(!lowered.costs().is_some_and(|costs| costs.is_empty()));

    let another = parse_activation_cost_rewrite("Sacrifice another creature")
        .expect("rewrite activation-cost parser should preserve 'another creature'");
    let rendered = another
        .segments
        .iter()
        .map(|segment| format!("{segment:?}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        rendered.contains("other: true"),
        "expected rewrite sacrifice CST to preserve 'another', got {rendered}"
    );
}

#[test]
pub(super) fn rewrite_discard_cost_error_mentions_discard_segment() {
    let error = parse_error_message(parse_activation_cost_rewrite("Discard"));
    assert!(
        error.contains("discard"),
        "expected discard context, got {error}"
    );
}

#[test]
pub(super) fn rewrite_sacrifice_cost_error_mentions_missing_filter() {
    let error = parse_error_message(parse_activation_cost_rewrite("Sacrifice"));
    assert!(
        error.contains("sacrifice"),
        "expected sacrifice context, got {error}"
    );
    assert!(
        error.contains("filter"),
        "expected missing filter context, got {error}"
    );
}

#[test]
pub(super) fn rewrite_activation_cost_parses_energy_and_counter_variants() {
    let energy = parse_activation_cost_rewrite("Pay {E}{E}")
        .expect("rewrite activation-cost parser should parse energy payment");
    let bare_energy = parse_activation_cost_rewrite("{E}{E}")
        .expect("rewrite activation-cost parser should parse bare energy payment");
    let counter_add = parse_activation_cost_rewrite("Put a +1/+1 counter on this creature")
        .expect("parser should parse add-counter cost");
    let counter_remove = parse_activation_cost_rewrite("Remove a +1/+1 counter from this creature")
        .expect("parser should parse remove-counter cost");
    let counter_remove_unspecified =
        parse_activation_cost_rewrite("Remove a counter from this creature")
            .expect("parser should parse unspecified remove-counter cost");
    let exile_hand = parse_activation_cost_rewrite("Exile a blue card from your hand")
        .expect("parser should parse exile-from-hand cost");
    let reveal_source = parse_activation_cost_rewrite("Reveal this card from your hand")
        .expect("parser should parse reveal-source-from-hand cost");
    let reveal_typed_source = parse_activation_cost_rewrite("Reveal this creature from your hand")
        .expect("parser should parse typed reveal-source-from-hand cost");

    assert!(matches!(
        energy.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Energy(2)]
    ));
    assert!(matches!(
        bare_energy.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Energy(2)]
    ));
    assert!(matches!(
        counter_add.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::PutCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: 1
            }
        ]
    ));
    assert!(matches!(
        counter_remove.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::RemoveCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: 1
            }
        ]
    ));
    assert!(matches!(
        counter_remove_unspecified.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type: None,
            count: 1,
            filter,
            display_x: false,
            dynamic: false,
            single_object: true,
        }] if filter.source
    ));
    assert!(matches!(
        exile_hand.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::ExileFromHand {
            count: 1,
            color_filter: Some(colors)
        }] if *colors == crate::color::ColorSet::BLUE
    ));
    assert!(matches!(
        reveal_source.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::RevealSourceFromHand]
    ));
    assert!(matches!(
        reveal_typed_source.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::RevealSourceFromHand]
    ));

    let reveal_x_green = parse_activation_cost_rewrite("Reveal X green cards from your hand")
        .expect("parser should parse reveal-X-color-from-hand costs");
    assert!(matches!(
        reveal_x_green.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::RevealFromHand {
            count: Value::X,
            color_filter: Some(colors),
            card_type: None,
        }] if *colors == ColorSet::GREEN
    ));
}

#[test]
pub(super) fn rewrite_activation_cost_parses_pay_mana_life_exert_and_bare_symbols() {
    let pay_life = parse_activation_cost_rewrite("Pay 3 life")
        .expect("rewrite activation-cost parser should parse life payment");
    let pay_mana = parse_activation_cost_rewrite("Pay {W}{U}")
        .expect("rewrite activation-cost parser should parse mana payment");
    let bare_mana = parse_activation_cost_rewrite("{W}{U}")
        .expect("rewrite activation-cost parser should parse bare mana payment");
    let tap = parse_activation_cost_rewrite("{T}")
        .expect("rewrite activation-cost parser should parse tap symbol");
    let untap = parse_activation_cost_rewrite("{Q}")
        .expect("rewrite activation-cost parser should parse untap symbol");
    let exert = parse_activation_cost_rewrite("Exert this creature")
        .expect("rewrite activation-cost parser should parse exert costs");

    assert!(matches!(
        pay_life.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Life(Value::Fixed(3))]
    ));
    match pay_mana.segments.as_slice() {
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Mana(cost)] => assert_eq!(
            cost.pips(),
            vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]
        ),
        other => panic!("expected mana payment, got {other:?}"),
    }
    match bare_mana.segments.as_slice() {
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Mana(cost)] => assert_eq!(
            cost.pips(),
            vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]
        ),
        other => panic!("expected bare mana payment, got {other:?}"),
    }
    assert!(matches!(
        tap.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Tap]
    ));
    assert!(matches!(
        untap.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Untap]
    ));
    assert!(matches!(
        exert.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::ExertSelf { display_text }]
            if display_text == "Exert this creature"
    ));
}

#[test]
pub(super) fn rewrite_activation_cost_parses_loyalty_shorthand_without_fallback_escape_hatch() {
    let plus = parse_activation_cost_rewrite("+1")
        .expect("rewrite activation-cost parser should parse +1 loyalty shorthand");
    let minus = parse_activation_cost_rewrite("-2")
        .expect("rewrite activation-cost parser should parse -2 loyalty shorthand");
    let minus_x = parse_activation_cost_rewrite("-X")
        .expect("rewrite activation-cost parser should parse -X loyalty shorthand");
    let zero = parse_activation_cost_rewrite("0")
        .expect("rewrite activation-cost parser should parse zero loyalty shorthand");

    assert!(matches!(
        plus.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::PutCounters {
                counter_type: CounterType::Loyalty,
                count: 1
            }
        ]
    ));
    assert!(matches!(
        minus.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::RemoveCounters {
                counter_type: CounterType::Loyalty,
                count: 2
            }
        ]
    ));
    assert!(matches!(
        minus_x.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type: Some(CounterType::Loyalty),
                display_x: true,
                ..
            }
        ]
    ));
    assert!(
        zero.segments.is_empty(),
        "zero loyalty shorthand should lower as a free cost"
    );
    assert!(
        lower_activation_cost_cst(&zero)
            .expect("zero loyalty shorthand should lower")
            .costs()
            .is_some_and(|costs| costs.is_empty())
    );
}

#[test]
pub(super) fn rewrite_activation_cost_preserves_shard_style_full_cost_branches() {
    let raw = parse_activation_cost_rewrite("{W}, {T} or {U}, {T}")
        .expect("rewrite activation-cost parser should parse shard-style costs");
    let tokens = lex_line("{W}, {T} or {U}, {T}", 0)
        .expect("lexer should classify shard-style activation cost");
    let lexed = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should parse shard-style costs");

    assert_eq!(format!("{raw:?}"), format!("{lexed:?}"));
    assert!(lexed.segments.is_empty());
    assert_eq!(lexed.alternative_branches.len(), 2);
    for (branch, symbol) in lexed
        .alternative_branches
        .iter()
        .zip([ManaSymbol::White, ManaSymbol::Blue])
    {
        match branch.segments.as_slice() {
            [
                crate::grammar::activation_costs::ActivationCostSegmentCst::Mana(cost),
                crate::grammar::activation_costs::ActivationCostSegmentCst::Tap,
            ] => assert_eq!(cost.pips(), vec![vec![symbol]]),
            other => panic!("expected mana plus tap alternative branch, got {other:?}"),
        }
    }
}

#[test]
pub(super) fn rewrite_activation_cost_preserves_alternative_and_dynamic_life_branches() {
    let alternative = parse_activation_cost_rewrite("Pay {3} or discard a card")
        .expect("activation-cost grammar should preserve alternative payments");
    assert!(alternative.segments.is_empty());
    assert_eq!(alternative.alternative_branches.len(), 2);
    let lowered = lower_activation_cost_cst(&alternative)
        .expect("alternative activation cost should lower recursively");
    assert_eq!(
        lowered.relationship,
        crate::model::CostRelationship::Alternative
    );
    let lowered_core = lowered.to_core_total_cost();
    let branches = lowered_core
        .as_one_of()
        .expect("alternative activation cost should lower to TotalCost::OneOf");
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].display(), "{3}");
    assert!(
        branches[1]
            .display()
            .to_ascii_lowercase()
            .contains("discard")
    );

    let dynamic = parse_activation_cost_rewrite("Pay 1 life for each card in your hand")
        .expect("activation-cost grammar should parse dynamic life payments");
    assert!(matches!(
        dynamic.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::Life(Value::CardsInHand(
                crate::target::PlayerFilter::You
            ))
        ]
    ));
}

#[test]
pub(super) fn rewrite_activation_cost_token_entrypoint_parses_pay_bare_symbol_and_exert_variants() {
    let pay_energy_tokens =
        lex_line("Pay two {E}", 0).expect("lexer should classify counted-energy activation cost");
    let pay_energy_cst = parse_activation_cost_tokens_rewrite(&pay_energy_tokens)
        .expect("token activation-cost parser should parse counted-energy costs");
    assert!(matches!(
        pay_energy_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Energy(2)]
    ));

    let pay_mana_tokens =
        lex_line("Pay {W}{U}", 0).expect("lexer should classify mana-payment activation cost");
    let pay_mana_cst = parse_activation_cost_tokens_rewrite(&pay_mana_tokens)
        .expect("token activation-cost parser should parse mana-payment costs");
    match pay_mana_cst.segments.as_slice() {
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Mana(cost)] => assert_eq!(
            cost.pips(),
            vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]
        ),
        other => panic!("expected mana payment, got {other:?}"),
    }

    let tap_tokens = lex_line("{T}", 0).expect("lexer should classify tap-symbol activation cost");
    let tap_cst = parse_activation_cost_tokens_rewrite(&tap_tokens)
        .expect("token activation-cost parser should parse tap-symbol costs");
    assert!(matches!(
        tap_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Tap]
    ));

    let untap_tokens =
        lex_line("{Q}", 0).expect("lexer should classify untap-symbol activation cost");
    let untap_cst = parse_activation_cost_tokens_rewrite(&untap_tokens)
        .expect("token activation-cost parser should parse untap-symbol costs");
    assert!(matches!(
        untap_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Untap]
    ));

    let exert_tokens =
        lex_line("Exert this creature", 0).expect("lexer should classify exert activation cost");
    let exert_cst = parse_activation_cost_tokens_rewrite(&exert_tokens)
        .expect("token activation-cost parser should parse exert costs");
    assert!(matches!(
        exert_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::ExertSelf { display_text }]
            if display_text == "Exert this creature"
    ));
}

#[test]
pub(super) fn rewrite_activation_cost_token_entrypoint_preserves_named_card_commas() {
    let tokens = lex_line("Discard a card named Mishra, Lost to Phyrexia", 0)
        .expect("lexer should preserve punctuation in named-card costs");
    let cst = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should keep named-card commas intact");

    assert!(matches!(
        cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::DiscardFiltered {
            name: Some(name),
            ..
        }] if name == "mishra, lost to phyrexia"
    ));
}

#[test]
pub(super) fn rewrite_activation_cost_string_entrypoint_matches_named_card_token_path() {
    let raw = parse_activation_cost_rewrite("Discard a card named Mishra, Lost to Phyrexia")
        .expect("string activation-cost parser should preserve named-card punctuation");
    let tokens = lex_line("Discard a card named Mishra, Lost to Phyrexia", 0)
        .expect("lexer should classify named-card activation cost");
    let lexed = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should preserve named-card punctuation");

    assert_eq!(format!("{raw:?}"), format!("{lexed:?}"));
}

#[test]
pub(super) fn rewrite_activation_cost_preserves_named_card_and_followup_segments() {
    let cost = parse_activation_cost_rewrite(
        "Discard another card named Skoa, Embermage, Sacrifice two Mountains",
    )
    .expect("activation-cost parser should split named discard from follow-up sacrifice");

    match cost.segments.as_slice() {
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::DiscardFiltered {
                name: Some(name),
                other,
                ..
            },
            crate::grammar::activation_costs::ActivationCostSegmentCst::SacrificeChosen {
                count,
                filter,
                ..
            },
        ] => {
            assert_eq!(name, "skoa, embermage");
            assert!(*other, "expected 'another' modifier to be preserved");
            assert_eq!(*count, crate::effect::ChoiceCount::exactly(2));
            assert_eq!(filter.subtypes, vec![Subtype::Mountain]);
        }
        other => panic!("unexpected activation cost segments: {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_activation_cost_parser_handles_exile_self_and_named_artifacts() {
    let cost = parse_activation_cost_rewrite(
        "Exile The Book of Vile Darkness and artifacts you control named Eye of Vecna and Hand of Vecna",
    )
    .expect("activation-cost parser should keep named artifact exile costs distinct");

    let debug = format!("{cost:?}");
    assert!(debug.contains("ExileSelfAndNamedArtifacts"), "{debug}");
    assert!(debug.contains("eye of vecna"), "{debug}");
    assert!(debug.contains("hand of vecna"), "{debug}");
}

#[test]
pub(super) fn rewrite_activation_cost_preserves_top_only_graveyard_selection_through_lowering() {
    let tokens = lex_line("Exile the top creature card of your graveyard", 0)
        .expect("lexer should classify ordered graveyard exile cost");
    let cst = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("activation-cost parser should preserve the ordered source");
    assert!(matches!(
        cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter,
            top_only: true,
            turn_face_up: false,
        }] if *choice_count == ChoiceCount::exactly(1)
            && filter.zone == Some(Zone::Graveyard)
            && filter.card_types == [CardType::Creature]
    ));

    let lowered = crate::activation_and_restrictions::parse_activation_cost(&tokens)
        .expect("ordered graveyard activation cost should lower");
    let [
        crate::model::CompilerCost::ExileChosen {
            filter, top_only, ..
        },
    ] = lowered
        .as_all()
        .expect("ordered graveyard activation cost should be sequential")
    else {
        panic!("expected one typed exile choice cost: {lowered:#?}");
    };
    assert!(*top_only);
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
}

#[test]
pub(super) fn rewrite_ordered_graveyard_direct_actions_lower_to_linked_top_only_choices() {
    let barrow = CardDefinitionBuilder::new(CardId::new(), "Barrow Ghoul")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.",
        )
        .expect("Barrow Ghoul ordered graveyard action should parse");
    let barrow_debug = format!("{barrow:#?}");
    assert!(
        barrow_debug.contains("ChooseObjectsEffect"),
        "{barrow_debug}"
    );
    assert!(barrow_debug.contains("top_only: true"), "{barrow_debug}");
    assert!(barrow_debug.contains("ExileEffect"), "{barrow_debug}");

    let digger = CardDefinitionBuilder::new(CardId::new(), "Soldevi Digger")
        .parse_text("{2}: Put the top card of your graveyard on the bottom of your library.")
        .expect("Soldevi Digger ordered graveyard move should parse");
    let digger_debug = format!("{digger:#?}");
    assert!(
        digger_debug.contains("ChooseObjectsEffect"),
        "{digger_debug}"
    );
    assert!(digger_debug.contains("top_only: true"), "{digger_debug}");
    assert!(digger_debug.contains("MoveToZoneEffect"), "{digger_debug}");
    assert!(digger_debug.contains("zone: Library"), "{digger_debug}");
    assert!(digger_debug.contains("to_top: false"), "{digger_debug}");
}

#[test]
pub(super) fn rewrite_activation_cost_token_entrypoint_parses_tap_return_and_exile_variants() {
    let tap_tokens = lex_line("Tap another untapped creature you control", 0)
        .expect("lexer should classify tap-chosen activation cost");
    let tap_cst = parse_activation_cost_tokens_rewrite(&tap_tokens)
        .expect("token activation-cost parser should parse tap-chosen costs");
    assert!(matches!(
        tap_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::TapChosen {
            count: 1,
            filter,
        }] if filter.card_types == [CardType::Creature]
            && filter.controller == Some(crate::target::PlayerFilter::You)
            && filter.untapped
            && filter.other
    ));

    let return_tokens = lex_line("Return a creature you control to its owner's hand", 0)
        .expect("lexer should classify return-to-hand activation cost");
    let return_cst = parse_activation_cost_tokens_rewrite(&return_tokens)
        .expect("token activation-cost parser should parse return-to-hand costs");
    assert!(matches!(
        return_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::ReturnChosenToHand { count: 1, filter }]
            if filter.card_types == [CardType::Creature]
                && filter.controller == Some(crate::target::PlayerFilter::You)
    ));

    let exile_tokens = lex_line("Exile one or more cards from your graveyard", 0)
        .expect("lexer should classify exile-from-graveyard activation cost");
    let exile_cst = parse_activation_cost_tokens_rewrite(&exile_tokens)
        .expect("token activation-cost parser should parse exile-from-graveyard costs");
    assert!(matches!(
        exile_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter,
            top_only: false,
            turn_face_up: false,
        }] if *choice_count == ChoiceCount::at_least(1)
            && filter.zone == Some(Zone::Graveyard)
            && filter.owner == Some(crate::target::PlayerFilter::You)
    ));

    let single_graveyard_tokens = lex_line("Exile a card from a single graveyard", 0)
        .expect("lexer should classify exile-from-single-graveyard activation cost");
    let single_graveyard_cst = parse_activation_cost_tokens_rewrite(&single_graveyard_tokens)
        .expect("token activation-cost parser should parse single-graveyard exile costs");
    assert!(matches!(
        single_graveyard_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter,
            top_only: false,
            turn_face_up: false,
        }] if *choice_count == ChoiceCount::exactly(1)
            && filter.zone == Some(Zone::Graveyard)
            && filter.single_graveyard
    ));

    let exile_hand_tokens = lex_line("Exile a nonland card from your hand", 0)
        .expect("lexer should classify exile-from-hand activation cost");
    let lowered = crate::activation_and_restrictions::parse_activation_cost(&exile_hand_tokens)
        .expect("activation-cost parser should support exiling a filtered card from hand");
    let [crate::model::CompilerCost::ExileChosen { filter, .. }] = lowered
        .as_all()
        .expect("exile-from-hand activation cost should lower to sequential costs")
    else {
        panic!("expected one typed hand exile choice: {lowered:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(filter.owner, Some(crate::target::PlayerFilter::You));
    assert_eq!(filter.excluded_card_types, vec![CardType::Land]);

    let exile_spell_tokens = lex_line("Exile an instant or sorcery spell you control", 0)
        .expect("lexer should classify exile-spell activation cost");
    let lowered = crate::activation_and_restrictions::parse_activation_cost(&exile_spell_tokens)
        .expect("activation-cost parser should support exiling a controlled spell");
    let lowered_debug = format!("{lowered:#?}");
    let lowered_debug_compact = lowered_debug.split_whitespace().collect::<String>();
    assert!(
        lowered_debug_compact.contains("zone:Some(Stack"),
        "expected exile-spell cost to choose from the stack, got {lowered_debug}"
    );
    assert!(
        lowered_debug_compact.contains("stack_kind:Some(Spell"),
        "expected exile-spell cost to require a spell stack object, got {lowered_debug}"
    );
    assert!(
        !lowered_debug_compact.contains("zone:Some(Battlefield"),
        "exile-spell costs must not target battlefield objects, got {lowered_debug}"
    );

    let top_library_tokens = lex_line("Exile the top two cards of your library", 0)
        .expect("lexer should classify exile-top-library activation cost");
    let top_library_cst = parse_activation_cost_tokens_rewrite(&top_library_tokens)
        .expect("token activation-cost parser should parse exile-top-library costs");
    assert!(matches!(
        top_library_cst.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::ExileTopLibrary {
                count: 2
            }
        ]
    ));
}

#[test]
pub(super) fn rewrite_activation_cost_token_entrypoint_parses_counter_variants() {
    let put_tokens = lex_line("Put a +1/+1 counter on a creature you control", 0)
        .expect("lexer should classify put-counter activation cost");
    let put_cst = parse_activation_cost_tokens_rewrite(&put_tokens)
        .expect("token activation-cost parser should parse put-counter costs");
    assert!(matches!(
        put_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::PutCountersChosen {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1,
            filter,
        }] if filter.card_types == [CardType::Creature]
            && filter.controller == Some(crate::target::PlayerFilter::You)
    ));
    let put_lowered = format!(
        "{:#?}",
        lower_activation_cost_cst(&put_cst).expect("chosen counter-placement cost should lower")
    );
    assert!(
        put_lowered.contains("PutCounters")
            && put_lowered.contains("ObjectFilter")
            && !put_lowered.contains("target: Source"),
        "chosen counter-placement cost must not collapse onto the source: {put_lowered}"
    );

    let singular_remove_tokens = lex_line(
        "Remove X counters from an artifact or creature you control",
        0,
    )
    .expect("lexer should classify singular dynamic remove-counter cost");
    let singular_remove_cst = parse_activation_cost_tokens_rewrite(&singular_remove_tokens)
        .expect("singular dynamic counter removal should parse");
    let singular_remove_lowered = format!(
        "{:#?}",
        lower_activation_cost_cst(&singular_remove_cst)
            .expect("singular dynamic counter removal should lower")
    );
    assert!(
        singular_remove_lowered.contains("RemoveCounters")
            && singular_remove_lowered.contains("single_object: true")
            && singular_remove_lowered.contains("Artifact")
            && singular_remove_lowered.contains("Creature")
            && singular_remove_lowered.contains("filter: Some"),
        "singular counter removal must preserve its chosen-object filter: {singular_remove_lowered}"
    );

    let processor_tokens = lex_line(
        "Put a card an opponent owns from exile into that player's graveyard",
        0,
    )
    .expect("lexer should classify processor activation cost");
    let processor_cst = parse_activation_cost_tokens_rewrite(&processor_tokens)
        .expect("token activation-cost parser should parse processor costs");
    assert!(matches!(
        processor_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::MoveOpponentOwnedExiledCardToGraveyard]
    ));

    let remove_tokens = lex_line(
        "Remove any number of charge counters from among artifacts you control",
        0,
    )
    .expect("lexer should classify remove-counter activation cost");
    let remove_cst = parse_activation_cost_tokens_rewrite(&remove_tokens)
        .expect("token activation-cost parser should parse remove-counter costs");
    assert!(matches!(
        remove_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type: Some(CounterType::Charge),
            count: 0,
            filter,
            display_x: false,
            dynamic: true,
            single_object: false,
        }] if filter.card_types == [CardType::Artifact]
            && filter.controller == Some(crate::target::PlayerFilter::You)
    ));

    let one_or_more_tokens = lex_line(
        "Remove one or more +1/+1 counters from among creatures you control",
        0,
    )
    .expect("lexer should classify one-or-more remove-counter activation cost");
    let one_or_more_cst = parse_activation_cost_tokens_rewrite(&one_or_more_tokens)
        .expect("token activation-cost parser should parse one-or-more remove-counter costs");
    assert!(matches!(
        one_or_more_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type: Some(CounterType::PlusOnePlusOne),
            count: 1,
            filter,
            display_x: false,
            dynamic: true,
            single_object: false,
        }] if filter.card_types == [CardType::Creature]
            && filter.controller == Some(crate::target::PlayerFilter::You)
    ));
}

#[test]
pub(super) fn rewrite_activation_cost_parser_keeps_among_list_with_commas_in_one_segment() {
    let tokens = lex_line(
        "Remove three counters from among other artifacts, creatures, and planeswalkers you control",
        0,
    )
    .expect("lexer should classify Tekuthal remove-counter activation cost");
    let cst = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("activation-cost parser should keep among-list as one segment");
    let [
        crate::grammar::activation_costs::ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type: None,
            count: 3,
            filter,
            display_x: false,
            dynamic: false,
            single_object: false,
        },
    ] = cst.segments.as_slice()
    else {
        panic!("unexpected segments: {:?}", cst.segments);
    };
    assert_eq!(
        filter.card_types,
        vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Planeswalker
        ]
    );
    assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
    assert!(filter.other);
}

#[test]
pub(super) fn rewrite_activation_cost_shared_parser_supports_behold_costs() {
    let cst = parse_activation_cost_rewrite("Behold an Elemental")
        .expect("shared activation-cost parser should support behold costs");
    assert!(matches!(
        cst.segments.as_slice(),
        [
            crate::grammar::activation_costs::ActivationCostSegmentCst::Behold {
                subtype: Subtype::Elemental,
                count: 1
            }
        ]
    ));

    let tokens =
        lex_line("Behold an Elemental", 0).expect("lexer should classify behold activation cost");
    let lowered = crate::activation_and_restrictions::parse_activation_cost(&tokens)
        .expect("activated ability entrypoint should use shared behold cost parser");
    assert!(
        !lowered.is_free(),
        "behold costs should survive lowering as a non-free activation cost"
    );
}

#[test]
pub(super) fn rewrite_activation_cost_shared_parser_supports_blight_costs() {
    let cst = parse_activation_cost_rewrite("Blight 1")
        .expect("shared activation-cost parser should support blight costs");
    assert!(matches!(
        cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Blight { count: 1 }]
    ));

    let tokens = lex_line("Blight 1", 0).expect("lexer should classify blight activation cost");
    let lowered = crate::activation_and_restrictions::parse_activation_cost(&tokens)
        .expect("activated ability entrypoint should use shared blight cost parser");
    assert!(
        !lowered.is_free(),
        "blight costs should survive lowering as a non-free activation cost"
    );
    assert_eq!(
        lowered.costs().len(),
        1,
        "blight cost should remain one compiler-owned cost component"
    );
    let lowered_raw = format!("{lowered:#?}");
    assert!(
        lowered_raw.contains("Blight") && lowered_raw.contains("count: 1"),
        "blight cost should preserve its typed count until lowering, got {lowered_raw}"
    );
}

#[test]
pub(super) fn rewrite_activation_cost_shared_parser_supports_mill_costs() {
    let cst = parse_activation_cost_rewrite("Mill two cards")
        .expect("shared activation-cost parser should support mill costs");
    assert!(matches!(
        cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Mill(2)]
    ));

    let tokens = lex_line("Mill two cards", 0).expect("lexer should classify mill activation cost");
    let token_cst = parse_activation_cost_tokens_rewrite(&tokens)
        .expect("token activation-cost parser should support mill costs");
    assert!(matches!(
        token_cst.segments.as_slice(),
        [crate::grammar::activation_costs::ActivationCostSegmentCst::Mill(2)]
    ));

    let lowered = crate::activation_and_restrictions::parse_activation_cost(&tokens)
        .expect("activated ability entrypoint should use shared mill cost parser");
    assert!(
        !lowered.is_free(),
        "mill costs should survive lowering as a non-free activation cost"
    );
}

#[test]
pub(super) fn rewrite_lowered_simple_card_parses() -> Result<(), CardTextError> {
    let text = "Type: Creature — Spirit\n{1}: This creature gets +1/+1 until end of turn.";
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Spirit");
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;
    assert_eq!(definition.abilities.len(), 1);
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_nonattacking_nonblocking_target_pump_keeps_target()
-> Result<(), CardTextError> {
    let lexed = lex_line(
        "Target nonattacking, nonblocking creature gets +0/+2 until end of turn.",
        0,
    )
    .expect("target pump should lex");
    let parsed_sentence =
        parse_effect_sentence_lexed(&lexed).expect("target pump sentence should parse");
    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::Pump {
                        target: sentence_target,
                        ..
                    },
                ..
            },
        ),
    ] = parsed_sentence.as_slice()
    else {
        panic!("expected target pump sentence, got {parsed_sentence:#?}");
    };
    let crate::cards::builders::TargetAst::Object(filter, target_span, _) = sentence_target else {
        panic!("expected target object, got {sentence_target:#?}");
    };
    assert!(target_span.is_some(), "expected explicit target span");
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(
        filter.nonattacking,
        "expected nonattacking target: {filter:?}"
    );
    assert!(
        filter.nonblocking,
        "expected nonblocking target: {filter:?}"
    );

    let builder = CardDefinitionBuilder::new(CardId::new(), "Unlikely Alliance")
        .card_types(vec![CardType::Enchantment]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{1}{W}: Target nonattacking, nonblocking creature gets +0/+2 until end of turn."
            .to_string(),
        false,
    )?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        panic!("expected activated ability, got {ability:?}");
    };

    assert!(!activated.choices.is_empty(), "{activated:#?}");
    let crate::target::ChooseSpec::Target(target) = &activated.choices[0] else {
        panic!("expected target choice, got {:#?}", activated.choices);
    };
    let crate::target::ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("expected object target choice, got {target:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(
        filter.nonattacking,
        "expected nonattacking target: {filter:?}"
    );
    assert!(
        filter.nonblocking,
        "expected nonblocking target: {filter:?}"
    );

    let effects = &activated.effects.segments[0].default_effects;
    let apply = effects
        .iter()
        .find_map(|effect| {
            effect.as_apply_continuous().or_else(|| {
                effect
                    .as_tagged()
                    .and_then(|tagged| tagged.effect.as_apply_continuous())
            })
        })
        .filter(|apply| {
            activated
                .choices
                .iter()
                .any(|choice| apply.target_spec.as_ref() == Some(choice))
        })
        .expect("expected continuous pump effect");
    assert_ne!(apply.target_spec, Some(crate::target::ChooseSpec::Source));
    assert!(
        activated
            .choices
            .iter()
            .any(|choice| apply.target_spec.as_ref() == Some(choice))
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_hyphenated_broad_pump_subjects_stay_filter_targets()
-> Result<(), CardTextError> {
    let lexed = lex_line("Non-Elf creatures get -2/-2 until end of turn.", 0)
        .expect("non-Elf pump should lex");
    let parsed_sentence =
        parse_effect_sentence_lexed(&lexed).expect("non-Elf pump sentence should parse");
    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::PumpAll {
                        filter,
                        power,
                        toughness,
                        duration,
                        ..
                    },
                ..
            },
        ),
    ] = parsed_sentence.as_slice()
    else {
        panic!("expected non-Elf broad pump, got {parsed_sentence:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.excluded_subtypes, vec![Subtype::Elf]);
    assert_eq!(*power, Value::Fixed(-2));
    assert_eq!(*toughness, Value::Fixed(-2));
    assert_eq!(*duration, crate::effect::Until::EndOfTurn);

    let lexed = lex_line("All non-Zombie creatures get -1/-1 until end of turn.", 0)
        .expect("non-Zombie pump should lex");
    let parsed_sentence =
        parse_effect_sentence_lexed(&lexed).expect("non-Zombie pump sentence should parse");
    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::PumpAll {
                        filter,
                        power,
                        toughness,
                        duration,
                        ..
                    },
                ..
            },
        ),
    ] = parsed_sentence.as_slice()
    else {
        panic!("expected non-Zombie broad pump, got {parsed_sentence:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.excluded_subtypes, vec![Subtype::Zombie]);
    assert_eq!(*power, Value::Fixed(-1));
    assert_eq!(*toughness, Value::Fixed(-1));
    assert_eq!(*duration, crate::effect::Until::EndOfTurn);
    Ok(())
}

#[test]
pub(super) fn rewrite_negated_chosen_type_pump_subject_uses_exclusion_filter()
-> Result<(), CardTextError> {
    let lexed = lex_line(
        "Creatures that aren't of the chosen type get -3/-3 until end of turn.",
        0,
    )
    .expect("negated chosen-type pump should lex");
    let parsed_sentence = parse_effect_sentence_lexed(&lexed)
        .expect("negated chosen-type pump sentence should parse");
    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::PumpAll {
                        filter,
                        power,
                        toughness,
                        duration,
                        ..
                    },
                ..
            },
        ),
    ] = parsed_sentence.as_slice()
    else {
        panic!("expected chosen-type exclusion broad pump, got {parsed_sentence:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(!filter.chosen_creature_type);
    assert!(filter.excluded_chosen_creature_type);
    assert_eq!(*power, Value::Fixed(-3));
    assert_eq!(*toughness, Value::Fixed(-3));
    assert_eq!(*duration, crate::effect::Until::EndOfTurn);
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_target_pump_with_duration_prefix_keeps_target()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Stegron Test")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{1}{R}, Discard this card: Until end of turn, target creature you control gets +3/+1 and becomes a Dinosaur in addition to its other types."
            .to_string(),
        false,
    )?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        panic!("expected activated ability, got {ability:?}");
    };

    assert!(!activated.choices.is_empty(), "{activated:#?}");
    let effects = &activated.effects.segments[0].default_effects;
    let apply = effects
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::ApplyContinuousEffect>(effect)
        })
        .filter(|apply| {
            activated
                .choices
                .iter()
                .any(|choice| apply.target_spec.as_ref() == Some(choice))
        })
        .expect("expected continuous pump effect");
    assert_ne!(apply.target_spec, Some(crate::target::ChooseSpec::Source));
    assert!(
        activated
            .choices
            .iter()
            .any(|choice| apply.target_spec.as_ref() == Some(choice))
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_mana_ability_preserves_fixed_mana_groups() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Ring")
        .card_types(vec![CardType::Artifact]);
    let (definition, _) =
        parse_text_with_annotations_lowered(builder, "{T}: Add {C}{C}.".to_string(), false)?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");

    match &ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(activated.is_mana_ability());
            assert_eq!(
                activated.mana_symbols(),
                &[ManaSymbol::Colorless, ManaSymbol::Colorless]
            );
        }
        other => panic!("expected activated mana ability, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_nested_mana_effect_marks_activated_mana_ability()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Selvala, Explorer Returned")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{T}: Each player reveals the top card of their library. For each nonland card revealed this way, add {G} and you gain 1 life. Then each player draws a card."
            .to_string(),
        false,
    )?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");

    match &ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(activated.produces_mana());
            assert!(activated.is_mana_ability());
            assert!(
                activated.mana_symbols().is_empty(),
                "nested mana production should resolve through its effect payload"
            );
            let debug = format!("{:#?}", activated.effects);
            assert!(debug.contains("ForPlayersEffect"), "{debug}");
            assert!(
                debug.contains("ForEachObject") || debug.contains("RepeatEffectsEffect"),
                "{debug}"
            );
            assert!(debug.contains("AddManaEffect"), "{debug}");
            assert!(debug.contains("AddManaEffect"), "{debug}");
            assert!(debug.contains("GainLifeEffect"), "{debug}");
        }
        other => panic!("expected activated mana ability, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_for_each_players_life_total_becomes_clause()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shaman Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{9}{G}{G}, {T}: Each player's life total becomes the number of creatures they control."
            .to_string(),
        false,
    )?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");

    let debug = format!("{:#?}", ability);
    assert!(
        debug.contains("SetLifeTotalEffect"),
        "expected set-life-total effect for each player, got {debug}"
    );
    assert!(
        debug.contains("IteratedPlayer"),
        "expected iterated player lowering for each player clause, got {debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_targeted_mana_production_is_not_a_mana_ability()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Font")
        .card_types(vec![CardType::Artifact]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{T}: Target player adds {G}.".to_string(),
        false,
    )?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");

    match &ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(activated.produces_mana());
            assert!(!activated.is_mana_ability());
            assert!(
                activated
                    .choices
                    .iter()
                    .any(crate::target::ChooseSpec::is_target)
            );
            assert!(
                activated.mana_symbols().is_empty(),
                "targeted mana production should resolve through its effect payload"
            );
        }
        other => panic!("expected activated ability, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_zero_loyalty_mana_production_is_not_a_mana_ability()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Walker")
        .card_types(vec![CardType::Planeswalker]);
    let (definition, _) =
        parse_text_with_annotations_lowered(builder, "0: Add {G}.".to_string(), false)?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");

    match &ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(activated.produces_mana());
            assert!(activated.is_loyalty_ability());
            assert!(!activated.is_mana_ability());
        }
        other => panic!("expected activated loyalty ability, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_merges_multiline_spell_when_you_do_followup()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Followup Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Sacrifice a creature.\nWhen you do, draw two cards.".to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.as_slice(),
        [RewriteSemanticItem::ParsedLine(_)]
    ));
    Ok(())
}

#[test]
pub(super) fn rewrite_when_one_or_more_this_way_prefix_only_rewrites_when_this_way_is_in_clause_prefix()
 {
    let tokens = lex_line(
        "Whenever one or more cards are exiled this way, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify when-one-or-more this-way follow-up");
    let rewritten =
        super::super::effect_sentences::rewrite_when_one_or_more_this_way_clause_prefix(&tokens);

    let words = token_word_refs(&rewritten);
    assert_eq!(words[..3], ["if", "you", "do"]);
}

#[test]
pub(super) fn rewrite_when_one_or_more_this_way_prefix_skips_tail_only_this_way_references() {
    let tokens = lex_line(
        "Whenever one or more Zombies you control deal combat damage to one or more of your opponents, you may draw cards equal to the number of opponents dealt damage this way.",
        0,
    )
    .expect("rewrite lexer should classify Hordewing Skaab trigger sentence");
    let rewritten =
        super::super::effect_sentences::rewrite_when_one_or_more_this_way_clause_prefix(&tokens);

    let words = token_word_refs(&rewritten);
    assert!(words[0].eq_ignore_ascii_case("whenever"));
    assert_ne!(words[..3], ["if", "you", "do"]);
}

#[test]
pub(super) fn hordewing_skaab_parses_and_keeps_if_you_do_discard_followup()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Hordewing Skaab")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Horror])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3));
    let text = "Flying\nOther Zombies you control have flying.\nWhenever one or more Zombies you control deal combat damage to one or more of your opponents, you may draw cards equal to the number of opponents dealt damage this way. If you do, discard that many cards.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;

    let debug = format!("{definition:?}");
    assert!(
        debug.contains("IfEffect"),
        "expected lowered if-you-do followup in Hordewing Skaab trigger: {debug}"
    );
    assert!(
        debug.contains("DiscardEffect"),
        "expected lowered discard clause for Hordewing Skaab: {debug}"
    );

    Ok(())
}

#[test]
pub(super) fn night_shift_parses_die_adjustment_and_zombie_employee_token()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("night_shift_parse_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(night_shift_parses_die_adjustment_and_zombie_employee_token_inner)
        .expect("night shift regression thread should spawn")
        .join()
        .expect("night shift regression thread should not panic")
}

pub(super) fn night_shift_parses_die_adjustment_and_zombie_employee_token_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Night Shift of the Living Dead")
        .card_types(vec![CardType::Enchantment]);
    let text = "After you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.\nWhenever you roll a 6, create a 2/2 black Zombie Employee creature token.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;

    let die_adjustment = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::DieRollResultAdjustment =>
            {
                match &static_ability.payload {
                    StaticAbilityPayload::DieRollResultAdjustment(spec) => Some(spec),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("expected lowered die-roll result adjustment static ability");
    assert_eq!(die_adjustment.life_cost, 1);
    assert_eq!(die_adjustment.amount, 1);
    assert!(
        die_adjustment.once_each_turn,
        "expected die-roll adjustment to keep once-each-turn restriction"
    );
    assert!(
        matches!(&die_adjustment.player, crate::target::PlayerFilter::You),
        "expected die-roll adjustment to apply to you"
    );

    let roll_trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if matches!(
                    &triggered.trigger.kind,
                    TriggerKind::PlayerRollsResult {
                        player: crate::target::PlayerFilter::You,
                        result: 6,
                    }
                ) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("expected die-roll trigger");
    let created_token = roll_trigger
        .effects
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("expected die-roll trigger to create a token");
    assert!(
        created_token.token.card.subtypes == vec![Subtype::Zombie, Subtype::Employee],
        "expected created token to keep both creature subtypes"
    );

    Ok(())
}

#[test]
pub(super) fn monitor_monitor_parses_paid_once_per_turn_reroll_modifier()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Monitor Monitor")
        .card_types(vec![CardType::Creature]);
    let text = "When this creature enters, open an Attraction.\nOnce each turn, you may pay {1} to reroll one or more dice you rolled.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;

    let reroll = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::DieRollResultAdjustment =>
            {
                match &static_ability.payload {
                    StaticAbilityPayload::DieRollResultAdjustment(spec) if spec.reroll => {
                        Some(spec)
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("expected lowered die-reroll modifier");
    assert_eq!(reroll.life_cost, 0);
    assert_eq!(reroll.amount, 0);
    assert_eq!(
        reroll.mana_cost,
        Some(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Generic(1),
        ]))
    );
    assert!(reroll.once_each_turn);
    assert!(matches!(reroll.player, crate::target::PlayerFilter::You));

    Ok(())
}

#[test]
pub(super) fn chance_encounter_parses_coin_flip_win_trigger() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Chance Encounter")
        .card_types(vec![CardType::Enchantment]);
    let text = "Whenever you win a coin flip, put a luck counter on this enchantment.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;

    assert!(definition.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Triggered(triggered)
                if matches!(
                    &triggered.trigger.kind,
                    TriggerKind::PlayerCoinFlipResult {
                        player: crate::target::PlayerFilter::You,
                        won: true,
                    }
                )
        )
    }));

    Ok(())
}

#[test]
pub(super) fn token_definition_keeps_multiple_creature_subtypes() {
    let token = super::super::compile_support::token_definition_for(
        "2/2 black Zombie Employee creature token",
    )
    .expect("Zombie Employee token should be recognized");

    assert_eq!(
        token.card.subtypes,
        vec![Subtype::Zombie, Subtype::Employee]
    );
}

#[test]
pub(super) fn token_definition_recognizes_fractal_creature_tokens() {
    let token = super::super::compile_support::token_definition_for(
        "0/0 green and blue Fractal creature token",
    )
    .expect("Fractal token should be recognized");

    assert_eq!(token.card.subtypes, vec![Subtype::Fractal]);
    assert_eq!(
        token.card.color_indicator,
        Some(crate::color::ColorSet::GREEN.union(crate::color::ColorSet::BLUE))
    );
    assert_eq!(
        token.card.power_toughness,
        Some(crate::PowerToughness::fixed(0, 0))
    );
}

#[test]
pub(super) fn rewrite_lowering_conditional_antecedent_prelude_carries_target_spec()
-> Result<(), CardTextError> {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Fight Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target creature you control gets +2/+2 until end of turn if its power is 2. Then it fights target creature you don't control.",
        )?;

    let program = def
        .spell_effect
        .as_ref()
        .expect("spell should lower to a resolution program");
    let conditional = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
        .expect("conditional pump should lower to a conditional effect");
    let condition_tag = match &conditional.condition {
        crate::effect::Condition::TaggedObjectMatches(tag, _) => tag,
        other => panic!("expected tagged-object condition, got {other:?}"),
    };

    let tagged_prelude = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| {
            let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
            (tagged.tag.as_str() == condition_tag.as_str()).then_some(tagged)
        })
        .expect("condition tag should be established by a prelude effect");

    assert!(
        tagged_prelude
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some(),
        "condition antecedent prelude must carry target metadata, got {tagged_prelude:?}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_keeps_triggered_double_sweep_body() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Zopandrel Variant")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of each combat, double the power and toughness of each creature you control until end of turn.".to_string(),
        false,
    )?;
    let parsed = crate::compiler_pipeline::parse_semantic_document(doc)?;

    match parsed.items.as_slice() {
        [crate::cards::builders::ParsedCardItem::Line(line)] => {
            let debug = format!("{:?}", line.chunks);
            assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
        }
        other => panic!("expected one parsed semantic line item, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_keeps_triggered_triple_sweep_body() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Triple Sweep Variant")
        .card_types(vec![CardType::Enchantment]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of each combat, triple the power and toughness of each creature you control until end of turn.".to_string(),
        false,
    )?;
    let parsed = crate::compiler_pipeline::parse_semantic_document(doc)?;

    match parsed.items.as_slice() {
        [crate::cards::builders::ParsedCardItem::Line(line)] => {
            let debug = format!("{:?}", line.chunks);
            assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
            assert!(debug.contains("multiplier: 2"), "{debug}");
        }
        other => panic!("expected one parsed semantic line item, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_keeps_nested_combat_whenever_trigger()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Nested Combat Trigger Variant")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of each combat, unless you pay {1}, whenever this creature attacks, draw a card.".to_string(),
        false,
    )?;

    let [item] = doc.items.as_slice() else {
        panic!("expected one triggered semantic item, got {:?}", doc.items);
    };
    let (trigger, effects, _) =
        rewrite_direct_triggered_chunk(item).expect("expected a typed triggered semantic chunk");
    let trigger_debug = format!("{trigger:?}");
    let effects_debug = format!("{effects:?}");
    assert!(
        trigger_debug.contains("BeginningOfCombat"),
        "{trigger_debug}"
    );
    assert!(effects_debug.contains("UnlessPays"), "{effects_debug}");
    assert!(
        effects_debug.contains("DelayedTriggerForDuration"),
        "{effects_debug}"
    );
    assert!(effects_debug.contains("Attacks"), "{effects_debug}");
    assert!(effects_debug.contains("Draw"), "{effects_debug}");

    Ok(())
}

#[test]
pub(super) fn nested_combat_payment_keeps_blocks_union_and_end_of_combat_lifetime()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Combat Flotilla Variant")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of each combat, unless you pay {R}, whenever this creature blocks or becomes blocked by a creature this combat, that creature gains first strike until end of turn.".to_string(),
        false,
    )?;
    let [item] = doc.items.as_slice() else {
        panic!("expected one nested combat item, got {:?}", doc.items);
    };
    let (trigger, effects, _) =
        rewrite_direct_triggered_chunk(item).expect("expected typed outer combat trigger");
    let trigger_debug = format!("{trigger:#?}");
    let effects_debug = format!("{effects:#?}");
    assert!(
        trigger_debug.contains("BeginningOfCombat"),
        "{trigger_debug}"
    );
    assert!(effects_debug.contains("UnlessPays"), "{effects_debug}");
    let [
        EffectAst::UnlessPays {
            effects: delayed, ..
        },
    ] = effects
    else {
        panic!("expected the payment gate to own the delayed trigger: {effects_debug}");
    };
    let [
        EffectAst::DelayedTriggerForDuration {
            trigger: TriggerSpec::BlocksOrBecomesBlockedByObject { subject, other },
            duration,
            ..
        },
    ] = delayed.as_slice()
    else {
        panic!("expected one canonical block-or-blocked trigger: {effects_debug}");
    };
    assert_eq!(*duration, crate::effect::Until::EndOfCombat);
    assert_eq!(subject.card_types, vec![CardType::Creature]);
    assert_eq!(other.card_types, vec![CardType::Creature]);
    assert!(effects_debug.contains("FirstStrike"), "{effects_debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_keeps_toggo_rock_token_rules_tail() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Toggo, Goblin Weaponsmith")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Landfall — Whenever a land you control enters, create a colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.".to_string(),
        false,
    )?;
    let parsed = crate::compiler_pipeline::parse_semantic_document(doc)?;

    let expect_toggo_token_shape = |effects: &[crate::cards::builders::EffectAst]| match effects {
        [
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::CreateTokenWithMods {
                            name,
                            definition,
                            ..
                        },
                    ..
                },
            ),
        ] => {
            let lower_name = name.to_ascii_lowercase();
            assert!(
                lower_name.contains("named rock"),
                "expected named rock token payload, got {name}"
            );
            let crate::model::token_definition::TokenDefinitionSpec::Artifact(artifact) =
                definition
            else {
                panic!("expected typed artifact token definition, got {definition:?}");
            };
            assert_eq!(artifact.name, "Rock");
            assert_eq!(artifact.subtypes, vec![Subtype::Equipment]);
            let equipment = artifact
                .equipment_rules
                .as_ref()
                .expect("Toggo should carry typed equipment rules before lowering");
            assert!(equipment.lines.iter().any(|line| matches!(
                line,
                crate::model::token_definition::EquipmentRuleLineShape::GrantedDamage {
                    grant: crate::model::token_definition::EquipmentDamageGrantShape {
                        generic_amount: Some(1),
                        tap_cost: true,
                        sacrifice_equipment: true,
                        damage_amount: 2,
                    },
                    ..
                }
            )));
            assert!(equipment.lines.iter().any(|line| matches!(
                line,
                crate::model::token_definition::EquipmentRuleLineShape::Equip(
                    crate::model::token_definition::TokenEquipShape { amount: 1 }
                )
            )));
        }
        other => panic!("expected a single token creation effect, got {other:?}"),
    };

    match parsed.items.as_slice() {
        [crate::cards::builders::ParsedCardItem::Line(line)] => match line.chunks.as_slice() {
            [crate::cards::builders::LineAst::Triggered { effects, .. }] => {
                expect_toggo_token_shape(effects);
            }
            [crate::cards::builders::LineAst::Ability(parsed)] => {
                let Some(effects) = parsed.effects_ast.as_ref() else {
                    panic!("expected landfall ability to keep parsed effects ast");
                };
                expect_toggo_token_shape(effects);
            }
            other => panic!("expected triggered line ast, got {other:?}"),
        },
        other => panic!("expected one triggered semantic item, got {other:?}"),
    }

    Ok(())
}

#[test]
pub(super) fn compile_definition_keeps_toggo_rock_token_rules_tail() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Toggo, Goblin Weaponsmith")
        .card_types(vec![CardType::Creature]);
    let compiled = crate::CompilerFacade::new().compile_definition(
        builder,
        "Landfall — Whenever a land you control enters, create a colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.\nPartner (You can have two commanders if both have partner.)",
        crate::CompilePolicy {
            allow_unsupported: false,
        },
    )?;

    assert!(
        !compiled.definition.abilities.is_empty(),
        "Toggo should compile through the canonical compiler facade"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_keeps_trigger_trigger_caps_and_first_time_suffixes()
-> Result<(), CardTextError> {
    let (capped_doc, _) = parse_text_to_semantic_document(
        CardDefinitionBuilder::new(CardId::new(), "Capped Trigger Variant")
            .card_types(vec![CardType::Enchantment]),
        "Whenever one or more creatures attack you, draw a card. This ability triggers only once each turn.".to_string(),
        false,
    )?;

    let [capped] = capped_doc.items.as_slice() else {
        panic!(
            "expected one triggered semantic item, got {:?}",
            capped_doc.items
        );
    };
    assert_eq!(
        rewrite_direct_triggered_chunk(capped).map(|(_, _, cap)| cap),
        Some(Some(1))
    );

    let (first_time_doc, _) = parse_text_to_semantic_document(
        CardDefinitionBuilder::new(CardId::new(), "First Time Trigger Variant")
            .card_types(vec![CardType::Enchantment]),
        "Whenever one or more creatures attack you for the first time each turn, draw a card."
            .to_string(),
        false,
    )?;

    let [first_time] = first_time_doc.items.as_slice() else {
        panic!(
            "expected one triggered semantic item, got {:?}",
            first_time_doc.items
        );
    };
    let (trigger, effects, cap) = rewrite_direct_triggered_chunk(first_time)
        .expect("expected first-time trigger to carry typed semantic data");
    assert_eq!(cap, Some(1));
    assert!(format!("{trigger:?}").contains("Attacks"));
    assert!(format!("{effects:?}").contains("Draw"));

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_accepts_do_this_only_once_each_turn_trigger_cap()
-> Result<(), CardTextError> {
    let (doc, annotations) = parse_text_to_semantic_document(
        CardDefinitionBuilder::new(CardId::new(), "Deep Gnome Terramancer")
            .card_types(vec![CardType::Creature]),
        "Flash\nMold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.".to_string(),
        false,
    )?;

    let triggered = doc
        .items
        .iter()
        .filter_map(rewrite_parsed_line)
        .find(|line| {
            line.chunks
                .iter()
                .any(|chunk| matches!(chunk, LineAst::Triggered { .. } | LineAst::Ability(_)))
        })
        .expect("expected Deep Gnome Terramancer to parse as a triggered line");

    let chunk_debug = format!("{:?}", triggered.chunks);
    let normalized = annotations
        .normalized_lines
        .get(&triggered.info.line_index)
        .expect("triggered line should retain its source annotation");
    assert!(
        chunk_debug.contains("max_triggers_per_turn: Some(1)")
            || chunk_debug.contains("DoThisMaxTimesEachTurn(1)"),
        "{chunk_debug}"
    );
    assert!(
        normalized
            .contains("one or more lands enter under an opponent's control without being played"),
        "unexpected normalized line: {}",
        normalized
    );
    assert!(
        normalized
            .to_ascii_lowercase()
            .contains("search your library for a plains card"),
        "unexpected normalized line: {}",
        normalized
    );
    assert!(
        !chunk_debug.contains("Do this only once each turn"),
        "cap sentence should stay out of the typed effect: {chunk_debug}",
    );

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_keeps_intervening_if_trigger_split()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Intervening If Trigger Variant")
        .card_types(vec![CardType::Enchantment]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "At the beginning of your upkeep, if you control an artifact, draw a card.".to_string(),
        false,
    )?;

    let [item] = doc.items.as_slice() else {
        panic!("expected one triggered semantic item, got {:?}", doc.items);
    };
    let (trigger, effects, _) =
        rewrite_direct_triggered_chunk(item).expect("expected typed intervening-if trigger");
    assert!(format!("{trigger:?}").contains("Upkeep"));
    assert!(format!("{effects:?}").contains("Draw"));

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_accepts_becomes_targeted_by_spell_filter_trigger()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Wild Defiance Variant")
        .card_types(vec![CardType::Enchantment]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Whenever a creature you control becomes the target of an instant or sorcery spell, that creature gets +3/+3 until end of turn.".to_string(),
        false,
    )?;

    let [item] = doc.items.as_slice() else {
        panic!("expected one triggered semantic item, got {:?}", doc.items);
    };
    let (trigger, effects, _) =
        rewrite_direct_triggered_chunk(item).expect("expected typed becomes-targeted trigger");
    assert!(format!("{trigger:?}").contains("Target"));
    assert!(format!("{effects:?}").contains("Pump"));

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_keeps_controller_on_targeting_spell_filter()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Targeted Drake Variant")
        .card_types(vec![CardType::Creature]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Whenever this creature becomes the target of a spell you control, draw a card."
            .to_string(),
        false,
    )?;

    let [item] = doc.items.as_slice() else {
        panic!("expected one triggered semantic item, got {:?}", doc.items);
    };
    let (trigger, effects, _) =
        rewrite_direct_triggered_chunk(item).expect("expected typed becomes-targeted trigger");
    let debug = format!("{trigger:?}");
    assert!(
        debug.contains("ThisBecomesTargetedBySpell") && debug.contains("controller: Some(You)"),
        "expected the triggering spell's controller constraint to survive, got {debug}"
    );
    assert!(format!("{effects:?}").contains("Draw"));

    Ok(())
}

#[test]
pub(super) fn rewrite_semantic_parse_marks_plumb_additional_cost_as_non_choice()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Plumb Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nYou draw a card and you lose 1 life.".to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.first(),
        Some(RewriteSemanticItem::Keyword(keyword))
            if keyword.kind == RewriteKeywordLineKind::AdditionalCost
    ));
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_former_section9_cases_parse_without_fallback_text()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("former_section9_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_former_section9_cases_parse_without_fallback_text_inner)
        .expect("former section-9 regression thread should spawn")
        .join()
        .expect("former section-9 regression thread should not panic")
}

pub(super) fn rewrite_lowered_former_section9_cases_parse_without_fallback_text_inner()
-> Result<(), CardTextError> {
    let cases = vec![
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Poison")
                .card_types(vec![CardType::Creature]),
            "Whenever this creature deals damage to a player, that player gets a poison counter. The player gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step. (A player with ten or more poison counters loses the game.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Unearth")
                .card_types(vec![CardType::Artifact, CardType::Creature]),
            "Permanents you control have \"Ward—Sacrifice a permanent.\"\nEach artifact card in your graveyard has unearth {1}{B}{R}. ({1}{B}{R}: Return the card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Unearth only as a sorcery.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Sticker")
                .card_types(vec![CardType::Sorcery]),
            "Put an art sticker on a nonland permanent you own. Then ask a person outside the game to rate its new art on a scale from 1 to 5, where 5 is the best. When they rate the art, up to that many target creatures can't block this turn.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Can’t Block")
                .card_types(vec![CardType::Creature]),
            "This creature can't be blocked by more than one creature.\nEach creature you control with a +1/+1 counter on it can't be blocked by more than one creature.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 White Destroy")
                .card_types(vec![CardType::Sorcery]),
            "Destroy target creature if it's white. A creature destroyed this way can't be regenerated.\nDraw a card at the beginning of the next turn's upkeep.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Spent")
                .card_types(vec![CardType::Instant]),
            "Create two 1/1 white Kithkin Soldier creature tokens if {W} was spent to cast this spell. Counter up to one target creature spell if {U} was spent to cast this spell. (Do both if {W}{U} was spent.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Goats")
                .card_types(vec![CardType::Artifact]),
            "{T}: Add {C}.\n{4}, {T}: Create a 0/1 white Goat creature token.\n{T}, Sacrifice X Goats: Add X mana of any one color. You gain X life.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Exile Top")
                .card_types(vec![CardType::Sorcery]),
            "Shuffle your library, then exile the top four cards. You may cast any number of spells with mana value 5 or less from among them without paying their mana costs. Lands you control don't untap during your next untap step.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Cloak")
                .card_types(vec![CardType::Sorcery]),
            "Exile target nontoken creature you own and the top two cards of your library in a face-down pile, shuffle that pile, then cloak those cards. They enter tapped. (To cloak a card, put it onto the battlefield face down as a 2/2 creature with ward {2}. Turn it face up any time for its mana cost if it's a creature card.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Toughness")
                .card_types(vec![CardType::Instant]),
            "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Or")
                .card_types(vec![CardType::Sorcery]),
            "Destroy all lands or all creatures. Creatures destroyed this way can't be regenerated.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Nonblack")
                .card_types(vec![CardType::Sorcery]),
            "Destroy two target nonblack creatures unless either one is a color the other isn't. They can't be regenerated.",
        ),
    ];

    let mut failures = Vec::new();

    for (builder, text) in cases {
        let (definition, _) =
            match parse_text_with_annotations_lowered(builder, text.to_string(), false) {
                Ok(parsed) => parsed,
                Err(err) => {
                    failures.push(format!(
                        "former section-9 case failed to parse: {text}\n{err:?}"
                    ));
                    continue;
                }
            };
        let _ = definition;
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));

    Ok(())
}

#[test]
pub(super) fn rewrite_trial_of_agony_other_clause_parses_as_other_tagged_restriction()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Trial of Agony")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Choose two target creatures controlled by the same opponent. That player chooses one of those creatures. Trial of Agony deals 5 damage to that creature, and the other can't block this turn.".to_string(),
        false,
    )?;
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("CantEffect"),
        "expected can't effect, got {debug}"
    );
    assert!(
        debug.contains("restriction: Block"),
        "expected block restriction, got {debug}"
    );
    assert!(
        debug.contains("IsNotTaggedObject"),
        "expected other-reference tagging, got {debug}"
    );

    Ok(())
}

#[test]
pub(super) fn flamebreak_damage_this_way_regeneration_restriction_tracks_damaged_creatures()
-> Result<(), CardTextError> {
    let builder =
        CardDefinitionBuilder::new(CardId::new(), "Flamebreak").card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Flamebreak deals 3 damage to each creature without flying and each player. Creatures dealt damage this way can't be regenerated this turn.".to_string(),
        false,
    )?;
    let debug = format!("{definition:#?}");

    assert!(debug.contains("CantEffect"), "{debug}");
    assert!(debug.contains("restriction: BeRegenerated"), "{debug}");
    assert!(debug.contains("\"damaged_0\""), "{debug}");
    assert!(debug.contains("relation: IsTaggedObject"), "{debug}");

    Ok(())
}

#[test]
pub(super) fn parse_subject_first_exile_top_library_then_play_permission_bundle() {
    let builder = CardDefinitionBuilder::new(CardId::from_raw(1), "Bundle Probe")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Target player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs."
            .to_string(),
        false,
    )
    .expect("the Fallen Shinobi style bundle should lower cleanly");
    let debug = format!("{:#?}", definition.spell_effect).to_ascii_lowercase();

    assert!(
        debug.contains("exiletopoflibraryeffect"),
        "expected top-library exile in the bundle, got {debug}"
    );
    assert!(
        debug.contains("grantplaytaggedeffect"),
        "expected play-from-exile permission in the bundle, got {debug}"
    );
    assert!(
        debug.contains("granttaggedspellfreecastuntilendofturneffect"),
        "expected free-cast permission in the bundle, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_preprocess_expands_same_is_true_trigger_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thunderous Orator Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, it gains flying until end of turn if you control a creature with flying. The same is true for first strike and vigilance.",
        )
        .expect("same-is-true trigger chain should parse");

    let rendered = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        rendered.contains("thisattacks")
            && rendered.contains("addability")
            && rendered.contains("flying")
            && rendered.contains("endofturn"),
        "expected flying attack-trigger branch to remain structurally, got {rendered}"
    );
    assert!(
        rendered.contains("first strike") && rendered.contains("vigilance"),
        "expected remaining borrowed keyword branches, got {rendered}"
    );
}

#[test]
pub(super) fn rewrite_preprocess_expands_same_is_true_static_graveyard_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cairn Wanderer Variant")
        .parse_text(
            "As long as a creature card with flying is in a graveyard, this creature has flying. The same is true for first strike and vigilance.",
        )
        .expect("same-is-true static graveyard chain should parse");

    let rendered = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        rendered.contains("flying"),
        "expected flying branch, got {rendered}"
    );
    assert!(
        rendered.contains("first strike") && rendered.contains("vigilance"),
        "expected same-is-true graveyard branches to expand, got {rendered}"
    );
}

#[test]
pub(super) fn rewrite_preprocess_expands_same_is_true_static_exile_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Urborg Scavengers Variant")
        .parse_text(
            "This creature has flying as long as a card exiled with it has flying. The same is true for trample and vigilance.",
        )
        .expect("same-is-true exile chain should parse");

    let rendered = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        rendered.contains("flying"),
        "expected exile-linked flying branch, got {rendered}"
    );
    assert!(
        rendered.contains("trample") && rendered.contains("vigilance"),
        "expected same-is-true exile branches to expand, got {rendered}"
    );
}

#[test]
pub(super) fn rewrite_preprocess_expands_same_is_true_static_delve_exile_chain() {
    for preposition in ["with", "by"] {
        let def = CardDefinitionBuilder::new(CardId::new(), "Soulflayer Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(format!(
                "Delve\nIf a creature card with flying was exiled {preposition} this creature's delve ability, this creature has flying. The same is true for first strike and vigilance.",
            ))
            .expect("same-is-true delve-linked exile chain should parse");

        let rendered = format!("{def:#?}").to_ascii_lowercase();
        assert!(
            rendered.contains("__source_exiled__"),
            "expected source-linked exile provenance in Soulflayer-style condition, got {rendered}"
        );
        assert!(
            rendered.contains("flying")
                && rendered.contains("first strike")
                && rendered.contains("vigilance"),
            "expected same-is-true delve branches to expand, got {rendered}"
        );
    }
}

#[test]
pub(super) fn parse_choose_then_do_same_for_filter_splits_one_of_mana_values() {
    let tokens = lex_line(
        "choose a creature card with mana value 1 in your graveyard, then do the same for creature cards with mana value 2 and 3.",
        0,
    )
    .expect("choose-then-do-the-same sentence should lex");

    let effects = crate::effect_sentences::parse_sentence_choose_then_do_same_for_filter(
        super::super::effect_sentences::SubjectVerbPrimitiveClause::new(&tokens),
    )
    .expect("choose-then-do-the-same primitive should not error")
    .expect("choose-then-do-the-same primitive should match");
    let debug = format!("{effects:#?}");

    assert_eq!(
        debug.matches("ChooseObjects").count(),
        3,
        "expected three choose-object AST nodes, got {debug}"
    );
    assert!(
        debug.contains("Equal(\n                    1,")
            && debug.contains("Equal(\n                    2,")
            && debug.contains("Equal(\n                    3,"),
        "expected mana values 1, 2, and 3 to be split into ordered choices, got {debug}"
    );
}

#[test]
pub(super) fn parse_choose_then_do_same_for_filter_building_blocks_match() {
    let head = lex_line(
        "choose a creature card with mana value 1 in your graveyard",
        0,
    )
    .expect("head clause should lex");
    let head_parsed = crate::activation_and_restrictions::parse_you_choose_objects_clause(&head)
        .expect("head choose helper should not error");
    assert!(
        head_parsed.is_some(),
        "expected head choose helper to match"
    );

    let tail =
        lex_line("creature cards with mana value 2 and 3", 0).expect("tail filter should lex");
    let tail_filter =
        crate::grammar::filters::parse_object_filter_with_grammar_entrypoint(&tail, false)
            .expect("tail filter should parse");
    assert!(
        tail_filter.zone == Some(crate::zone::Zone::Battlefield)
            && tail_filter.owner.is_none()
            && tail_filter.controller.is_none(),
        "expected followup filter to stay unowned/uncontrolled and keep the default battlefield zone, got {tail_filter:?}"
    );
    assert!(
        matches!(
            tail_filter.mana_value,
            Some(crate::filter::Comparison::OneOf(ref values)) if values == &[2, 3]
        ),
        "expected followup filter to preserve OneOf(2,3), got {tail_filter:?}"
    );
}

#[test]
pub(super) fn rewrite_grammar_unique_hand_leader_predicate_parses() {
    let tokens = lex_line("a player has more cards in hand than each other player", 0)
        .expect("rewrite lexer should classify unique hand-leader predicate");

    assert_eq!(
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse"),
        crate::cards::builders::PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
            player: crate::cards::builders::PlayerAst::Any,
        }
    );
}

#[test]
pub(super) fn rewrite_grammar_unique_life_leader_predicate_parses() {
    let tokens = lex_line("a player has more life than each other player", 0)
        .expect("rewrite lexer should classify unique life-leader predicate");

    assert_eq!(
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse"),
        crate::cards::builders::PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
            player: crate::cards::builders::PlayerAst::Any,
        }
    );
}

#[test]
pub(super) fn rewrite_grammar_unique_creature_control_leader_predicate_parses() {
    let tokens = lex_line("a player controls more creatures than each other player", 0)
        .expect("rewrite lexer should classify unique creature-control leader predicate");

    let parsed = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
        .expect("predicate should parse");
    assert!(
        matches!(
            parsed,
            crate::cards::builders::PredicateAst::PlayerControlsMoreThanEachOtherPlayer {
                player: crate::cards::builders::PlayerAst::Any,
                ref filter,
            } if filter.card_types == [CardType::Creature]
        ),
        "{parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_grammar_no_opponent_has_more_life_than_that_player_predicate_parses() {
    let tokens = lex_line("no opponent has more life than that player", 0)
        .expect("rewrite lexer should classify no-opponent life predicate");

    assert_eq!(
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse"),
        crate::cards::builders::PredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
            player: crate::cards::builders::PlayerAst::That,
        }
    );
}

#[test]
pub(super) fn rewrite_grammar_opponent_has_zero_or_less_life_predicate_parses() {
    let tokens = lex_line("opponent has 0 or less life", 0)
        .expect("rewrite lexer should classify opponent life-threshold predicate");

    assert_eq!(
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse"),
        crate::cards::builders::PredicateAst::ValueComparison {
            left: crate::effect::Value::LifeTotal(crate::filter::PlayerFilter::Opponent),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: crate::effect::Value::Fixed(0),
        }
    );
}

#[test]
pub(super) fn heaven_sent_strict_compile_succeeds() {
    let oracle_text = "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI, II - Investigate.\nIII - This Saga deals 1 damage to each opponent. Then if an opponent has 0 or less life, draw seven cards. Otherwise, exile this Saga and you may cast it this turn.";

    let def = CardDefinitionBuilder::new(CardId::new(), "Heaven Sent Variant")
        .parse_text(oracle_text)
        .expect("Heaven Sent text should parse");
    let debug = format!("{:?}", def.abilities);

    assert!(debug.contains("ValueComparison"), "{debug}");
    assert!(debug.contains("LifeTotal(Opponent)"), "{debug}");
    assert!(debug.contains("LessThanOrEqual"), "{debug}");
    assert!(debug.contains("Fixed(0)"), "{debug}");
}

#[test]
pub(super) fn ruthless_cullblade_strict_compile_succeeds() {
    let oracle_text = "As long as an opponent has 10 or less life, Ruthless Cullblade gets +2/+1.";

    let def = CardDefinitionBuilder::new(CardId::new(), "Ruthless Cullblade")
        .parse_text(oracle_text)
        .expect("Ruthless Cullblade text should parse");

    assert_eq!(def.name(), "Ruthless Cullblade");
    let debug = format!("{:?}", def.abilities);
    assert!(debug.contains("ValueComparison"), "{debug}");
    assert!(debug.contains("LifeTotal(Opponent)"), "{debug}");
    assert!(debug.contains("LessThanOrEqual"), "{debug}");
    assert!(debug.contains("Fixed(10)"), "{debug}");
}

#[test]
pub(super) fn ruthless_cullblade_compiled_condition_uses_opponent_life_threshold() {
    let oracle_text = "As long as an opponent has 10 or less life, Ruthless Cullblade gets +2/+1.";

    let def = CardDefinitionBuilder::new(CardId::new(), "Ruthless Cullblade")
        .parse_text(oracle_text)
        .expect("Ruthless Cullblade text should parse");

    let rendered = format!("{def:#?}");
    assert!(
        rendered.contains("LifeTotal(")
            && rendered.contains("Opponent")
            && rendered.contains("LessThanOrEqual")
            && rendered.contains("Fixed(")
            && rendered.contains("10"),
        "expected compiled condition to retain opponent life-threshold clause, got: {rendered}"
    );
}

#[test]
pub(super) fn rewrite_grammar_battlefield_count_predicate_parses_other_creatures() {
    let tokens = lex_line(
        "there are two or more other creatures on the battlefield",
        0,
    )
    .expect("rewrite lexer should classify battlefield-count predicate");

    let debug = format!(
        "{:?}",
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse")
    );
    assert!(debug.contains("ValueComparison"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_permanent_you_controlled_left_battlefield_predicate_parses() {
    let tokens = lex_line(
        "a permanent you controlled left the battlefield this turn",
        0,
    )
    .expect("rewrite lexer should classify revolt-style permanent-left predicate");

    assert_eq!(
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse"),
        crate::cards::builders::PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn {
            surface: crate::PermanentLeftBattlefieldControlSurface::YouControlledLeft,
        }
    );
}

#[test]
pub(super) fn rewrite_grammar_land_you_controlled_put_into_graveyard_predicate_parses() {
    let tokens = lex_line(
        "a land you controlled was put into a graveyard from the battlefield this turn",
        0,
    )
    .expect("rewrite lexer should classify land graveyard-history predicate");

    let parsed = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
        .expect("predicate should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("ObjectPutIntoGraveyardFromBattlefieldThisTurn"),
        "{debug}"
    );
    assert!(debug.contains("Land"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_creature_card_put_into_your_graveyard_from_anywhere_predicate_parses()
{
    let tokens = lex_line(
        "a creature card was put into your graveyard from anywhere this turn",
        0,
    )
    .expect("rewrite lexer should classify creature-card graveyard-history predicate");

    assert_eq!(
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse"),
        crate::cards::builders::PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn
    );
}

#[test]
pub(super) fn rewrite_grammar_artifact_entered_under_your_control_predicate_parses() {
    let tokens = lex_line(
        "an artifact entered the battlefield under your control this turn",
        0,
    )
    .expect("rewrite lexer should classify artifact-entered predicate");

    let parsed = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
        .expect("predicate should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("ObjectEnteredBattlefieldThisTurn"),
        "{debug}"
    );
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_you_lost_life_this_turn_threshold_predicate_parses() {
    let tokens = lex_line("you lost 2 or more life this turn", 0)
        .expect("rewrite lexer should classify life-lost threshold predicate");

    let parsed = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
        .expect("predicate should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ValueComparison"), "{debug}");
    assert!(debug.contains("LifeLostThisTurn(You)"), "{debug}");
    assert!(debug.contains("GreaterThanOrEqual"), "{debug}");
    assert!(debug.contains("Fixed(2)"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_conjoined_named_spells_cast_this_turn_predicate_parses() {
    let tokens = lex_line(
        "you've cast a spell named Peer Through Depths and a spell named Reach Through Mists this turn",
        0,
    )
    .expect("rewrite lexer should classify conjoined named-spell predicate");

    let parsed = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
        .expect("predicate should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("And("), "{debug}");
    assert!(debug.contains("SpellsCastThisTurnMatching"), "{debug}");
    assert!(debug.contains("peer through depths"), "{debug}");
    assert!(debug.contains("reach through mists"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_no_permanents_left_battlefield_this_turn_predicate_parses() {
    let tokens = lex_line("no permanents left the battlefield this turn", 0)
        .expect("rewrite lexer should classify the global no-permanents-left predicate");

    assert_eq!(
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens)
            .expect("predicate should parse"),
        crate::cards::builders::PredicateAst::Not(Box::new(
            crate::cards::builders::PredicateAst::PermanentLeftBattlefieldThisTurn,
        ))
    );
}

#[test]
pub(super) fn rewrite_parse_subject_player_with_most_cards_in_hand() {
    let tokens = lex_line("the player who has the most cards in hand", 0)
        .expect("rewrite lexer should classify most-cards subject");

    assert_eq!(
        super::super::util::parse_subject(&tokens),
        crate::util::SubjectAst::Player(crate::cards::builders::PlayerAst::MostCardsInHand)
    );
}

#[test]
pub(super) fn rewrite_parse_subject_with_most_life() {
    let tokens = lex_line("the player with the most life", 0)
        .expect("rewrite lexer should classify most-life subject");

    assert_eq!(
        super::super::util::parse_subject(&tokens),
        crate::util::SubjectAst::Player(crate::cards::builders::PlayerAst::MostLifeTied)
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_keeps_unique_life_leader_intervening_if() {
    let text = "At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify upkeep intervening-if");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("triggered intervening-if line should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(
        debug.contains("PlayerHasMoreLifeThanEachOtherPlayer"),
        "{debug}"
    );
    assert!(debug.contains("MostLifeTied"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_keeps_unique_creature_control_leader_intervening_if() {
    let text = "At the beginning of your upkeep, if a player controls more creatures than each other player, the player who controls the most creatures gains control of this creature.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify creature-control upkeep intervening-if");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("triggered intervening-if line should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(
        debug.contains("PlayerControlsMoreThanEachOtherPlayer"),
        "{debug}"
    );
    assert!(debug.contains("card_types: [Creature]"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_keeps_toughness_greater_than_power_gate() {
    let text = "At the beginning of combat on your turn, if you control three or more creatures that each have toughness greater than their power, transform this creature.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Catapult-style trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("Catapult-style triggered line should parse");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("BeginningOfCombat") && debug.contains("You"),
        "{debug}"
    );
    assert!(debug.contains("PlayerHasAtLeast"), "{debug}");
    assert!(debug.contains("power_toughness_relation"), "{debug}");
    assert!(debug.contains("ToughnessGreaterThanPower"), "{debug}");
    assert!(
        !debug.contains("toughness: Some(GreaterThanExpr(SourcePower))"),
        "{debug}"
    );
    assert!(
        !debug.contains("power: Some(LessThanExpr(SourceToughness))"),
        "{debug}"
    );
    assert!(debug.contains("Transform"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_keeps_guild_artisan_life_gate() {
    let text = "Whenever this creature attacks a player, if no opponent has more life than that player, you create two Treasure tokens.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Guild Artisan trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("Guild Artisan triggered line should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ThisAttacks"), "{debug}");
    assert!(
        debug.contains("PlayerHasNoOpponentWithMoreLifeThan"),
        "{debug}"
    );
    assert!(debug.contains("CreateToken"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_accepts_attack_target_tail() {
    let tokens = lex_line("this creature attacks a player", 0)
        .expect("rewrite lexer should classify attack trigger clause");

    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("attack trigger clause with player tail should parse");

    assert!(matches!(
        parsed,
        crate::cards::builders::TriggerSpec::ThisAttacks
    ));
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_keeps_attacked_player_land_count_gate() {
    let tokens = lex_line(
        "this creature attacks a player who controls eight or more lands",
        0,
    )
    .expect("rewrite lexer should classify attack trigger clause");

    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("attack trigger clause with defending-player land count should parse");

    match parsed {
        crate::cards::builders::TriggerSpec::ThisAttacksPlayerWhoControlsAtLeast {
            count,
            filter,
        } => {
            assert_eq!(count, 8);
            assert_eq!(filter.card_types, vec![crate::types::CardType::Land]);
        }
        other => panic!("expected land-count attack trigger, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_keeps_attacked_player_relative_life_gate() {
    let tokens = lex_line(
        "this creature attacks an opponent who has more life than you",
        0,
    )
    .expect("rewrite lexer should classify relative-life attack trigger clause");

    let parsed = crate::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("relative-life attack trigger clause should parse");

    match parsed {
        crate::cards::builders::TriggerSpec::Attacks(filter) => {
            assert!(
                filter.source,
                "expected the attack source to remain identity-bound"
            );
            assert_eq!(
                filter.attacking_player_or_planeswalker_controlled_by,
                Some(crate::target::PlayerFilter::HasMoreLifeThanYou {
                    base: Box::new(crate::target::PlayerFilter::Opponent),
                })
            );
            assert_eq!(
                filter.targets_only_player,
                Some(crate::target::PlayerFilter::HasMoreLifeThanYou {
                    base: Box::new(crate::target::PlayerFilter::Opponent),
                }),
                "the trigger must not match a planeswalker protected by that opponent"
            );
        }
        other => panic!("expected a source attack with a relative-life defender, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_preserves_attacking_looked_card_bundle() {
    let text = "Look at the top eight cards of your library. You may put a creature card from among them onto the battlefield tapped and attacking that player. Put the rest on the bottom of your library in a random order.";
    let sentences = registry_sentence_inputs(text);

    let matched = super::super::effect_sentences::try_parse_document_program(&sentences, 0)
        .expect("registry lookup should not error")
        .expect("registry should match attacking looked-card battlefield/bottom sequence");
    let debug = format!("{:?}", matched.effects);

    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(
        debug.contains("ChooseObjects") || debug.contains("ChooseTaggedObjectsInZone"),
        "{debug}"
    );
    assert!(debug.contains("battlefield_attacking: true"), "{debug}");
    assert!(
        debug.contains(
            "battlefield_attack_target_player_or_planeswalker_controlled_by: Some(Defending)"
        ),
        "{debug}"
    );
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_static_grant_line_ignores_inner_has_in_quoted_trigger() {
    let text = "Commander creatures you own have \"Whenever this creature attacks a player, if no opponent has more life than that player, you create two Treasure tokens.\"";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Guild Artisan grant line");

    let parsed = super::super::clause_support::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Guild Artisan static grant should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantObjectAbilityForFilter") || debug.contains("GrantObjectAbility"),
        "{debug}"
    );
    assert!(
        debug.contains("PlayerHasNoOpponentWithMoreLifeThan"),
        "{debug}"
    );
    assert!(debug.contains("ThisAttacks"), "{debug}");
    assert!(
        debug.contains("intervening_if: Some")
            || debug.contains("Conditional { predicate: PlayerHasNoOpponentWithMoreLifeThan"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lowered_background_quoted_grant_with_inner_target_pump_stays_static()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Hardy Outlander")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Background]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Commander creatures you own have \"Whenever this creature attacks a player, if no opponent has more life than that player, another target creature you control gets +X/+X until end of turn, where X is this creature's power.\""
            .to_string(),
        false,
    )?;

    assert!(
        definition.spell_effect.is_none(),
        "quoted static grant should not lower as a spell effect: {:#?}",
        definition.spell_effect
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("GrantObjectAbilityForFilter"), "{debug}");
    assert!(
        debug.contains("ThisAttacksTrigger") || debug.contains("this_attacks"),
        "{debug}"
    );
    assert!(debug.contains("ModifyPowerToughness"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn jubilant_skybonder_full_document_keeps_quoted_tax_as_a_filtered_grant()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Jubilant Skybonder")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Flying\nCreatures you control with flying have \"Spells your opponents cast that target this creature cost {2} more to cast.\""
            .to_string(),
        false,
    )?;

    let grant = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => match &static_ability.payload {
                StaticAbilityPayload::GrantAbility(grant) => Some(grant.as_ref()),
                _ => None,
            },
            _ => None,
        })
        .expect("full document lowering should retain the filtered static-ability grant");
    assert_eq!(
        grant.filter.controller,
        Some(crate::target::PlayerFilter::You)
    );
    assert_eq!(grant.filter.card_types, [CardType::Creature]);
    assert_eq!(grant.filter.static_abilities, [StaticAbilityId::Flying]);

    let AbilityKind::Static(granted_tax) = &grant.ability.kind else {
        panic!("expected the granted ability to be a static cost tax");
    };
    let StaticAbilityPayload::CostIncrease(increase) = &granted_tax.payload else {
        panic!("expected the granted static ability to carry a cost increase");
    };
    assert_eq!(increase.amount, crate::effect::Value::Fixed(2));
    assert_eq!(
        increase.filter.cast_by,
        Some(crate::target::PlayerFilter::Opponent)
    );
    assert!(
        increase
            .filter
            .targets_object
            .as_deref()
            .is_some_and(|target| target.source),
        "\"this creature\" must bind to each flying creature receiving the grant"
    );
    Ok(())
}

#[test]
pub(super) fn staff_of_eden_full_compile_keeps_excluded_self_name_surface()
-> Result<(), CardTextError> {
    let compiled = super::super::compile_card_text(
        CardDefinitionBuilder::new(CardId::new(), "Staff of Eden, Vault's Key")
            .card_types(vec![CardType::Artifact]),
        "When Staff of Eden enters, put target legendary permanent card not named Staff of Eden, Vault's Key from a graveyard onto the battlefield under your control.\n{T}: Draw a card for each permanent you control but don't own.",
        false,
    )?;
    let debug = format!("{:#?}", compiled.definition);

    assert!(
        debug.contains("excluded_name: Some(") && debug.contains("\"staff of eden vaults key\""),
        "the semantic excluded-name key should remain normalized: {debug}"
    );
    assert!(
        debug.contains("\"Staff of Eden, Vault's Key\""),
        "the exact excluded-name surface should survive full compilation: {debug}"
    );
    assert!(
        debug.contains("controller: Some(")
            && debug.contains("owner: Some(")
            && debug.contains("NotYou"),
        "the draw-count filter should retain both controller and inverse-owner scope: {debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_attack_with_trigger_preserves_that_attacking_player_may() {
    let text = "Whenever a player attacks enchanted player with one or more creatures, that attacking player may create a tapped 2/2 black Zombie creature token.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify Curse of Shallow Graves trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("Curse of Shallow Graves trigger should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("AttacksOneOrMore"), "{debug}");
    assert!(debug.contains("MayByPlayer"), "{debug}");
    assert!(debug.contains("player: Attacking"), "{debug}");
    assert!(debug.contains("CreateToken"), "{debug}");
}

#[test]
pub(super) fn rewrite_lowered_attack_trigger_preserves_shared_attacking_player_draw_and_loss()
-> Result<(), CardTextError> {
    let text = "Whenever an opponent attacks another one of your opponents, you and the attacking player each draw a card and lose 1 life.";
    let builder = CardDefinitionBuilder::new(CardId::new(), "Karazikar-like Trigger")
        .card_types(vec![CardType::Creature]);

    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;
    let debug = format!("{:#?}", definition.abilities);

    assert!(
        debug.contains("ForPlayersEffect") || debug.contains("SequenceEffect"),
        "{debug}"
    );
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    assert!(debug.contains("player: You"), "{debug}");
    assert!(debug.contains("player: Attacking"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn witchbane_orb_player_hexproof_and_attached_curse_destroy_parse() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Witchbane Orb Variant")
        .parse_text(
            "You have hexproof.\nWhen this artifact enters, destroy all Curses attached to you.",
        )
        .expect("Witchbane Orb text should parse");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeTargetedPlayerFrom")
            && debug.contains("Opponent")
            && debug.contains("DestroyEffect")
            && debug.contains("attached_to_player: Some(You)")
            && debug.contains("Curse"),
        "expected player hexproof restriction and attached-curse destroy effect, got {debug}"
    );
}

#[test]
pub(super) fn absolute_virtue_player_protection_from_opponents_parses_as_targeting_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Absolute Virtue")
        .parse_text("You have protection from each of your opponents.")
        .expect("Absolute Virtue static line should parse");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeTargetedPlayerFrom")
            && debug.contains("You")
            && debug.contains("controller: Some(Opponent)"),
        "expected player targeting restriction from opponent-controlled sources, got {debug}"
    );
}

#[test]
pub(super) fn gaeas_revenge_source_filtered_targeting_restriction_parses_strictly() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gaea's Revenge")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This spell can't be countered.\n\
             Haste\n\
             This creature can't be the target of nongreen spells or abilities from nongreen sources.",
        )
        .expect("Gaea's Revenge text should parse");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeCountered")
            && debug.contains("Haste")
            && debug.contains("BeTargetedFrom")
            && debug.contains("excluded_colors"),
        "expected uncounterable, haste, and nongreen source targeting restriction, got {debug}"
    );
}

#[test]
pub(super) fn bartel_runeaxe_aura_spell_targeting_restriction_parses_strictly() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bartel Runeaxe")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant, Subtype::Warrior])
        .parse_text("Vigilance\nBartel Runeaxe can't be the target of Aura spells.")
        .expect("Bartel Runeaxe text should parse");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Vigilance")
            && debug.contains("BeTargetedFrom")
            && debug.contains("stack_kind: Some(Spell)")
            && debug.contains("subtypes: [Aura]"),
        "expected vigilance and an Aura spell targeting restriction, got {debug}"
    );
}

#[test]
pub(super) fn source_filtered_targeting_restriction_rejects_mismatched_spell_and_source_filters() {
    let err = parse_error_message(
        CardDefinitionBuilder::new(CardId::new(), "Mismatched Target Restriction")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "This creature can't be the target of nongreen spells or abilities from nonblue sources.",
            ),
    );
    assert!(
        err.contains("unsupported source-filtered target restriction tail"),
        "expected mismatched spell/source qualifiers to remain unsupported, got {err}"
    );
}

#[test]
pub(super) fn maddening_hex_damage_equal_to_die_result_binds_prior_roll() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Maddening Hex Variant")
        .parse_text(
            "Enchant player\nWhenever enchanted player casts a noncreature spell, roll a d6. Maddening Hex deals damage to that player equal to the result. Then attach Maddening Hex to another one of your opponents chosen at random.",
        )
        .expect("Maddening Hex trigger should parse");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("RollDieEffect")
            && debug.contains("WithIdEffect")
            && debug.contains("EffectValue")
            && debug.contains("DealDamageEffect")
            && debug.contains("TaggedPlayer(TagKey(\"enchanted\"))"),
        "expected damage amount to reference the prior die roll result and target the enchanted player, got {debug}"
    );
    assert!(
        !debug.contains("target: Player(IteratedPlayer)"),
        "that player in this non-loop trigger must not lower to an unbound iterated player, got {debug}"
    );
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("Excluding{base:Opponent")
            && compact.contains("TaggedPlayer(TagKey(\"enchanted\"))")
            && compact.contains("random:true"),
        "the random opponent must exclude the currently enchanted player: {debug}"
    );
}

#[test]
pub(super) fn death_by_dragons_targets_one_player_and_excludes_that_player_from_the_fanout() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Death by Dragons")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each player other than target player creates a 5/5 red Dragon creature token with flying.",
        )
        .expect("Death by Dragons should parse");
    let debug = format!("{:#?}", def.spell_effect);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(
        compact.contains("Excluding{base:Any,excluded:Target(")
            && compact.contains("controller:IteratedPlayer")
            && debug.contains("Dragon")
            && debug.contains("Flying"),
        "the chosen player must be excluded from the Dragon fanout: {debug}"
    );
}

#[test]
pub(super) fn curse_of_surveillance_keeps_the_enchanted_player_distinct_from_draw_recipients() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Curse of Surveillance")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .parse_text(
            "Enchant player\nAt the beginning of enchanted player's upkeep, any number of target players other than that player each draw cards equal to the number of Curses attached to that player.",
        )
        .expect("Curse of Surveillance should parse");
    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("min:0,max:None")
            && compact.contains("Excluding{base:Any,excluded:TaggedPlayer(")
            && compact.contains("ForPlayersEffect{filter:Target(Excluding")
            && compact.contains("attached_to_player:Some(TaggedPlayer(")
            && compact.matches("TagKey(\"enchanted\"").count() >= 3,
        "the exclusion/count anchor must remain the enchanted player while the loop iterates targets: {debug}"
    );
}

#[test]
pub(super) fn crown_of_doom_target_excludes_the_source_owner_not_the_controller() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Crown of Doom")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a creature attacks you or a planeswalker you control, it gets +2/+0 until end of turn.\n{2}: Target player other than this artifact's owner gains control of it. Activate only during your turn.",
        )
        .expect("Crown of Doom should parse");
    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("Excluding{base:Any,excluded:OwnerOf(Tagged(")
            && compact.contains("TagKey(\"__source_object__\"")
            && compact.contains("target_spec:Some(Source")
            && (debug.contains("ChangeControllerToPlayer") || debug.contains("GainControlEffect")),
        "the activation target must exclude the artifact's owner and transfer the source artifact itself: {debug}"
    );
}

#[test]
pub(super) fn lord_of_pain_another_target_excludes_the_triggering_caster_and_damage_aliases_it() {
    let def = CardDefinitionBuilder::new(CardId::new(), "The Lord of Pain")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Menace\nYour opponents can't gain life.\nWhenever a player casts their first spell each turn, choose another target player. The Lord of Pain deals damage equal to that spell's mana value to the chosen player.",
        )
        .expect("The Lord of Pain should parse");
    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("explicit_declaration:true")
            && compact.contains("Excluding{base:Any,excluded:ControllerOf(Tagged(")
            && compact.contains("TagKey(\"triggering\"")
            && compact.contains("AliasedTarget(Excluding"),
        "the authored choice must exclude the triggering caster and the damage must hit that choice: {debug}"
    );
    assert!(!debug.contains("ChosenPlayer"), "{debug}");
}

#[test]
pub(super) fn splinter_aging_champion_joint_draw_retains_the_other_target_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Splinter, Aging Champion")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When Splinter enters, destroy up to one target tapped creature.\nWhen Splinter leaves the battlefield, you and another target player each draw a card.",
        )
        .expect("Splinter, Aging Champion should parse");
    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    assert_eq!(
        debug.matches("DrawCardsEffect").count(),
        2,
        "both players must draw: {debug}"
    );
    assert!(
        compact.contains("Excluding{base:Any,excluded:You") && compact.contains("player:Excluding"),
        "the second draw must use the selected player other than you: {debug}"
    );
}

#[test]
pub(super) fn parse_trigger_clause_supports_one_or_more_energy_player_gain() {
    let tokens = lex_line("you get one or more {E}", 0)
        .expect("rewrite lexer should tokenize one-or-more energy trigger clause");
    let parsed =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &tokens,
        );
    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::TriggerSpec::PlayerGetsCounters {
                player: crate::target::PlayerFilter::You,
                counter_type: Some(crate::object::CounterType::Energy),
                one_or_more: true,
            })
        ),
        "expected one-or-more energy player-counters trigger, got {parsed:?}"
    );
}

#[test]
pub(super) fn parse_trigger_clause_supports_one_or_more_creature_tokens_created_by_you() {
    let tokens = lex_line("you create one or more creature tokens", 0)
        .expect("rewrite lexer should tokenize one-or-more token-created trigger clause");
    let parsed =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &tokens,
        );
    let Ok(crate::cards::builders::TriggerSpec::TokensCreated {
        player,
        filter,
        one_or_more,
    }) = parsed
    else {
        panic!("expected token-created trigger, got {parsed:?}");
    };

    assert_eq!(player, crate::target::PlayerFilter::You);
    assert!(one_or_more, "expected one-or-more count mode");
    assert!(
        filter.token,
        "expected creature tokens to keep token filter"
    );
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
}

#[test]
pub(super) fn chroma_full_cards_keep_filtered_mana_symbol_aggregates() {
    fn compact_debug(debug: &str) -> String {
        debug
            .chars()
            .filter(|character| !character.is_whitespace() && *character != ',')
            .collect()
    }

    fn parsed_debug(name: &str, card_type: CardType, text: &str) -> String {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should parse: {error}"));
        format!("{definition:#?}")
    }

    let primalcrux = parsed_debug(
        "Primalcrux",
        CardType::Creature,
        "Trample\nChroma — This creature's power and toughness are each equal to the number of green mana symbols in the mana costs of permanents you control.",
    );
    let primalcrux_compact = compact_debug(&primalcrux);
    assert!(
        primalcrux_compact.contains("ManaSymbolsInManaCostOf{spec:All(")
            && primalcrux_compact.contains("zone:Some(Battlefield)")
            && primalcrux_compact.contains("controller:Some(You)")
            && primalcrux_compact.contains("color:Green"),
        "{primalcrux}"
    );

    let umbra = parsed_debug(
        "Umbra Stalker",
        CardType::Creature,
        "Chroma — Umbra Stalker's power and toughness are each equal to the number of black mana symbols in the mana costs of cards in your graveyard.",
    );
    let umbra_compact = compact_debug(&umbra);
    assert!(
        umbra_compact.contains("ManaSymbolsInManaCostOf{spec:All(")
            && umbra_compact.contains("zone:Some(Graveyard)")
            && umbra_compact.contains("owner:Some(You)")
            && umbra_compact.contains("color:Black"),
        "{umbra}"
    );

    for (name, text, effect_marker, color) in [
        (
            "Outrage Shaman",
            "Chroma — When this creature enters, it deals damage to target creature equal to the number of red mana symbols in the mana costs of permanents you control.",
            "DealDamageEffect",
            "Red",
        ),
        (
            "Springjack Shepherd",
            "Chroma — When this creature enters, create a 0/1 white Goat creature token for each white mana symbol in the mana costs of permanents you control.",
            "CreateTokenEffect",
            "White",
        ),
        (
            "Heartlash Cinder",
            "Haste\nChroma — When this creature enters, it gets +X/+0 until end of turn, where X is the number of red mana symbols in the mana costs of permanents you control.",
            "ModifyPowerToughness",
            "Red",
        ),
    ] {
        let debug = parsed_debug(name, CardType::Creature, text);
        let compact = compact_debug(&debug);
        assert!(debug.contains(effect_marker), "{name}: {debug}");
        assert!(
            compact.contains("ManaSymbolsInManaCostOf{spec:All(")
                && compact.contains("zone:Some(Battlefield)")
                && compact.contains("controller:Some(You)")
                && compact.contains(&format!("color:{color}")),
            "{name}: {debug}"
        );
    }

    let bombardment = parsed_debug(
        "Fiery Bombardment",
        CardType::Enchantment,
        "Chroma — {2}, Sacrifice a creature: This enchantment deals damage to any target equal to the number of red mana symbols in the sacrificed creature's mana cost.",
    );
    let bombardment_compact = compact_debug(&bombardment);
    assert!(
        bombardment_compact.contains("ManaSymbolsInManaCostOf{spec:Tagged(")
            && bombardment_compact.contains("color:Red")
            && !bombardment_compact.contains("ManaSymbolsInManaCostOf{spec:All("),
        "{bombardment}"
    );
}
