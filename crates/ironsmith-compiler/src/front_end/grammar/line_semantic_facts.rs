use crate::model::facts::{
    AsEntersEffectProgramFacts, AsTransformsEffectProgramFacts, InsteadFollowupFacts,
    LineSemanticFacts, StatementConditionIntro, StatementLineSemanticFacts,
    StaticLineSemanticFacts, ThisSpellCostFacts, TriggerFrequencyFacts, TriggerFunctionalZoneFacts,
    TriggeredLineSemanticFacts,
};

use super::super::lexer::OwnedLexToken;
use super::{
    activated_lines, document_shapes, effects, functional_zones, leaf, lowering_surfaces,
    primitives, structure, trigger_surface,
};

fn parse_as_enters_effect_program_facts(
    tokens: &[OwnedLexToken],
) -> Option<AsEntersEffectProgramFacts> {
    let tokens = document_shapes::parse_statement_label_strip_tokens(tokens).body_tokens;
    if !tokens.first().is_some_and(|token| token.is_word("as"))
        || !tokens.get(1).is_some_and(|token| token.is_word("this"))
    {
        return None;
    }
    let comma_idx =
        crate::slice_primitives::select_position(&tokens[2..], |token| token.is_comma())? + 2;
    if comma_idx + 1 >= tokens.len() {
        return None;
    }
    let (subject_end_idx, also_turns_face_up, turns_face_up_only) = if let Some(enters_idx) =
        crate::slice_primitives::select_position(&tokens[..comma_idx], |token| {
            token.is_word("enters")
        }) {
        let also_turns_face_up = crate::word_primitives::parse_sequence_complete(
            &super::super::lexer::token_word_refs(&tokens[enters_idx + 1..comma_idx]),
            &["or", "is", "turned", "face", "up"],
        );
        (enters_idx, also_turns_face_up, false)
    } else {
        let is_idx = crate::slice_primitives::select_position(&tokens[..comma_idx], |token| {
            token.is_word("is")
        })?;
        let is_turned_face_up = crate::word_primitives::parse_sequence_complete(
            &super::super::lexer::token_word_refs(&tokens[is_idx..comma_idx]),
            &["is", "turned", "face", "up"],
        );
        if !is_turned_face_up {
            return None;
        }
        (is_idx, true, true)
    };
    if subject_end_idx <= 1 {
        return None;
    }
    let uses_enters_with_counter_surface =
        primitives::find_prefix(&tokens[comma_idx + 1..], || {
            primitives::phrase(&["enters", "with"])
        })
        .is_some();
    let source_pronoun_enters_with_counter_surface =
        primitives::find_prefix(&tokens[comma_idx + 1..], || {
            primitives::phrase(&["it", "enters", "with"])
        })
        .is_some();
    Some(AsEntersEffectProgramFacts {
        subject: super::super::lexer::render_token_slice(&tokens[1..subject_end_idx]),
        also_turns_face_up,
        turns_face_up_only,
        uses_enters_with_counter_surface,
        source_pronoun_enters_with_counter_surface,
    })
}

