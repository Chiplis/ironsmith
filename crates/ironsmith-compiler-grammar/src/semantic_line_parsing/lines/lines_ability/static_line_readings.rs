//! The readings of one static ability line at the CST-to-semantic boundary:
//! the typed line shapes (cost modifiers, standard reminder keywords, the
//! keyword-line group, ...) that lower before the broad static-ability grammar
//! reads the line. Formerly a first-match ladder in `lines_ability`; every
//! reading runs (two different readings are an ambiguity error); the
//! ability-word marker is the wrapper's fallback. The chosen option wraps the
//! resolved reading.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct StaticLine<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) line: &'a RewriteStaticLine,
    /// The broad static-ability reading of the line, computed once for the
    /// readings that build on it.
    pub(super) broad_static: std::cell::OnceCell<
        Result<Option<Vec<crate::cards::builders::StaticAbilityAst>>, CardTextError>,
    >,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl StaticLine<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// The broad static-ability grammar's reading of the whole line.
    pub(super) fn broad_static(
        &self,
    ) -> Result<Option<Vec<crate::cards::builders::StaticAbilityAst>>, CardTextError> {
        self.broad_static
            .get_or_init(|| parse_static_ability_ast_line_lexed(self.tokens))
            .clone()
    }

    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<LineAst>, CardTextError>) -> ParseOutcome<LineAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("static-line-lowering-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&StaticLine<'_>) -> bool,
    read: fn(&StaticLine<'_>) -> ParseOutcome<LineAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("static-line-lowering-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("if-this-spell-costs-less-to-cast-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_if_this_spell_costs_less_to_cast_line(input)),
    },
    Reading {
        id: RuleId::new("spell-additional-life-cost-per-target-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_spell_additional_life_cost_per_target_line(input)),
    },
    Reading {
        id: RuleId::new("spell-cost-increase-per-target-beyond-first-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_spell_cost_increase_per_target_beyond_first_line(input)),
    },
    Reading {
        id: RuleId::new("quoted-granted-ability-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quoted_granted_ability_line(input)),
    },
    Reading {
        id: RuleId::new("spell-and-player-activated-ability-cost-modifier-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_spell_and_player_activated_ability_cost_modifier_line(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("spells-cost-reduction-and-cant-be-countered-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_spells_cost_reduction_and_cant_be_countered_line(input)),
    },
    Reading {
        id: RuleId::new("first-spell-cost-reduction-and-flash-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_first_spell_cost_reduction_and_flash_line(input)),
    },
    Reading {
        id: RuleId::new("spells-cost-modifier-line"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("first-spell-cost-reduction-and-flash-line")
                && !input.read_by("spell-and-player-activated-ability-cost-modifier-line")
                && !input.read_by("spell-cost-increase-per-target-beyond-first-line")
                && !input.read_by("spells-cost-reduction-and-cant-be-countered-line")
        },
        read: |input| input.outcome(read_spells_cost_modifier_line(input)),
    },
    Reading {
        id: RuleId::new("compound-buff-and-unblockable-static-chunk"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_compound_buff_and_unblockable_static_chunk(input)),
    },
    Reading {
        id: RuleId::new("combined-spell-and-activation-tax-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_combined_spell_and_activation_tax_line(input)),
    },
    Reading {
        id: RuleId::new("double-counters-replacement-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_double_counters_replacement_line(input)),
    },
    Reading {
        id: RuleId::new("standard-menace-reminder"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_standard_menace_reminder(input)),
    },
    Reading {
        id: RuleId::new("standard-flanking-reminder"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_standard_flanking_reminder(input)),
    },
    Reading {
        id: RuleId::new("source-keyword-tail"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_keyword_tail(input)),
    },
    Reading {
        id: RuleId::new("additional-land-play-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_additional_land_play_line(input)),
    },
    Reading {
        id: RuleId::new("keyword-group-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_keyword_group_line(input)),
    },
    Reading {
        id: RuleId::new("broad-static-ability-line"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("double-counters-replacement-line")
                && !input.read_by("keyword-group-line")
                && !input.read_by("source-keyword-tail")
                && !input.read_by("spell-additional-life-cost-per-target-line")
                && !input.read_by("spells-cost-modifier-line")
        },
        read: |input| input.outcome(read_broad_static_ability_line(input)),
    },
    Reading {
        id: RuleId::new("skip-keyword-action-probe-line"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("broad-static-ability-line")
                // Readings ranked above this one that read the input read it.
                && !input.read_by("standard-flanking-reminder")
                && !input.read_by("standard-menace-reminder")
        },
        read: |input| input.outcome(read_skip_keyword_action_probe_line(input)),
    },
    Reading {
        id: RuleId::new("split-static-chunk"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("broad-static-ability-line")
        },
        read: |input| input.outcome(read_split_static_chunk(input)),
    },
];

/// The line's reading, if a rule has one. Every admitted reading runs; two
/// different readings of one line are an ambiguity error.
pub(super) fn read(input: &StaticLine<'_>) -> ParseOutcome<RuleMatch<LineAst>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<LineAst>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_if_this_spell_costs_less_to_cast_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(lexed)? {
        return Ok(Some(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}
fn read_spell_additional_life_cost_per_target_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(ability) = parse_spell_additional_life_cost_per_target_line(lexed)? {
        return Ok(Some(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}
fn read_spell_cost_increase_per_target_beyond_first_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(ability) = parse_spell_cost_increase_per_target_beyond_first_line(lexed)? {
        return Ok(Some(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}
fn read_quoted_granted_ability_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    // A quoted cost modifier is the ability granted by the subject before
    // the quote, not a cost modifier whose spell filter includes that outer
    // subject. The static AST router binds the quoted ability to its grant
    // before the broad cost parser scans the whole line for "spells ... cost".
    // Keep that same precedence at the CST-to-semantic boundary: this is the
    // document path used by ordinary card compilation.
    if lexed.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(abilities) = input.broad_static()?
    {
        return Ok(Some(LineAst::StaticAbilities(abilities)));
    }
    Ok(None)
}
fn read_spell_and_player_activated_ability_cost_modifier_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(lexed)? {
        return Ok(Some(LineAst::StaticAbilities(
            abilities.into_iter().map(Into::into).collect(),
        )));
    }
    Ok(None)
}
fn read_spells_cost_reduction_and_cant_be_countered_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    // Keep a compound spell-cost line intact before the broad single cost
    // modifier parser accepts its left clause and discards the terminal
    // countering restriction. The specialized parser reuses one typed spell
    // filter for both executable static abilities.
    if let Some(abilities) =
        crate::keyword_static::parse_spells_cost_reduction_and_cant_be_countered_line(lexed)?
    {
        return Ok(Some(LineAst::StaticAbilities(
            abilities.into_iter().map(Into::into).collect(),
        )));
    }
    Ok(None)
}
fn read_first_spell_cost_reduction_and_flash_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    // Preserve a shared first-spell filter across the coordinated reduction
    // and flash permission before the ordinary cost parser consumes only the
    // left side of the sentence.
    if let Some(abilities) =
        crate::keyword_static::parse_first_spell_cost_reduction_and_flash_line(lexed)?
    {
        return Ok(Some(LineAst::StaticAbilities(abilities)));
    }
    Ok(None)
}
fn read_spells_cost_modifier_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(ability) = parse_spells_cost_modifier_line(lexed)? {
        return Ok(Some(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}
fn read_compound_buff_and_unblockable_static_chunk(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let parse_tokens = input.tokens;
    if let Some(chunk) = parse_compound_buff_and_unblockable_static_chunk(parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
fn read_combined_spell_and_activation_tax_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if semantic_grammar::parse_combined_spell_and_activation_tax_tokens(lexed).is_some()
        && let Some(abilities) = input.broad_static()?
    {
        return Ok(Some(LineAst::StaticAbilities(abilities)));
    }
    Ok(None)
}
fn read_double_counters_replacement_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(ability) = crate::keyword_static::parse_double_counters_replacement_line(lexed)? {
        return Ok(Some(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}
fn read_standard_menace_reminder(input: &StaticLine<'_>) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let lexed = input.tokens;
    if has_standard_menace_reminder(&line.info.source_tokens)
        && matches!(
            parse_ability_line_lexed(lexed).as_deref(),
            Some([KeywordAction::Menace])
        )
    {
        return Ok(Some(LineAst::StaticAbility(
            StaticAbility::menace()
                .with_text(STANDARD_MENACE_REMINDER)
                .into(),
        )));
    }
    Ok(None)
}
fn read_standard_flanking_reminder(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let lexed = input.tokens;
    if has_standard_flanking_reminder(&line.info.raw_line)
        && matches!(
            parse_ability_line_lexed(lexed).as_deref(),
            Some([KeywordAction::Flanking])
        )
    {
        return Ok(Some(LineAst::StaticAbility(
            StaticAbility::flanking()
                .with_text(STANDARD_FLANKING_REMINDER)
                .into(),
        )));
    }
    Ok(None)
}
fn read_source_keyword_tail(input: &StaticLine<'_>) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(actions) = semantic_grammar::parse_source_keyword_tail_tokens(lexed)
        .and_then(|tail| parse_ability_line_lexed(tail.ability_tokens))
    {
        return Ok(Some(LineAst::Abilities(actions)));
    }
    Ok(None)
}
fn read_additional_land_play_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    if let Some(abilities) = crate::keyword_static::parse_additional_land_play_line(lexed)? {
        let abilities = abilities
            .into_iter()
            .map(crate::cards::builders::StaticAbilityAst::Static)
            .collect();
        return Ok(Some(LineAst::StaticAbilities(abilities)));
    }
    Ok(None)
}
fn read_keyword_group_line(input: &StaticLine<'_>) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    // A complete comma-separated keyword line is one authored ability line,
    // even when an individual keyword (for example cascade) also has a
    // specialized static-ability representation. Keep the group provenance
    // before the broad static parser claims each member independently.
    if let Some(actions) = parse_ability_line_lexed(lexed)
        && actions.len() > 1
    {
        return Ok(Some(LineAst::Abilities(actions)));
    }
    Ok(None)
}
fn read_broad_static_ability_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let parse_tokens = input.tokens;
    match input.broad_static() {
        Ok(Some(mut abilities)) => {
            restore_copy_static_variant_source_display(&mut abilities, &line.info.raw_line);
            restore_named_characteristic_subject_surface(&mut abilities, &line.info.source_tokens);
            return Ok(Some(LineAst::StaticAbilities(abilities)));
        }
        Ok(None) => {}
        Err(_)
            if parse_tokens
                .iter()
                .any(|token| token.kind == TokenKind::Period) => {}
        Err(err) => return Err(err),
    }
    Ok(None)
}
fn read_skip_keyword_action_probe_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let lexed = input.tokens;
    let parse_tokens = input.tokens;
    if semantic_grammar::parse_skip_keyword_action_probe_tokens(parse_tokens).is_none()
        && let Some(actions) = parse_ability_line_lexed(lexed)
    {
        return Ok(Some(LineAst::Abilities(actions)));
    }
    Ok(None)
}
fn read_split_static_chunk(input: &StaticLine<'_>) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let parse_tokens = input.tokens;
    if let Some(chunk) = parse_split_static_chunk(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
pub(super) fn read_ability_word_marker_line(
    input: &StaticLine<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let parse_tokens = input.tokens;
    if semantic_grammar::parse_ability_word_marker_tokens(parse_tokens).is_some() {
        return Ok(Some(LineAst::StaticAbility(
            StaticAbility::keyword_marker(render_token_slice(parse_tokens).trim().to_string())
                .into(),
        )));
    }
    Ok(None)
}
