use super::super::lexer::OwnedLexToken;
use super::super::rule_engine::{LexClauseView, LexUnsupportedDiagnoser, LexUnsupportedRuleDef};
use super::dispatch_inner as inner;
use crate::cards::builders::CardTextError;
use crate::recognition::{ParseOutcome, RuleId};
use crate::registry::{HeadDiscriminator, RegistryRuleMetadata};

const SENTENCE_UNSUPPORTED_RULES_LEXED: [LexUnsupportedRuleDef; 28] = [
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("enters-as-copy"),
            HeadDiscriminator::words(&[]),
        ),
        shape_mask: 0,
        message: "unsupported enters-as-copy replacement clause",
        predicate: inner::sentence_has_enters_as_copy_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("each-player-lose-discard-sacrifice-chain"),
            HeadDiscriminator::words(&["each"]),
        ),
        shape_mask: 0,
        message: "unsupported each-player lose/discard/sacrifice chain clause",
        predicate: inner::sentence_has_each_player_lose_discard_sacrifice_chain_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("each-player-exile-sacrifice-return-this-way"),
            HeadDiscriminator::words(&["each"]),
        ),
        shape_mask: 0,
        message: "unsupported each-player exile/sacrifice/return-this-way clause",
        predicate: inner::sentence_has_each_player_exile_sacrifice_return_exiled_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("put-one-into-hand-rest-zone"),
            HeadDiscriminator::words(&["put", "then"]),
        ),
        shape_mask: 0,
        message: "unsupported put-into-hand with rest clause",
        predicate: inner::sentence_has_put_one_of_them_into_hand_rest_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("lose-all-abilities-with-becomes"),
            HeadDiscriminator::words(&["target", "that", "it", "this", "creatures"]),
        ),
        shape_mask: 0,
        message: "unsupported loses-all-abilities with becomes clause",
        predicate: inner::sentence_has_loses_all_abilities_with_becomes_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("spent-to-cast-conditional"),
            HeadDiscriminator::words(&["if", "unless", "when", "as"]),
        ),
        shape_mask: 0,
        message: "unsupported spent-to-cast conditional clause",
        predicate: inner::sentence_has_spent_to_cast_this_spell_without_condition_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("would-enter-instead"),
            HeadDiscriminator::words(&["if", "that", "it", "this"]),
        ),
        shape_mask: 0,
        message: "unsupported would-enter replacement clause",
        predicate: inner::sentence_has_would_enter_instead_replacement_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("different-mana-value-constraint"),
            HeadDiscriminator::words(&[]),
        ),
        shape_mask: 0,
        message: "unsupported different-mana-value constraint clause",
        predicate: inner::sentence_has_different_mana_value_constraint_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("most-common-color-constraint"),
            HeadDiscriminator::words(&["choose", "destroy", "exile", "return"]),
        ),
        shape_mask: 0,
        message: "unsupported most-common-color constraint clause",
        predicate: inner::sentence_has_most_common_color_constraint_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("power-vs-count-constraint"),
            HeadDiscriminator::words(&["if", "target", "destroy", "exile", "return"]),
        ),
        shape_mask: 0,
        message: "unsupported power-vs-count conditional clause",
        predicate: inner::sentence_has_power_vs_count_constraint_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("put-into-graveyards-from-battlefield-this-turn"),
            HeadDiscriminator::words(&["for", "choose", "target", "destroy"]),
        ),
        shape_mask: 0,
        message: "unsupported put-into-graveyards-from-battlefield count clause",
        predicate: inner::sentence_has_put_into_graveyards_from_battlefield_this_turn_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("phase-out-until-leaves"),
            HeadDiscriminator::words(&["phase", "target", "it", "that"]),
        ),
        shape_mask: 0,
        message: "unsupported phase-out-until-leaves clause",
        predicate: inner::sentence_has_phase_out_until_leaves_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("same-name-as-another-in-hand"),
            HeadDiscriminator::words(&["target", "choose", "discard"]),
        ),
        shape_mask: 0,
        message: "unsupported same-name-as-another-in-hand discard clause",
        predicate: inner::sentence_has_same_name_as_another_in_hand_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("for-each-mana-from-spent"),
            HeadDiscriminator::words(&["for"]),
        ),
        shape_mask: 0,
        message: "unsupported for-each-mana-from-spent clause",
        predicate: inner::sentence_has_for_each_mana_from_spent_to_cast_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("when-you-sacrifice-this-way"),
            HeadDiscriminator::words(&["when"]),
        ),
        shape_mask: 0,
        message: "unsupported when-you-sacrifice-this-way clause",
        predicate: inner::sentence_has_when_you_sacrifice_this_way_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("greatest-mana-value"),
            HeadDiscriminator::words(&["choose", "destroy", "exile", "return"]),
        ),
        shape_mask: 0,
        message: "unsupported greatest-mana-value selection clause",
        predicate: inner::sentence_has_greatest_mana_value_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("least-power-among-creatures"),
            HeadDiscriminator::words(&["choose", "destroy", "exile", "return"]),
        ),
        shape_mask: 0,
        message: "unsupported least-power-among-creatures selection clause",
        predicate: inner::sentence_has_least_power_among_creatures_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("villainous-choice"),
            HeadDiscriminator::words(&["villainous"]),
        ),
        shape_mask: 0,
        message: "unsupported villainous-choice clause",
        predicate: inner::sentence_has_villainous_choice_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("divided-evenly"),
            HeadDiscriminator::words(&["divide", "deals", "deal", "distribute"]),
        ),
        shape_mask: 0,
        message: "unsupported divided-evenly damage clause",
        predicate: inner::sentence_has_divided_evenly_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("different-names"),
            HeadDiscriminator::words(&["choose", "target", "destroy", "exile"]),
        ),
        shape_mask: 0,
        message: "unsupported different-names selection clause",
        predicate: inner::sentence_has_different_names_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("chosen-at-random"),
            HeadDiscriminator::words(&["choose", "target", "discard", "exile"]),
        ),
        shape_mask: 0,
        message: "unsupported chosen-at-random clause",
        predicate: inner::sentence_has_chosen_at_random_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("defending-players-choice"),
            HeadDiscriminator::words(&["defending", "target", "of"]),
        ),
        shape_mask: 0,
        message: "unsupported defending-players-choice clause",
        predicate: inner::sentence_has_defending_players_choice_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("creature-token-player-planeswalker-target"),
            HeadDiscriminator::words(&["target"]),
        ),
        shape_mask: 0,
        message: "unsupported creature-token/player/planeswalker target clause",
        predicate: inner::sentence_has_target_creature_token_player_planeswalker_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("if-you-sacrifice-an-island-this-way"),
            HeadDiscriminator::words(&["if"]),
        ),
        shape_mask: 0,
        message: "unsupported if-you-sacrifice-an-island-this-way clause",
        predicate: inner::sentence_has_if_you_sacrifice_an_island_this_way_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("spent-to-cast-condition"),
            HeadDiscriminator::words(&["if", "unless", "when", "as"]),
        ),
        shape_mask: 0,
        message: "unsupported spent-to-cast condition clause",
        predicate: inner::sentence_has_spent_to_cast_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("face-down"),
            HeadDiscriminator::words(&["face", "turn", "cast", "exile", "manifest"]),
        ),
        shape_mask: 0,
        message: "unsupported face-down clause",
        predicate: inner::sentence_has_face_down_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("return-each-creature-that-isnt-list"),
            HeadDiscriminator::words(&["return"]),
        ),
        shape_mask: 0,
        message: "unsupported return-each-creature-that-isnt-list clause",
        predicate: inner::sentence_has_return_each_creature_that_isnt_list_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("negated-untap"),
            HeadDiscriminator::words(&["this", "that", "target", "it", "creatures", "players"]),
        ),
        shape_mask: 0,
        message: "unsupported negated untap clause",
        predicate: inner::sentence_has_unsupported_negated_untap_clause_rule_lexed,
    },
];