fn parse_as_transforms_effect_program_facts(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> Option<AsTransformsEffectProgramFacts> {
    let tokens = document_shapes::parse_statement_label_strip_tokens(tokens).body_tokens;
    if !tokens.first().is_some_and(|token| token.is_word("as"))
        || !tokens.get(1).is_some_and(|token| token.is_word("this"))
    {
        return None;
    }
    let transforms_idx =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("transforms"))?;
    if transforms_idx <= 1
        || !tokens
            .get(transforms_idx + 1)
            .is_some_and(|token| token.is_word("into"))
    {
        return None;
    }
    let destination_start = transforms_idx + 2;
    let comma_idx = tokens[destination_start..]
        .iter()
        .enumerate()
        .find_map(|(offset, token)| {
            if !token.is_comma() {
                return None;
            }
            let comma_idx = destination_start + offset;
            let tail = &tokens[comma_idx + 1..];
            (tail.first().is_some_and(|token| token.is_word("it"))
                && tail.get(1).is_some_and(|token| token.is_word("becomes")))
            .then_some(comma_idx)
        })
        .or_else(|| {
            crate::slice_primitives::select_position(&tokens[destination_start..], |token| {
                token.is_comma()
            })
            .map(|offset| destination_start + offset)
        })?;
    if transforms_idx + 2 >= comma_idx || comma_idx + 1 >= tokens.len() {
        return None;
    }
    let parsed_destination =
        super::super::lexer::render_token_slice(&tokens[transforms_idx + 2..comma_idx]);
    let destination = context
        .map(|context| context.source().card_name.as_str())
        .and_then(|source_name| {
            if source_name.eq_ignore_ascii_case(&parsed_destination) {
                return Some(source_name.to_string());
            }
            let short_name = source_name.split(',').next()?.trim();
            short_name
                .eq_ignore_ascii_case(&parsed_destination)
                .then(|| short_name.to_string())
        })
        .unwrap_or(parsed_destination);
    Some(AsTransformsEffectProgramFacts {
        subject: super::super::lexer::render_token_slice(&tokens[1..transforms_idx]),
        destination,
    })
}

fn parse_trailing_instead_if_predicate(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::ast::PredicateAst> {
    let instead_idx =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("instead"))?;
    structure::parse_trailing_instead_if_predicate_lexed(&tokens[instead_idx..])
}

fn has_repeatable_instant_timing_payment_until_end_of_turn(tokens: &[OwnedLexToken]) -> bool {
    let words = super::super::lexer::parser_token_word_refs(tokens);
    crate::word_primitives::sequence_occurs(&words, &["until", "end", "of", "turn"])
        && crate::word_primitives::sequence_occurs(&words, &["that", "permanent", "or", "player"])
        && crate::word_primitives::sequence_occurs(
            &words,
            &["any", "time", "you", "could", "cast", "an", "instant", "if"],
        )
}

