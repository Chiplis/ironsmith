use crate::cards::builders::ForEachEffectAst;
use super::super::clause_support::parse_triggered_line_lexed;
use super::super::grammar::effects::{
    clause_primitive_shapes as clause_shapes, parse_unless_pays_shape_tokens,
    split_change_target_clause_lexed, split_change_target_unless_clause_lexed,
    split_choose_new_targets_clause_lexed,
};
use super::super::lexer::{LexedClause, TokenWordView, render_token_slice};
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    parse_additional_land_plays_clause, parse_cast_or_play_tagged_clause,
    parse_cast_spells_as_though_they_had_flash_clause,
    parse_unsupported_play_cast_permission_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::util::{
    parse_subject, parse_target_phrase, source_reference_surface_for_words, span_from_tokens,
};
use super::parse_restriction_duration;
use super::sentence_helpers::*;
use super::subject_verb_primitives::SubjectVerbPrimitiveClause;
use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, LineAst, OwnedLexToken, PlayerAst,
    RetargetModeAst, SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, LifeResourceActionAst, DamageActionAst, DelayedEffectAst, ConditionalEffectAst,
};
use crate::effect::Value;
use crate::grammar::effects::typed_clause_heads::classify_typed_clause_head;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::zone::Zone;
use winnow::Parser;
use winnow::combinator::alt;

pub type ClausePrimitiveParser = fn(&[OwnedLexToken]) -> ParseOutcome<EffectAst>;

pub struct ClausePrimitive {
    pub metadata: RegistryRuleMetadata,
    pub phase: ClausePrimitivePhase,
    pub parser: ClausePrimitiveParser,
}

fn clause_primitive_outcome(
    rule: RuleId,
    tokens: &[OwnedLexToken],
    result: Result<Option<EffectAst>, CardTextError>,
) -> ParseOutcome<EffectAst> {
    let span = span_from_tokens(tokens);
    match result {
        Ok(Some(effect)) => ParseOutcome::matched(effect, span),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(rule, span, error)),
    }
}

macro_rules! specific_primitive {
    ($id:literal, $heads:expr, $parser:path $(,)?) => {
        ClausePrimitive::specific($id, $heads, |tokens| {
            clause_primitive_outcome(RuleId::new($id), tokens, $parser(tokens))
        })
    };
}

macro_rules! fallback_primitive {
    ($id:literal, $parser:path $(,)?) => {
        ClausePrimitive::fallback($id, |tokens| {
            clause_primitive_outcome(RuleId::new($id), tokens, $parser(tokens))
        })
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClausePrimitivePhase {
    Specific,
    GenericFallback,
}

impl ClausePrimitive {
    const fn specific(
        id: &'static str,
        heads: &'static [&'static str],
        parser: ClausePrimitiveParser,
    ) -> Self {
        Self {
            metadata: RegistryRuleMetadata::distinct(
                RuleId::new(id),
                HeadDiscriminator::words(heads),
            ),
            phase: ClausePrimitivePhase::Specific,
            parser,
        }
    }

    const fn fallback(id: &'static str, parser: ClausePrimitiveParser) -> Self {
        Self {
            metadata: RegistryRuleMetadata::distinct(
                RuleId::new(id),
                HeadDiscriminator::grammar("typed-effect-clause-head"),
            ),
            phase: ClausePrimitivePhase::GenericFallback,
            parser,
        }
    }
}
pub fn parse_retarget_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_choose_new_targets_clause(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_change_target_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}

pub fn parse_copy_targets_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_copy_targets_shape(tokens) else {
        return Ok(None);
    };
    if shape.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target after copy-target clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }
    let fixed_target = match parse_target_phrase(shape.target_tokens) {
        Ok(target) => target,
        Err(_) if crate::lexer::is_bare_card_name_phrase(shape.target_tokens) => {
            TargetAst::Source(LexedClause::new(shape.target_tokens).span())
        }
        Err(error) => return Err(error),
    };
    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::CopiedStackObject.bind(),
            LexedClause::new(tokens).span(),
        ),
        RetargetModeAst::OneToFixed {
            target: fixed_target,
        },
        false,
    )))
}

pub fn parse_choose_new_targets_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(split) = split_choose_new_targets_clause_lexed(tokens) else {
        return Ok(None);
    };
    let plural_copy_reference = crate::word_primitives::sequence_occurs(
        &crate::lexer::token_word_refs(tokens),
        &["copies"],
    );
    if split.reference_target {
        let reference_tag = match clause_shapes::parse_retarget_reference_shape(split.target_tokens)
        {
            Some(clause_shapes::RetargetReferenceShape::Copy) => {
                crate::tag::CompilerReferenceTag::CopiedStackObject.bind()
            }
            Some(clause_shapes::RetargetReferenceShape::Other) => {
                crate::tag::CompilerReferenceTag::It.bind()
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "missing typed retarget reference shape (clause: '{}')",
                    LexedClause::new(tokens).text()
                )));
            }
        };
        let target = TargetAst::Tagged(reference_tag, span_from_tokens(split.target_tokens));
        return Ok(Some(
            EffectAst::subject_verb_retarget_stack_object(
                PlayerAst::Implicit,
                target,
                RetargetModeAst::All,
                false,
            )
            .with_retarget_plural_copy_reference(plural_copy_reference),
        ));
    }
    let tail_tokens = split.target_tokens;
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing choose-new-targets target".to_string(),
        ));
    }

    let filter = parse_stack_retarget_filter(tail_tokens)?;

    let mut target = TargetAst::Object(
        filter,
        if split.explicit_target {
            span_from_tokens(tail_tokens)
        } else {
            None
        },
        None,
    );
    if let Some(count) = split.count {
        target = TargetAst::WithCount(Box::new(target), count);
    }

    Ok(Some(
        EffectAst::subject_verb_retarget_stack_object(
            PlayerAst::Implicit,
            target,
            RetargetModeAst::All,
            false,
        )
        .with_retarget_plural_copy_reference(plural_copy_reference),
    ))
}

pub fn parse_change_target_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.first_word() != Some("change") {
        return Ok(None);
    }

    if let Some((main_tokens, unless_tokens)) = split_change_target_unless_clause_lexed(tokens) {
        let Some(inner) = parse_change_target_clause_inner(main_tokens)? else {
            return Ok(None);
        };
        let (player, cost) = parse_unless_pays_clause(unless_tokens)?;
        return Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
            effects: vec![inner],
            player,
            cost,
            before_delayed_step: false,
        })));
    }

    parse_change_target_clause_inner(tokens)
}

