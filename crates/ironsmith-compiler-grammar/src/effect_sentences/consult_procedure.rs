//! Consult-traversal procedures composed statement by statement.
//!
//! "Reveal cards from the top of your library until you reveal a creature
//! card. Put that card into your hand and the rest on the bottom of your
//! library in a random order." is a traversal statement that binds two groups
//! — every card traversed and the card that stopped the traversal — followed by
//! statements over those groups. The traversal sentence is recognized by the
//! consult family's own sentence grammar; this module carries its two groups
//! to the sentences that follow, the way [`super::looked_procedure`] carries a
//! viewed group. The registry programs these statements replace wrapped an
//! optional traversal ("you may reveal ...") and an "if you do," gate around
//! the whole procedure; the close does the same.

use super::dispatch_entry::{
    ConsultCastCost, ConsultSentenceParts, SentenceInput, consult_cast_effects,
    consult_stop_rule_is_single_match, parse_consult_bottom_remainder_clause,
    parse_consult_cast_clause, parse_consult_traversal_sentence,
};
use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::{
    parse_gated_optional_consult_traversal_sentence, parse_optional_consult_traversal_sentence,
    strip_leading_if_you_do_sentence, wrap_optional_consult_effects,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, LibraryConsultModeAst, ObjectFilter, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TargetAst,
};
use crate::grammar::effects::{
    self as effect_grammar, ConsultBattlefieldGraveyardShape, ConsultMoveBottomShape,
    ConsultMoveSelectionShape,
};
use crate::tag::TagKey;
use crate::zone::Zone;

/// The three readings of a traversal sentence the registry programs used.
/// The plain reading takes the sentence as written, prefix and all; the
/// optional reading strips "you may" and remembers it; the gated reading also
/// strips "if you do," and remembers that. Each program chose one, and a
/// statement here takes the reading its program took.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reading {
    Plain,
    Optional,
    Gated,
}

struct Readings {
    plain: Option<ConsultSentenceParts>,
    optional: Option<(ConsultSentenceParts, bool)>,
    gated: Option<(ConsultSentenceParts, bool, bool)>,
}

impl Readings {
    fn take(&mut self, reading: Reading) -> Option<(ConsultSentenceParts, bool, bool)> {
        match reading {
            Reading::Plain => self.plain.take().map(|parts| (parts, false, false)),
            Reading::Optional => self
                .optional
                .take()
                .map(|(parts, optional)| (parts, optional, false)),
            Reading::Gated => self.gated.take(),
        }
    }

    fn peek(&self, reading: Reading) -> Option<(&ConsultSentenceParts, bool, bool)> {
        match reading {
            Reading::Plain => self.plain.as_ref().map(|parts| (parts, false, false)),
            Reading::Optional => self
                .optional
                .as_ref()
                .map(|(parts, optional)| (parts, *optional, false)),
            Reading::Gated => self
                .gated
                .as_ref()
                .map(|(parts, optional, gated)| (parts, *optional, *gated)),
        }
    }
}

/// The two groups a traversal bound, and the statements made over them so far.
pub(super) struct ConsultedGroup {
    /// The readings still available before a statement chooses one.
    readings: Readings,
    /// The reading the first statement chose.
    parts: Option<ConsultSentenceParts>,
    /// The sentence after the next one puts the rest on the bottom of the
    /// library, which admits a cast without paying its mana cost from any
    /// traversal.
    following_bottom_remainder: bool,
    mode: LibraryConsultModeAst,
    single_match: bool,
    /// "You may reveal cards ..." — the traversal is optional and what follows
    /// happens only if it was done.
    optional: bool,
    /// The traversal opened under "if you do,".
    gated: bool,
    /// The traversal sentence opened under "if <predicate>," and the whole
    /// procedure is conditional on it.
    condition: Option<PredicateAst>,
    /// A follow-up opened under "if you do,".
    gate_on_result: bool,
    /// The follow-ups are spelled as two source sentences rather than one
    /// wrapped program (the hand-and-graveyard program did this when nothing
    /// was optional or gated).
    split_source_sentences: bool,
    /// A cast statement without paying the mana cost was made; a bottom
    /// remainder may follow it.
    cast_without_paying: bool,
    /// A cast statement was made whose declined branch may follow: "If you
    /// don't cast that card this way, put it into your hand."
    cast_declinable: bool,
    /// The cleanup statement, held so a reflexive trigger that follows it can
    /// be spelled first.
    pending_cleanup: Option<EffectAst>,
    /// A pump of the triggering creature was made; the revealed cards' move
    /// to the graveyard may follow.
    pumped: bool,
    followups: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn it() -> TargetAst {
    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None)
}

