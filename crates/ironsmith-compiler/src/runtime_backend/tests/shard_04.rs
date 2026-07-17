#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_05::*;
use super::shard_06::*;
use super::*;

#[test]
fn ajani_goldmane_keeps_separate_token_ability_presentation_before_runtime_conversion() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Ajani Goldmane")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "+1: You gain 2 life.\n−6: Create a white Avatar creature token. It has \"This token's power and toughness are each equal to your life total.\"",
        )
        .expect("Ajani Goldmane should parse");
    let create = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
                .find_map(|effect| effect.as_create_token()),
            _ => None,
        })
        .expect("Ajani's ultimate must create its Avatar token");

    assert_eq!(
        create.ability_presentation,
        Some(ironsmith_core::TokenAbilityPresentation::SeparateSentence)
    );
}

#[test]
pub(super) fn rewrite_lexed_conditional_parser_routes_commaless_clause_through_structure_splitter()
{
    let text = "If at least three blue mana was spent to cast this spell create a Food token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma-less if clause");

    let parsed = super::super::effect_sentences::parse_conditional_sentence_lexed(&lexed)
        .expect("comma-less if clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate: _,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action: crate::cards::builders::SubjectVerbActionAst::CreateTokenWithMods { .. },
                        ..
                    }
                )]
            ));
        }
        other => panic!("expected conditional comma-less if clause, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_leading_may_trailing_if_keeps_condition_outside_permission() {
    let text = "You may draw an additional card if this is enchanted.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify may-if draw sentence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("may-if draw sentence should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] => {
            assert!(matches!(
                predicate,
                crate::cards::builders::PredicateAst::SourceIsEnchanted
            ));
            assert!(if_false.is_empty());
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::MayByPlayer { .. }]
            ));
        }
        other => panic!("expected conditional may draw, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_conditional_parser_keeps_if_you_dont_result_predicate() {
    let text = "If you don't, create a Treasure token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify if-you-don't conditional");

    let parsed = super::super::effect_sentences::parse_conditional_sentence_lexed(&lexed)
        .expect("if-you-don't conditional should parse");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::DidNot,
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_sentence_primitive_routes_tagged_cards_remain_exiled_through_grammar_family()
{
    let lexed = lex_line("If those cards remain exiled, create a Treasure token.", 0)
        .expect("rewrite lexer should classify tagged-cards-remain-exiled sentence");

    let parsed = super::super::effect_sentences::parse_sentence_if_tagged_cards_remain_exiled(
        super::super::effect_sentences::SubjectVerbPrimitiveClause::new(&lexed),
    )
    .expect("subject/verb primitive should succeed")
    .expect("subject/verb primitive should recognize tagged-cards-remain-exiled");
    let grammar =
        super::super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed(
            &lexed,
            super::super::effect_sentences::parse_effect_chain_lexed,
        )
        .expect("grammar conditional entrypoint should parse tagged-cards-remain-exiled sentence");

    assert_eq!(format!("{parsed:?}"), format!("{grammar:?}"));
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_keeps_when_you_do_result_prefix_after_structure_cutover()
 {
    let lexed = lex_line("When you do, draw a card.", 0)
        .expect("rewrite lexer should classify when-you-do sentence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("when-you-do sentence should parse through structure helper");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::WhenResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_lexed_gain_ability_sentence_keeps_if_you_do_result_prefix() {
    let lexed = lex_line(
        "If you do, this creature gains \"When this creature leaves the battlefield, target opponent draws a card.\"",
        0,
    )
    .expect("rewrite lexer should classify if-you-do gain-ability sentence");

    let parsed = parse_effect_sentence_lexed(&lexed).expect("if-you-do gain ability should parse");
    let debug = format!("{parsed:?}");

    assert!(
        matches!(
            parsed.as_slice(),
            [crate::cards::builders::EffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::Did,
                ..
            }]
        ),
        "{debug}"
    );
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(debug.contains("LeavesBattlefield"), "{debug}");
    assert!(
        debug.contains("ThisPermanentType(\"this creature\")"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_gain_ability_named_creature_subject_prefers_this_creature_surface() {
    crate::runtime_backend::front_end::shared::util::with_card_source_reference_context(
        "Thief of Existence",
        &[CardType::Creature],
        &[],
        || {
            let lexed = lex_line(
                "If you do, Thief of Existence gains \"When this creature leaves the battlefield, target opponent draws a card.\"",
                0,
            )
            .expect("rewrite lexer should classify named source gain-ability sentence");

            let parsed = parse_effect_sentence_lexed(&lexed)
                .expect("named source gain ability should parse");
            let debug = format!("{parsed:?}");

            assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
            assert!(
                debug.contains("ThisPermanentType(\"this creature\")"),
                "{debug}"
            );
            assert!(
                !debug.contains("FullName(\"Thief of Existence\")"),
                "{debug}"
            );
        },
    );
}

#[test]
pub(super) fn rewrite_preprocessed_named_gain_ability_keeps_card_identity_surface() {
    let (semantic, _) = parse_text_to_semantic_document(
        CardDefinitionBuilder::new(CardId::from_raw(1), "Thief of Existence")
            .card_types(vec![CardType::Creature]),
        "Thief of Existence gains \"When this creature leaves the battlefield, target opponent draws a card.\"."
            .to_string(),
        false,
    )
    .expect("preprocessed named source gain ability should parse semantically");
    let normalized = semantic
        .items
        .iter()
        .find_map(rewrite_parsed_line)
        .map(|line| line.info.normalized.normalized.as_str());

    assert_eq!(
        normalized,
        Some(
            "this creature gains \"when this creature leaves the battlefield, target opponent draws a card. \"."
        )
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_keeps_trailing_if_clause_after_structure_cutover() {
    let lexed = lex_line("Destroy target creature if it's white.", 0)
        .expect("rewrite lexer should classify trailing-if sentence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("trailing-if sentence should parse through structure helper");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::Conditional { .. }]
    ));
}

#[test]
pub(super) fn rewrite_copy_clause_keeps_trailing_if_after_structure_cutover() {
    let tokens = lex_line("Copy it if it's blue", 0)
        .expect("rewrite lexer should classify copy clause with trailing if");

    let parsed = super::super::clause_pattern_helpers::parse_copy_spell_clause(&tokens)
        .expect("copy clause parser should succeed")
        .expect("copy clause should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("CopySpell"), "{debug}");
    assert!(debug.contains("ItMatches"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_delayed_this_turn_trigger_via_lexed_trigger_parser()
 {
    let lexed = lex_line("When this creature dies this turn, exile this creature.", 0)
        .expect("rewrite lexer should classify delayed this-turn trigger sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("lexed delayed this-turn trigger sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
    assert!(debug.contains("ThisDies"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_leading_this_turn_targeted_unblocked_trigger()
{
    let lexed = lex_line(
        "This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. If you do, it assigns no combat damage this turn.",
        0,
    )
    .expect("rewrite lexer should classify leading delayed this-turn trigger sentence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("leading delayed this-turn trigger sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
    assert!(debug.contains("AttacksAndIsntBlocked"), "{debug}");
    assert!(debug.contains("IfResult"), "{debug}");
    assert!(debug.contains("AssignNoCombatDamage"), "{debug}");
    assert!(debug.contains("source: Tagged"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_preserves_conditional_for_leading_instead_followup() {
    let text = "If it's a Human, instead it gets +3/+3 and gains indestructible until end of turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify leading-instead conditional");

    let parsed = parse_effect_sentence_lexed(&lexed).expect("leading-instead conditional");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_for_each_player_doesnt_predicate() {
    let text = "Each player discards a card. Then each player who didn't discard a creature card this way loses 4 life.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify for-each-player-doesnt sequence");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ForEachPlayerDoesNot"), "{debug}");
    assert!(debug.contains("PlayerTaggedObjectMatches"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_each_player_on_your_team_filter() {
    let text = "Each player on your team may discard a card, then each player who discarded a card this way draws a card.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify for-each-team-player sequence");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ForEachPlayersFiltered"), "{debug}");
    assert!(debug.contains("Excluding"), "{debug}");
    assert!(debug.contains("Opponent"), "{debug}");
    assert!(debug.contains("May"), "{debug}");
    assert!(debug.contains("ForEachPlayerDid"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_builds_self_replacement_for_return_followup() {
    let text = "Return target creature card from your graveyard to your hand. If you gained 7 or more life this turn, return that card to the battlefield instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify return followup");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
    assert!(matches!(
        parsed.as_slice(),
        [EffectAst::SelfReplacement {
            attach_to_previous_ability: false,
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_builds_self_replacement_for_damage_followup() {
    let text = "This creature deals 1 damage to any target. If that land is a Mountain, this creature deals 2 damage instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify damage followup");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
    assert!(matches!(
        parsed.as_slice(),
        [EffectAst::SelfReplacement {
            attach_to_previous_ability: false,
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_builds_self_replacement_for_toxic_followup() {
    let text = "Target creature you control gets +1/+1 until end of turn. If that creature has toxic, instead it gets +2/+2 until end of turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify toxic followup");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
    assert!(matches!(
        parsed.as_slice(),
        [EffectAst::SelfReplacement {
            attach_to_previous_ability: false,
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_builds_self_replacement_for_creatures_died_count_followup()
 {
    let text = "If a creature died this turn, you draw a card and you lose 1 life. If seven or more creatures died this turn, instead you draw seven cards and you lose 7 life.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify died-count followup");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
    assert!(matches!(
        parsed.as_slice(),
        [EffectAst::SelfReplacement {
            attach_to_previous_ability: true,
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_builds_self_replacement_for_full_party_followup() {
    let text = "Creatures you control get +1/+0 until end of turn. If you have a full party, creatures you control get +3/+0 until end of turn instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify full-party followup");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_wraps_extra_turn_end_step_followup() {
    let text = "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify extra-turn followup");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("DelayedUntilEndStepOfExtraTurn"), "{debug}");
}

pub(super) fn registry_sentence_inputs(
    text: &str,
) -> Vec<super::super::effect_sentences::SentenceInput> {
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify registry test text");
    split_lexed_sentences(&lexed)
        .into_iter()
        .map(super::super::effect_sentences::SentenceInput::from_lexed)
        .collect()
}

#[test]
pub(super) fn rewrite_sequence_registry_keeps_initial_exile_collection_across_player_actions() {
    let sentences = registry_sentence_inputs(
        "Exile all creatures. Each player may put any number of creature cards from their hand onto the battlefield. Then put all cards exiled this way into their owners' hands. Exile this spell.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match exiled-collection return bundle");
    assert_eq!(
        matched.name,
        "exile-each-player-put-return-exiled-exile-source"
    );
    assert_eq!(matched.consumed_sentences, 4);

    let [
        crate::cards::builders::EffectAst::TagAffected { tag, .. },
        _,
        return_exiled,
        _,
    ] = matched.effects.as_slice()
    else {
        panic!(
            "expected tagged exile collection and return: {:#?}",
            matched.effects
        );
    };
    assert!(matches!(
        return_exiled,
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::MoveToZone {
                    target: crate::cards::builders::TargetAst::Tagged(return_tag, _),
                    zone: crate::zone::Zone::Hand,
                    ..
                },
                ..
            }
        ) if return_tag == tag
    ));
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_reciprocal_control_after_initial_untap() {
    let sentences = registry_sentence_inputs(
        "Untap all creatures you control and all creatures target opponent controls. You and that opponent each gain control of all creatures the other controls until end of turn. Those creatures gain haste until end of turn.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match initial-untap reciprocal control bundle");
    assert_eq!(matched.name, "reciprocal-creature-control");
    assert_eq!(matched.consumed_sentences, 3);
    let debug = format!("{:#?}", matched.effects);
    assert!(debug.contains("TagMatchingObjects"), "{debug}");
    assert!(debug.contains("GainControl"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_search_upkeep_lose_game_bundle() {
    let sentences = registry_sentence_inputs(
        "search your library for a green creature card, reveal it, put it into your hand, then shuffle. at the beginning of your next upkeep, pay {2}{g}{g}. if you don't, you lose the game.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match search upkeep bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "effect-then-next-upkeep-unless-pays-lose-game"
    );
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("DelayedUntilNextUpkeep"), "{debug}");
    assert!(debug.contains("LoseGame"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_counterspell_upkeep_lose_game_bundle() {
    let sentences = registry_sentence_inputs(
        "counter target spell. at the beginning of your next upkeep, pay {3}{u}{u}. if you don't, you lose the game.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match pact upkeep bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "effect-then-next-upkeep-unless-pays-lose-game"
    );
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("Counter"), "{debug}");
    assert!(debug.contains("DelayedUntilNextUpkeep"), "{debug}");
    assert!(debug.contains("LoseGame"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_look_one_hand_other_bottom_bundle() {
    let text = "look at the top two cards of your library. put one of them into your hand and the other on the bottom of your library.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify looked-card bundle");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("looked-card hand/bottom sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_look_one_hand_other_graveyard_bundle() {
    let text = "look at the top two cards of your library. put one of them into your hand and the other into your graveyard.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify looked-card bundle");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("looked-card hand/graveyard sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("ForEachTagged"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_reveal_top_may_put_match_rest_graveyard_bundle() {
    let sentences = registry_sentence_inputs(
        "Reveal the top five cards of your library. You may put a creature or enchantment card from among them into your hand. Put the rest into your graveyard.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match reveal-top hand/graveyard bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "top-cards-put-match-into-hand-rest-graveyard");
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("min: 0"), "{debug}");
    assert!(debug.contains("zone: Library"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn flow_state_replacement_reuses_looked_source_and_requires_both_card_types() {
    let text = "Look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order. If there is an instant card and a sorcery card in your graveyard, instead put two of them into your hand and the rest on the bottom of your library in any order.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Flow State")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("Flow State should lower with its looked-card self-replacement");
    let program = def.spell_effect.as_ref().expect("spell resolution");
    let segment = program.segments.first().expect("resolution segment");
    let default_look = segment.default_effects[0]
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .expect("default looked-card producer");
    let replacement = segment
        .self_replacements
        .first()
        .expect("graveyard-card replacement");

    let crate::effect::Condition::And(left, right) = &replacement.condition else {
        panic!(
            "independently articled graveyard cards should be conjunctive: {:#?}",
            replacement.condition
        );
    };
    for (condition, expected_type) in [left.as_ref(), right.as_ref()]
        .into_iter()
        .zip([CardType::Instant, CardType::Sorcery])
    {
        let crate::effect::Condition::PlayerControls { player, filter } = condition else {
            panic!("expected a typed graveyard existential: {condition:#?}");
        };
        assert_eq!(*player, crate::target::PlayerFilter::You);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(crate::target::PlayerFilter::You));
        assert_eq!(filter.card_types, vec![expected_type]);
    }

    let replacement_look = replacement.replacement_effects[0]
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .expect("replacement should execute the looked-card producer");
    assert_eq!(replacement_look.tag, default_look.tag);
    let choose = replacement
        .replacement_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .expect("replacement two-card choice");
    assert_eq!(choose.count, crate::effect::ChoiceCount::exactly(2));
    assert!(choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == default_look.tag
    }));
    let remainder = replacement
        .replacement_effects
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        })
        .expect("replacement looked-card remainder");
    assert_eq!(remainder.tag, default_look.tag);
}

#[test]
pub(super) fn gather_the_pack_replacement_keeps_revealed_source_set_for_remainder() {
    let text = "Reveal the top five cards of your library. You may put a creature card from among them into your hand. Put the rest into your graveyard.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, put up to two creature cards from among the revealed cards into your hand instead of one.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Gather the Pack")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("Gather the Pack should lower with its self-replacement");
    let program = def.spell_effect.as_ref().expect("spell resolution");
    let segment = program.segments.first().expect("resolution segment");
    let replacement = segment
        .self_replacements
        .first()
        .expect("spell-mastery replacement");
    let default_look = segment.default_effects[0]
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .expect("default revealed-card producer");
    let replacement_look = replacement.replacement_effects[0]
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .expect("replacement should execute the revealed-card producer");
    assert_eq!(replacement_look.tag, default_look.tag);
    let replacement_reveal = replacement.replacement_effects[1]
        .downcast_ref::<crate::effects::RevealTaggedEffect>()
        .expect("replacement should reveal the produced collection");
    assert_eq!(replacement_reveal.tag, default_look.tag);
    let choose = replacement
        .replacement_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .expect("replacement creature choice");
    assert_eq!(choose.zone, Some(Zone::Library));
    assert_eq!(choose.filter.zone, Some(Zone::Library));
    assert_eq!(choose.count, crate::effect::ChoiceCount::up_to(2));
    assert_eq!(choose.filter.card_types, vec![CardType::Creature]);
    let revealed_source = choose
        .filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint
                    .tag
                    .as_str()
                    .starts_with("__sentence_helper_revealed_")
        })
        .map(|constraint| constraint.tag.clone())
        .expect("choice should be scoped to the revealed collection");
    let remainder = replacement
        .replacement_effects
        .iter()
        .filter_map(|effect| {
            effect.downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
        })
        .find(|for_each| for_each.tag == revealed_source)
        .expect("remainder should iterate the revealed source collection");
    assert_ne!(remainder.tag, choose.tag);
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_reveal_top_counted_match_hand_rest_graveyard_bundle()
 {
    let text = "Reveal the top five cards of your library. Put up to two instant and/or sorcery cards from among them into your hand and the rest into your graveyard.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify counted looked-card bundle");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("counted looked-card hand/graveyard sequence");
    let debug = format!("{parsed:#?}");
    let compact_debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("Instant"), "{debug}");
    assert!(debug.contains("Sorcery"), "{debug}");
    assert!(debug.contains("min: 0"), "{debug}");
    assert!(compact_debug.contains("max: Some(2)"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_splits_and_or_subtype_reveal_choices_from_looked_cards()
{
    let text = "Look at the top four cards of your library. You may reveal a Cleric card, a Rogue card, a Warrior card, and/or a Wizard card from among them and put those cards into your hand. Put the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify repeated subtype looked-card bundle");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("repeated subtype looked-card hand/bottom sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(
        debug.matches("ChooseObjects").count() >= 4,
        "expected one choice per listed subtype, got {debug}"
    );
    for subtype in ["Cleric", "Rogue", "Warrior", "Wizard"] {
        assert!(debug.contains(subtype), "missing {subtype} branch: {debug}");
    }
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_from_among_battlefield_rest_hand_bundle() {
    let text = "Look at the top five cards of your library. Put any number of permanent cards from among them onto the battlefield and the rest into your hand.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify Genesis-style looked-card bundle");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("looked-card battlefield/hand sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("MoveTaggedGroupToZone"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(!debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_counted_battlefield_rest_bottom_bundle() {
    let text = "Look at the top seven cards of your library. Put up to two planeswalker cards from among them onto the battlefield. Put the rest on the bottom of your library in a random order.";
    let sentences = registry_sentence_inputs(text);

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match counted looked-card battlefield/bottom sequence");
    let debug = format!("{:?}", matched.effects);

    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("max: Some(2)"), "{debug}");
    assert!(debug.contains("ForEachTagged"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_moves_all_matching_viewed_cards_without_choice() {
    let sentences = registry_sentence_inputs(
        "Reveal the top X cards of your library. Put all land cards from among them onto the battlefield tapped and the rest on the bottom of your library in a random order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match mandatory all-matching remainder sequence");
    let debug = format!("{:#?}", matched.effects);

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("TagMatchingObjects"), "{debug}");
    assert!(!debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("battlefield_tapped: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_dynamic_battlefield_rest_bottom_bundle() {
    let text = "Look at the top seven cards of your library. Put up to X artifact and/or creature cards with mana value 3 or less from among them onto the battlefield. Put the rest on the bottom of your library in a random order.";
    let sentences = registry_sentence_inputs(text);

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match dynamic looked-card battlefield/bottom sequence");
    let debug = format!("{:?}", matched.effects);

    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("dynamic_x: true"), "{debug}");
    assert!(debug.contains("up_to_x: true"), "{debug}");
    assert_eq!(
        debug.matches("mana_value: Some").count(),
        2,
        "expected shared mana-value cap on both artifact and creature branches: {debug}"
    );
    assert!(debug.contains("ForEachTagged"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_preserves_dynamic_revealed_matching_set_and_remainder() {
    let sentences = registry_sentence_inputs(
        "Reveal the top X cards of your library. Put all creature cards revealed this way into your hand and the rest on the bottom of your library in any order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match a dynamic revealed-set partition");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "reveal-top-matching-into-hand-rest-graveyard");
    assert!(debug.contains("count: X"), "{debug}");
    assert!(debug.contains("TagMatchingObjects"), "{debug}");
    assert!(
        debug.contains("card_types: [") && debug.contains("Creature"),
        "{debug}"
    );
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_keeps_conditional_looked_partition_in_one_result_branch() {
    let sentences = registry_sentence_inputs(
        "If you do, look at the top two cards of your library. Put one of them into your hand and the other into your graveyard.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match a conditional looked-card partition");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "look-at-top-partition-selected-and-remainder");
    assert!(debug.contains("IfResult"), "{debug}");
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("TagMatchingObjects"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_builds_one_hidden_pile_then_cloaks_it() {
    let sentences = registry_sentence_inputs(
        "Exile target nontoken creature you own and the top two cards of your library in a face-down pile, shuffle that pile, then cloak those cards. They enter tapped.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match the hidden-pile cloak procedure");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "exile-face-down-pile-then-cloak-tapped");
    assert_eq!(matched.consumed_sentences, 2);
    assert!(debug.contains("ExileTopOfLibrary"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(debug.contains("accumulated_tags"), "{debug}");
    assert!(debug.contains("cloak: true"), "{debug}");
    assert!(debug.contains("shuffle_before: true"), "{debug}");
    assert!(debug.contains("tapped: true"), "{debug}");
    assert!(!debug.contains("LookAtTopCards"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_noncreature_nonland_permanent_filter() {
    let text = "Look at the top seven cards of your library. Put up to two noncreature, nonland permanent cards with mana value 3 or less from among them onto the battlefield. Put the rest on the bottom of your library in a random order.";
    let sentences = registry_sentence_inputs(text);

    let matched = super::super::effect_sentences::try_parse_subject_verb_sequence_rule(
        &sentences, 0,
    )
    .expect("registry lookup should not error")
    .expect("registry should match restricted permanent looked-card battlefield/bottom sequence");
    let debug = format!("{:?}", matched.effects);

    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(
        debug.contains("card_types: [Artifact, Creature, Enchantment, Land, Planeswalker, Battle]"),
        "{debug}"
    );
    assert!(
        debug.contains("excluded_card_types: [Creature, Land]"),
        "{debug}"
    );
    assert!(debug.contains("mana_value: Some"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_from_among_battlefield_rest_graveyard_bundle() {
    let sentences = registry_sentence_inputs(
        "Look at the top X cards of your library. You may put any number of land and/or legendary permanent cards with mana value X or less from among them onto the battlefield. Put the rest into your graveyard.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match looked-card battlefield/graveyard bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("any_of"), "{debug}");
    assert!(debug.contains("ForEachTagged"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_consult_land_cards_battlefield_tapped_rest_bottom()
{
    let sentences = registry_sentence_inputs(
        "Then reveal cards from the top of your library until you reveal X land cards, where X is the number of legendary creatures you control. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match consult land battlefield/bottom bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.consumed_sentences, 2);
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MatchCount"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("battlefield_tapped: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_consult_remainder_first_battlefield_graveyard_bundle()
 {
    let sentences = registry_sentence_inputs(
        "Target opponent reveals cards from the top of their library until they reveal a creature card. That player puts all noncreature cards revealed this way into their graveyard, then you put the creature card onto the battlefield under your control.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match Telemin-style consult bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "consult-match-into-battlefield-others-graveyard"
    );
    assert_eq!(matched.consumed_sentences, 2);
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(debug.contains("PutOntoBattlefield"), "{debug}");
    assert!(debug.contains("controller: You"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_tempting_offer_copy_spell_bundle() {
    let sentences = registry_sentence_inputs(
        "Tempting offer — Choose target instant or sorcery spell. Each opponent may copy that spell and may choose new targets for the copy they control. You copy that spell once plus an additional time for each opponent who copied the spell this way. You may choose new targets for the copies you control.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match Tempt with Mayhem-style copy bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "tempting-offer-copy-spell");
    assert_eq!(matched.consumed_sentences, 4);
    assert!(debug.contains("TargetOnly"), "{debug}");
    assert!(debug.contains("ForEachOpponent"), "{debug}");
    assert!(debug.contains("MayByPlayer"), "{debug}");
    assert!(debug.contains("PendingEffectMetricOffset"), "{debug}");
    assert!(debug.contains("PlayersWithPositiveCount"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_reciprocal_creature_control_bundle() {
    let sentences = registry_sentence_inputs(
        "You and target opponent each gain control of all creatures the other controls until end of turn. Untap those creatures. Those creatures gain haste until end of turn.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match reciprocal creature-control bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "reciprocal-creature-control");
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("TagMatchingObjects"), "{debug}");
    assert!(debug.contains("GainControl"), "{debug}");
    assert!(debug.contains("TargetOpponent"), "{debug}");
    assert!(debug.contains("UntapAll"), "{debug}");
    assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
    assert!(debug.contains("haste"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_counted_revealed_cards_hand_rest_bottom_bundle() {
    let sentences = registry_sentence_inputs(
        "Look at the top four cards of your library. You may reveal up to two instant and/or sorcery cards from among them and put the revealed cards into your hand. Put the rest on the bottom of your library in any order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match counted revealed-cards hand/bottom bundle");
    let debug = format!("{:#?}", matched.effects);
    let compact_debug = format!("{:?}", matched.effects);

    assert_eq!(
        matched.name,
        "top-cards-reveal-any-matching-to-hand-rest-bottom"
    );
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("Instant"), "{debug}");
    assert!(debug.contains("Sorcery"), "{debug}");
    assert!(compact_debug.contains("max: Some(2)"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_splits_and_or_single_revealed_cards_hand_rest_bottom_bundle()
 {
    let sentences = registry_sentence_inputs(
        "Look at the top four cards of your library. You may reveal a creature card and/or a land card from among them and put the revealed cards into your hand. Put the rest on the bottom of your library in any order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match split revealed-cards hand/bottom bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "top-cards-reveal-any-matching-to-hand-rest-bottom"
    );
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
    assert!(
        debug.matches("ChooseObjects").count() >= 2,
        "expected separate up-to-one choices for and/or card types, got {debug}"
    );
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_links_and_or_revealed_choice_to_x_destination_override() {
    let sentences = registry_sentence_inputs(
        "Reveal the top X plus one cards of your library. Choose a creature card and/or a land card from among them. Put those cards into your hand and the rest on the bottom of your library in a random order. If X is 5 or more, instead put the chosen cards onto the battlefield or into your hand and the rest on the bottom of your library in a random order.",
    );

    let choice_shape = crate::runtime_backend::grammar::effects::sequence_quad_shapes::parse_choose_looked_card_and_or_shape(
        sentences[1].lowered(),
    )
    .expect("typed looked-card choice should parse");
    assert!(choice_shape.uses_and_or);
    assert!(
        crate::runtime_backend::grammar::effects::sequence_quad_shapes::parse_chosen_cards_hand_remainder_shape(
            sentences[2].lowered(),
        )
        .is_some(),
        "default chosen-card disposition should parse"
    );
    assert!(
        crate::runtime_backend::grammar::effects::sequence_quad_shapes::parse_chosen_cards_destination_replacement_shape(
            sentences[3].lowered(),
        )
        .is_some(),
        "replacement chosen-card disposition should parse"
    );
    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match revealed and/or destination replacement bundle");
    assert_eq!(matched.name, "revealed-and-or-choice-destination-override");
    assert_eq!(matched.consumed_sentences, 4);

    let [
        crate::cards::builders::EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability,
        },
    ] = matched.effects.as_slice()
    else {
        panic!(
            "expected one typed self-replacement, got {:#?}",
            matched.effects
        );
    };
    assert!(!*attach_to_previous_ability);
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ValueComparison {
            left: crate::effect::Value::X,
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(5),
        }
    ));

    let default_debug = format!("{if_false:#?}");
    let replacement_debug = format!("{if_true:#?}");
    for debug in [&default_debug, &replacement_debug] {
        assert!(debug.contains("LookAtTopCards"), "{debug}");
        assert_eq!(
            debug.matches("ChooseTaggedObjectsInZone").count(),
            2,
            "expected one independently tagged choice per and/or card type: {debug}"
        );
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Land"), "{debug}");
        assert!(debug.contains("IsTaggedObject"), "{debug}");
        assert!(debug.contains("IsNotTaggedObject"), "{debug}");
        assert!(
            debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
            "{debug}"
        );
    }
    assert!(default_debug.contains("MoveTaggedGroupToZone"));
    assert!(replacement_debug.contains("ChooseOneOf"));
    assert!(replacement_debug.contains("Battlefield"));
    assert!(replacement_debug.contains("Hand"));
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_reveal_one_gain_mana_value_other_revealed_graveyard_bundle()
 {
    let sentences = registry_sentence_inputs(
        "Reveal the top three cards of your library and put one of them into your hand. You gain life equal to that card's mana value. Put all other cards revealed this way into your graveyard.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match revealed-card value/remainder bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "reveal-top-one-hand-gain-mana-value-rest-graveyard"
    );
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("reveal: true"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_tap_lock_followup() {
    let sentences = registry_sentence_inputs(
        "tap all creatures target player controls. they don't untap during their controllers' next untap steps for as long as this artifact remains tapped.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match tap-lock bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "tap-all-then-they-dont-untap-while-source-tapped"
    );
    assert_eq!(matched.consumed_sentences, 2);
    assert!(debug.contains("TapAll"), "{debug}");
    assert!(debug.contains("Untap"), "{debug}");
    assert!(debug.contains("SourceIsTapped"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_damage_prevention_counter_followup() {
    let sentences = registry_sentence_inputs(
        "prevent the next 1 damage that would be dealt to target creature this turn. for each 1 damage prevented this way, put a +1/+1 counter on it.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match damage-prevention bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "damage-prevention-then-put-counters");
    assert_eq!(matched.consumed_sentences, 2);
    assert!(
        debug.contains("PreventDamageToTargetPutCounters"),
        "{debug}"
    );
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
}

#[test]
pub(super) fn rewrite_subject_verb_damage_replacement_counter_clause_uses_captured_target() {
    let tokens = lex_line(
        "If damage would be dealt to target creature this turn, prevent that damage and put that many +1/+1 counters on it.",
        0,
    )
    .expect("damage replacement counter clause should lex");

    let parsed = super::super::effect_sentences::parse_top_level_subject_verb_recognition(&tokens)
        .expect("damage replacement counter clause should parse")
        .expect("subject-verb recognizer should match damage replacement counter clause");
    let debug = format!("{:#?}", parsed.1);

    assert_eq!(
        parsed.0,
        "subject-verb verb=Prevent subject=implicit recognizer=damage-replacement-counters"
    );
    assert!(
        debug.contains("PreventDamageToTargetPutCounters")
            && debug.contains("target: Object(")
            && debug.contains("PlusOnePlusOne"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_subject_verb_cant_blocked_then_base_pt_keeps_both_effects() {
    let tokens = lex_line(
        "That creature can't be blocked this turn and has base power and toughness 1/1 until end of turn.",
        0,
    )
    .expect("cant-blocked/base-pt line should lex");

    let parsed = super::super::effect_sentences::parse_top_level_subject_verb_recognition(&tokens)
        .expect("cant-blocked/base-pt line should parse")
        .expect("subject-verb recognizer should match cant-blocked/base-pt line");
    let debug = format!("{:#?}", parsed.1);

    assert_eq!(
        parsed.0,
        "subject-verb verb=Cant subject=target recognizer=cant-blocked-base-pt"
    );
    assert!(debug.contains("Cant"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
}

#[test]
pub(super) fn rewrite_subject_verb_meld_result_uses_captured_name() {
    let tokens = lex_line("Exile them, then meld them into Chittering Host.", 0)
        .expect("meld result clause should lex");

    let parsed = super::super::effect_sentences::parse_top_level_subject_verb_recognition(&tokens)
        .expect("meld result clause should parse")
        .expect("subject-verb recognizer should match meld result clause");
    let debug = format!("{:#?}", parsed.1);

    assert_eq!(
        parsed.0,
        "subject-verb verb=Meld subject=explicit recognizer=meld-result"
    );
    assert!(
        debug.contains("Meld")
            && debug.contains("chittering host")
            && !debug.contains("then meld them into"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_may_cast_target_graveyard_spell_replacement() {
    let sentences = registry_sentence_inputs(
        "You may cast target instant or sorcery card from your graveyard. If that spell would be put into your graveyard, exile it instead.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match may-cast/replacement bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "may-cast-target-graveyard-spell-then-exile-replacement"
    );
    assert_eq!(matched.consumed_sentences, 2);
    assert!(debug.contains("TargetOnly"), "{debug}");
    assert!(debug.contains("TagAffected"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("IfResult"), "{debug}");
    assert!(debug.contains("RegisterFutureZoneReplacement"), "{debug}");
    assert!(debug.contains("cause_policy: Any"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_preserves_may_cast_target_graveyard_spell_mana_value_limit()
{
    let sentences = registry_sentence_inputs(
        "You may cast target instant or sorcery card with mana value 4 or less from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match may-cast/replacement bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "may-cast-target-graveyard-spell-then-exile-replacement"
    );
    assert!(
        debug.contains("LessThanOrEqual") && debug.contains("4,"),
        "{debug}"
    );
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_may_cast_target_graveyard_artifact_or_spell_replacement()
 {
    let sentences = registry_sentence_inputs(
        "You may cast target instant, sorcery, or artifact card from your graveyard without paying its mana cost. If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match targeted graveyard free-cast/replacement bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "may-cast-target-graveyard-spell-then-exile-replacement"
    );
    assert_eq!(matched.consumed_sentences, 2);
    assert!(debug.contains("TargetOnly"), "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("RegisterFutureZoneReplacement"), "{debug}");
}

#[test]
pub(super) fn rewrite_indefinite_creature_death_uses_multi_use_future_replacement() {
    let lexed = lex_line("If a creature would die this turn, exile it instead.", 0)
        .expect("future creature replacement should lex");
    let parsed =
        parse_effect_sentence_lexed(&lexed).expect("future creature replacement should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("RegisterFutureZoneReplacement"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("duration: UntilEndOfTurn"), "{debug}");
    assert!(debug.contains("cause_policy: Any"), "{debug}");
    assert!(debug.contains("link_exiled_to_source: false"), "{debug}");
}

#[test]
pub(super) fn rewrite_filtered_future_exile_and_delayed_return_links_all_objects() {
    let sentences = registry_sentence_inputs(
        "If a permanent you control would be put into a graveyard from the battlefield this turn, exile it instead. Return it to the battlefield under its owner's control at the beginning of the next end step.",
    );
    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("filtered future replacement sequence should match");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "filtered-future-exile-then-return-next-end-step"
    );
    assert_eq!(matched.consumed_sentences, 2);
    assert!(debug.contains("duration: UntilEndOfTurn"), "{debug}");
    assert!(debug.contains("cause_policy: Any"), "{debug}");
    assert!(debug.contains("link_exiled_to_source: true"), "{debug}");
    assert!(debug.contains("DelayedUntilNextEndStep"), "{debug}");
    assert!(debug.contains(crate::tag::SOURCE_EXILED_TAG), "{debug}");
    assert!(debug.contains("controller: Owner"), "{debug}");
}

#[test]
pub(super) fn rewrite_dealt_damage_by_source_would_die_static_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kumano's Blessing Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\nIf a creature dealt damage by enchanted creature this turn would die, exile it instead.",
        )
        .expect("source-damaged static replacement should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("ExileWouldDieInstead"), "{debug}");
    assert!(debug.contains("damaged_by: Some"), "{debug}");
    assert!(debug.contains("EnchantedCreature"), "{debug}");
}

#[test]
pub(super) fn rewrite_nontoken_opponent_creature_would_die_static_replacement_with_token_followup()
{
    let def = CardDefinitionBuilder::new(CardId::new(), "Kalitas Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token.",
        )
        .expect("nontoken opponent creature replacement should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("ExileWouldDieInstead"), "{debug}");
    assert!(debug.contains("nontoken: true"), "{debug}");
    assert!(debug.contains("follow_up_effects"), "{debug}");
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
    assert!(debug.contains("name: \"Zombie\""), "{debug}");
}

#[test]
pub(super) fn rewrite_nontoken_opponent_creature_would_die_with_counter_static_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Draugr Necromancer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If a nontoken creature an opponent controls would die, exile that card with an ice counter on it instead.",
        )
        .expect("ice-counter exile replacement should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("ExileWouldDieInstead"), "{debug}");
    assert!(debug.contains("nontoken: true"), "{debug}");
    assert!(debug.contains("exile_with_counters"), "{debug}");
    assert!(debug.contains("Ice"), "{debug}");
    assert!(debug.contains("follow_up_effects: []"), "{debug}");
}

#[test]
pub(super) fn rewrite_rayami_nontoken_creature_would_die_with_blood_counter_static_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rayami, First of the Fallen")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If a nontoken creature would die, exile that card with a blood counter on it instead.",
        )
        .expect("rayami replacement clause should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("ExileWouldDieInstead"), "{debug}");
    assert!(debug.contains("nontoken: true"), "{debug}");
    assert!(debug.contains("Blood"), "{debug}");
}

#[test]
pub(super) fn rewrite_this_creature_would_die_static_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Self-Exiling Creature Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("If this creature would die, exile it instead.")
        .expect("self death replacement should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("ExileWouldDieInstead"), "{debug}");
    assert!(debug.contains("source: true"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
}

#[test]
pub(super) fn rewrite_exile_counter_cast_permission_with_mana_permission_static_line() {
    let line = "You may cast spells from among cards in exile your opponents own with ice counters on them, and you may spend mana from snow sources as though it were mana of any color to cast those spells.";
    let tokens = lex_line(line, 0).expect("permission line should lex");
    let direct =
        super::super::keyword_static::parse_you_may_cast_exile_counter_cards_with_mana_permission_line(
            &tokens,
        )
        .expect("direct parser should not error");
    assert!(
        direct.is_some(),
        "expected direct parser to accept {:?}",
        crate::runtime_backend::token_word_refs(&tokens)
    );
    let direct_abilities = direct.expect("direct parser should produce abilities");
    assert_eq!(direct_abilities.len(), 2);
    let grant = direct_abilities
        .iter()
        .find_map(|ability| match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::Grants(spec) => Some(spec),
            _ => None,
        })
        .expect("expected PlayFrom grant");
    assert!(matches!(
        &grant.grantable,
        crate::grant::Grantable::PlayFrom
    ));
    assert_eq!(grant.zone, crate::zone::Zone::Exile);
    assert_eq!(grant.beneficiary, crate::filter::PlayerFilter::You);
    assert_eq!(grant.filter.zone, Some(crate::zone::Zone::Exile));
    assert_eq!(
        grant.filter.owner,
        Some(crate::filter::PlayerFilter::Opponent)
    );
    assert!(grant.filter.excluded_card_types.contains(&CardType::Land));
    assert_eq!(
        grant.filter.with_counter,
        Some(crate::filter::CounterConstraint::Typed(CounterType::Ice))
    );
    assert_eq!(
        grant.display(),
        "You may cast spells from among cards in exile your opponents own with ice counters on them"
    );

    let permission = direct_abilities
        .iter()
        .find_map(|ability| match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::ManaSpendPermission {
                permission,
                ..
            } => {
                assert_eq!(permission.player, crate::filter::PlayerFilter::You);
                Some(permission)
            }
            _ => None,
        })
        .expect("expected mana spend permission");
    let mana_filter = match &permission.scope {
        ironsmith_core::ManaSpendScope::CastingSpellsMatching(filter) => filter,
        other => panic!("expected casting mana permission, got {other:?}"),
    };
    assert_eq!(mana_filter.zone, Some(crate::zone::Zone::Exile));
    assert_eq!(
        mana_filter.owner,
        Some(crate::filter::PlayerFilter::Opponent)
    );
    assert_eq!(
        mana_filter.with_counter,
        Some(crate::filter::CounterConstraint::Typed(CounterType::Ice))
    );
    let source_filter = permission
        .mana_source_filter
        .as_ref()
        .expect("expected snow source restriction");
    assert!(source_filter.supertypes.contains(&Supertype::Snow));

    let parsed_static = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("static permission parser should not error");
    assert!(
        parsed_static.is_some(),
        "expected static parser to accept {:?}",
        crate::runtime_backend::token_word_refs(&tokens)
    );

    let def = CardDefinitionBuilder::new(CardId::new(), "Draugr Necromancer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(line)
        .expect("exile counter cast permission should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("Grants"), "{debug}");
    assert!(debug.contains("grantable: PlayFrom"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
    assert!(debug.contains("Opponent"), "{debug}");
    assert!(debug.contains("Ice"), "{debug}");
    assert!(debug.contains("ManaSpendPermission"), "{debug}");
    assert!(debug.contains("CastingSpellsMatching"), "{debug}");
}

#[test]
pub(super) fn rewrite_source_exiled_counter_play_and_cast_permission_static_line() {
    let line = "You may play lands and cast noncreature spells from among cards you exiled that have fetch counters on them, and you may spend mana as though it were mana of any color to cast those spells.";
    let tokens = lex_line(line, 0).expect("Haldan-style permission line should lex");
    let direct =
        super::super::keyword_static::parse_you_may_cast_exile_counter_cards_with_mana_permission_line(
            &tokens,
        )
        .expect("direct parser should not error");
    assert!(
        direct.is_some(),
        "expected direct parser to accept {:?}",
        crate::runtime_backend::token_word_refs(&tokens)
    );

    let direct_abilities = direct.expect("direct parser should produce abilities");
    assert_eq!(direct_abilities.len(), 2);
    let grant = direct_abilities
        .iter()
        .find_map(|ability| match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::Grants(spec) => Some(spec),
            _ => None,
        })
        .expect("expected PlayFrom grant");

    assert!(matches!(
        &grant.grantable,
        crate::grant::Grantable::PlayFrom
    ));
    assert_eq!(grant.zone, crate::zone::Zone::Exile);
    assert_eq!(grant.beneficiary, crate::filter::PlayerFilter::You);
    assert_eq!(grant.filter.any_of.len(), 2);
    assert!(grant.filter.any_of.iter().any(|candidate| {
        candidate.card_types == vec![CardType::Land]
            && candidate.zone == Some(crate::zone::Zone::Exile)
            && candidate
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    }));
    assert!(grant.filter.any_of.iter().any(|candidate| {
        candidate.excluded_card_types.contains(&CardType::Creature)
            && candidate.excluded_card_types.contains(&CardType::Land)
            && candidate.with_counter
                == Some(crate::filter::CounterConstraint::Typed(CounterType::Named(
                    "fetch",
                )))
    }));
    assert_eq!(
        grant.display(),
        "You may play lands and cast noncreature spells from among cards you exiled that have fetch counters on them"
    );

    let permission = direct_abilities
        .iter()
        .find_map(|ability| match &ability.payload {
            crate::static_abilities::StaticAbilityPayload::ManaSpendPermission {
                permission,
                ..
            } => {
                assert_eq!(permission.player, crate::filter::PlayerFilter::You);
                Some(permission)
            }
            _ => None,
        })
        .expect("expected mana spend permission");
    let mana_filter = match &permission.scope {
        ironsmith_core::ManaSpendScope::CastingSpellsMatching(filter) => filter,
        other => panic!("expected casting mana permission, got {other:?}"),
    };
    assert_eq!(mana_filter.any_of.len(), 2);
    assert!(mana_filter.any_of.iter().all(|candidate| {
        candidate.zone == Some(crate::zone::Zone::Exile)
            && candidate
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    }));

    let def = CardDefinitionBuilder::new(CardId::new(), "Haldan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(line)
        .expect("Haldan-style line should parse");
    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("Grantable::PlayFrom") || debug.contains("grantable: PlayFrom"),
        "{debug}"
    );
    assert!(debug.contains("fetch"), "{debug}");
    assert!(debug.contains("ManaSpendPermission"), "{debug}");
}

#[test]
pub(super) fn rewrite_source_you_control_noncombat_damage_to_opponent_creature_as_counters() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Soul-Scar Mage Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If a source you control would deal noncombat damage to a creature an opponent controls, put that many -1/-1 counters on that creature instead.",
        )
        .expect("damage-to-counters replacement should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        debug.contains("ReplaceDamageWithCountersInstead"),
        "{debug}"
    );
    assert!(debug.contains("MinusOneMinusOne"), "{debug}");
    assert!(debug.contains("combat_only: Some"), "{debug}");
}

#[test]
pub(super) fn rewrite_prowess_keyword_lowers_to_spell_cast_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Prowess Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Prowess")
        .expect("prowess keyword should parse");
    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();

    assert!(debug.contains("Triggered"), "{debug}");
    assert!(debug.contains("SpellCast"), "{debug}");
    assert!(debug.contains("ModifyPowerToughnessEffect"), "{debug}");
    assert!(compact.contains("Keyword(Prowess"), "{debug}");
}

#[test]
pub(super) fn rewrite_damage_this_way_would_die_registers_source_history_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Yamabushi's Storm Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Yamabushi's Storm deals 1 damage to each creature. If a creature dealt damage this way would die this turn, exile it instead.",
        )
        .expect("damage-this-way replacement should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(
        debug.contains("RegisterDamagedBySourceZoneReplacementEffect"),
        "{debug}"
    );
    assert!(debug.contains("mode: UntilEndOfTurn"), "{debug}");
}

#[test]
pub(super) fn rewrite_serial_damage_fanout_emits_distinct_damage_effects() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Serpentine Spike")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Serpentine Spike deals 2 damage to target creature, 3 damage to another target creature, and 4 damage to a third target creature. If a creature dealt damage this way would die this turn, exile it instead.",
        )
        .expect("serial damage fanout should parse");
    let debug = format!("{:#?}", def.spell_effect);
    let compact = debug.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(debug.matches("DealDamageEffect").count() >= 3, "{debug}");
    assert!(compact.contains("amount: Fixed( 2, )"), "{debug}");
    assert!(compact.contains("amount: Fixed( 3, )"), "{debug}");
    assert!(compact.contains("amount: Fixed( 4, )"), "{debug}");
    assert!(
        debug.contains("RegisterDamagedBySourceZoneReplacementEffect"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_destroy_all_keeps_named_counter_filter() {
    let lexed = lex_line("Destroy each permanent with a doom counter on it.", 0)
        .expect("rewrite lexer should classify destroy-with-counter text");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("destroy-with-counter sentence should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DestroyAll"), "{debug}");
    assert!(debug.contains("with_counter: Some"), "{debug}");
    assert!(debug.contains("doom"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_destroy_one_of_those_at_random_keeps_tagged_random_target() {
    let lexed = lex_line("Destroy one of those permanents at random.", 0)
        .expect("rewrite lexer should classify destroy-one-of-those text");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("destroy-one-of-those sentence should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("Destroy"), "{debug}");
    assert!(debug.contains("WithCount"), "{debug}");
    assert!(debug.contains("random: true"), "{debug}");
    assert!(debug.contains("TaggedObjectConstraint"), "{debug}");
}

#[test]
pub(super) fn rewrite_lowered_intervening_counter_gate_binds_destroy_to_gate_filter()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Counter Gate Variant")
        .card_types(vec![CardType::Enchantment]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "At the beginning of your end step, if two or more permanents you don't control have an aim counter on them, destroy one of those permanents at random."
            .to_string(),
        false,
    )?;
    let debug = format!("{definition:#?}");

    assert!(debug.contains("DestroyEffect"), "{debug}");
    assert!(debug.contains("random: true"), "{debug}");
    assert!(
        debug.matches("with_counter: Some").count() >= 2,
        "expected both the intervening condition and destroy target to keep the counter filter, got {debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_spell_cast_trigger_keeps_comma_color_list()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Comma Color Spell")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever you cast a spell that's white, blue, black, or red, put a +1/+1 counter on this creature."
            .to_string(),
        false,
    )?;
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let TriggerKind::SpellCastQualified {
        filter: Some(filter),
        ..
    } = &triggered.trigger.kind
    else {
        panic!(
            "expected qualified spell-cast trigger: {:#?}",
            triggered.trigger
        );
    };
    let expected_colors = ColorSet::WHITE
        .union(ColorSet::BLUE)
        .union(ColorSet::BLACK)
        .union(ColorSet::RED);

    assert_eq!(filter.colors, Some(expected_colors));
    assert!(
        format!("{:#?}", triggered.effects).contains("PutCountersEffect"),
        "{:#?}",
        triggered.effects
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_activated_ability_trigger_keeps_comma_type_list()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Comma Type Ability")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever an opponent activates an ability of an artifact, creature, or land on the battlefield, if it isn't a mana ability, this creature deals 2 damage to that player."
            .to_string(),
        false,
    )?;
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let TriggerKind::AbilityActivatedQualified {
        filter,
        non_mana_only,
        ..
    } = &triggered.trigger.kind
    else {
        panic!(
            "expected qualified ability-activated trigger: {:#?}",
            triggered.trigger
        );
    };

    assert!(
        filter.card_types.contains(&CardType::Artifact),
        "{filter:#?}"
    );
    assert!(
        filter.card_types.contains(&CardType::Creature),
        "{filter:#?}"
    );
    assert!(filter.card_types.contains(&CardType::Land), "{filter:#?}");
    assert!(*non_mana_only, "{:#?}", triggered.trigger);
    assert!(
        format!("{:#?}", triggered.effects).contains("DealDamageEffect"),
        "{:#?}",
        triggered.effects
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_activated_ability_trigger_keeps_that_non_mana_type_list()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Activation Punisher")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 3));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever an opponent activates an ability of an artifact, creature, or land that isn't a mana ability, this creature deals 1 damage to that player.".to_string(),
        false,
    )?;
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let TriggerKind::AbilityActivatedQualified {
        filter,
        non_mana_only,
        ..
    } = &triggered.trigger.kind
    else {
        panic!(
            "expected qualified ability-activated trigger: {:#?}",
            triggered.trigger
        );
    };

    assert!(
        filter.card_types.contains(&CardType::Artifact),
        "{filter:#?}"
    );
    assert!(
        filter.card_types.contains(&CardType::Creature),
        "{filter:#?}"
    );
    assert!(filter.card_types.contains(&CardType::Land), "{filter:#?}");
    assert!(*non_mana_only, "{:#?}", triggered.trigger);
    assert!(
        format!("{:#?}", triggered.effects).contains("DealDamageEffect"),
        "{:#?}",
        triggered.effects
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_becomes_blocked_by_binds_that_creature_to_blocker()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Blocked By Artifact")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever this creature becomes blocked by an artifact creature, destroy that creature."
            .to_string(),
        false,
    )?;
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let TriggerKind::ThisBecomesBlockedByObject { filter } = &triggered.trigger.kind else {
        panic!(
            "expected by-object blocked trigger: {:#?}",
            triggered.trigger
        );
    };

    assert!(
        filter.all_card_types.contains(&CardType::Artifact),
        "{filter:#?}"
    );
    assert!(
        filter.all_card_types.contains(&CardType::Creature),
        "{filter:#?}"
    );
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("TagTriggeringBlockersEffect")
            && effects_debug.contains("\"blocking\"")
            && effects_debug.contains("DestroyEffect"),
        "{effects_debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_becomes_blocked_by_color_with_regeneration_followup_keeps_blocker_filter()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Blocked By Green")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever this creature becomes blocked by a green creature, destroy that creature. It can't be regenerated.".to_string(),
        false,
    )?;
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let TriggerKind::ThisBecomesBlockedByObject { filter } = &triggered.trigger.kind else {
        panic!(
            "expected by-object blocked trigger: {:#?}",
            triggered.trigger
        );
    };

    assert_eq!(filter.card_types, vec![CardType::Creature], "{filter:#?}");
    assert_eq!(filter.colors, Some(ColorSet::GREEN), "{filter:#?}");
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("TagTriggeringBlockersEffect")
            && effects_debug.contains("\"blocking\"")
            && effects_debug.contains("DestroyEffect"),
        "{effects_debug}"
    );
    assert!(
        format!("{:#?}", definition.abilities).contains("BeRegenerated"),
        "{:#?}",
        definition.abilities
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_generic_becomes_blocked_trigger_keeps_damage_effect()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Close Quarters")
        .card_types(vec![CardType::Enchantment]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever a creature you control becomes blocked, this enchantment deals 1 damage to any target."
            .to_string(),
        false,
    )?;
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let TriggerKind::BecomesBlocked { filter } = &triggered.trigger.kind else {
        panic!(
            "expected generic becomes-blocked trigger: {:#?}",
            triggered.trigger
        );
    };

    assert_eq!(filter.card_types, vec![CardType::Creature], "{filter:#?}");
    assert_eq!(
        filter.controller,
        Some(crate::target::PlayerFilter::You),
        "{filter:#?}"
    );
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("DealDamageEffect"),
        "{effects_debug}"
    );
    assert!(effects_debug.contains("Fixed"), "{effects_debug}");
    assert!(effects_debug.contains("AnyTarget"), "{effects_debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_delayed_combat_damage_player_trigger_this_turn_compiles()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Delayed Combat Trigger")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "This turn, whenever a creature you control deals combat damage to a player, draw a card."
            .to_string(),
        false,
    )?;
    let debug = format!("{definition:#?}");

    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("DealsCombatDamageToPlayer"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_triggered_effect_keeps_delayed_that_creature_dies_followup()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Delayed Dies Followup")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 2));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever you attack, target attacking creature gets +1/+0 until end of turn. When that creature dies this turn, surveil 1.".to_string(),
        false,
    )?;
    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("ScheduleDelayedTriggerEffect")
            && effects_debug.contains("ThisDies")
            && effects_debug.contains("target_tag: Some")
            && effects_debug.contains("until_end_of_turn: true")
            && effects_debug.contains("SurveilEffect"),
        "{effects_debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_trigger_keeps_counter_linked_land_subtype_as_effect_followup()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Counter-Linked Land Subtype")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(6, 6));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever this creature enters or attacks, put a flood counter on target land. That land is an Island in addition to its other types for as long as it has a flood counter on it."
            .to_string(),
        false,
    )?;
    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let debug = format!("{:#?}", triggered.effects);
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("AddSubtypes("), "{debug}");
    assert!(debug.contains("Island"), "{debug}");
    assert!(
        debug.contains("ForAsLongAs") && debug.contains("Flood"),
        "{debug}"
    );
    Ok(())
}

#[test]
pub(super) fn obsidian_fireheart_full_card_keeps_counter_linked_land_trigger_grant()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Obsidian Fireheart")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(4, 4));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{1}{R}{R}: Put a blaze counter on target land without a blaze counter on it. For as long as that land has a blaze counter on it, it has \"At the beginning of your upkeep, this land deals 1 damage to you.\" (The land continues to burn after this creature has left the battlefield.)"
            .to_string(),
        false,
    )?;

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("ActivatedAbility"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(
        debug.contains("Named(") && debug.contains("\"blaze\""),
        "{debug}"
    );
    assert!(debug.contains("ForAsLongAs"), "{debug}");
    assert!(debug.contains("AddAbilityGeneric"), "{debug}");
    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn mathas_full_card_keeps_counter_linked_creature_trigger_grant()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Mathas, Fiend Seeker")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Menace\nAt the beginning of your end step, put a bounty counter on target creature an opponent controls. For as long as that creature has a bounty counter on it, it has \"When this creature dies, each opponent draws a card and gains 2 life.\""
            .to_string(),
        false,
    )?;

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("Menace"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("ForAsLongAs"), "{debug}");
    assert!(debug.to_ascii_lowercase().contains("bounty"), "{debug}");
    assert!(debug.contains("AddAbilityGeneric"), "{debug}");
    assert!(debug.contains("ThisDies"), "{debug}");
    assert!(
        debug.contains("Draw") && debug.contains("GainLife"),
        "{debug}"
    );
    Ok(())
}

#[test]
pub(super) fn aquitects_will_full_card_keeps_flood_duration_and_conditional_draw()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Aquitect's Will")
        .card_types(vec![CardType::Kindred, CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Put a flood counter on target land. That land is an Island in addition to its other types for as long as it has a flood counter on it. If you control a Merfolk, draw a card."
            .to_string(),
        false,
    )?;

    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("AddSubtypes("), "{debug}");
    assert!(debug.contains("Island"), "{debug}");
    assert!(
        debug.contains("ForAsLongAs") && debug.contains("Flood"),
        "{debug}"
    );
    assert!(debug.contains("Merfolk"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_that_creature_delayed_trigger_uses_typed_attachment_fact()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Typed Delayed Followup")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 2));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever you attack, target attacking creature gets +1/+0 until end of turn. Whenever that creature is dealt damage this turn, draw a card.".to_string(),
        false,
    )?;

    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("ScheduleDelayedTriggerEffect")
            && effects_debug.contains("IsDealtDamage")
            && effects_debug.contains("DrawCardsEffect"),
        "{effects_debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_unrelated_delayed_trigger_does_not_attach_to_previous_ability()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Unrelated Delayed Trigger")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 2));
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever you attack, target attacking creature gets +1/+0 until end of turn. Whenever you draw a card this turn, gain 1 life.".to_string(),
        false,
    )?;

    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected triggered ability: {:#?}", definition.abilities);
    };
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        !effects_debug.contains("ScheduleDelayedTriggerEffect")
            && !effects_debug.contains("GainLifeEffect"),
        "{effects_debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_consult_cast_bottom_bundle() {
    let sentences = registry_sentence_inputs(
        "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost. Put all cards exiled this way that weren't cast this way on the bottom of your library in a random order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match consult cast-bottom bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "exile-until-match-cast-rest-bottom");
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_target_opponent_consult_cast_bottom_bundle() {
    let sentences = registry_sentence_inputs(
        "Target opponent exiles cards from the top of their library until they exile an instant or sorcery card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of that library in a random order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match target-opponent consult cast-bottom bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "exile-until-match-cast-rest-bottom");
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("TargetOpponent"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_looked_cards_kicker_override_bundle() {
    let sentences = registry_sentence_inputs(
        "Look at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead. Put the rest on the bottom of your library in a random order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match looked-cards kicker override bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "look-at-top-put-counted-into-hand-rest-bottom-kicker-override"
    );
    assert_eq!(matched.consumed_sentences, 4);
    assert!(debug.contains("ThisSpellWasKicked"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("MoveTaggedGroupToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_looked_cards_reveal_top_rest_bottom_bundle() {
    let sentences = registry_sentence_inputs(
        "Look at the top four cards of your library. You may reveal a creature or land card from among them and put it on top of your library. Put the rest on the bottom of your library in a random order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match looked-cards reveal-top bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(matched.name, "look-at-top-reveal-match-put-top-rest-bottom");
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("to_top: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_may_rearrange_looked_cards_bundle() {
    let text = "Whenever a creature you control enters, you may look at the top X cards of your library, where X is that creature's power. If you do, put one of those cards on top of your library and the rest on the bottom of your library in any order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify Cream of the Crop text");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("Cream of the Crop sequence should parse");
    let debug = format!("{parsed:#?}").to_ascii_lowercase();

    assert!(debug.contains("lookattopcards"), "{debug}");
    assert!(debug.contains("powerof"), "{debug}");
    assert!(debug.contains("choosetaggedobjectsinzone"), "{debug}");
    assert!(
        debug.contains("puttaggedremainderonbottomoflibrary"),
        "{debug}"
    );
    assert!(
        debug.contains("min: 1") && debug.contains("dynamic_x: false"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_divvy_pile_choice_bundle() {
    let text = "Exile up to five target permanent cards from your graveyard and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard. (Piles can be empty.)";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify divvy pile text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("divvy_source"), "{debug}");
    assert!(debug.contains("divvy_pile"), "{debug}");
    assert!(debug.contains("ChooseObjectsAcrossZones"), "{debug}");
    assert!(debug.contains("ReturnToHand"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_divvy_choose_one_of_them_bundle() {
    let text = "You may search your library for exactly two cards not named Burning-Rune Demon that have different names. If you do, reveal those cards. An opponent chooses one of them. Put the chosen card into your hand and the other into your graveyard, then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify choose-one-of-them text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("May"), "{debug}");
    assert!(debug.contains("min: 2"), "{debug}");
    assert!(debug.contains("distinct_names: true"), "{debug}");
    assert!(debug.contains("divvy_source"), "{debug}");
    assert!(debug.contains("divvy_chosen"), "{debug}");
    assert!(debug.contains("ChooseObjectsAcrossZones"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("ShuffleLibrary"), "{debug}");
}

#[test]
pub(super) fn choose_one_of_exiled_top_cards_lowers_choice_from_exiled_collection() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exiled Choice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile the top two cards of your library. Choose one of them. Until end of turn, you may play that card.",
        )
        .expect("exile-top choose-one play sequence should lower");
    let effects = &def
        .spell_effect
        .as_ref()
        .expect("spell should lower")
        .segments[0]
        .default_effects;
    let exile = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>())
        .expect("exile-top effect should be present");
    let exiled_tag = exile
        .moved_tags
        .first()
        .expect("exile-top effect should tag the exiled collection");
    let choose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .expect("choose-one effect should be present");
    assert_eq!(choose.zone, Some(Zone::Exile));
    assert_eq!(choose.filter.zone, Some(Zone::Exile));
    assert_eq!(choose.filter.owner, None);
    assert!(
        choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == *exiled_tag
        }),
        "choose filter should reference the exiled collection tag: {choose:#?}"
    );

    let grant = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>())
        .expect("grant-play effect should be present");
    assert_eq!(grant.tag, choose.tag);
    assert_eq!(
        grant.duration,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
    );
}

#[test]
pub(super) fn death_trigger_with_counters_exiles_top_cards_equal_to_counter_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Countered Death Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a creature you control with one or more counters on it dies, exile that many cards from the top of your library. Until your next end step, you may play those cards.",
        )
        .expect("countered death trigger should lower");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("TagTriggeringObjectEffect"), "{debug}");
    assert!(debug.contains("ExileTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("CountersOn"), "{debug}");
    assert!(debug.contains("\"triggering\""), "{debug}");
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
    assert!(debug.contains("cast_pool_is_plural: true"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_bend_or_break_divvy_bundle() {
    let text = "Each player separates all nontoken lands they control into two piles. For each player, one of their piles is chosen by one of their opponents of their choice. Destroy all lands in the chosen piles. Tap all lands in the other piles.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify bend or break text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("divvy_opponent"), "{debug}");
    assert!(debug.contains("divvy_chosen"), "{debug}");
    assert!(debug.contains("ChoosePlayer"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("filter: Opponent"), "{debug}");
    assert!(debug.contains("player: That"), "{debug}");
    assert!(debug.contains("TapAll"), "{debug}");
}

#[test]
pub(super) fn bend_or_break_lowers_the_opponent_choice_into_the_pile_choice() {
    let text = "Each player separates all nontoken lands they control into two piles. For each player, one of their piles is chosen by one of their opponents of their choice. Destroy all lands in the chosen piles. Tap all lands in the other piles.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Bend or Break Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text.to_string())
        .expect("Bend or Break should lower cleanly");

    let debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();

    assert!(debug.contains("chooseplayereffect"), "{debug}");
    assert!(debug.contains("chooser: iteratedplayer"), "{debug}");
    assert!(debug.contains("filter: opponent"), "{debug}");
    assert!(debug.contains("divvy_opponent"), "{debug}");
    assert!(debug.contains("chooseobjectseffect"), "{debug}");
    assert!(debug.contains("chooser: taggedplayer"), "{debug}");
    assert!(debug.contains("divvy_chosen"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_gifts_ungiven_divvy_bundle() {
    let text = "Search your library for up to four cards with different names and reveal them. Target opponent chooses two of those cards. Put the chosen cards into your graveyard and the rest into your hand. Then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify Gifts Ungiven text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("distinct_names: true"), "{debug}");
    assert!(debug.contains("divvy_source"), "{debug}");
    assert!(debug.contains("divvy_chosen"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("ShuffleLibrary"), "{debug}");
}

#[test]
pub(super) fn rewrite_ecological_appreciation_multi_zone_search_keeps_the_divvy_bundle_shape() {
    let text = "Mana cost: {X}{2}{G}\nType: Sorcery\nSearch your library and graveyard for up to four creature cards with different names that each have mana value X or less and reveal them. An opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest onto the battlefield. Exile Ecological Appreciation.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ecological Appreciation")
        .parse_text(text)
        .expect("Ecological Appreciation should parse");

    let rendered = format!("{def:#?}").to_ascii_lowercase();
    let compact_rendered = rendered.split_whitespace().collect::<String>();
    assert!(
        compact_rendered.contains("zone:some(library")
            && compact_rendered.contains("additional_zones:[graveyard")
            && compact_rendered.contains("revealtaggedeffect")
            && compact_rendered.contains("shufflelibraryeffect"),
        "expected the compiled search structure to preserve the multi-zone reveal/shuffle shape, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            || debug.contains("divvy_source")
            || debug.contains("divvy_chosen"),
        "expected multi-zone chooser data, got {debug}"
    );
    assert!(
        debug.contains("RevealTaggedEffect")
            || debug.contains("reveal")
            || debug.contains("ShuffleLibrary"),
        "expected reveal/shuffle follow-ups, got {debug}"
    );
    assert!(
        debug.contains("divvy_chosen")
            || debug.contains("ForEachTaggedEffect")
            || debug.contains("ConditionalEffect"),
        "expected opponent pile-choice loop shape, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_choose_land_or_nonland_consult_family() {
    let text = "Choose land or nonland. Reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify Abundant Harvest text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ChooseNamedOption"), "{debug}");
    assert!(debug.contains("SourceChosenOption(\"land\")"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_consult_hand_bottom_family() {
    let text = "Reveal cards from the top of your library until you reveal an artifact card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify consult-hand-bottom text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_up_to_counted_looked_cards_into_hand_rest_bottom()
 {
    let text = "Look at the top X cards of your library. Put up to two of them into your hand and the rest on the bottom of your library in a random order.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify The Mana Rig activated text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(
        debug.contains("ChooseTaggedObjectsInZone") && debug.contains("up_to"),
        "expected up-to counted looked-card choose, got {debug}"
    );
    assert!(debug.contains("MoveTaggedGroupToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "expected remainder to bottom, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_reveal_top_optional_subset_preserves_split_random_remainder() {
    let text = "Reveal the top X cards of your library, where X is the number of creature cards in your graveyard. You may put a green permanent card with mana value X or less from among them onto the battlefield. Put the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("Hatchery Spider sequence should lex");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("reveal-top subset sequence should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(debug.contains("ForEachTagged"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("keep_tagged: Some"), "{debug}");
    assert!(debug.contains("order: Random"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_reveal_top_counted_subset_preserves_inline_random_remainder() {
    let text = "Reveal the top eight cards of your library. Put up to two noncreature artifact cards with total mana value less than or equal to the sacrificed artifact's mana value from among them onto the battlefield and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("Smelting Vat sequence should lex");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("inline reveal-top subset sequence should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(
        debug.contains("ChooseObjectsWithAggregateConstraint"),
        "{debug}"
    );
    assert!(debug.contains("metric: ManaValue"), "{debug}");
    assert!(debug.contains("sacrifice_cost_0"), "{debug}");
    assert!(
        !debug.contains("mana_value: Some"),
        "the total mana-value limit must constrain the selected group, not each card: {debug}"
    );
    assert!(
        debug.contains("min: 0") && debug.contains("max: Some(2)"),
        "{debug}"
    );
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("keep_tagged: Some"), "{debug}");
    assert!(debug.contains("order: Random"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_reveal_top_subset_preserves_inline_graveyard_remainder() {
    let text = "Reveal the top five cards of your library. Put a land card from among them onto the battlefield and the rest into your graveyard.";
    let lexed = lex_line(text, 0).expect("Cavalier of Thorns sequence should lex");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("inline reveal-top graveyard sequence should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("keep_tagged"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_optional_consult_battlefield_graveyard_family() {
    let text = "You may reveal cards from the top of your library until you reveal a land card. If you do, put that card onto the battlefield and put all other cards revealed this way into your graveyard.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify optional consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("IfResult"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(!debug.contains("RevealTop"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_inline_consult_battlefield_bottom_family() {
    let text = "Reveal cards from the top of your library until you reveal a nonlegendary creature card with lesser mana value, put it onto the battlefield, then put the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify inline consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    let relative_filter = parsed
        .iter()
        .find_map(|effect| match effect {
            EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
                SubjectVerbActionAst::ConsultTopOfLibrary { filter, .. } => Some(filter),
                _ => None,
            },
            _ => None,
        })
        .expect("inline consult should carry its typed stop filter");
    assert!(relative_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::target::TaggedOpbjectRelation::ManaValueLtTagged
    }));
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("Random"), "{debug}");
    assert!(!debug.contains("RevealTop"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_inline_consult_battlefield_bottom_preserves_any_order() {
    let text = "Reveal cards from the top of your library until you reveal a creature card, put it onto the battlefield, then put the rest on the bottom of your library in any order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify inline consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("ChooserChooses"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_prefixed_inline_consult_battlefield_bottom_preserves_remainder() {
    let text = "If you do, reveal cards from the top of your library until you reveal a nonlegendary creature card with lesser mana value, put it onto the battlefield, then put the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify prefixed consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("ManaValueLtTagged"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("Random"), "{debug}");
    assert!(!debug.contains("RevealTop"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_if_you_do_consult_battlefield_bottom_pair_preserves_remainder() {
    let text = "If you do, reveal cards from the top of your library until you reveal a nonland permanent card. Put that card onto the battlefield and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify split consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("IfResult"), "{debug}");
    assert!(debug.contains("predicate: Did"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(debug.contains("Battlefield"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("Random"), "{debug}");
    assert!(!debug.contains("RevealTop"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_consult_dispositions_preserve_typed_collection_tags() {
    for text in [
        "That player reveals cards from the top of their library until they reveal a creature card. Put that card onto the battlefield under your control. That player puts the rest of the revealed cards into their graveyard.",
        "Reveal cards from the top of your library until you reveal three nonland cards. Put the nonland cards revealed this way into your hand, then put the rest of the revealed cards on the bottom of your library in any order.",
        "Reveal cards from the top of your library until you reveal that many creature cards, put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
        "Target opponent reveals cards from the top of their library until an artifact card or X cards are revealed, whichever comes first. If an artifact card is revealed this way, put it onto the battlefield under your control and sacrifice this artifact. Put the rest of the revealed cards into that player's graveyard.",
    ] {
        let lexed = lex_line(text, 0).expect("consult disposition should lex");
        let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
            .expect("consult disposition should parse");
        let debug = format!("{parsed:#?}");
        assert!(debug.contains("ConsultTopOfLibrary"), "{text}: {debug}");
        assert!(
            debug.contains("PutTaggedRemainderInZone")
                || debug.contains("PutTaggedRemainderOnBottomOfLibrary")
                || debug.contains("ShuffleLibrary"),
            "{text}: {debug}"
        );
        assert!(
            !debug.contains("Tagged(\n            TagKey(\n                \"rest\""),
            "{text}: {debug}"
        );
    }
}

#[test]
pub(super) fn rewrite_lexed_consult_any_number_and_repeated_moves_keep_explicit_subsets() {
    let vivid = lex_line(
        "Reveal cards from the top of your library until you reveal X permanent cards, where X is the number of colors among permanents you control. Put any number of those permanent cards onto the battlefield, then put the rest of the revealed cards on the bottom of your library in a random order.",
        0,
    )
    .unwrap();
    let vivid = super::super::clause_support::parse_effect_sentences_lexed(&vivid)
        .expect("vivid consult disposition should parse");
    let vivid_debug = format!("{vivid:#?}");
    assert!(vivid_debug.contains("ColorsAmong"), "{vivid_debug}");
    assert!(vivid.iter().any(|effect| matches!(
        effect,
        EffectAst::ChooseObjects { count, .. } if count.is_any_number()
    )));
    assert!(
        vivid_debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{vivid_debug}"
    );

    let glimpse = lex_line(
        "Shuffle all permanents you own into your library, then reveal that many cards from the top of your library. Put all non-Aura permanent cards revealed this way onto the battlefield, then do the same for Aura cards, then put the rest on the bottom of your library in a random order.",
        0,
    )
    .unwrap();
    let glimpse = super::super::clause_support::parse_effect_sentences_lexed(&glimpse)
        .expect("repeated reveal disposition should parse");
    let glimpse_debug = format!("{glimpse:#?}");
    assert!(
        glimpse_debug.contains("SnapshotLastObjectTag"),
        "{glimpse_debug}"
    );
    assert!(
        glimpse_debug.contains("TagMatchingObjects"),
        "{glimpse_debug}"
    );
    assert!(
        glimpse_debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{glimpse_debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_reveal_top_count_handles_their_library() {
    let text = "That player reveals the top two cards of their library. You choose one of those cards and put it into their graveyard.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify reveal-top-count text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(!debug.contains("RevealTop"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_three_way_looked_card_dispositions_keep_distinct_subsets() {
    for (text, expected_middle_zone, expects_library_top) in [
        (
            "Look at the top three cards of your library. Put one of those cards into your hand, one on top of your library, and one on the bottom of your library.",
            "zone: Library",
            true,
        ),
        (
            "Look at the top three cards of your library. Put one of those cards into your hand, one into your graveyard, and one on the bottom of your library.",
            "zone: Graveyard",
            false,
        ),
    ] {
        let lexed = lex_line(text, 0).expect("three-way looked-card text should lex");
        let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
            .expect("three-way looked-card disposition should parse");
        let debug = format!("{parsed:#?}");

        assert_eq!(
            debug.matches("ChooseTaggedObjectsInZone").count(),
            3,
            "each destination must receive its own one-card subset: {debug}"
        );
        assert!(debug.contains("looked_candidates"), "{debug}");
        assert!(debug.contains("IsNotTaggedObject"), "{debug}");
        assert!(debug.contains("zone: Hand"), "{debug}");
        assert!(debug.contains(expected_middle_zone), "{debug}");
        assert_eq!(
            debug.contains("to_top: true"),
            expects_library_top,
            "{text}: {debug}"
        );
        assert!(debug.contains("to_top: false"), "{text}: {debug}");
    }
}

#[test]
pub(super) fn rewrite_lexed_counted_looked_card_partition_tags_the_exact_remainder() {
    let text = "Look at the top three cards of your library. Put two of them into your hand and the other into your graveyard. Dark Bargain deals 2 damage to you.";
    let lexed = lex_line(text, 0).expect("counted looked-card partition should lex");
    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("counted looked-card partition should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("min: 2"), "{debug}");
    assert!(debug.contains("TagMatchingObjects"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_revealed_card_choices_are_scoped_to_the_revealed_candidates() {
    for text in [
        "That player reveals the top two cards of their library. You choose one of those cards and put it into their graveyard.",
        "Reveal the top three cards of your library. Target opponent chooses one of those cards. Put that card into your graveyard, then draw two cards.",
    ] {
        let lexed = lex_line(text, 0).expect("revealed-card choice text should lex");
        let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
            .expect("revealed-card choice should parse");
        let debug = format!("{parsed:#?}");

        assert!(debug.contains("revealed_candidates"), "{text}: {debug}");
        assert!(debug.contains("revealed_choice"), "{text}: {debug}");
        assert!(
            debug.contains("ChooseTaggedObjectsInZone"),
            "{text}: {debug}"
        );
        assert!(debug.contains("IsTaggedObject"), "{text}: {debug}");
        assert!(debug.contains("zone: Library"), "{text}: {debug}");
        assert!(debug.contains("zone: Graveyard"), "{text}: {debug}");
    }
}

#[test]
pub(super) fn rewrite_lexed_consult_defers_dynamic_mana_value_gate_without_reveal_top_fallback() {
    let text = "Target opponent reveals cards from the top of their library until they reveal a card with mana value equal to 1 plus the exiled spell's mana value. Exile that card, then that player shuffles. You may cast that exiled card without paying its mana cost.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify mana-value consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("EqualExpr"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("__source_exiled__"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(!debug.contains("RevealTop"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_prefixed_consult_sequence() {
    let text = "Draw a card. Reveal cards from the top of your library until you reveal an artifact card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify prefixed consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_preserves_up_to_four_target_count() {
    let text = "Choose up to four target creatures you don't control.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify target choice text");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("target choice should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("TargetOnly") && debug.contains("min: 0") && debug.contains("max: Some(4)"),
        "expected a typed up-to-four target count, got {debug}"
    );
}

#[test]
pub(super) fn combat_damage_trigger_exile_top_keeps_damaged_players_library_as_object() {
    let built = CardDefinitionBuilder::new(CardId::from_raw(98_502), "Vaan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Scouts, Pirates, and/or Rogues you control deal combat damage to a player, exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.")
        .expect("combat-damage exile-top trigger should compile");
    let debug = format!("{built:#?}");

    assert!(
        debug.contains("ExileTopOfLibraryEffect")
            && debug.contains("player: IteratedPlayer")
            && !debug.contains("ChooseObjectsEffect"),
        "expected exile-top to retain the damaged player's library, got {debug}"
    );
}

#[test]
pub(super) fn typed_villainous_choice_statement_lowers_without_reparsing_its_target_clause() {
    let built = CardDefinitionBuilder::new(CardId::from_raw(98_503), "Villainous Choice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose up to four target creatures you don't control. For each of them, that creature's controller faces a villainous choice — That creature becomes a 1/1 white Human creature and loses all abilities, or you create a token that's a copy of it.")
        .expect("typed villainous-choice statement should compile");
    let debug = format!("{built:#?}");

    assert!(
        debug.contains("TargetOnlyEffect")
            && debug.contains("ForEachTaggedEffect")
            && debug.contains("VillainousChoiceEffect")
            && debug.contains("white Human creature"),
        "expected typed target selection and per-target villainous choice, got {debug}"
    );
}

#[test]
pub(super) fn each_player_optional_hand_wheel_keeps_discard_and_draw_in_one_may_scope() {
    let tokens = lex_line(
        "Each player may discard their hand and draw seven cards.",
        0,
    )
    .expect("optional hand-wheel sentence should lex");
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("optional hand-wheel sentence should parse");

    let [crate::cards::builders::EffectAst::ForEachPlayer { effects }] = effects.as_slice() else {
        panic!("expected each-player wrapper, got {effects:#?}");
    };
    let [crate::cards::builders::EffectAst::May { effects }] = effects.as_slice() else {
        panic!("expected one iterated-player may scope, got {effects:#?}");
    };
    assert!(matches!(
        effects.as_slice(),
        [
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::DiscardHand,
                    subject: crate::cards::builders::SubjectVerbSubjectAst {
                        player: crate::cards::builders::PlayerAst::Implicit,
                        ..
                    },
                    ..
                }
            ),
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::Draw {
                        count: crate::effect::Value::Fixed(7)
                    },
                    subject: crate::cards::builders::SubjectVerbSubjectAst {
                        player: crate::cards::builders::PlayerAst::Implicit,
                        ..
                    },
                    ..
                }
            )
        ]
    ));
}

#[test]
pub(super) fn conditional_optional_hand_wheel_keeps_the_typed_sequence_inside_one_may_scope() {
    let tokens = lex_line(
        "If embark gets more votes or the vote is tied, each player may discard their hand and draw seven cards.",
        0,
    )
    .expect("conditional optional hand-wheel sentence should lex");
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("conditional optional hand-wheel sentence should parse");

    let [crate::cards::builders::EffectAst::Conditional { if_true, .. }] = effects.as_slice()
    else {
        panic!("expected conditional wrapper, got {effects:#?}");
    };
    let [crate::cards::builders::EffectAst::ForEachPlayer { effects }] = if_true.as_slice() else {
        panic!("expected each-player wrapper inside conditional, got {if_true:#?}");
    };
    let optional_effects = match effects.as_slice() {
        [crate::cards::builders::EffectAst::May { effects }]
        | [crate::cards::builders::EffectAst::MayByPlayer { effects, .. }] => effects,
        _ => panic!("expected one may scope inside conditional, got {effects:#?}"),
    };
    assert_eq!(
        optional_effects.len(),
        2,
        "discard and draw must remain in the same may scope: {optional_effects:#?}"
    );
}

#[test]
pub(super) fn activated_self_move_from_the_command_zone_is_functional_there() {
    let built = CardDefinitionBuilder::new(CardId::from_raw(98_504), "Derevi")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}{G}{W}{U}: Put Derevi onto the battlefield from the command zone.")
        .expect("command-zone self-move activation should compile");

    let activated = built
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, crate::ability::AbilityKind::Activated(_)))
        .expect("expected activated ability");
    assert_eq!(activated.functional_zones, vec![crate::zone::Zone::Command]);
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_keeps_consult_cast_bottom_family_parseable() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost. Put all cards exiled this way that weren't cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify consult-cast-bottom text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_keeps_reveal_consult_cast_bottom_family_parseable() {
    let text = "Reveal cards from the top of your library until you reveal a nonland card. You may cast that card without paying its mana cost. Then put all revealed cards not cast this way on the bottom of your library in a random order.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify reveal consult-cast-bottom text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_each_player_exile_top_cast_nonland_exiled_this_way()
 {
    let text = "Exile the top card of each player's library, then you may cast any number of spells from among the nonland cards exiled this way without paying their mana costs.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify each-player exile-top cast text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ForEachPlayer"), "{debug}");
    assert!(debug.contains("ForEachObject"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_target_opponent_consult_until_eot_cast() {
    let text = "Target opponent exiles cards from the top of their library until they exile a nonland card. Until end of turn, you may cast that card without paying its mana cost.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify target-opponent consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("TargetOpponent"), "{debug}");
    assert!(debug.contains("GrantPlayTaggedUntilEndOfTurn"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_target_opponent_consult_cast_bottom_family() {
    let text = "Target opponent exiles cards from the top of their library until they exile an instant or sorcery card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of that library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify chaos-wand consult text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_consult_dynamic_mana_value_gate() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast the exiled card without paying its mana cost if it's a spell with mana value less than or equal to this's power. Put the exiled cards not cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify dynamic consult gate");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
    assert!(
        debug.contains("operator: LessThanOrEqual") && debug.contains("right: SourcePower"),
        "{debug}"
    );
    assert!(debug.contains("SourcePower"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_consult_fixed_or_less_gate() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if that spell's mana value is 3 or less. Put the exiled cards not cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify fixed consult gate");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("operator: LessThanOrEqual"), "{debug}");
    assert!(debug.contains("Fixed(3)"), "{debug}");
    assert!(debug.contains("May"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_copy_cast_cost_reduction_followup() {
    let text = "Copy that card and you may cast the copy. That copy costs {2} less to cast.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify copy-cast reduction text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("as_copy: true"), "{debug}");
    assert!(debug.contains("cost_reduction: Some"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_copy_exiled_card_then_cast_copy() {
    let text = "You may copy a card exiled with this artifact. If you do, you may cast the copy without paying its mana cost.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify copy exiled card text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(!debug.contains("TargetOnly"), "{debug}");
    assert!(debug.contains("\"__source_exiled__\""), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("as_copy: true"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_return_all_not_chosen_this_way_tracks_it_tag_exclusion() {
    let text = "Return all nonland permanents not chosen this way to their owners' hands.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify chosen-this-way return");

    let parsed = parse_effect_sentence_lexed(&lexed).expect("return-all sentence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ReturnAllToHand"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_destroy_each_chosen_this_way_tracks_it_tag() {
    let text = "Destroy each permanent chosen this way.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify chosen-this-way destroy");

    let parsed = parse_effect_sentence_lexed(&lexed).expect("destroy-each sentence");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DestroyAll"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(debug.contains("\"__it__\""), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sequence_parses_tainted_pact_loop() {
    let text = "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify tainted pact text");

    let parsed =
        super::super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("RepeatProcess"), "{debug}");
    assert!(debug.contains("ExileTopOfLibrary"), "{debug}");
    assert!(debug.contains("MayMoveToZone"), "{debug}");
}

#[test]
pub(super) fn rewrite_semantic_parse_supports_adamant_spent_to_cast_statement_line()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Adamant Variant")
        .card_types(vec![CardType::Sorcery]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token."
            .to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.as_slice(),
        [RewriteSemanticItem::ParsedLine(_)]
    ));
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_supports_adamant_spent_to_cast_statement_line()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Adamant Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_supports_ardenvale_paladin_adamant_enters_with_counter()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Ardenvale Paladin")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Adamant - If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("EnterWithCountersIfCondition"), "{debug}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("White"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_ardenvale_paladin_adamant_counter_condition_keeps_threshold_and_color()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Ardenvale Paladin")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("amount: 3"), "{debug}");
    assert!(debug.contains("symbol: Some("), "{debug}");
    assert!(debug.contains("White"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_keeps_counter_entry_before_conditional_tapped_entry()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Counter Entry Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "This creature enters with a number of stun counters on it equal to three minus X. If X is 2 or less, it enters tapped."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("EnterWithCounters"), "{debug}");
    assert!(debug.contains("XTimes(\n                    -1"), "{debug}");
    assert!(debug.contains("enters tapped"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_spent_to_cast_followup_on_that_permanent() {
    let text = "Tap target artifact or creature an opponent controls. If {S} was spent to cast this spell, that permanent doesn't untap during its controller's next untap step.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify Berg Strider-style effect sequence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("Berg Strider-style effect sequence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Tap"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("Untap"), "{debug}");
    assert!(
        debug.contains("Artifact") && debug.contains("Creature"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lowered_supports_spent_to_cast_conditional_chain() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Firespout Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Firespout deals 3 damage to each creature without flying if {R} was spent to cast this spell and 3 damage to each creature with flying if {G} was spent to cast this spell."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("Red,"), "{debug}");
    assert!(debug.contains("Green,"), "{debug}");
    assert!(debug.contains("excluded_static_abilities"), "{debug}");
    assert!(debug.contains("static_abilities"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_keeps_each_invert_the_skies_mana_condition_on_its_arm()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Invert the Skies")
        .card_types(vec![CardType::Instant]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Creatures your opponents control lose flying until end of turn if {G} was spent to cast this spell, and creatures you control gain flying until end of turn if {U} was spent to cast this spell."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert_eq!(
        debug.matches("ManaSpentToCastThisSpellAtLeast").count(),
        2,
        "{debug}"
    );
    let removal = debug
        .split_once("RemoveAbilityGeneric")
        .map(|(_, tail)| tail)
        .expect("full-card output should contain the flying-removal effect");
    let removal = removal
        .split_once("source_type:")
        .map(|(effect, _)| effect)
        .expect("flying-removal effect should retain its condition field");
    assert!(
        removal.contains("ManaSpentToCastThisSpellAtLeast") && removal.contains("Green"),
        "{debug}"
    );
    assert!(debug.contains("Blue"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_wishful_merfolk_shares_end_of_turn_across_activated_arms()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Wishful Merfolk")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Defender\n{1}{U}: This creature loses defender and becomes a Human until end of turn."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    let removal = debug
        .split_once("RemoveAbilityGeneric")
        .map(|(_, tail)| tail)
        .expect("full-card output should contain the defender-removal effect");
    let removal = removal
        .split_once("condition:")
        .map(|(effect, _)| effect)
        .expect("defender-removal effect should retain its duration field");
    assert!(removal.contains("until: EndOfTurn"), "{debug}");
    assert!(debug.contains("RemoveAllSubtypesOfFamily"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_preserves_etb_spent_to_cast_it_intervening_if()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("etb_spent_to_cast_intervening_if_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_preserves_etb_spent_to_cast_it_intervening_if_inner)
        .expect("etb spent-to-cast regression thread should spawn")
        .join()
        .expect("etb spent-to-cast regression thread should not panic")
}

pub(super) fn rewrite_lowered_preserves_etb_spent_to_cast_it_intervening_if_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Gruul Scrapper")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "When this creature enters, if {R} was spent to cast it, it gains haste until end of turn."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("intervening_if"), "{debug}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("Red"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_retargets_spell_cast_spent_mana_intervening_if()
-> Result<(), CardTextError> {
    let builder =
        CardDefinitionBuilder::new(CardId::new(), "Sahagin").card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, put a +1/+1 counter on this creature."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("TriggeringSpellManaSpentToCastAtLeast"),
        "{debug}"
    );
    assert!(debug.contains("amount: 4"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_keeps_spent_mana_counter_and_unblockable_followup()
-> Result<(), CardTextError> {
    let builder =
        CardDefinitionBuilder::new(CardId::new(), "Sahagin").card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, put a +1/+1 counter on this creature and it can't be blocked this turn."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("TriggeringSpellManaSpentToCastAtLeast"),
        "{debug}"
    );
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("CantEffect"), "{debug}");
    assert!(debug.contains("BeBlocked"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_effect_sentence_keeps_target_pump_before_unblockable_followup() {
    let tokens = lex_line(
        "Target creature gets +1/+0 until end of turn and can't be blocked this turn.",
        0,
    )
    .expect("lex");
    let parsed = parse_effect_sentence_lexed(&tokens).expect("parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("BeBlocked"), "{debug}");
    assert!(!debug.contains("TargetOnly"), "{debug}");
}

#[test]
pub(super) fn rewrite_lowered_binds_self_replacement_it_condition_to_default_target()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Will Variant")
        .card_types(vec![CardType::Instant]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Target creature gets +2/+2 until end of turn. If it's blocking, instead put two +1/+1 counters on it."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("TaggedObjectMatches"), "{debug}");
    assert!(debug.contains("blocking: true"), "{debug}");
    assert!(!debug.contains("SourceMatches"), "{debug}");
    assert!(!debug.contains("TargetMatches"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_binds_phase_out_self_replacement_condition_to_default_target()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Divine Variant")
        .card_types(vec![CardType::Instant]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Target creature or planeswalker an opponent controls phases out. If that permanent is black, exile it instead."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("TaggedObjectMatches"), "{debug}");
    assert!(debug.contains("colors: Some"), "{debug}");
    assert!(!debug.contains("SourceMatches"), "{debug}");
    assert!(!debug.contains("TargetMatches"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_binds_self_damage_followup_condition_to_damage_source_target()
-> Result<(), CardTextError> {
    let builder =
        CardDefinitionBuilder::new(CardId::new(), "Wisecrack").card_types(vec![CardType::Instant]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Target creature deals damage equal to its power to itself. If that creature is attacking, Wisecrack deals 2 damage to that creature's controller."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("TaggedObjectMatches"), "{debug}");
    assert!(debug.contains("damage_source_0"), "{debug}");
    assert!(debug.contains("attacking: true"), "{debug}");
    assert!(!debug.contains("SourceMatches"), "{debug}");
    assert!(!debug.contains("TargetMatches"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_retargets_spell_cast_no_colored_mana_intervening_if()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Void Mirror")
        .card_types(vec![CardType::Artifact]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever a player casts a spell, if no colored mana was spent to cast it, counter that spell."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("Not"), "{debug}");
    assert!(
        debug.contains("TriggeringSpellColoredManaSpentToCastAtLeast"),
        "{debug}"
    );
    assert!(debug.contains("CounterEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_retargets_spell_cast_no_mana_intervening_if()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Vexing Bauble")
        .card_types(vec![CardType::Artifact]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever a player casts a spell, if no mana was spent to cast it, counter that spell."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("Not"), "{debug}");
    assert!(
        debug.contains("TriggeringSpellManaSpentToCastAtLeast"),
        "{debug}"
    );
    assert!(
        !debug.contains("TargetSpellManaSpentToCastAtLeast"),
        "{debug}"
    );
    assert!(debug.contains("CounterEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_binds_event_object_intervening_if_to_triggering_object()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("event_object_intervening_if_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_binds_event_object_intervening_if_to_triggering_object_inner)
        .expect("event-object intervening-if regression thread should spawn")
        .join()
        .expect("event-object intervening-if regression thread should not panic")
}

pub(super) fn rewrite_lowered_binds_event_object_intervening_if_to_triggering_object_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Deathknell Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "When this creature dies, if its power was 3 or greater, create a 2/2 black Zombie Berserker creature token."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("TaggedObjectMatchedLastKnown"), "{debug}");
    assert!(debug.contains("\"triggering\""), "{debug}");
    assert!(!debug.contains("TaggedObjectMatches("), "{debug}");
    assert!(!debug.contains("TargetMatches"), "{debug}");
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_preserves_negative_last_known_death_gate() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Infernal Vessel Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "When this creature dies, if it wasn't a Demon, return it to the battlefield under its owner's control with two +1/+1 counters on it. It's a Demon in addition to its other types."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("Not("), "{debug}");
    assert!(debug.contains("TaggedObjectMatchedLastKnown"), "{debug}");
    assert!(debug.contains("Demon"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_keeps_followup_death_lki_as_body_conditional()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Overgrowth Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever another creature you control dies, you gain 1 life. If that creature was an Elemental, put a +1/+1 counter on this creature."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("GainLifeEffect"), "{debug}");
    assert!(debug.contains("ConditionalEffect"), "{debug}");
    assert!(debug.contains("TaggedObjectMatchedLastKnown"), "{debug}");
    assert!(debug.contains("Elemental"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_second_landfall_intervening_if_comes_from_predicate_grammar()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("second_landfall_intervening_if_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_second_landfall_intervening_if_comes_from_predicate_grammar_inner)
        .expect("second-landfall regression thread should spawn")
        .join()
        .expect("second-landfall regression thread should not panic")
}

pub(super) fn rewrite_lowered_second_landfall_intervening_if_comes_from_predicate_grammar_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Tunnel Ignus Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever a land enters under an opponent's control, if that player had another land enter the battlefield under their control this turn, this creature deals 3 damage to that player."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("intervening_if: Some"), "{debug}");
    assert!(debug.contains("LandsEnteredBattlefieldThisTurn"), "{debug}");
    assert!(debug.contains("GreaterThanOrEqual"), "{debug}");
    assert!(debug.contains("ControllerOf"), "{debug}");
    assert!(debug.contains("\"triggering\""), "{debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_generic_damage_to_object_trigger_preserves_recipient_filter()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("generic_damage_to_object_trigger_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_generic_damage_to_object_trigger_preserves_recipient_filter_inner)
        .expect("generic damage-to-object regression thread should spawn")
        .join()
        .expect("generic damage-to-object regression thread should not panic")
}

pub(super) fn rewrite_lowered_generic_damage_to_object_trigger_preserves_recipient_filter_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Kusari-Gama Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Equipped creature has \"{2}: This creature gets +1/+0 until end of turn.\"\nWhenever equipped creature deals damage to a blocking creature, this Equipment deals that much damage to each other creature defending player controls.\nEquip {3}"
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("DealsDamageTo"), "{debug}");
    assert!(debug.contains("blocking: true"), "{debug}");
    assert!(debug.contains("EventValue"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_spell_countered_trigger_comes_from_trigger_grammar()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("spell_countered_trigger_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_spell_countered_trigger_comes_from_trigger_grammar_inner)
        .expect("spell-countered regression thread should spawn")
        .join()
        .expect("spell-countered regression thread should not panic")
}

pub(super) fn rewrite_lowered_spell_countered_trigger_comes_from_trigger_grammar_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Multani's Presence Variant")
        .card_types(vec![CardType::Enchantment]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever a spell you've cast is countered, draw a card.".to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("SpellCountered"), "{debug}");
    assert!(debug.contains("controller: You"), "{debug}");
    assert!(!debug.contains("SpellCast"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_return_all_cycled_or_discarded_cards_keeps_history_filter() {
    let text =
        "Return all cards in your graveyard that you cycled or discarded this turn to your hand.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify cycled/discarded return");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("cycled/discarded return sentence should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ReturnAllToHand"), "{debug}");
    assert!(
        debug.contains("discarded_or_cycled_this_turn_by: Some"),
        "{debug}"
    );
    assert!(debug.contains("You"), "{debug}");
}

#[test]
pub(super) fn rewrite_lowered_cycled_or_discarded_graveyard_return_comes_from_filter_grammar()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("cycled_or_discarded_return_filter_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_cycled_or_discarded_graveyard_return_comes_from_filter_grammar_inner)
        .expect("cycled/discarded return regression thread should spawn")
        .join()
        .expect("cycled/discarded return regression thread should not panic")
}

pub(super) fn rewrite_lowered_cycled_or_discarded_graveyard_return_comes_from_filter_grammar_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shadow of the Grave Variant")
        .card_types(vec![CardType::Instant]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Return to your hand all cards in your graveyard that you cycled or discarded this turn."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("ReturnToHandEffect"), "{debug}");
    assert!(
        debug.contains("discarded_or_cycled_this_turn_by: Some"),
        "{debug}"
    );
    assert!(debug.contains("You"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_telemin_consult_sequence_comes_from_sequence_grammar()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("telemin_consult_sequence_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_telemin_consult_sequence_comes_from_sequence_grammar_inner)
        .expect("Telemin consult regression thread should spawn")
        .join()
        .expect("Telemin consult regression thread should not panic")
}

pub(super) fn rewrite_lowered_telemin_consult_sequence_comes_from_sequence_grammar_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Telemin Performance Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Target opponent reveals cards from the top of their library until they reveal a creature card. That player puts all noncreature cards revealed this way into their graveyard, then you put the creature card onto the battlefield under your control."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("PutOntoBattlefieldEffect"), "{debug}");
    assert!(debug.contains("ForEachTagged"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_tempting_offer_copy_spell_comes_from_sequence_grammar()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("tempting_offer_copy_spell_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_tempting_offer_copy_spell_comes_from_sequence_grammar_inner)
        .expect("Tempting offer copy-spell regression thread should spawn")
        .join()
        .expect("Tempting offer copy-spell regression thread should not panic")
}

pub(super) fn rewrite_lowered_tempting_offer_copy_spell_comes_from_sequence_grammar_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Tempt with Mayhem Variant")
        .card_types(vec![CardType::Instant]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Tempting offer — Choose target instant or sorcery spell. Each opponent may copy that spell and may choose new targets for the copy they control. You copy that spell once plus an additional time for each opponent who copied the spell this way. You may choose new targets for the copies you control."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("CopySpellEffect"), "{debug}");
    assert!(debug.contains("copier: IteratedPlayer"), "{debug}");
    assert!(debug.contains("PlayersWithPositiveCount"), "{debug}");
    assert!(!debug.contains("PendingEffectMetric"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_reciprocal_creature_control_comes_from_sequence_grammar()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("reciprocal_creature_control_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_reciprocal_creature_control_comes_from_sequence_grammar_inner)
        .expect("reciprocal creature-control regression thread should spawn")
        .join()
        .expect("reciprocal creature-control regression thread should not panic")
}

pub(super) fn rewrite_lowered_reciprocal_creature_control_comes_from_sequence_grammar_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Twist Allegiance Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "You and target opponent each gain control of all creatures the other controls until end of turn. Untap those creatures. Those creatures gain haste until end of turn."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("TagMatchingObjectsEffect"), "{debug}");
    assert!(debug.contains("__twist_your_creatures__"), "{debug}");
    assert!(debug.contains("__twist_opponent_creatures__"), "{debug}");
    assert!(debug.contains("ChangeControllerToPlayer"), "{debug}");
    assert!(debug.contains("Target("), "{debug}");
    assert!(debug.contains("Opponent"), "{debug}");
    assert!(debug.contains("UntapEffect"), "{debug}");
    assert!(debug.contains("AddAbility"), "{debug}");
    assert!(debug.contains("haste"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_tempting_offer_return_uses_iterated_opponent_chooser()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("tempting_offer_return_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_tempting_offer_return_uses_iterated_opponent_chooser_inner)
        .expect("tempting-offer return regression thread should spawn")
        .join()
        .expect("tempting-offer return regression thread should not panic")
}

pub(super) fn rewrite_lowered_tempting_offer_return_uses_iterated_opponent_chooser_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Tempting Offer Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Tempting offer — Return a creature card from your graveyard to the battlefield. Each opponent may return a creature card from their graveyard to the battlefield. For each opponent who does, return a creature card from your graveyard to the battlefield."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:?}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("decider: Some(IteratedPlayer)"), "{debug}");
    assert!(debug.contains("chooser: IteratedPlayer"), "{debug}");
    assert!(debug.contains("owner: Some(IteratedPlayer)"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_binds_effect_level_it_condition_to_prior_chosen_object()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Howlpack Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{1}{G}, {T}: You may put a creature card from your hand onto the battlefield. If it's a Wolf or Werewolf, untap this creature. Activate only as a sorcery."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("TaggedObjectMatches"), "{debug}");
    assert!(
        debug.contains("__sentence_helper_chosen")
            || debug.contains("__sentence_helper_moved")
            || debug.contains("moved_"),
        "{debug}"
    );
    assert!(!debug.contains("TargetMatches"), "{debug}");
    assert!(debug.contains("UntapEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_binds_effect_level_it_condition_to_prior_search_result()
-> Result<(), CardTextError> {
    std::thread::Builder::new()
        .name("effect_level_it_search_result_regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(rewrite_lowered_binds_effect_level_it_condition_to_prior_search_result_inner)
        .expect("effect-level search-result regression thread should spawn")
        .join()
        .expect("effect-level search-result regression thread should not panic")
}

pub(super) fn rewrite_lowered_binds_effect_level_it_condition_to_prior_search_result_inner()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Oriq Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{T}: Search your library for a card, put it into your graveyard, then shuffle. If it's an instant or sorcery card, create a 3/2 red and white Spirit creature token."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("TaggedObjectMatches"), "{debug}");
    assert!(
        debug.contains("__sentence_helper_searched") || debug.contains("searched_"),
        "{debug}"
    );
    assert!(!debug.contains("TargetMatches"), "{debug}");
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_preserves_literal_target_condition_that_compares_to_prior_choice()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Guard Dogs")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{2}{W}, {T}: Choose a permanent you control. Prevent all combat damage target creature would deal this turn if it shares a color with that permanent."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("TargetMatches"), "{debug}");
    assert!(debug.contains("SharesColorWithTagged"), "{debug}");
    assert!(!debug.contains("TaggedObjectMatches"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_implicit_gain_control_in_for_each_opponent_stays_effect_controller()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Mass Mutiny Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "For each opponent, gain control of up to one target creature that player controls until end of turn. Untap those creatures. They gain haste until end of turn."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("ChangeControllerToEffectController"),
        "{debug}"
    );
    assert!(!debug.contains("ChangeControllerToPlayer"), "{debug}");
    Ok(())
}

#[test]
pub(super) fn rewrite_lowered_each_player_gains_control_of_owned_objects_uses_iterated_player()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Homeward Path Variant")
        .card_types(vec![CardType::Land]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{T}: Each player gains control of all creatures they own.".to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(compact.contains("owner:Some(IteratedPlayer"), "{debug}");
    assert!(
        compact.contains("ChangeControllerToPlayer(IteratedPlayer"),
        "{debug}"
    );
    assert!(
        !debug.contains("ChangeControllerToEffectController"),
        "{debug}"
    );
    Ok(())
}

#[test]
pub(super) fn forced_block_it_keeps_the_prior_target_across_the_blocker_choice()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Feral Contest Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Put a +1/+1 counter on target creature you control. Another target creature blocks it this turn if able."
            .to_string(),
        false,
    )?;

    let effects = definition
        .spell_effect
        .as_ref()
        .expect("spell resolution")
        .flattened_default_effects();
    let prior_target_tags = effects
        .iter()
        .find_map(|effect| {
            let mut current = effect;
            let mut tags = Vec::new();
            while let Some(tagged) = current.downcast_ref::<crate::effects::TaggedEffect>() {
                tags.push(tagged.tag.clone());
                current = &tagged.effect;
            }
            current
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .map(|_| tags)
        })
        .expect("counter effect should retain its target tag");
    let cant = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::CantEffect>())
        .expect("forced-block restriction");
    let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
        &cant.restriction
    else {
        panic!("expected a specific forced-block restriction: {cant:#?}");
    };
    let [attacker_constraint] = attacker.tagged_constraints.as_slice() else {
        panic!("expected one attacker tag: {attacker:#?}");
    };
    let [blocker_constraint] = blockers.tagged_constraints.as_slice() else {
        panic!("expected one blocker tag: {blockers:#?}");
    };

    assert!(
        prior_target_tags.contains(&attacker_constraint.tag),
        "the attacker should be the creature targeted by the prior sentence: {effects:#?}"
    );
    assert_ne!(
        attacker_constraint.tag, blocker_constraint.tag,
        "the newly chosen blocker must not replace the prior attacker reference"
    );
    Ok(())
}