pub fn parse_change_target_clause_inner(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(split) = split_change_target_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target after change-the-target clause".to_string(),
        ));
    }

    let tail_tokens = split.target_tokens;
    let mut filter = parse_stack_retarget_filter(&tail_tokens)?;

    for constraint in clause_shapes::parse_retarget_constraint_shapes(&tail_tokens) {
        filter = apply_retarget_constraint(filter, constraint);
    }

    let target = TargetAst::Object(filter, span_from_tokens(tokens), None);

    let mode = if split.fixed_to_source {
        RetargetModeAst::OneToFixed {
            target: TargetAst::Source(span_from_tokens(tokens)),
        }
    } else {
        RetargetModeAst::All
    };

    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        target,
        mode,
        true,
    )))
}

fn apply_retarget_constraint(
    filter: ObjectFilter,
    constraint: clause_shapes::RetargetConstraintShape,
) -> ObjectFilter {
    match constraint {
        clause_shapes::RetargetConstraintShape::SingleTarget => filter.target_count_exact(1),
        clause_shapes::RetargetConstraintShape::SingleCreatureTarget => filter
            .targeting_only_object(ObjectFilter::creature())
            .target_count_exact(1),
        clause_shapes::RetargetConstraintShape::SourceOnlyTarget => filter
            .targeting_only_object(ObjectFilter::source())
            .target_count_exact(1),
        clause_shapes::RetargetConstraintShape::YouOnlyTarget => filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1),
        clause_shapes::RetargetConstraintShape::AnyPlayerTarget => filter
            .targeting_only_player(PlayerFilter::Any)
            .target_count_exact(1),
    }
}

pub fn parse_unless_pays_clause(
    tokens: &[OwnedLexToken],
) -> Result<
    (
        PlayerAst,
        ironsmith_core::TotalCost<crate::model::CompilerCost>,
    ),
    CardTextError,
> {
    let shape = parse_unless_pays_shape_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing typed unless-payment shape (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))
    })?;
    let player = match parse_subject(shape.player_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };
    let cost = crate::activation_and_restrictions::parse_payment_clause_as_total_cost(
        shape.payment_tokens,
    )?
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported unless-payment clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))
    })?;

    Ok((player, cost))
}

pub fn parse_stack_retarget_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let Some(shape) = clause_shapes::parse_stack_retarget_filter_shape(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported retarget target clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    };
    let mut filter = match shape.kind {
        clause_shapes::StackRetargetFilterKind::ActivatedAbility => {
            ObjectFilter::activated_ability()
        }
        clause_shapes::StackRetargetFilterKind::SpellOrAbility => ObjectFilter::spell_or_ability(),
        clause_shapes::StackRetargetFilterKind::Ability => ObjectFilter::ability(),
        clause_shapes::StackRetargetFilterKind::InstantOrSorcery => {
            ObjectFilter::instant_or_sorcery()
        }
        clause_shapes::StackRetargetFilterKind::Spell => ObjectFilter::spell(),
    };
    filter.other = shape.other;

    Ok(filter)
}