fn parse_statement_semantic_facts(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> StatementLineSemanticFacts {
    let instead = effects::parse_instead_followup_shape_tokens(tokens);
    let leading_condition_intro =
        leaf::parse_leaf_condition_intro_prefix_tokens(tokens).map(|prefix| match prefix.intro {
            leaf::ConditionIntro::If => StatementConditionIntro::If,
            leaf::ConditionIntro::Unless => StatementConditionIntro::Unless,
            leaf::ConditionIntro::AsLongAs => StatementConditionIntro::AsLongAs,
            leaf::ConditionIntro::ForAsLongAs => StatementConditionIntro::ForAsLongAs,
        });
    let replacement_surfaces: Vec<_> =
        lowering_surfaces::parse_statement_replacement_surface_tokens(tokens)
            .into_iter()
            .collect();
    let instead_semantics = if replacement_surfaces.is_empty() {
        instead.semantics
    } else {
        crate::cards::builders::InsteadSemantics::SelfReplacement
    };

    StatementLineSemanticFacts {
        instead_followup: InsteadFollowupFacts {
            semantics: instead_semantics,
            conditional_intro: instead.conditional_intro,
            leading_instead_surface: instead.leading_instead_surface,
        },
        trailing_instead_if_predicate: parse_trailing_instead_if_predicate(tokens),
        replacement_surfaces,
        as_enters_effect_program: parse_as_enters_effect_program_facts(tokens),
        as_transforms_effect_program: parse_as_transforms_effect_program_facts(context, tokens),
        presentation_label: None,
        creature_type_choice_buff: lowering_surfaces::parse_creature_type_choice_buff_tokens(
            tokens,
        )
        .is_some(),
        leading_condition_intro,
        repeatable_instant_timing_payment_until_end_of_turn:
            has_repeatable_instant_timing_payment_until_end_of_turn(tokens),
    }
}

fn has_leading_unless_resolution_surface(tokens: &[OwnedLexToken]) -> bool {
    let Some((comma_idx, _, _)) =
        primitives::find_prefix(tokens, || (primitives::comma(), primitives::kw("unless")))
    else {
        return false;
    };
    crate::slice_primitives::select_position(&tokens[comma_idx + 2..], OwnedLexToken::is_comma)
        .is_some()
}

pub fn parse_line_semantic_facts_tokens(tokens: &[OwnedLexToken]) -> LineSemanticFacts {
    parse_line_semantic_facts_tokens_with_optional_context(None, tokens)
}

pub fn parse_line_semantic_facts_tokens_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> LineSemanticFacts {
    parse_line_semantic_facts_tokens_with_optional_context(Some(context), tokens)
}

fn parse_line_semantic_facts_tokens_with_optional_context(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> LineSemanticFacts {
    let static_cost = lowering_surfaces::parse_this_spell_cost_surface_tokens(tokens);
    let trigger_zones = functional_zones::parse_trigger_functional_zone_facts_tokens(tokens);
    let trigger_frequency = trigger_surface::parse_trigger_frequency_tokens(tokens);

    LineSemanticFacts {
        static_ability: StaticLineSemanticFacts {
            explicit_functional_zones: functional_zones::parse_static_functional_zones_tokens(
                tokens,
            ),
            references_this_ability_cost:
                activated_lines::parse_this_ability_cost_reference_prefix_tokens(tokens).is_some(),
            this_spell_cost: static_cost.map(|surface| ThisSpellCostFacts {
                reduction_cap: surface.reduction_cap,
            }),
            presentation_label: None,
        },
        statement: parse_statement_semantic_facts(context, tokens),
        triggered_ability: TriggeredLineSemanticFacts {
            compiler_ability: None,
            intro_surface: trigger_surface::parse_trigger_intro_surface_tokens(tokens),
            presentation_label: None,
            functional_zones: TriggerFunctionalZoneFacts {
                explicit_zone: trigger_zones.explicit_zone,
                returns_self_from_graveyard: trigger_zones.returns_self_from_graveyard,
                discards_this_card: trigger_zones.discards_this_card,
            },
            becomes_tapped_during_your_turn:
                trigger_surface::parse_becomes_tapped_during_your_turn_tokens(tokens).is_some(),
            frequency: TriggerFrequencyFacts {
                first_time_each_or_this_turn: trigger_frequency.first_time_each_or_this_turn,
                first_time_during_each_of_your_turns: trigger_frequency
                    .first_time_during_each_of_your_turns,
                becomes_crewed: trigger_frequency.becomes_crewed,
                do_this_limit_each_turn: trigger_frequency.do_this_limit_each_turn,
            },
            leading_unless_surface: has_leading_unless_resolution_surface(tokens),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::InsteadSemantics;
    use crate::lexer::lex_line;
    use crate::model::facts::StatementReplacementSurfaceKind;
    use crate::zone::Zone;

    fn facts(text: &str) -> LineSemanticFacts {
        parse_line_semantic_facts_tokens(&lex_line(text, 0).expect("fixture should lex"))
    }

    #[test]
    fn collects_static_zone_reference_and_cost_facts() {
        let graveyard = facts("You may cast this card from your graveyard.");
        assert_eq!(
            graveyard.static_ability.explicit_functional_zones,
            Some(vec![Zone::Graveyard])
        );

        let referenced = facts("This ability costs {1} less to activate.");
        assert!(referenced.static_ability.references_this_ability_cost);

        let spell_cost = facts("This spell costs {1} less to cast, but not by more than {3}.");
        assert_eq!(
            spell_cost
                .static_ability
                .this_spell_cost
                .map(|cost| cost.reduction_cap),
            Some(Some(3))
        );
    }

    #[test]
    fn classifies_repeatable_instant_timing_payment_without_lowering_source_text() {
        let exact = facts(
            "Prevent the next X damage that would be dealt to any target this turn. Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.",
        );
        assert!(
            exact
                .statement
                .repeatable_instant_timing_payment_until_end_of_turn
        );

        for near_miss in [
            "Until end of turn, you may pay {1}. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.",
            "You may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.",
            "Until end of turn, you may pay {1} any time you could cast an instant. If you do, draw a card.",
        ] {
            assert!(
                !facts(near_miss)
                    .statement
                    .repeatable_instant_timing_payment_until_end_of_turn,
                "near miss must remain an ordinary resolution: {near_miss}"
            );
        }
    }

    #[test]
    fn collects_trigger_zone_turn_and_frequency_facts_before_suffix_stripping() {
        let parsed = facts(
            "Whenever this creature becomes tapped during your turn, if this card is in your graveyard, return this card from your graveyard. Do this only twice each turn.",
        );

        assert_eq!(
            parsed.triggered_ability.functional_zones.explicit_zone,
            Some(Zone::Graveyard)
        );
        assert!(
            parsed
                .triggered_ability
                .functional_zones
                .returns_self_from_graveyard
        );
        assert!(parsed.triggered_ability.becomes_tapped_during_your_turn);
        assert_eq!(
            parsed.triggered_ability.frequency.do_this_limit_each_turn,
            Some(2)
        );
    }

    #[test]
    fn collects_statement_instead_predicate_and_condition_intro_facts() {
        let parsed =
            facts("If you control a creature, draw a card instead if you control an artifact.");

        assert_eq!(
            parsed.statement.instead_followup.semantics,
            InsteadSemantics::SelfReplacement
        );
        assert!(parsed.statement.instead_followup.conditional_intro);
        assert_eq!(
            parsed.statement.leading_condition_intro,
            Some(StatementConditionIntro::If)
        );
        assert!(parsed.statement.trailing_instead_if_predicate.is_some());

        let trailing = facts(
            "You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.",
        );
        assert!(
            trailing.statement.trailing_instead_if_predicate.is_some(),
            "the trailing condition after an expanded instead-of phrase must remain typed: {trailing:#?}"
        );
    }

    #[test]
    fn collects_as_enters_program_timing_after_an_ability_word_label() {
        let parsed = facts(
            "Imprint — As this Vehicle enters or is turned face up, exile a creature card from a graveyard.",
        );
        let as_enters = parsed
            .statement
            .as_enters_effect_program
            .expect("as-enters timing should be retained as typed semantic facts");

        assert_eq!(as_enters.subject, "this Vehicle");
        assert!(as_enters.also_turns_face_up);
        assert!(!as_enters.turns_face_up_only);
        assert!(!as_enters.uses_enters_with_counter_surface);
        assert!(!as_enters.source_pronoun_enters_with_counter_surface);

        let face_up_only =
            facts("As this creature is turned face up, put four +1/+1 counters on it.")
                .statement
                .as_enters_effect_program
                .expect("face-up-only timing should be retained as typed semantic facts");
        assert_eq!(face_up_only.subject, "this creature");
        assert!(face_up_only.also_turns_face_up);
        assert!(face_up_only.turns_face_up_only);
        assert!(!face_up_only.uses_enters_with_counter_surface);
        assert!(!face_up_only.source_pronoun_enters_with_counter_surface);

        let counter_surface = facts(
            "As this creature enters, remove all counters from all permanents. This creature enters with a +1/+1 counter on it for each counter removed this way.",
        )
        .statement
        .as_enters_effect_program
        .expect("entry-counter wording should retain the enclosing as-enters timing");
        assert!(counter_surface.uses_enters_with_counter_surface);
        assert!(!counter_surface.source_pronoun_enters_with_counter_surface);

        let source_pronoun = facts(
            "As this creature enters, you may sacrifice any number of creatures. If you do, it enters with twice that many +1/+1 counters on it.",
        )
        .statement
        .as_enters_effect_program
        .expect("source-relative entry-counter wording should remain typed");
        assert!(source_pronoun.uses_enters_with_counter_surface);
        assert!(source_pronoun.source_pronoun_enters_with_counter_surface);
    }

    #[test]
    fn collects_as_transforms_program_timing_after_an_ability_word_label() {
        let parsed =
            facts("Burning Chains — As this creature transforms into Shinryu, choose an opponent.");
        let as_transforms = parsed
            .statement
            .as_transforms_effect_program
            .expect("as-transforms timing should be retained as typed semantic facts");

        assert_eq!(as_transforms.subject, "this creature");
        assert_eq!(as_transforms.destination, "Shinryu");
        assert!(parsed.statement.as_enters_effect_program.is_none());

        let text = "As this creature transforms into shinryu, choose an opponent.";
        let tokens = lex_line(text, 0).expect("normalized transform fixture should lex");
        let context = crate::parse_context::ParseContext::for_fragment(
            "Shinryu, Transcendent Rival",
            Vec::new(),
            Vec::new(),
            text,
        );
        let normalized = parse_line_semantic_facts_tokens_with_context(context.view(), &tokens);
        assert_eq!(
            normalized
                .statement
                .as_transforms_effect_program
                .expect("normalized destination should remain typed")
                .destination,
            "Shinryu"
        );

        let punctuated = facts(
            "As this creature transforms into Olag, Ludevic's Hubris, it becomes a copy of a creature card exiled with it.",
        );
        assert_eq!(
            punctuated
                .statement
                .as_transforms_effect_program
                .expect("a comma-bearing transform destination should remain intact")
                .destination,
            "Olag, Ludevic's Hubris"
        );
    }

    #[test]
    fn distinguishes_leading_and_trailing_unless_trigger_surfaces() {
        let leading = facts(
            "At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature.",
        );
        assert!(leading.triggered_ability.leading_unless_surface);

        let trailing =
            facts("At the beginning of your upkeep, sacrifice this creature unless you pay {2}.");
        assert!(!trailing.triggered_ability.leading_unless_surface);
    }

    #[test]
    fn collects_every_special_statement_replacement_surface() {
        let fixtures = [
            (
                "If this spell was bargained, put one of those cards with mana value 4 or less onto the battlefield instead of putting it into your hand.",
                StatementReplacementSurfaceKind::BargainedReturnToBattlefield,
            ),
            (
                "If this spell was kicked, put two of those cards into your hand instead. Otherwise, put one of those cards into your hand.",
                StatementReplacementSurfaceKind::KickedCountOverride,
            ),
            (
                "If this spell was kicked, put those cards onto the battlefield instead of putting them into your hand.",
                StatementReplacementSurfaceKind::KickedMultiZoneToBattlefield,
            ),
            (
                "Clash with an opponent, then return target creature to its owner's hand. If you win, you may put that creature on top of its owner's library instead.",
                StatementReplacementSurfaceKind::ClashWinTopOfLibrary,
            ),
            (
                "If a creature died this turn, put that card onto the battlefield instead of putting it into your hand.",
                StatementReplacementSurfaceKind::MorbidSearchToBattlefield,
            ),
        ];

        for (text, expected) in fixtures {
            assert!(
                facts(text).statement.has_replacement_surface(expected),
                "missing {expected:?} for {text}"
            );
        }
    }

    #[test]
    fn collects_creature_type_choice_and_rejects_unrelated_statement_surfaces() {
        assert!(
            facts("Creatures of the creature type of your choice get +2/+2 until end of turn.")
                .statement
                .creature_type_choice_buff
        );

        let unrelated = facts("If this spell was kicked, draw two cards.");
        assert_eq!(
            unrelated.statement.instead_followup.semantics,
            InsteadSemantics::NonReplacement
        );
        assert!(unrelated.statement.replacement_surfaces.is_empty());
        assert!(!unrelated.statement.creature_type_choice_buff);
        assert!(unrelated.statement.trailing_instead_if_predicate.is_none());
    }
}