fn traversal(parts: &ConsultSentenceParts) -> Option<(LibraryConsultModeAst, bool)> {
    match parts.effects.last() {
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    mode, stop_rule, ..
                },
            ..
        })) => Some((*mode, consult_stop_rule_is_single_match(stop_rule))),
        _ => None,
    }
}

/// Which statement the next sentence is, under which reading of the traversal.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Statement {
    /// "Put all cards revealed this way into your hand." (optional reading)
    MatchedMove,
    /// "Put that card into your hand and the rest on the bottom ..." (optional reading)
    MoveBottom,
    /// "You may cast that card without paying its mana cost." (plain reading,
    /// an exile traversal stopping at one match)
    Cast,
    /// The same cast, without paying its mana cost, when "put the rest on the
    /// bottom of your library" follows it (plain reading, any traversal).
    CastThenBottom,
    /// "Put that card into your hand and exile the rest." (plain reading, reveal)
    HandExileOthers,
    /// "Put all cards revealed this way into your graveyard." (plain reading)
    AllToGraveyard,
    /// "Put that card into your hand and the rest into your graveyard." (gated reading, reveal)
    HandOthersGraveyard,
    /// "Put that card onto the battlefield and the rest into your graveyard." (gated reading, reveal)
    BattlefieldGraveyard,
    /// "Put two time counters on that card." (plain reading, an exile
    /// traversal stopping at one match)
    PutCountersOnMatch,
    /// "You may put it onto the battlefield. If you don't, put it into its
    /// owner's hand." (plain reading, an exile traversal)
    BattlefieldOrHand,
    /// "Put the revealed cards on the bottom of your library in a random
    /// order." read on its own: the cleanup, which a reflexive trigger that
    /// follows it precedes in the spelling (plain reading)
    Cleanup,
    /// "The creature gets +1/+0 until end of turn for each card revealed this
    /// way." (plain reading, a reveal traversal)
    RevealPump,
}

impl Statement {
    const fn reading(self) -> Reading {
        match self {
            Self::MatchedMove | Self::MoveBottom => Reading::Optional,
            Self::Cast
            | Self::CastThenBottom
            | Self::HandExileOthers
            | Self::AllToGraveyard
            | Self::PutCountersOnMatch
            | Self::BattlefieldOrHand
            | Self::Cleanup
            | Self::RevealPump => Reading::Plain,
            Self::HandOthersGraveyard | Self::BattlefieldGraveyard => Reading::Gated,
        }
    }
}

/// The first statement the sentence after a traversal makes, if any, given
/// the readings available.
fn first_statement(
    readings: &Readings,
    next: &SentenceInput,
    following_bottom_remainder: bool,
) -> Option<Statement> {
    let tokens = crate::lexer::trim_lexed_commas(next.lowered());
    let (stripped, _) = strip_leading_if_you_do_sentence(next.lowered());
    let mode_of = |reading: Reading| {
        readings
            .peek(reading)
            .and_then(|(parts, _, _)| traversal(parts))
    };
    if let Some((mode, single_match)) = mode_of(Reading::Optional) {
        let _ = (mode, single_match);
        if effect_grammar::parse_consult_matched_move_shape(tokens)
            .is_some_and(|shape| shape.selection == ConsultMoveSelectionShape::AllMatched)
        {
            return Some(Statement::MatchedMove);
        }
        if effect_grammar::parse_consult_move_bottom_shape(tokens).is_some() {
            return Some(Statement::MoveBottom);
        }
    }
    if let Some((mode, single_match)) = mode_of(Reading::Plain) {
        if let Some(clause) = parse_consult_cast_clause(next.lowered()) {
            if mode == LibraryConsultModeAst::Exile && single_match {
                return Some(Statement::Cast);
            }
            if following_bottom_remainder
                && matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost)
            {
                return Some(Statement::CastThenBottom);
            }
        }
        if mode == LibraryConsultModeAst::Reveal
            && effect_grammar::is_consult_hand_then_exile_others_shape(&stripped)
        {
            return Some(Statement::HandExileOthers);
        }
        if mode == LibraryConsultModeAst::Exile
            && single_match
            && counters_on_match(next).is_some()
        {
            return Some(Statement::PutCountersOnMatch);
        }
        if mode == LibraryConsultModeAst::Exile
            && effect_grammar::is_consult_battlefield_or_hand_shape(tokens)
        {
            return Some(Statement::BattlefieldOrHand);
        }
        if mode == LibraryConsultModeAst::Reveal && reveal_pump(next).is_some() {
            return Some(Statement::RevealPump);
        }
        if cleanup(next).is_some() {
            return Some(Statement::Cleanup);
        }
    }
    if let Some((LibraryConsultModeAst::Reveal, _)) = mode_of(Reading::Gated) {
        if effect_grammar::is_consult_hand_others_graveyard_shape(&stripped) {
            return Some(Statement::HandOthersGraveyard);
        }
        if effect_grammar::parse_consult_battlefield_graveyard_shape(&stripped).is_some() {
            return Some(Statement::BattlefieldGraveyard);
        }
    }
    // Everything revealed to the graveyard comes last: the program that read
    // it yielded to the battlefield-and-graveyard program.
    if mode_of(Reading::Plain).is_some()
        && effect_grammar::is_consult_move_all_to_graveyard_shape(next.lowered())
    {
        return Some(Statement::AllToGraveyard);
    }
    None
}