pub fn run_clause_primitives(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    const PRIMITIVES: &[ClausePrimitive] = &[
        specific_primitive!(
            "choose-card-name-clause",
            &["choose"],
            parse_choose_card_name_clause,
        ),
        specific_primitive!(
            "repeat-this-process-clause",
            &["repeat"],
            parse_repeat_this_process_clause,
        ),
        specific_primitive!(
            "retain-mana-through-steps-clause",
            &["you", "mana"],
            parse_dont_lose_this_mana_as_steps_and_phases_end_clause,
        ),
        specific_primitive!(
            "retarget-clause",
            &["choose", "change"],
            parse_retarget_clause,
        ),
        specific_primitive!(
            "copy-targets-clause",
            &["the", "copy"],
            parse_copy_targets_clause,
        ),
        specific_primitive!("copy-spell-clause", &["copy"], parse_copy_spell_clause),
        specific_primitive!(
            "win-game-clause",
            &["you", "target", "that", "its"],
            parse_win_the_game_clause,
        ),
        specific_primitive!(
            "damage-equal-power-clause",
            &["it", "that", "target", "this"],
            parse_deal_damage_equal_to_power_clause,
        ),
        specific_primitive!(
            "anaphoric-object-damage-clause",
            &["it", "that", "those"],
            parse_anaphoric_object_deals_damage_clause,
        ),
        specific_primitive!("fight-clause", &[], parse_fight_clause),
        specific_primitive!(
            "clash-clause",
            &["clash", "you", "target"],
            parse_clash_clause,
        ),
        specific_primitive!(
            "for-each-target-player-clause",
            &["for", "any"],
            parse_for_each_target_players_clause,
        ),
        specific_primitive!(
            "each-player-exiles-hand-clause",
            &["each"],
            parse_each_player_exiles_hand_face_down_and_draws_clause,
        ),
        specific_primitive!(
            "each-player-return-counter-clause",
            &["each"],
            parse_each_player_return_with_additional_counter_clause,
        ),
        specific_primitive!(
            "for-each-opponent-clause",
            &["for", "each"],
            parse_for_each_opponent_clause,
        ),
        specific_primitive!(
            "for-each-player-clause",
            &["for", "each"],
            parse_for_each_player_clause,
        ),
        specific_primitive!(
            "double-counters-clause",
            &["double"],
            parse_double_counters_clause,
        ),
        specific_primitive!(
            "distribute-counters-clause",
            &["distribute"],
            parse_distribute_counters_clause,
        ),
        specific_primitive!(
            "until-end-turn-play-tagged-clause",
            &["until", "you"],
            parse_until_end_of_turn_may_play_tagged_clause,
        ),
        specific_primitive!(
            "until-next-turn-play-tagged-clause",
            &["until", "you"],
            parse_until_your_next_turn_may_play_tagged_clause,
        ),
        specific_primitive!(
            "additional-land-play-clause",
            &["you", "that"],
            parse_additional_land_plays_clause,
        ),
        specific_primitive!(
            "cast-as-flash-clause",
            &["you", "spells"],
            parse_cast_spells_as_though_they_had_flash_clause,
        ),
        specific_primitive!(
            "unsupported-play-cast-permission-clause",
            &["you", "that"],
            parse_unsupported_play_cast_permission_clause,
        ),
        specific_primitive!(
            "cast-or-play-tagged-clause",
            &["you", "that", "its"],
            parse_cast_or_play_tagged_clause,
        ),
        specific_primitive!(
            "prevent-next-damage-clause",
            &["prevent", "the"],
            parse_prevent_next_damage_clause,
        ),
        specific_primitive!(
            "prevent-all-damage-clause",
            &["prevent", "all"],
            parse_prevent_all_damage_clause,
        ),
        specific_primitive!(
            "attack-as-though-no-defender-clause",
            &["it", "they", "target"],
            parse_can_attack_as_though_no_defender_clause,
        ),
        specific_primitive!(
            "block-additional-creature-clause",
            &["it", "they", "target"],
            parse_can_block_additional_creature_this_turn_clause,
        ),
        specific_primitive!(
            "attack-or-block-if-able-clause",
            &[
                "all", "another", "attack", "attacks", "block", "blocks", "each", "it", "that",
                "they", "those", "target", "up",
            ],
            parse_attack_or_block_this_turn_if_able_clause,
        ),
        specific_primitive!(
            "attack-if-able-clause",
            &[
                "all", "another", "attack", "attacks", "each", "it", "that", "they", "those",
                "target", "up",
            ],
            parse_attack_this_turn_if_able_clause,
        ),
        specific_primitive!(
            "must-be-blocked-clause",
            &["it", "they", "target"],
            parse_must_be_blocked_if_able_clause,
        ),
        specific_primitive!(
            "must-block-clause",
            &[
                "all", "another", "each", "it", "that", "they", "those", "target",
            ],
            parse_must_block_if_able_clause,
        ),
        specific_primitive!(
            "until-duration-triggered-clause",
            &["until"],
            parse_until_duration_triggered_clause,
        ),
        specific_primitive!(
            "keyword-mechanic-clause",
            &[],
            parse_keyword_mechanic_clause,
        ),
        specific_primitive!(
            "connive-clause",
            &["connive", "target"],
            parse_connive_clause,
        ),
        fallback_primitive!(
            "choose-target-action-fallback",
            parse_choose_target_and_verb_clause,
        ),
        fallback_primitive!("verb-first-clause-fallback", parse_verb_first_clause),
    ];

    fn recognize_phase(
        tokens: &[OwnedLexToken],
        primitives: &'static [ClausePrimitive],
        phase: ClausePrimitivePhase,
    ) -> ParseOutcome<RuleMatch<EffectAst>> {
        let typed_head = match classify_typed_clause_head(tokens)
            .within(RuleId::new("effect-clause-primitive-registry"))
        {
            ParseOutcome::NoMatch => return ParseOutcome::NoMatch,
            ParseOutcome::Match(matched) => matched.value,
            ParseOutcome::Error(diagnostic) => return ParseOutcome::Error(diagnostic),
        };
        if phase == ClausePrimitivePhase::GenericFallback && !typed_head.permits_action_fallback() {
            return ParseOutcome::NoMatch;
        }
        let mut candidates = Vec::new();
        let mut diagnostics = Vec::new();
        for primitive in primitives.iter().filter(|primitive| {
            primitive.phase == phase && primitive.metadata.head.accepts(typed_head.first_word)
        }) {
            let outcome = (primitive.parser)(tokens).within(primitive.metadata.id);
            match outcome {
                ParseOutcome::NoMatch => {}
                ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                    primitive.metadata,
                    matched.value,
                    matched.span,
                )),
                ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
            }
        }
        // The equal-to-power grammar is the typed specialization of the
        // general anaphoric damage sentence and retains the correlated value
        // source. Once it claims the complete clause, the generic anaphoric
        // interpretation is not an independent semantic candidate.
        if candidates
            .iter()
            .any(|candidate| candidate.metadata.id.as_str() == "damage-equal-power-clause")
        {
            candidates.retain(|candidate| {
                candidate.metadata.id.as_str() != "anaphoric-object-damage-clause"
            });
        }
        resolve_registry_candidates(
            RuleId::new("effect-clause-primitive-registry"),
            candidates,
            diagnostics,
        )
    }

    let recognized = match recognize_phase(tokens, PRIMITIVES, ClausePrimitivePhase::Specific) {
        ParseOutcome::NoMatch => {
            recognize_phase(tokens, PRIMITIVES, ClausePrimitivePhase::GenericFallback)
        }
        outcome => outcome,
    };
    match recognized {
        ParseOutcome::NoMatch => Ok(None),
        ParseOutcome::Match(matched) => Ok(Some(matched.value.value)),
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
    }
}

pub fn parse_choose_card_name_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_choose_card_name_shape(tokens) else {
        return Ok(None);
    };
    let filter = shape
        .filter_tokens
        .map(|filter_tokens| {
            parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported choose-card-name filter (clause: '{}')",
                    LexedClause::new(tokens).text()
                ))
            })
        })
        .transpose()?;

    Ok(Some(EffectAst::subject_verb_choose_card_name(
        shape.player,
        filter,
        crate::tag::CompilerReferenceTag::ChosenName.bind(),
    )))
}

pub fn parse_repeat_this_process_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    Ok(
        clause_shapes::parse_repeat_process_shape(tokens).map(|shape| match shape {
            clause_shapes::RepeatProcessShape::Required => EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess),
            clause_shapes::RepeatProcessShape::Once => EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce),
            clause_shapes::RepeatProcessShape::May => EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay),
        }),
    )
}

pub fn parse_dont_lose_this_mana_as_steps_and_phases_end_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if clause_shapes::is_dont_lose_mana_between_steps_shape(tokens) {
        return Ok(Some(
            EffectAst::subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn(),
        ));
    }
    Ok(None)
}

pub fn parse_each_player_exiles_hand_face_down_and_draws_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if !clause_shapes::is_each_player_exiles_hand_face_down_and_draws_shape(tokens) {
        return Ok(None);
    }

    let mut hand_cards = ObjectFilter::default();
    hand_cards.zone = Some(Zone::Hand);
    hand_cards.owner = Some(PlayerFilter::IteratedPlayer);

    Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_exile(TargetAst::Object(hand_cards, None, None), true),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::That,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                    count: Value::Fixed(7),
                }),
            ),
        ],
    })))
}

pub fn parse_each_player_return_with_additional_counter_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(mut effects) = parse_sentence_each_player_return_with_additional_counter(
        SubjectVerbPrimitiveClause::new(tokens),
    )?
    else {
        return Ok(None);
    };

    Ok(Some(if effects.len() == 1 {
        effects.remove(0)
    } else {
        EffectAst::Sequence { effects }
    }))
}

