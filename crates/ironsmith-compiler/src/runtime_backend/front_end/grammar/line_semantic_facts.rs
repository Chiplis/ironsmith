use crate::runtime_backend::shared_types::{
    InsteadFollowupFacts, LineSemanticFacts, StatementConditionIntro, StatementLineSemanticFacts,
    StaticLineSemanticFacts, ThisSpellCostFacts, TriggerFrequencyFacts, TriggerFunctionalZoneFacts,
    TriggeredLineSemanticFacts,
};

use super::super::lexer::OwnedLexToken;
use super::{
    activated_lines, effects, functional_zones, leaf, lowering_surfaces, primitives, structure,
    trigger_surface,
};

fn parse_trailing_instead_if_predicate(
    tokens: &[OwnedLexToken],
) -> Option<crate::runtime_backend::ast::PredicateAst> {
    let (instead_idx, _, _) =
        primitives::find_prefix(tokens, || (primitives::kw("instead"), primitives::kw("if")))?;
    structure::parse_trailing_instead_if_predicate_lexed(&tokens[instead_idx..])
}

fn parse_statement_semantic_facts(tokens: &[OwnedLexToken]) -> StatementLineSemanticFacts {
    let instead = effects::parse_instead_followup_shape_tokens(tokens);
    let leading_condition_intro =
        leaf::parse_leaf_condition_intro_prefix_tokens(tokens).map(|prefix| match prefix.intro {
            leaf::ConditionIntro::If => StatementConditionIntro::If,
            leaf::ConditionIntro::Unless => StatementConditionIntro::Unless,
            leaf::ConditionIntro::AsLongAs => StatementConditionIntro::AsLongAs,
            leaf::ConditionIntro::ForAsLongAs => StatementConditionIntro::ForAsLongAs,
        });
    let replacement_surfaces =
        lowering_surfaces::parse_statement_replacement_surface_tokens(tokens)
            .into_iter()
            .collect();

    StatementLineSemanticFacts {
        instead_followup: InsteadFollowupFacts {
            semantics: instead.semantics,
            conditional_intro: instead.conditional_intro,
        },
        trailing_instead_if_predicate: parse_trailing_instead_if_predicate(tokens),
        replacement_surfaces,
        presentation_label: None,
        creature_type_choice_buff: lowering_surfaces::parse_creature_type_choice_buff_tokens(
            tokens,
        )
        .is_some(),
        leading_condition_intro,
    }
}

pub(crate) fn parse_line_semantic_facts_tokens(tokens: &[OwnedLexToken]) -> LineSemanticFacts {
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
        },
        statement: parse_statement_semantic_facts(tokens),
        triggered_ability: TriggeredLineSemanticFacts {
            intro_surface: trigger_surface::parse_trigger_intro_surface_tokens(tokens),
            functional_zones: TriggerFunctionalZoneFacts {
                explicit_zone: trigger_zones.explicit_zone,
                returns_self_from_graveyard: trigger_zones.returns_self_from_graveyard,
                discards_this_card: trigger_zones.discards_this_card,
            },
            becomes_tapped_during_your_turn:
                trigger_surface::parse_becomes_tapped_during_your_turn_tokens(tokens).is_some(),
            frequency: TriggerFrequencyFacts {
                first_time_each_or_this_turn: trigger_frequency.first_time_each_or_this_turn,
                becomes_crewed: trigger_frequency.becomes_crewed,
                do_this_limit_each_turn: trigger_frequency.do_this_limit_each_turn,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::InsteadSemantics;
    use crate::runtime_backend::front_end::lexer::lex_line;
    use crate::runtime_backend::shared_types::StatementReplacementSurfaceKind;
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
                "Clash with an opponent. If you win, put it on top of its owner's library instead.",
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