/// "Put two time counters on that card": one counter statement whose target
/// is the match.
fn counters_on_match(next: &SentenceInput) -> Option<SubjectVerbEffectAst> {
    let effects =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(next.lowered()))?;
    let [EffectAst::SubjectVerb(effect)] = effects.as_slice() else {
        return None;
    };
    let SubjectVerbActionAst::PutCounters { target, .. } = &effect.action else {
        return None;
    };
    super::dispatch_entry::target_references_it(target).then(|| effect.clone())
}

/// "Put the revealed cards on the bottom of your library in a random order."
/// parsed on its own.
fn cleanup(next: &SentenceInput) -> Option<EffectAst> {
    let effects =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(next.lowered()))?;
    let [effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
        ..
    })] = effects.as_slice()
    else {
        return None;
    };
    Some(effect.clone())
}

/// "The creature gets +1/+0 until end of turn for each card revealed this
/// way.": a pump of the triggering creature by the traversal's count. The
/// authored numeric and type words are kept (a source alias can match them).
fn reveal_pump(next: &SentenceInput) -> Option<EffectAst> {
    let tokens = crate::lexer::trim_lexed_commas(next.lexed());
    let mut effects =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(tokens))?;
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::PumpForEach { target, count, .. },
        ..
    })] = effects.as_mut_slice()
    else {
        return None;
    };
    if !count.has_surface_hint(ironsmith_core::ValueSurfaceHint::CardsRevealedThisWay) {
        return None;
    }
    let definite_creature_subject = tokens
        .iter()
        .filter_map(crate::lexer::OwnedLexToken::as_word)
        .take(2)
        .eq(["the", "creature"]);
    if !definite_creature_subject {
        return None;
    }
    *target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None);
    effects.pop()
}

/// "That player puts the revealed cards into their graveyard.": every
/// traversed card to the graveyard, after a pump.
fn revealed_to_graveyard(sentence: &SentenceInput, all_tag: &TagKey) -> Option<EffectAst> {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let mut effects =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(tokens))?;
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::MoveToZone {
                target: cleanup_target,
                zone: Zone::Graveyard,
                target_plural_surface,
                ..
            },
        ..
    })] = effects.as_mut_slice()
    else {
        return None;
    };
    if !tokens.iter().any(|token| token.is_word("revealed"))
        || !tokens
            .iter()
            .any(|token| matches!(token.as_word(), Some("card" | "cards")))
    {
        return None;
    }
    *cleanup_target = TargetAst::Tagged(all_tag.clone(), None);
    *target_plural_surface = true;
    effects.pop()
}

/// "When you reveal a nonland card this way, ..." after the cleanup: the
/// reflexive trigger is spelled before the cleanup, as its program had it.
fn when_result(sentence: &SentenceInput) -> Option<EffectAst> {
    let effects =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(sentence.lowered()))?;
    let [effect @ EffectAst::WhenResult { .. }] = effects.as_slice() else {
        return None;
    };
    Some(effect.clone())
}

fn readings_of(tokens: &[crate::lexer::OwnedLexToken]) -> Readings {
    // A reading that errors is a reading the sentence does not have; the
    // registry treated a program's error the same way, as a diagnostic that
    // did not stop the other programs.
    Readings {
        plain: parse_consult_traversal_sentence(tokens).unwrap_or_default(),
        optional: parse_optional_consult_traversal_sentence(tokens).unwrap_or_default(),
        gated: parse_gated_optional_consult_traversal_sentence(tokens).unwrap_or_default(),
    }
}