pub fn parse_attack_or_block_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some(shape) = clause_shapes::parse_combat_requirement_shape(tokens) else {
        return Ok(None);
    };
    if shape.kind != clause_shapes::CombatRequirementKind::AttackOrBlock {
        return Ok(None);
    }
    let duration = match shape.duration {
        clause_shapes::CombatRequirementDuration::Turn => Until::EndOfTurn,
        clause_shapes::CombatRequirementDuration::Combat => Until::EndOfCombat,
    };
    let subject_clause = LexedClause::new(shape.subject_tokens).trimmed();
    let result_filter = parse_dealt_damage_this_way_subject_filter(subject_clause.tokens())?;
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span())
    } else if let Some(filter) = result_filter.clone() {
        TargetAst::Object(filter, None, clause.span())
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };
    let abilities = vec![GrantedAbilityAst::MustAttack, GrantedAbilityAst::MustBlock];

    // A demonstrative subject ("that creature") is a back-reference, not a
    // filtered requirement over every creature. The dealt-damage-this-way
    // subjects keep their filtered form.
    let demonstrative_backref = result_filter.is_none()
        && subject_clause
            .tokens()
            .first()
            .and_then(crate::lexer::OwnedLexToken::as_word)
            .is_some_and(|word| matches!(word, "that" | "it"));
    if subject_clause.is_empty()
        || starts_with_target_indicator(subject_clause.tokens())
        || demonstrative_backref
    {
        let target = if demonstrative_backref {
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span())
        } else {
            target
        };
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target, abilities, duration,
        )));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker/blocker subject in attacks-or-blocks-if-able clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter, abilities, duration,
    )))
}

pub fn parse_attack_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some(shape) = clause_shapes::parse_combat_requirement_shape(tokens) else {
        return Ok(None);
    };
    if shape.kind != clause_shapes::CombatRequirementKind::Attack {
        return Ok(None);
    }
    let duration = match shape.duration {
        clause_shapes::CombatRequirementDuration::Turn => Until::EndOfTurn,
        clause_shapes::CombatRequirementDuration::Combat => Until::EndOfCombat,
    };
    let subject_clause = LexedClause::new(shape.subject_tokens).trimmed();
    let result_filter = parse_dealt_damage_this_way_subject_filter(subject_clause.tokens())?;
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span())
    } else if let Some(filter) = result_filter.clone() {
        TargetAst::Object(filter, None, clause.span())
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };
    let ability = GrantedAbilityAst::MustAttack;

    // A demonstrative subject ("that creature") is a back-reference, not a
    // filtered requirement over every creature. The dealt-damage-this-way
    // subjects keep their filtered form.
    let demonstrative_backref = result_filter.is_none()
        && subject_clause
            .tokens()
            .first()
            .and_then(crate::lexer::OwnedLexToken::as_word)
            .is_some_and(|word| matches!(word, "that" | "it"));
    if subject_clause.is_empty()
        || starts_with_target_indicator(subject_clause.tokens())
        || demonstrative_backref
    {
        let target = if demonstrative_backref {
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span())
        } else {
            target
        };
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target,
            vec![ability],
            duration,
        )));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in attacks-if-able clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter,
        vec![ability],
        duration,
    )))
}

fn parse_dealt_damage_this_way_subject_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(suffix_start) = tokens.iter().enumerate().find_map(|(index, token)| {
        (token.as_word() == Some("dealt")
            && tokens.get(index + 1).and_then(OwnedLexToken::as_word) == Some("damage")
            && tokens.get(index + 2).and_then(OwnedLexToken::as_word) == Some("this")
            && tokens.get(index + 3).and_then(OwnedLexToken::as_word) == Some("way"))
        .then_some(index)
    }) else {
        return Ok(None);
    };
    if tokens[suffix_start + 4..]
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }
    let target = parse_target_phrase(&tokens[..suffix_start])?;
    let Some(mut filter) = target_ast_to_object_filter(target) else {
        return Ok(None);
    };
    filter = filter.match_tagged(
        crate::tag::CompilerReferenceTag::It.bind(),
        TaggedOpbjectRelation::IsTaggedObject,
    );
    Ok(Some(filter))
}

pub fn parse_must_be_blocked_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some(shape) = clause_shapes::parse_combat_requirement_shape(tokens) else {
        return Ok(None);
    };
    if shape.kind != clause_shapes::CombatRequirementKind::MustBeBlocked {
        return Ok(None);
    }
    let subject_clause = LexedClause::new(shape.subject_tokens).trimmed();
    if subject_clause.is_empty() {
        return Ok(None);
    }
    if starts_with_target_indicator(subject_clause.tokens()) {
        let attacker_target = parse_target_phrase(subject_clause.tokens())?;
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(attacker_target),
                EffectAst::subject_verb_cant(
                    crate::effect::Restriction::must_be_blocked(ObjectFilter::tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                    )),
                    Until::EndOfTurn,
                    None,
                ),
            ],
        }));
    }

    // A demonstrative subject ("that creature") back-references the tagged
    // antecedent rather than restricting a filtered set.
    if subject_clause
        .tokens()
        .first()
        .is_some_and(|token| token.is_word("that") || token.is_word("it"))
    {
        return Ok(Some(EffectAst::subject_verb_cant(
            crate::effect::Restriction::must_be_blocked(ObjectFilter::tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            )),
            Until::EndOfTurn,
            None,
        )));
    }

    let attacker_target = parse_target_phrase(subject_clause.tokens())?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in must-be-blocked clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_cant(
        crate::effect::Restriction::must_be_blocked(attacker_filter),
        Until::EndOfTurn,
        None,
    )))
}

fn tagged_forced_block_target(target: TargetAst, tag: TagKey) -> EffectAst {
    EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_target_only(target)),
        tag: crate::tag::TagRef::of(tag),
    }
}

fn forced_block_effect(
    mut target_declarations: Vec<EffectAst>,
    blockers: ObjectFilter,
    attacker: ObjectFilter,
    duration: crate::effect::Until,
) -> EffectAst {
    let restriction = EffectAst::subject_verb_cant(
        crate::effect::Restriction::must_block_specific_attacker(blockers, attacker),
        duration,
        None,
    );
    if target_declarations.is_empty() {
        restriction
    } else {
        target_declarations.push(restriction);
        EffectAst::Sequence {
            effects: target_declarations,
        }
    }
}