const SENTENCE_UNSUPPORTED_DIAGNOSER_LEXED: LexUnsupportedDiagnoser =
    LexUnsupportedDiagnoser::new(&SENTENCE_UNSUPPORTED_RULES_LEXED);

pub(super) fn diagnose_sentence_unsupported_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CardTextError> {
    if inner::sentence_looks_like_supported_negated_untap_clause(tokens) {
        return None;
    }
    let view = LexClauseView::from_tokens(tokens);
    match SENTENCE_UNSUPPORTED_DIAGNOSER_LEXED.diagnose(&view, "clause") {
        ParseOutcome::NoMatch => None,
        ParseOutcome::Error(diagnostic) => Some(diagnostic.into_legacy_error()),
        ParseOutcome::Match(_) => None,
    }
}

pub(super) fn diagnose_known_partial_parse_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CardTextError> {
    let partial_shape =
        crate::grammar::effects::has_different_mana_value_constraint_sentence_lexed(
            tokens,
        )
        || crate::grammar::effects::has_put_into_graveyards_from_battlefield_this_turn_sentence_lexed(
            tokens,
        )
        || crate::grammar::effects::has_phase_out_until_leaves_clause_sentence_lexed(
            tokens,
        );
    partial_shape
        .then(|| diagnose_sentence_unsupported_lexed(tokens))
        .flatten()
}