impl Readings {
    fn is_empty(&self) -> bool {
        self.plain.is_none() && self.optional.is_none() && self.gated.is_none()
    }

    /// Whether the sentence after the next one puts the rest on the bottom of
    /// the library, which admits a cast without paying its mana cost from any
    /// traversal.
    fn following_bottom_remainder(&self, following: Option<&SentenceInput>) -> bool {
        following.is_some_and(|following| {
            [Reading::Plain, Reading::Optional, Reading::Gated]
                .into_iter()
                .filter_map(|reading| self.peek(reading))
                .filter_map(|(parts, _, _)| traversal(parts))
                .any(|(mode, _)| {
                    parse_consult_bottom_remainder_clause(following.lowered(), mode).is_some()
                })
        })
    }
}

/// Open a procedure at a traversal sentence when the next sentence makes a
/// statement over it. The traversal may sit under "if <predicate>," or "if you
/// do,": the condition then scopes the whole procedure.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<ConsultedGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let following = sentences.get(sentence_idx + 2);
    let mut readings = readings_of(sentence.lowered());
    let mut following_bottom_remainder = readings.following_bottom_remainder(following);
    let mut condition = None;
    let mut gated_by_condition = false;
    if readings.is_empty()
        || first_statement(&readings, next, following_bottom_remainder).is_none()
    {
        let trimmed = crate::lexer::trim_lexed_commas(sentence.lowered());
        let Some(shape) = effect_grammar::parse_conditional_consult_shape(trimmed) else {
            if !readings.is_empty() {
                crate::parse_trace::event(format!(
                    "consult procedure: no statement follows the traversal (readings: plain={} optional={} gated={})",
                    readings.plain.is_some(),
                    readings.optional.is_some(),
                    readings.gated.is_some()
                ));
            }
            return Ok(None);
        };
        let effect_tokens = crate::lexer::trim_lexed_commas(&trimmed[shape.effect]);
        let conditional_readings = readings_of(effect_tokens);
        let conditional_following_bottom_remainder =
            conditional_readings.following_bottom_remainder(following);
        if conditional_readings.is_empty()
            || first_statement(
                &conditional_readings,
                next,
                conditional_following_bottom_remainder,
            )
            .is_none()
        {
            return Ok(None);
        }
        if shape.if_result {
            gated_by_condition = true;
        } else {
            let predicate_tokens = crate::lexer::trim_lexed_commas(&trimmed[shape.predicate]);
            let Ok(predicate) =
                crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
                    predicate_tokens,
                )
            else {
                return Ok(None);
            };
            condition = Some(predicate);
        }
        readings = conditional_readings;
        following_bottom_remainder = conditional_following_bottom_remainder;
    }
    Ok(Some(ConsultedGroup {
        readings,
        parts: None,
        following_bottom_remainder,
        mode: LibraryConsultModeAst::Reveal,
        single_match: false,
        optional: false,
        gated: gated_by_condition,
        condition,
        gate_on_result: false,
        split_source_sentences: false,
        cast_without_paying: false,
        cast_declinable: false,
        pending_cleanup: None,
        pumped: false,
        followups: Vec::new(),
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// Continue an open procedure with the next sentence. Returns false, leaving
/// the group untouched, when the sentence is not one of its statements.
pub(super) fn continue_with(
    group: &mut ConsultedGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.cast_declinable
        && let Some(parts) = group.parts.as_ref()
        && let Some(hand_effects) = super::dispatch_entry::parse_if_declined_put_match_into_hand(
            sentence.lowered(),
            parts.match_tag.clone(),
        )
    {
        // "If you don't cast that card this way, put it into your hand."
        group.cast_declinable = false;
        group.cast_without_paying = false;
        match group.followups.pop() {
            Some(EffectAst::Conditional {
                predicate,
                mut if_true,
                mut if_false,
            }) if group.followups.is_empty() => {
                if_true.push(EffectAst::IfResult {
                    predicate: IfResultPredicate::WasDeclined,
                    effects: hand_effects.clone(),
                });
                if_false.extend(hand_effects);
                group.followups.push(EffectAst::Conditional {
                    predicate,
                    if_true,
                    if_false,
                });
            }
            Some(last) => {
                group.followups.push(last);
                group.followups.push(EffectAst::IfResult {
                    predicate: IfResultPredicate::WasDeclined,
                    effects: hand_effects,
                });
            }
            None => {
                group.followups.push(EffectAst::IfResult {
                    predicate: IfResultPredicate::WasDeclined,
                    effects: hand_effects,
                });
            }
        }
        group.consumed += 1;
        return Ok(true);
    }
    if group.pending_cleanup.is_some() {
        if let Some(trigger) = when_result(sentence) {
            group.followups.push(trigger);
            group.consumed += 1;
            return Ok(true);
        }
        return Ok(false);
    }
    if group.pumped
        && let Some(parts) = group.parts.as_ref()
        && let Some(cleanup) = revealed_to_graveyard(sentence, &parts.all_tag)
    {
        group.pumped = false;
        group.followups.push(cleanup);
        group.consumed += 1;
        return Ok(true);
    }
    if group.cast_without_paying && !group.followups.is_empty() {
        // "Put the rest on the bottom of your library in a random order."
        if let Some(order) = parse_consult_bottom_remainder_clause(sentence.lowered(), group.mode) {
            let parts = group
                .parts
                .as_ref()
                .expect("a cast statement chose a reading");
            group.followups.push(
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    parts.all_tag.clone(),
                    None,
                    order,
                    parts.player,
                ),
            );
            group.cast_without_paying = false;
            group.consumed += 1;
            return Ok(true);
        }
        return Ok(false);
    }
    if !group.followups.is_empty() {
        return Ok(false);
    }
    let Some(statement) =
        first_statement(&group.readings, sentence, group.following_bottom_remainder)
    else {
        return Ok(false);
    };
    let Some((parts, optional, gated)) = group.readings.take(statement.reading()) else {
        return Ok(false);
    };
    let Some((mode, single_match)) = traversal(&parts) else {
        return Ok(false);
    };
    group.mode = mode;
    group.single_match = single_match;
    group.optional = optional;
    group.gated |= gated;
    let match_tag = parts.match_tag.clone();
    let all_tag = parts.all_tag.clone();
    let player = parts.player;
    group.parts = Some(parts);

    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let (stripped, gate_on_result) = strip_leading_if_you_do_sentence(sentence.lowered());
    match statement {
        Statement::MatchedMove => {
            let matched = effect_grammar::parse_consult_matched_move_shape(tokens)
                .expect("the statement was recognized");
            group.followups.push(
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(match_tag, None),
                    matched.zone,
                    false,
                    controller(matched.controller_you),
                    false,
                    None,
                )
                .with_move_to_zone_plural_surface_if(matched.target_plural_surface),
            );
        }
        Statement::MoveBottom => {
            match effect_grammar::parse_consult_move_bottom_shape(tokens)
                .expect("the statement was recognized")
            {
                ConsultMoveBottomShape::MatchedToBattlefieldAndShuffle => {
                    let remainder = TargetAst::Object(
                        ObjectFilter::tagged(all_tag).not_tagged(match_tag.clone()),
                        None,
                        None,
                    );
                    group.followups.push(
                        EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(match_tag, None),
                            Zone::Battlefield,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        )
                        .with_move_to_zone_plural_surface(),
                    );
                    group
                        .followups
                        .push(EffectAst::subject_verb_shuffle_objects_into_library(
                            player, remainder,
                        ));
                }
                ConsultMoveBottomShape::MoveMatchAndBottom {
                    zone,
                    battlefield_tapped,
                    order,
                } => {
                    group.followups.push(EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(match_tag.clone(), None),
                        zone,
                        false,
                        ReturnControllerAst::Preserve,
                        battlefield_tapped,
                        None,
                    ));
                    group.followups.push(
                        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                            all_tag,
                            Some(match_tag),
                            order,
                            player,
                        ),
                    );
                }
            }
        }
        Statement::Cast | Statement::CastThenBottom => {
            let clause = parse_consult_cast_clause(sentence.lowered())
                .expect("the statement was recognized");
            group.cast_without_paying =
                matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost);
            group.cast_declinable = statement == Statement::Cast
                && group.cast_without_paying
                && !clause.allow_land;
            group
                .followups
                .extend(consult_cast_effects(&clause, match_tag)?);
        }
        Statement::BattlefieldOrHand => {
            group.followups.push(EffectAst::May {
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(match_tag.clone(), None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            });
            group.followups.push(EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(match_tag, None),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::You,
                    false,
                    None,
                )],
            });
        }
        Statement::Cleanup => {
            group.pending_cleanup = cleanup(sentence);
        }
        Statement::RevealPump => {
            group
                .followups
                .push(reveal_pump(sentence).expect("the statement was recognized"));
            group.pumped = true;
        }
        Statement::HandExileOthers => {
            group.followups.push(EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(match_tag.clone(), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
            group.followups.push(EffectAst::ForEachTagged {
                tag: all_tag,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::TaggedMatches(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        ObjectFilter::tagged(match_tag),
                    ),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_exile(it(), false)],
                }],
            });
        }
        Statement::PutCountersOnMatch => {
            let mut effect = counters_on_match(sentence).expect("the statement was recognized");
            if let SubjectVerbActionAst::PutCounters { target, .. } = &mut effect.action {
                let reference_span = match &*target {
                    TargetAst::Tagged(_, span) | TargetAst::Source(span) => *span,
                    TargetAst::Object(_, _, span) => *span,
                    _ => None,
                };
                *target = TargetAst::Tagged(match_tag, reference_span);
            }
            group.followups.push(EffectAst::SubjectVerb(effect));
        }
        Statement::AllToGraveyard => {
            group.followups.push(EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(all_tag, None),
                Zone::Graveyard,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
        }
        Statement::HandOthersGraveyard => {
            group.gate_on_result = gate_on_result;
            group.followups.push(EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(match_tag.clone(), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
            group
                .followups
                .push(others_into_graveyard(all_tag, match_tag));
            group.split_source_sentences = !optional && !gate_on_result && !gated;
        }
        Statement::BattlefieldGraveyard => {
            group.gate_on_result = gate_on_result;
            match effect_grammar::parse_consult_battlefield_graveyard_shape(&stripped)
                .expect("the statement was recognized")
            {
                ConsultBattlefieldGraveyardShape::RemainderThenMatch { controller_you } => {
                    group.followups.push(EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::Implicit,
                        SubjectVerbActionAst::PutTaggedRemainderInZone {
                            tag: all_tag,
                            keep_tagged: match_tag.clone(),
                            zone: Zone::Graveyard,
                            surface: ironsmith_core::LibraryRemainderSurface::Rest,
                        },
                    ));
                    group
                        .followups
                        .push(EffectAst::subject_verb_put_onto_battlefield(
                            PlayerAst::Implicit,
                            TargetAst::Tagged(match_tag, None),
                            false,
                            controller(controller_you),
                        ));
                }
                ConsultBattlefieldGraveyardShape::Combined {
                    controller_you,
                    tapped,
                } => {
                    group.followups.push(EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(match_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        controller(controller_you),
                        tapped,
                        None,
                    ));
                    group
                        .followups
                        .push(others_into_graveyard(all_tag, match_tag));
                }
            }
        }
    }
    group.consumed += 1;
    Ok(true)
}