pub fn parse_must_block_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    let Some(shape) = clause_shapes::parse_must_block_shape(tokens) else {
        return Ok(None);
    };
    match shape {
        clause_shapes::MustBlockShape::SubjectThisTurn { subject_tokens } => {
            let subject_clause = LexedClause::new(subject_tokens).trimmed();
            let target = parse_target_phrase(subject_clause.tokens())?;
            let ability = GrantedAbilityAst::MustBlock;
            if starts_with_target_indicator(subject_clause.tokens()) {
                return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
                    target,
                    vec![ability],
                    Until::EndOfTurn,
                )));
            }
            let filter = target_ast_to_object_filter(target).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported blocker subject in blocks-if-able clause (clause: '{}')",
                    clause_text
                ))
            })?;
            Ok(Some(EffectAst::subject_verb_grant_abilities_all(
                filter,
                vec![ability],
                Until::EndOfTurn,
            )))
        }
        clause_shapes::MustBlockShape::AllCreatures {
            attacker_and_duration_tokens,
        } => {
            let (duration, attacker_tokens) = if let Some((duration, remainder)) =
                parse_restriction_duration(attacker_and_duration_tokens)?
            {
                (duration, remainder)
            } else {
                (Until::EndOfTurn, attacker_and_duration_tokens.to_vec())
            };
            let attacker_clause = LexedClause::new(&attacker_tokens).trimmed();
            if attacker_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attacker in must-block clause (clause: '{}')",
                    clause_text
                )));
            }
            let attacker_target = parse_target_phrase(attacker_clause.tokens())?;
            if super::super::grammar::activation_restrictions::parse_target_indicator_tokens(
                attacker_clause.tokens(),
            )
            .is_some()
            {
                let attacker_tag =
                    helper_tag_for_tokens(attacker_clause.tokens(), "targeted_attacker");
                return Ok(Some(forced_block_effect(
                    vec![tagged_forced_block_target(
                        attacker_target,
                        attacker_tag.clone().into(),
                    )],
                    ObjectFilter::creature(),
                    ObjectFilter::tagged(attacker_tag),
                    duration,
                )));
            }
            let attacker_filter =
                target_ast_to_object_filter(attacker_target).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported attacker target in must-block clause (clause: '{}')",
                        clause_text
                    ))
                })?;
            Ok(Some(forced_block_effect(
                Vec::new(),
                ObjectFilter::creature(),
                attacker_filter,
                duration,
            )))
        }
        clause_shapes::MustBlockShape::SubjectAgainstAttacker {
            subject_tokens,
            attacker_and_duration_tokens,
        } => {
            let subject_clause = LexedClause::new(subject_tokens).trimmed();
            let blocker_is_target =
                super::super::grammar::activation_restrictions::parse_target_indicator_tokens(
                    subject_clause.tokens(),
                )
                .is_some();
            let mut target_declarations = Vec::new();
            let blockers_filter = if blocker_is_target {
                let blocker_target = parse_target_phrase(subject_clause.tokens())?;
                let blocker_tag =
                    helper_tag_for_tokens(subject_clause.tokens(), "targeted_blocker");
                target_declarations.push(tagged_forced_block_target(
                    blocker_target,
                    blocker_tag.clone().into(),
                ));
                ObjectFilter::tagged(blocker_tag)
            } else {
                parse_subject_object_filter(subject_clause.tokens())?.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported blocker subject in must-block clause (clause: '{}')",
                        clause_text
                    ))
                })?
            };
            let (duration, attacker_tokens) = if let Some((duration, remainder)) =
                parse_restriction_duration(attacker_and_duration_tokens)?
            {
                (duration, remainder)
            } else {
                (Until::EndOfTurn, attacker_and_duration_tokens.to_vec())
            };
            let attacker_clause = LexedClause::new(&attacker_tokens).trimmed();
            if attacker_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attacker in must-block clause (clause: '{}')",
                    clause_text
                )));
            }
            let attacker_filter = if clause_shapes::is_it_reference_shape(attacker_clause.tokens())
            {
                // Preserve the object denoted by `it` before lowering the
                // blocker target declaration. In a triggered ability this is
                // already the triggering object; in a spell such as Feral
                // Contest it is the target established by the prior sentence.
                // The snapshot is lowering-only and emits no runtime effect.
                target_declarations.insert(
                    0,
                    EffectAst::SnapshotLastObjectTag {
                        into: crate::tag::CompilerReferenceTag::Triggering.bind(),
                    },
                );
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::Triggering.bind())
            } else if super::super::grammar::activation_restrictions::parse_target_indicator_tokens(
                attacker_clause.tokens(),
            )
            .is_some()
            {
                let attacker_target = parse_target_phrase(attacker_clause.tokens())?;
                let attacker_tag =
                    helper_tag_for_tokens(attacker_clause.tokens(), "targeted_attacker");
                target_declarations.push(tagged_forced_block_target(
                    attacker_target,
                    attacker_tag.clone().into(),
                ));
                ObjectFilter::tagged(attacker_tag)
            } else {
                let attacker_target = parse_target_phrase(attacker_clause.tokens())?;
                target_ast_to_object_filter(attacker_target).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported attacker target in must-block clause (clause: '{}')",
                        clause_text
                    ))
                })?
            };
            Ok(Some(forced_block_effect(
                target_declarations,
                blockers_filter,
                attacker_filter,
                duration,
            )))
        }
    }
}

pub fn parse_until_duration_triggered_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    fn combat_source_filter_mut(
        trigger: &mut crate::model::ast::TriggerSpec,
    ) -> Option<&mut ObjectFilter> {
        match trigger {
            crate::model::ast::TriggerSpec::WithIntro { trigger, .. }
            | crate::model::ast::TriggerSpec::ConditionQualified { trigger, .. } => {
                combat_source_filter_mut(trigger)
            }
            crate::model::ast::TriggerSpec::DealsCombatDamage(source)
            | crate::model::ast::TriggerSpec::DealsCombatDamageToPlayer { source, .. }
            | crate::model::ast::TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                source, ..
            }
            | crate::model::ast::TriggerSpec::DealsCombatDamageTo { source, .. } => Some(source),
            _ => None,
        }
    }

    fn watched_set_surface(tokens: &[OwnedLexToken]) -> Option<String> {
        let (start, _, _) = crate::grammar::primitives::find_prefix(tokens, || {
            (
                alt((
                    crate::grammar::primitives::kw("any"),
                    crate::grammar::primitives::kw("either"),
                )),
                crate::grammar::primitives::phrase(&["of", "those"]),
            )
                .void()
        })?;
        let (relative_end, _, _) =
            crate::grammar::primitives::find_prefix(&tokens[start..], || {
                crate::grammar::primitives::kw("deals").void()
            })?;
        let end = start + relative_end;
        (end > start)
            .then(|| render_token_slice(&tokens[start..end]).trim().to_string())
            .filter(|surface| !surface.is_empty())
    }

    let clause = LexedClause::new(tokens);
    if clause_shapes::parse_duration_trigger_prefix_shape(tokens).is_none() {
        return Ok(None);
    }

    let Some((duration, trigger_tokens)) = parse_restriction_duration(tokens)? else {
        return Ok(None);
    };
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing trigger after duration clause (clause: '{}')",
            clause.text()
        )));
    }

    let trigger_clause = LexedClause::new(&trigger_tokens);
    let trigger_words = trigger_clause.word_refs();
    if clause_shapes::parse_trigger_clause_intro_shape(trigger_clause.tokens()).is_none() {
        return Ok(None);
    }

    let (mut trigger, effects, max_triggers_per_turn) =
        match parse_triggered_line_lexed(&trigger_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => (trigger, effects, max_triggers_per_turn),
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported duration-triggered clause (clause: '{}')",
                    clause.text()
                )));
            }
        };

    if max_triggers_per_turn.is_some() {
        return Err(CardTextError::ParseError(format!(
            "duration-scoped delayed triggers with a per-turn frequency limit are not supported (clause: '{}')",
            clause.text()
        )));
    }

    if let Some(surface) = watched_set_surface(&trigger_tokens)
        && let Some(source) = combat_source_filter_mut(&mut trigger)
    {
        if !source.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }) {
            source
                .tagged_constraints
                .push(crate::target::TaggedObjectConstraint {
                    tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
        }
        source.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            surface,
        ));
    }

    let either_of_watched_objects =
        crate::word_primitives::sequence_occurs(&trigger_words, &["either", "of", "those"]);

    Ok(Some(EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
        trigger,
        effects,
        one_shot: false,
        duration,
        either_of_watched_objects,
        while_any_tagged_object_in_zone: None,
    })))
}

pub fn is_damage_source_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _)
    )
}

pub fn parse_anaphoric_object_deals_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let Some(deal_idx) =
        crate::slice_primitives::select_position(&words, |word| matches!(*word, "deal" | "deals"))
    else {
        return Ok(None);
    };
    let source_words = &words[..deal_idx];
    let source_surface = source_reference_surface_for_words(source_words).or_else(|| {
        crate::word_primitives::parse_any_sequence_complete(
            source_words,
            &[&["he"], &["she"], &["they"]],
        )
        .then(|| {
            crate::target::SourceReferenceSurface::ThisPermanentType(source_words[0].to_string())
        })
    });
    if source_surface.is_none()
        && !crate::word_primitives::parse_any_sequence_complete(
            source_words,
            &[
                &["it"],
                &["that", "token"],
                &["that", "creature"],
                &["that", "land"],
                &["that", "permanent"],
                &["that", "card"],
            ],
        )
    {
        return Ok(None);
    }
    // Leave conjoined damage clauses to the generic effect-chain parser. It
    // expands the elided second verb ("and that much damage ...") into a
    // sibling damage effect, which lets reference resolution bind that amount
    // to the first damage effect instead of collapsing both clauses here.
    if crate::word_primitives::sequence_occurs(&words, &["and", "that", "much", "damage"]) {
        return Ok(None);
    }
    let source_range = word_view
        .token_span_for_words(0, deal_idx)
        .ok_or_else(|| CardTextError::ParseError("missing damage source".to_string()))?;
    let body_range = word_view
        .token_span_for_words(deal_idx + 1, word_view.len())
        .ok_or_else(|| CardTextError::ParseError("missing damage amount".to_string()))?;
    let source_tokens = &tokens[source_range];
    let body_tokens = &tokens[body_range.clone()];
    let body_words = TokenWordView::new(body_tokens).to_word_refs();
    // In a follow-up damage clause, "it deals an additional ..." continues
    // the preceding spell or ability's damage event. Binding that "it" to
    // last-object memory instead can incorrectly turn the previous damage
    // target into the source of the delayed damage.
    let source_span = span_from_tokens(source_tokens);
    let source = if let Some(surface) = source_surface {
        TargetAst::Object(
            ObjectFilter::source_with_surface(surface),
            None,
            source_span,
        )
    } else if crate::word_primitives::parse_sequence_complete(source_words, &["it"])
        && crate::word_primitives::parse_sequence_prefix(&body_words, &["an", "additional"])
    {
        TargetAst::Source(span_from_tokens(source_tokens))
    } else {
        let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
        if crate::word_primitives::parse_sequence_complete(source_words, &["that", "land"]) {
            // Identity remains the typed trigger-object constraint while the
            // authored demonstrative is explicit rendering provenance.
            filter.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "that land".to_string(),
            ));
        }
        TargetAst::Object(filter, None, source_span)
    };
    let distributed_source =
        if crate::word_primitives::parse_sequence_complete(source_words, &["that", "creature"]) {
            let mut filter = ObjectFilter::creature();
            filter.zone = Some(Zone::Battlefield);
            filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                    relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                });
            TargetAst::Object(filter, None, span_from_tokens(source_tokens))
        } else {
            source.clone()
        };
    let parsed = super::verb_handlers::parse_deal_damage(body_tokens)?;
    let EffectAst::SubjectVerb(effect) = parsed else {
        return Ok(None);
    };
    match effect.action {
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
            amount,
            target,
            unpreventable: false,
        }) => Ok(Some(EffectAst::subject_verb_damage_with_source(
            source, amount, target,
        ))),
        SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
            amount,
            target,
            chooser,
            distribution,
            ..
        }) => Ok(Some(
            EffectAst::subject_verb_distributed_damage_with_source_and_mode(
                amount,
                target,
                distributed_source,
                chooser,
                distribution,
            ),
        )),
        _ => Ok(None),
    }
}