fn controller(controller_you: bool) -> ReturnControllerAst {
    if controller_you {
        ReturnControllerAst::You
    } else {
        ReturnControllerAst::Preserve
    }
}

/// Every traversed card that is not the match goes to the graveyard.
fn others_into_graveyard(all_tag: TagKey, match_tag: TagKey) -> EffectAst {
    EffectAst::ForEachTagged {
        tag: all_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::tag::CompilerReferenceTag::It.bind(),
                ObjectFilter::tagged(match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                it(),
                Zone::Graveyard,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }],
    }
}

/// Close the procedure: the traversal, then its follow-ups, wrapped as the
/// traversal's optionality and gates require.
pub(super) fn finish(mut group: ConsultedGroup) -> Vec<EffectAst> {
    if let Some(cleanup) = group.pending_cleanup.take() {
        group.followups.push(cleanup);
    }
    let parts = group
        .parts
        .expect("a procedure closes only after a statement chose a reading");
    if group.split_source_sentences {
        return vec![
            EffectAst::SourceSentence {
                effects: parts.effects,
                leading_then: false,
                starting_with_controller: false,
            },
            EffectAst::SourceSentence {
                effects: group.followups,
                leading_then: false,
                starting_with_controller: false,
            },
        ];
    }
    let effects = wrap_optional_consult_effects(
        parts,
        group.optional,
        group.followups,
        group.gate_on_result,
        group.gated,
    );
    match group.condition {
        Some(predicate) => vec![EffectAst::Conditional {
            predicate,
            if_true: effects,
            if_false: Vec::new(),
        }],
        None => effects,
    }
}