pub fn parse_deal_damage_equal_to_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_power_damage_shape(tokens)? else {
        return Ok(None);
    };
    let source = if shape.source_is_tagged {
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(shape.source_tokens),
        )
    } else {
        parse_target_phrase(shape.source_tokens)?
    };
    if !is_damage_source_target(&source) {
        return Err(CardTextError::ParseError(format!(
            "unsupported damage source target phrase (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }
    let source_words = TokenWordView::new(shape.source_tokens).to_word_refs();
    let iterated_source_filter = if source_words.first() == Some(&"each") {
        if shape.source_is_tagged {
            let filter_tokens = if crate::word_primitives::parse_sequence_prefix(
                &source_words,
                &["each", "of", "those"],
            ) && shape.source_tokens.len() > 3
            {
                &shape.source_tokens[3..]
            } else {
                let tapped_idx =
                    crate::slice_primitives::select_position(shape.source_tokens, |token| {
                        token.as_word() == Some("tapped")
                    })
                    .ok_or_else(|| {
                        CardTextError::ParseError("missing tagged-set source qualifier".to_string())
                    })?;
                &shape.source_tokens[1..tapped_idx]
            };
            let mut filter = parse_object_filter(filter_tokens, false)?;
            if crate::word_primitives::parse_sequence_prefix(
                &source_words,
                &["each", "of", "those"],
            ) {
                filter
                    .set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
            }
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                    relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                });
            Some(filter)
        } else if let TargetAst::Object(filter, _, _) = &source {
            Some(filter.clone())
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported iterated damage source phrase (clause: '{}')",
                LexedClause::new(tokens).text()
            )));
        }
    } else {
        None
    };
    let amount = if iterated_source_filter.is_some() {
        bind_iterated_source_possessive_characteristic(shape.amount)
    } else {
        shape.amount
    };
    let effect = match shape.target {
        clause_shapes::PowerDamageTargetShape::EachPlayer => Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_damage_with_source(
                source,
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }))),
        clause_shapes::PowerDamageTargetShape::EachOtherPlayer => {
            Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::NotYou,
                effects: vec![EffectAst::subject_verb_damage_with_source(
                    source,
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            })))
        }
        clause_shapes::PowerDamageTargetShape::EachOpponent => {
            Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
                effects: vec![EffectAst::subject_verb_damage_with_source(
                    source,
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            })))
        }
        clause_shapes::PowerDamageTargetShape::Source => Ok(Some(
            EffectAst::subject_verb_damage_with_source(source.clone(), amount, source),
        )),
        clause_shapes::PowerDamageTargetShape::Tokens(target_tokens) => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(Some(EffectAst::subject_verb_damage_with_source(
                source, amount, target,
            )))
        }
    }?;
    if let Some(filter) = iterated_source_filter {
        Ok(effect.map(|effect| EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        })))
    } else {
        Ok(effect)
    }
}

fn bind_iterated_source_possessive_characteristic(value: Value) -> Value {
    match value {
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_iterated_source_possessive_characteristic(*value)),
            hints,
        },
        Value::PowerOf(spec)
            if matches!(
                spec.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ) && matches!(
                spec.source_reference_surface(),
                Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface))
                    if matches!(surface.as_str(), "it" | "its")
            ) =>
        {
            let hints = spec.surface_hints().to_vec();
            Value::PowerOf(Box::new(ChooseSpec::Source.with_surface_hints(hints)))
        }
        Value::ToughnessOf(spec)
            if matches!(
                spec.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ) && matches!(
                spec.source_reference_surface(),
                Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface))
                    if matches!(surface.as_str(), "it" | "its")
            ) =>
        {
            let hints = spec.surface_hints().to_vec();
            Value::ToughnessOf(Box::new(ChooseSpec::Source.with_surface_hints(hints)))
        }
        value => value,
    }
}

pub fn parse_fight_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_fight_shape(tokens) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    if shape.right_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_text
        )));
    }

    if shape.right_is_tagged_other
        && shape.left_tokens.is_some_and(|left| {
            crate::word_primitives::parse_any_sequence_complete(
                &crate::lexer::parser_token_word_refs(left),
                &[&["those", "creatures"], &["the", "chosen", "creatures"]],
            )
        })
    {
        let tag = crate::tag::CompilerReferenceTag::ChosenObjects.bind();
        return Ok(Some(
            EffectAst::subject_verb_fight(
                TargetAst::Tagged(
                    tag.clone(),
                    span_from_tokens(shape.left_tokens.unwrap_or_default()),
                ),
                TargetAst::Tagged(tag, span_from_tokens(shape.right_tokens)),
            )
            .with_mutual_fight_surface(),
        ));
    }

    let creature1 = if let Some(left_tokens) = shape.left_tokens {
        if let Some(filter) = parse_for_each_object_subject(left_tokens)? {
            let creature2 = parse_target_phrase(shape.right_tokens)?;
            if matches!(
                creature2,
                TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
            ) {
                return Err(CardTextError::ParseError(format!(
                    "fight target must be a creature (clause: '{}')",
                    clause_text
                )));
            }
            return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::subject_verb_fight_iterated(creature2)],
            })));
        }
        let left_words = crate::lexer::parser_token_word_refs(left_tokens);
        if crate::word_primitives::parse_sequence_complete(&left_words, &["it"])
            || crate::word_primitives::parse_sequence_suffix(
                &left_words,
                &["you", "may", "have", "it"],
            )
        {
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(left_tokens),
            )
        } else {
            parse_target_phrase(left_tokens)?
        }
    } else {
        TargetAst::Source(None)
    };
    let creature2 = if shape.right_is_tagged_other {
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(shape.right_tokens),
        )
    } else {
        parse_target_phrase(shape.right_tokens)?
    };

    for target in [&creature1, &creature2] {
        if matches!(
            target,
            TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
        ) {
            return Err(CardTextError::ParseError(format!(
                "fight target must be a creature (clause: '{}')",
                clause_text
            )));
        }
    }

    Ok(Some(EffectAst::subject_verb_fight(creature1, creature2)))
}

pub fn parse_clash_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    Ok(clause_shapes::parse_clash_shape(tokens).map(EffectAst::subject_verb_clash))
}

#[cfg(test)]
mod result_subject_tests {
    use crate::cards::builders::PermissionEffectAst;
    use crate::cards::builders::StackActionAst;
    use crate::cards::builders::LibraryActionAst;
    use super::*;
    use crate::model::ast::SubjectVerbEffectAst;
    use crate::types::CardType;

    #[test]
    fn registry_routes_any_number_target_players_each_to_the_typed_fanout() {
        let tokens = crate::lexer::lex_line(
            "Any number of target players each mill half their library, rounded down",
            0,
        )
        .expect("lex target-player fanout");
        let effect = run_clause_primitives(&tokens)
            .expect("parse target-player fanout")
            .expect("registry should claim target-player fanout");

        assert!(matches!(
            effect,
            EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers {
                count,
                filter: PlayerFilter::Any,
                effects,
            }) if count.is_any_number()
                && matches!(
                    effects.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: crate::model::ast::SubjectVerbSubjectAst {
                            player: PlayerAst::That,
                            ..
                        },
                        action: SubjectVerbActionAst::Library(LibraryActionAst::Mill { .. }),
                    })]
                )
        ));
    }

    #[test]
    fn copy_retarget_keeps_authored_reference_number() {
        for (text, expected_plural) in [
            ("You may choose new targets for the copy.", false),
            ("You may choose new targets for the copies.", true),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).expect("lex copy retarget");
            let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
                .expect("parse optional copy retarget");
            assert!(matches!(
                effects.as_slice(),
                [EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                    player: PlayerAst::You,
                    effects,
                })] if matches!(
                    effects.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject {
                            copy_reference_plural,
                            ..
                        }),
                        ..
                    })] if *copy_reference_plural == expected_plural
                )
            ));
        }
    }

    #[test]
    fn dealt_damage_this_way_attack_subject_keeps_result_tag() {
        let tokens = crate::lexer::lex_line(
            "Each creature dealt damage this way attacks this turn if able.",
            0,
        )
        .expect("lex attack followup");
        let effect = parse_attack_this_turn_if_able_clause(&tokens)
            .expect("parse attack followup")
            .expect("match attack followup");
        let debug = format!("{effect:#?}");
        assert!(debug.contains("IsTaggedObject"), "{debug}");
        assert!(
            debug.contains(crate::tag::CompilerReferenceTag::It.as_str()),
            "{debug}"
        );
    }

    #[test]
    fn named_source_damage_to_each_other_player_excludes_only_controller() {
        let tokens = crate::lexer::lex_line("This spell deals 2 damage to each other player.", 0)
            .expect("lex each-other-player damage");
        let effects = super::super::parse_effect_sentence_lexed(&tokens)
            .expect("parse damage sentence through the ordinary dispatcher");

        assert!(matches!(
            effects.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::NotYou,
                ..
            })]
        ));
    }

    #[test]
    fn named_source_excess_damage_keeps_source_and_damaged_target_identity() {
        let tokens = crate::lexer::lex_line(
            "Excess Herald deals damage equal to the excess to any target other than that permanent.",
            0,
        )
        .expect("lex named-source excess damage");
        let context = crate::parse_context::ParseContext::for_fragment(
            "Excess Herald",
            vec![CardType::Creature],
            vec![],
            "Excess Herald deals damage equal to the excess to any target other than that permanent.",
        );
        let effects =
            super::super::parse_effect_sentence_lexed_with_context(context.view(), &tokens)
                .expect("parse named-source excess damage");

        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                        source: TargetAst::Object(source_filter, None, _),
                        amount,
                        target: TargetAst::ObjectOrPlayer(filter, PlayerFilter::Any, Some(_)),
                        ..
                    }),
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected explicit-source damage with a mixed target domain: {effects:#?}");
        };
        assert!(source_filter.source);
        assert_eq!(
            source_filter.source_surface,
            Some(crate::target::SourceReferenceSurface::FullName(
                "Excess Herald".to_string()
            ))
        );
        assert!(matches!(
            amount.unhinted(),
            Value::EventValue(crate::effect::EventValueSpec::Amount)
        ));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "damaged"
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn each_object_power_damage_preserves_the_source_set_as_an_iteration() {
        let tokens = crate::lexer::lex_line(
            "Each creature with power 4 or greater you control deals damage equal to its power to that permanent.",
            0,
        )
        .expect("lex each-object damage");
        let effect = parse_deal_damage_equal_to_power_clause(&tokens)
            .expect("parse each-object damage")
            .expect("match each-object damage");

        let EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, effects }) = effect else {
            panic!("expected an object-source iteration");
        };
        assert_eq!(
            filter.power,
            Some(crate::filter::Comparison::GreaterThanOrEqual(4))
        );
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source: TargetAst::Object(_, _, _),
                    amount: Value::PowerOf(spec),
                    target: TargetAst::Object(target_filter, _, _),
                    ..
                }),
                ..
            })] if matches!(spec.base(), ChooseSpec::Source)
                && target_filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
        ));
    }

    #[test]
    fn additional_damage_pronoun_keeps_the_spell_or_ability_as_source() {
        let tokens = crate::lexer::lex_line("It deals an additional 3 damage to that player.", 0)
            .expect("lex additional damage");
        let effect = parse_anaphoric_object_deals_damage_clause(&tokens)
            .expect("parse additional damage")
            .expect("match additional damage");

        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source: TargetAst::Source(_),
                    ..
                }),
                ..
            })
        ));
    }

    #[test]
    fn personal_pronoun_damage_keeps_the_authored_source_surface() {
        let tokens = crate::lexer::lex_line(
            "She deals that much damage to target opponent or planeswalker.",
            0,
        )
        .expect("lex personal-pronoun damage");
        let effect = parse_anaphoric_object_deals_damage_clause(&tokens)
            .expect("parse personal-pronoun damage")
            .expect("match personal-pronoun damage");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { source, .. }),
            ..
        }) = effect
        else {
            panic!("expected typed explicit-source damage: {effect:#?}");
        };
        assert!(matches!(
            source,
            TargetAst::Object(filter, None, _)
                if filter.source
                    && filter.source_surface
                        == Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                            "she".to_string()
                        ))
        ));
    }

    #[test]
    fn demonstrative_land_damage_keeps_the_trigger_object_and_authored_noun() {
        let tokens = crate::lexer::lex_line("That land deals 1 damage to that player.", 0)
            .expect("lex demonstrative-land damage");
        let effect = parse_anaphoric_object_deals_damage_clause(&tokens)
            .expect("parse demonstrative-land damage")
            .expect("match demonstrative-land damage");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source: TargetAst::Object(source_filter, None, _),
                    amount: Value::Fixed(1),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, _),
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected typed triggering-land damage: {effect:#?}");
        };
        assert_eq!(
            source_filter.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "that land".to_string()
            ))
        );
        assert!(source_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn combat_scoped_attack_or_block_requirement_does_not_become_a_prohibition() {
        let tokens = crate::lexer::lex_line(
            "Up to one target creature attacks or blocks this combat if able and up to one target creature can't attack or block this combat.",
            0,
        )
        .expect("lex asymmetric combat clauses");
        let effects = super::super::parse_effect_sentence_lexed(&tokens)
            .expect("parse asymmetric combat clauses");
        let debug = format!("{effects:#?}");

        assert_eq!(
            debug.matches("GrantAbilitiesToTarget").count(),
            1,
            "{debug}"
        );
        assert_eq!(debug.matches("Cant {").count(), 1, "{debug}");
        assert!(debug.contains("MustAttack"), "{debug}");
        assert!(debug.contains("MustBlock"), "{debug}");
        assert_eq!(debug.matches("duration: EndOfCombat").count(), 2, "{debug}");
        assert_eq!(debug.matches("min: 0").count(), 2, "{debug}");
        assert_eq!(debug.matches("max: Some(").count(), 2, "{debug}");
    }
}
