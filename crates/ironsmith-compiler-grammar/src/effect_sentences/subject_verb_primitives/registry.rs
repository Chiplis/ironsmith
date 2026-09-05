use super::*;
use crate::grammar::effects::subject_verb_registry_shapes as registry_shapes;
use crate::grammar::effects::typed_clause_heads::classify_typed_clause_head;
use crate::parse_trace;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};
pub(super) const MECHANIC_MARKER_PREFIXES: &[&[&str]] = &[
    &["you", "choose", "one", "of", "them"],
    &[
        "you", "may", "put", "a", "land", "card", "from", "among", "them", "into", "your", "hand",
    ],
    &["stand", "and", "fight"],
    &["venture", "into", "the", "dungeon"],
    &["it", "doesnt", "untap", "during"],
];
pub type SubjectVerbPrimitiveParser =
    for<'a> fn(SubjectVerbPrimitiveClause<'a>) -> ParseOutcome<Vec<EffectAst>>;
pub(super) type SubjectVerbPrimitiveNormalizedWords<'a> = TokenWordView<'a>;

const REGISTRY_CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const PRIMITIVE_ROUTE_VERBS: &[(&[&str], &str)] = &[
    (&["choose"], "Choose"),
    (&["search"], "Search"),
    (&["reveal"], "Reveal"),
    (&["exile"], "Exile"),
    (&["destroy"], "Destroy"),
    (&["return"], "Return"),
    (&["sacrifice"], "Sacrifice"),
    (&["counter", "sticker"], "Put"),
    (&["draw"], "Draw"),
    (&["damage"], "Deal"),
    (&["gain"], "Gain"),
    (&["shuffle"], "Shuffle"),
    (&["copy"], "Copy"),
    (&["transform"], "Transform"),
    (&["cant"], "Cant"),
    (&["become", "type"], "Become"),
    (&["distribute"], "Distribute"),
    (&["fight"], "Fight"),
    (&["unless-pays"], "Pay"),
];
const PRIMITIVE_ITERATED_SUBJECT_PREFIXES: &[&str] =
    &["each-player", "for-each-player", "each-opponent"];
const PRIMITIVE_EXPLICIT_SUBJECT_PREFIXES: &[&str] = &["you", "target"];
const THAT_PLAYER_SUBJECT_WORDS: &[&str] = &["that", "player"];
const YOU_SUBJECT_WORDS: &[&str] = &["you"];
const THEIR_HAND_OWNER_WORD: &str = "their";
const YOUR_HAND_OWNER_WORD: &str = "your";

fn registry_token_matches_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn registry_word_is_card_or_cards(word: &str) -> bool {
    REGISTRY_CARD_OR_CARDS_WORDS.contains(&word)
}

fn registry_token_is_card_or_cards(token: &OwnedLexToken) -> bool {
    token.as_word().is_some_and(registry_word_is_card_or_cards)
}

fn registry_token_is_life(token: &OwnedLexToken) -> bool {
    registry_token_matches_word(token, "life")
}

#[derive(Debug, Clone, Copy)]
pub struct SubjectVerbPrimitiveClause<'a> {
    tokens: &'a [OwnedLexToken],
}

impl<'a> SubjectVerbPrimitiveClause<'a> {
    pub fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self { tokens }
    }

    fn lexed(self) -> LexedClause<'a> {
        LexedClause::new(self.tokens)
    }

    pub fn tokens(self) -> &'a [OwnedLexToken] {
        self.tokens
    }

    pub fn len(self) -> usize {
        self.lexed().len()
    }

    pub fn is_empty(self) -> bool {
        self.lexed().is_empty()
    }

    pub fn token(self, idx: usize) -> Option<&'a OwnedLexToken> {
        self.lexed().token(idx)
    }

    pub fn before(self, idx: usize) -> Self {
        Self::new(self.lexed().before(idx).tokens())
    }

    pub fn from(self, idx: usize) -> Self {
        Self::new(self.lexed().from(idx).tokens())
    }

    pub fn between(self, start: usize, end: usize) -> Self {
        Self::new(self.lexed().between(start, end).tokens())
    }

    pub fn words(self) -> SubjectVerbPrimitiveNormalizedWords<'a> {
        self.lexed().words()
    }

    pub fn word_refs(self) -> Vec<&'a str> {
        self.lexed().word_refs()
    }

    pub fn text(self) -> String {
        self.lexed().text()
    }

    pub fn span(self) -> Option<TextSpan> {
        span_from_tokens(self.tokens)
    }

    pub fn first_word(self) -> Option<&'a str> {
        self.lexed().first_word()
    }

    pub fn token_index_after_words(self, word_count: usize) -> Option<usize> {
        self.lexed().token_index_after_words(word_count)
    }

    pub fn before_word(self, word_idx: usize) -> Option<Self> {
        registry_shapes::split_registry_clause_at_word(self.tokens, word_idx)
            .map(|split| Self::new(split.before))
    }

    pub fn from_word(self, word_idx: usize) -> Option<Self> {
        registry_shapes::split_registry_clause_at_word(self.tokens, word_idx)
            .map(|split| Self::new(split.after))
    }

    pub fn after_words(self, word_count: usize) -> Option<Self> {
        let token_idx = self.token_index_after_words(word_count)?;
        Some(self.from(token_idx))
    }

    pub fn find_token_word(self, expected: &str) -> Option<usize> {
        self.lexed().find_token_word(expected)
    }

    pub fn find_token_word_where(
        self,
        expected: &str,
        mut predicate: impl FnMut(usize, Self) -> bool,
    ) -> Option<usize> {
        self.lexed().find_token_word_where(expected, |idx, tail| {
            predicate(idx, Self::new(tail.tokens()))
        })
    }

    pub fn find_unquoted_token_word(self, expected: &str) -> Option<usize> {
        self.lexed().find_unquoted_token_word(expected)
    }

    pub fn split_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_word(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub fn split_once_on_word_trimmed(self, expected: &str) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_word_trimmed(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub fn split_once_on_word_any(self, expected: &[&str]) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_word_any(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub fn split_once_on_comma(self) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_comma()
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub fn trim(self) -> Vec<OwnedLexToken> {
        self.lexed().trim()
    }

    pub fn trimmed(self) -> Self {
        Self::new(self.lexed().trimmed().tokens())
    }

    pub fn trimmed_word_refs(self) -> Vec<&'a str> {
        self.lexed().trimmed_word_refs()
    }

    pub fn trimmed_and_comma_segments(self) -> Vec<Self> {
        self.lexed()
            .trimmed_and_comma_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub fn trimmed_period_segments(self) -> Vec<Self> {
        self.lexed()
            .trimmed_period_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub fn split_once_on_then_trimmed(self) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_then_trimmed()
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub fn parse_with_lexed(
        self,
        parser: fn(&[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        parser(self.tokens)
    }

    pub fn parse_one_with_lexed(
        self,
        parser: fn(&[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError>,
    ) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        Ok(parser(self.tokens)?.map(|effect| vec![effect]))
    }

    pub fn parse_value_with_lexed<T>(
        self,
        parser: fn(&[OwnedLexToken]) -> Result<Option<T>, CardTextError>,
    ) -> Result<Option<T>, CardTextError> {
        parser(self.tokens)
    }
}

impl<'a> std::ops::Deref for SubjectVerbPrimitiveClause<'a> {
    type Target = [OwnedLexToken];

    fn deref(&self) -> &Self::Target {
        self.tokens
    }
}

#[derive(Debug, Clone)]
pub struct SubjectVerbPrimitiveOwnedClause {
    tokens: Vec<OwnedLexToken>,
}

impl SubjectVerbPrimitiveOwnedClause {
    pub fn new(tokens: Vec<OwnedLexToken>) -> Self {
        Self { tokens }
    }

    pub fn from_clause(clause: SubjectVerbPrimitiveClause<'_>) -> Self {
        Self::new(clause.tokens().to_vec())
    }

    pub fn from_comma_trimmed_clause(clause: SubjectVerbPrimitiveClause<'_>) -> Self {
        Self::new(clause.trim())
    }

    pub fn as_clause(&self) -> SubjectVerbPrimitiveClause<'_> {
        SubjectVerbPrimitiveClause::new(&self.tokens)
    }

    pub fn tokens(&self) -> &[OwnedLexToken] {
        &self.tokens
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn first_word(&self) -> Option<&str> {
        self.as_clause().first_word()
    }

    pub fn from_tokens(&self, idx: usize) -> &[OwnedLexToken] {
        &self.tokens[idx.min(self.tokens.len())..]
    }

    pub fn append_comma_then(&mut self, clause: SubjectVerbPrimitiveClause<'_>) {
        self.tokens
            .push(OwnedLexToken::comma(TextSpan::synthetic()));
        self.tokens.extend_from_slice(clause.tokens());
    }

    pub fn append_clause(&mut self, clause: SubjectVerbPrimitiveClause<'_>) {
        self.tokens.extend_from_slice(clause.tokens());
    }

    pub fn extend_from_slice(&mut self, tokens: &[OwnedLexToken]) {
        self.tokens.extend_from_slice(tokens);
    }

    pub fn insert_leading_word(&mut self, word: &str) {
        self.tokens.insert(
            0,
            OwnedLexToken::word(word.to_string(), TextSpan::synthetic()),
        );
    }

    pub fn replace_leading_word(&mut self, word: &str) -> bool {
        if let Some(token) = self.tokens.first_mut()
            && token.as_word().is_some()
        {
            token.replace_word(word);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectVerbPrimitiveStage {
    PreDiagnostic,
    PostDiagnostic,
}

pub struct SubjectVerbPrimitive {
    pub id: &'static str,
    pub metadata: RegistryRuleMetadata,
    pub stage: SubjectVerbPrimitiveStage,
    pub head_hints: &'static [LexRuleHeadHint],
    pub shape_mask: u32,
    pub parser: SubjectVerbPrimitiveParser,
}

impl SubjectVerbPrimitive {
    pub const fn new(
        id: &'static str,
        stage: SubjectVerbPrimitiveStage,
        head_hints: &'static [LexRuleHeadHint],
        parser: SubjectVerbPrimitiveParser,
    ) -> Self {
        Self {
            id,
            metadata: RegistryRuleMetadata::distinct(
                RuleId::new(id),
                HeadDiscriminator::grammar("typed-effect-clause-head"),
            ),
            stage,
            head_hints,
            shape_mask: 0,
            parser,
        }
    }
}

pub(super) fn parse_pluralized_subtype_word(word: &str) -> Option<Subtype> {
    parse_subtype_flexible(word)
}

fn summarize_effects(effects: &[EffectAst]) -> String {
    effects
        .iter()
        .map(|effect| {
            let debug = format!("{effect:?}");
            debug
                .split([' ', '{', '('])
                .next()
                .unwrap_or("Effect")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn primitive_subject_verb_route(id: &str) -> String {
    let verb = primitive_route_verb(id);
    let subject = if primitive_route_starts_with_any(id, PRIMITIVE_ITERATED_SUBJECT_PREFIXES) {
        "iterated"
    } else if primitive_route_starts_with_any(id, PRIMITIVE_EXPLICIT_SUBJECT_PREFIXES) {
        "explicit"
    } else {
        "implicit"
    };
    format!("subject-verb verb={verb} subject={subject} recognizer={id}")
}

fn primitive_route_verb(id: &str) -> &'static str {
    PRIMITIVE_ROUTE_VERBS
        .iter()
        .find_map(|(needles, label)| primitive_route_contains_any(id, needles).then_some(*label))
        .unwrap_or("Do")
}

fn primitive_route_contains_any(id: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| id.contains(needle))
}

fn primitive_route_starts_with_any(id: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| id.starts_with(prefix))
}

pub fn subject_verb_primitive_outcome(
    id: RuleId,
    clause: SubjectVerbPrimitiveClause<'_>,
    result: Result<Option<Vec<EffectAst>>, CardTextError>,
) -> ParseOutcome<Vec<EffectAst>> {
    let span = crate::util::span_from_tokens(clause.tokens());
    match result {
        Ok(Some(effects)) if effects.is_empty() => ParseOutcome::Error(ParseDiagnostic::invariant(
            id,
            span,
            format!("primitive '{}' produced empty effects", id.as_str()),
        )),
        Ok(Some(effects)) => {
            let stage = format!(
                "parse_effect_sentence:subject-verb-primitive-hit:{}",
                id.as_str()
            );
            parser_trace(&stage, clause.tokens());
            parse_trace::event(format!(
                "effect subject/verb primitive: {} -> {}",
                id.as_str(),
                summarize_effects(&effects)
            ));
            parse_trace::event(format!(
                "effect-route: {}",
                primitive_subject_verb_route(id.as_str())
            ));
            ParseOutcome::matched(effects, span)
        }
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            if parser_trace_enabled() {
                eprintln!(
                    "[parser-flow] stage=parse_effect_sentence:subject-verb-primitive-error primitive={} clause='{}' error={error:?}",
                    id.as_str(),
                    clause.text()
                );
            }
            parse_trace::event(format!(
                "effect subject/verb primitive: {} errored: {error:?}",
                id.as_str()
            ));
            ParseOutcome::Error(ParseDiagnostic::from_card_text_error(id, span, error))
        }
    }
}

fn run_sentence_primitive(
    primitive: &SubjectVerbPrimitive,
    tokens: &[OwnedLexToken],
) -> ParseOutcome<Vec<EffectAst>> {
    let clause = SubjectVerbPrimitiveClause::new(tokens);
    (primitive.parser)(clause).within(primitive.metadata.id)
}

fn normalize_parser_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    for token in &mut normalized {
        match token.kind {
            crate::lexer::TokenKind::Word
            | crate::lexer::TokenKind::Number
            | crate::lexer::TokenKind::Tilde => {
                let replacement = token.parser_text().to_string();
                let _ = token.replace_word(replacement);
            }
            _ => {}
        }
    }
    normalized
}

fn run_sentence_primitive_lexed(
    primitive: &SubjectVerbPrimitive,
    tokens: &[OwnedLexToken],
    lowered: &OnceCell<Vec<OwnedLexToken>>,
) -> ParseOutcome<Vec<EffectAst>> {
    // Possessive owner subjects carry executable target provenance in the
    // apostrophe itself (`target creature's owner`). The shared parser-token
    // normalizer intentionally strips that punctuation, so this one typed
    // grammar rule must inspect the authored token stream before lowering.
    if primitive.id == "shuffle-object-into-library" {
        return run_sentence_primitive(primitive, tokens);
    }
    let lowered_tokens = lowered.get_or_init(|| normalize_parser_tokens(tokens));
    run_sentence_primitive(primitive, lowered_tokens)
}

fn recognize_subject_verb_primitives_lexed(
    tokens: &[OwnedLexToken],
    primitives: &'static [SubjectVerbPrimitive],
    index: &LexRuleHintIndex,
) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    let typed_head = match classify_typed_clause_head(tokens)
        .within(RuleId::new("subject-verb-primitive-registry"))
    {
        ParseOutcome::NoMatch => return ParseOutcome::NoMatch,
        ParseOutcome::Match(matched) => matched.value,
        ParseOutcome::Error(diagnostic) => return ParseOutcome::Error(diagnostic),
    };
    let lowered = OnceCell::new();
    let view = LexClauseView::from_tokens(tokens);
    let candidate_indices = index.candidate_indices(typed_head.first_word, typed_head.second_word);
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for idx in candidate_indices {
        let primitive = &primitives[idx];
        if primitive.shape_mask != 0 && (view.shape & primitive.shape_mask) != primitive.shape_mask
        {
            continue;
        }
        let outcome =
            run_sentence_primitive_lexed(primitive, tokens, &lowered).within(primitive.metadata.id);
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

    resolve_registry_candidates(
        RuleId::new("subject-verb-primitive-registry"),
        candidates,
        diagnostics,
    )
}

pub fn run_subject_verb_primitives_lexed(
    tokens: &[OwnedLexToken],
    primitives: &'static [SubjectVerbPrimitive],
    index: &LexRuleHintIndex,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    match recognize_subject_verb_primitives_lexed(tokens, primitives, index) {
        ParseOutcome::NoMatch => Ok(None),
        ParseOutcome::Match(matched) => Ok(Some(matched.value.value)),
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
    }
}

pub(super) fn parse_preconditional_subject_verb_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    debug_assert!(
        PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
            .iter()
            .all(|primitive| primitive.stage == SubjectVerbPrimitiveStage::PreDiagnostic)
    );
    recognize_subject_verb_primitives_lexed(
        view.tokens,
        PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )
    .map(|matched| matched.value)
}

pub(super) fn parse_postconditional_subject_verb_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> ParseOutcome<Vec<EffectAst>> {
    debug_assert!(
        POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
            .iter()
            .all(|primitive| primitive.stage == SubjectVerbPrimitiveStage::PostDiagnostic)
    );
    let matched = match recognize_subject_verb_primitives_lexed(
        view.tokens,
        POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    ) {
        ParseOutcome::NoMatch => return ParseOutcome::NoMatch,
        ParseOutcome::Match(matched) => matched.value,
        ParseOutcome::Error(diagnostic) => return ParseOutcome::Error(diagnostic),
    };
    let mut effects = matched.value;
    if let Err(error) = super::super::chain_carry::append_missing_coordinated_return_discard_tail(
        view.tokens,
        &mut effects,
    ) {
        return ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
            matched.rule,
            matched.span,
            error,
        ));
    }
    ParseOutcome::matched(effects, matched.span)
}

pub const SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("preconditional-subject-verb-primitives"),
            HeadDiscriminator::words(&[]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_preconditional_subject_verb_primitives_rule_lexed),
    }];

pub const SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        metadata: RegistryRuleMetadata::distinct(
            RuleId::new("postconditional-subject-verb-primitives"),
            HeadDiscriminator::words(&[]),
        ),
        shape_mask: 0,
        run: LexRuleHandler::Structured(parse_postconditional_subject_verb_primitives_rule_lexed),
    }];

pub const SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_RULES_LEXED);

pub const SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_RULES_LEXED);

pub fn parse_sentence_return_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_return_with_counters_on_it(SubjectVerbPrimitiveClause::new(tokens))
}

pub fn parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_put_onto_battlefield_with_counters_on_it(SubjectVerbPrimitiveClause::new(tokens))
}

pub fn parse_sentence_exile_source_with_counters_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_exile_source_with_counters(SubjectVerbPrimitiveClause::new(tokens))
}

pub fn parse_you_and_target_player_each_draw_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_joint_draw_shape(clause.tokens()) else {
        return Ok(None);
    };
    let amount_clause = SubjectVerbPrimitiveClause::new(shape.amount_tokens);
    let clause_text = clause.text();
    let remainder_words = amount_clause.word_refs();
    let count = if let Some((count, used_words)) =
        parse_half_rounded_down_draw_count_words(&remainder_words)
    {
        if !remainder_words[used_words..].is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing shared draw clause (clause: '{}')",
                clause_text
            )));
        }
        count
    } else {
        let (count, used) = parse_value(amount_clause.tokens()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing draw count in shared draw sentence (clause: '{}')",
                clause_text
            ))
        })?;
        if amount_clause
            .tokens()
            .get(used)
            .and_then(OwnedLexToken::as_word)
            .is_none_or(|word| !registry_word_is_card_or_cards(word))
        {
            return Err(CardTextError::ParseError(format!(
                "missing card keyword in shared draw sentence (clause: '{}')",
                clause_text
            )));
        }
        if !amount_clause.from(used + 1).word_refs().is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing shared draw clause (clause: '{}')",
                clause_text
            )));
        }
        count
    };
    let mut effects = vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: count.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            if shape.another_target_player {
                PlayerAst::That
            } else {
                shape.other_player
            },
            SubjectVerbActionAst::Draw { count },
        ),
    ];
    if shape.another_target_player {
        effects.insert(
            0,
            EffectAst::subject_verb_target_only(TargetAst::Player(
                PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::You),
                None,
            )),
        );
    }
    Ok(Some(effects))
}

pub fn parse_sentence_you_and_target_player_each_draw(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_target_player_each_draw_sentence(clause)
}

/// "You and that player each sacrifice a creature." Each actor makes an
/// independent choice from the permanents they control, so lower two typed
/// sacrifice actions inside one coordinated sentence boundary.
pub fn parse_you_and_player_each_sacrifice_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_joint_sacrifice_shape(clause.tokens()) else {
        return Ok(None);
    };
    let you = super::super::zone_handlers::parse_sacrifice(
        shape.object_tokens,
        Some(SubjectAst::Player(PlayerAst::You)),
        None,
    )?;
    let other = super::super::zone_handlers::parse_sacrifice(
        shape.object_tokens,
        Some(SubjectAst::Player(shape.other_player)),
        None,
    )?;
    Ok(Some(vec![EffectAst::Coordinated {
        effects: vec![you, other],
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub fn parse_sentence_you_and_player_each_sacrifice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_player_each_sacrifice_sentence(clause)
}

/// "You and that player each gain that much life." / "You and target opponent
/// each lose 2 life." — the joint-subject analog of the shared draw sentence.
pub fn parse_you_and_player_each_gain_or_lose_life_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_joint_life_shape(clause.tokens()) else {
        return Ok(None);
    };
    let amount_clause = SubjectVerbPrimitiveClause::new(shape.amount_tokens);
    let Some((amount, used)) = parse_value(amount_clause.tokens()) else {
        return Ok(None);
    };
    if amount_clause
        .tokens()
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "life")
        || !amount_clause.from(used + 1).word_refs().is_empty()
    {
        return Ok(None);
    }
    let action = |amount: Value| {
        if shape.gains {
            SubjectVerbActionAst::GainLife { amount }
        } else {
            SubjectVerbActionAst::LoseLife { amount }
        }
    };
    Ok(Some(vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            action(amount.clone()),
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            shape.other_player,
            action(amount),
        ),
    ]))
}

pub fn parse_sentence_you_and_player_each_gain_or_lose_life(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_player_each_gain_or_lose_life_sentence(clause)
}

/// "You and that player each create three 1/1 white Spirit creature tokens
/// with flying." — joint-subject token creation: parse the verb phrase once
/// and emit one copy per subject.
pub fn parse_you_and_player_each_create_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_joint_create_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Ok(EffectAst::SubjectVerb(template)) =
        super::super::parse_create(shape.effect_tokens, None)
    else {
        return Ok(None);
    };
    fn with_subject_player(
        template: &SubjectVerbEffectAst,
        player: PlayerAst,
    ) -> SubjectVerbEffectAst {
        let mut copy = template.clone();
        copy.subject.player = player;
        if let SubjectVerbActionAst::CreateTokenWithMods {
            player: action_player,
            ..
        } = &mut copy.action
        {
            *action_player = player;
        }
        copy
    }
    Ok(Some(vec![
        EffectAst::SubjectVerb(with_subject_player(&template, PlayerAst::You)),
        EffectAst::SubjectVerb(with_subject_player(&template, shape.other_player)),
    ]))
}

pub fn parse_sentence_you_and_player_each_create(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_player_each_create_sentence(clause)
}

/// "This creature and that creature each get ..." applies one authored
/// action chain independently to the ability source and to the previously
/// tagged object. Keeping this as a joint-subject primitive preserves both
/// actors without teaching the ordinary chain splitter to clone arbitrary
/// conjunctions.
pub fn parse_source_and_tagged_object_each_actions_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_joint_object_each_actions_shape(clause.tokens())
    else {
        return Ok(None);
    };

    let parse_for_subject = |subject: &[OwnedLexToken]| {
        let mut sentence = Vec::with_capacity(subject.len() + shape.action_tokens.len());
        sentence.extend_from_slice(subject);
        sentence.extend_from_slice(shape.action_tokens);
        // The authored joint subject takes the plural verb (`each get`).
        // Each independently lowered singular subject takes `gets`; later
        // coordinated verbs retain their ordinary base form and are carried
        // by the existing chain parser.
        if let Some(action_head) = sentence.get_mut(subject.len())
            && action_head.as_word() == Some("get")
        {
            action_head.replace_word("gets");
        }
        parse_effect_chain_lexed(&sentence)
    };
    let mut effects = parse_for_subject(shape.source_tokens)?;
    effects.extend(parse_for_subject(shape.tagged_tokens)?);
    Ok(Some(vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub fn parse_sentence_source_and_tagged_object_each_actions(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_source_and_tagged_object_each_actions_sentence(clause)
}

pub fn parse_sentence_choose_player_to_effect(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_choose_player_to_effect_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Some((chooser, filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(shape.choose_tokens)?
    else {
        return Ok(None);
    };
    let mut tail_effects = parse_effect_chain(shape.effect_tokens)?;
    for effect in &mut tail_effects {
        bind_implicit_player_context(effect, PlayerAst::That);
    }
    let mut effects = vec![EffectAst::subject_verb_choose_player(
        chooser,
        filter,
        crate::tag::CompilerReferenceTag::It.bind(),
        random,
        exclude_previous_choices,
    )];
    effects.extend(tail_effects);
    Ok(Some(effects))
}

pub fn parse_sentence_return_half_the_creatures_they_control_to_their_owners_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_return_half_controlled_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter(shape.filter_tokens, false)?;
    if filter.controller.is_none() {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
    }
    let count_value = Value::HalfRoundedDown(Box::new(Value::Add(
        Box::new(Value::Count(filter.clone())),
        Box::new(Value::Fixed(1)),
    )));
    let chosen_tag = crate::tag::CompilerReferenceTag::Chosen.bind();
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::dynamic_x(),
            count_value: Some(count_value),
            player: PlayerAst::That,
            tag: chosen_tag.clone(),
        },
        EffectAst::subject_verb_return_all_to_hand(ObjectFilter::tagged(chosen_tag)),
    ]))
}

pub fn parse_sentence_damage_to_that_player_half_damage_of_those_spells(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_historical_half_damage_shape(clause.tokens()) else {
        return Ok(None);
    };
    let card_type = parse_card_type(shape.card_type_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported spell type in historical half-damage sentence (clause: '{}')",
            clause.text()
        ))
    })?;
    Ok(Some(vec![
        EffectAst::subject_verb_choose_spell_cast_history(
            PlayerAst::You,
            PlayerAst::That,
            ObjectFilter::default().with_type(card_type),
            crate::tag::CompilerReferenceTag::It.bind(),
        ),
        EffectAst::subject_verb_damage(
            Value::HalfRoundedDown(Box::new(Value::DamageDealtThisTurnByTaggedSpellCast(
                crate::tag::CompilerReferenceTag::It.bind(),
            ))),
            TargetAst::Player(PlayerFilter::target_player(), None),
        ),
    ]))
}

pub fn parse_draw_for_each_card_exiled_from_hand_this_way_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_draw_for_exiled_hand_shape(clause.tokens()) else {
        return Ok(None);
    };
    let subject_words = LexedClause::new(shape.subject_tokens).word_refs();
    let hand_owner = match shape.hand_owner {
        registry_shapes::ExiledHandOwner::Your => Some(YOUR_HAND_OWNER_WORD),
        registry_shapes::ExiledHandOwner::Their => Some(THEIR_HAND_OWNER_WORD),
    };
    let Some((player, mut effects)) = draw_exiled_hand_this_way_actor(
        &subject_words,
        hand_owner,
        shape.shuffles_first,
        shape.starts_with_draws,
    ) else {
        return Ok(None);
    };
    let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
    if matches!(player, PlayerAst::That) {
        filter.owner = Some(PlayerFilter::IteratedPlayer);
    }
    effects.push(EffectAst::subject_verb_draw_for_each_tagged_matching(
        player,
        crate::tag::CompilerReferenceTag::It.bind(),
        filter,
    ));
    Ok(Some(effects))
}

fn draw_exiled_hand_this_way_actor(
    subject_words: &[&str],
    hand_owner: Option<&str>,
    shuffles_first: bool,
    starts_with_draws: bool,
) -> Option<(PlayerAst, Vec<EffectAst>)> {
    if subject_words == THAT_PLAYER_SUBJECT_WORDS && hand_owner == Some(THEIR_HAND_OWNER_WORD) {
        let effects = if shuffles_first {
            vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            )]
        } else {
            Vec::new()
        };
        return Some((PlayerAst::That, effects));
    }
    if !shuffles_first
        && subject_words == YOU_SUBJECT_WORDS
        && hand_owner == Some(YOUR_HAND_OWNER_WORD)
    {
        return Some((PlayerAst::You, Vec::new()));
    }
    if !shuffles_first
        && subject_words.is_empty()
        && hand_owner == Some(THEIR_HAND_OWNER_WORD)
        && starts_with_draws
    {
        return Some((PlayerAst::That, Vec::new()));
    }
    if !shuffles_first && subject_words.is_empty() && hand_owner == Some(YOUR_HAND_OWNER_WORD) {
        return Some((PlayerAst::Implicit, Vec::new()));
    }
    None
}

pub fn parse_sentence_draw_for_each_card_exiled_from_hand_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_for_each_card_exiled_from_hand_this_way_sentence(clause)
}

pub fn parse_sentence_you_and_attacking_player_each_draw_and_lose(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_attacking_player_draw_lose_shape(clause.tokens())
    else {
        return Ok(None);
    };
    let draw_clause = SubjectVerbPrimitiveClause::new(shape.draw_tokens);
    let draw_words = draw_clause.word_refs();
    let draw_count =
        if let Some((count, used_words)) = parse_half_rounded_down_draw_count_words(&draw_words) {
            if !draw_words[used_words..].is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing shared draw clause (clause: '{}')",
                    clause.text()
                )));
            }
            count
        } else {
            let (count, used) = parse_value(draw_clause.tokens()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing shared draw count (clause: '{}')",
                    clause.text()
                ))
            })?;
            if draw_clause
                .tokens()
                .get(used)
                .is_none_or(|token| !registry_token_is_card_or_cards(token))
                || !draw_clause.from(used + 1).word_refs().is_empty()
            {
                return Err(CardTextError::ParseError(format!(
                    "missing card keyword in shared draw/lose sentence (clause: '{}')",
                    clause.text()
                )));
            }
            count
        };
    let lose_clause = SubjectVerbPrimitiveClause::new(shape.lose_tokens);
    let (lose_amount, lose_used) = parse_value(lose_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared life-loss amount (clause: '{}')",
            clause.text()
        ))
    })?;
    if lose_clause
        .tokens()
        .get(lose_used)
        .is_none_or(|token| !registry_token_is_life(token))
        || !lose_clause.from(lose_used + 1).word_refs().is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in shared draw/lose sentence (clause: '{}')",
            clause.text()
        )));
    }
    Ok(Some(vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: draw_count.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Attacking,
            SubjectVerbActionAst::Draw { count: draw_count },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife {
                amount: lose_amount.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Attacking,
            SubjectVerbActionAst::LoseLife {
                amount: lose_amount,
            },
        ),
    ]))
}

pub fn parse_sentence_sacrifice_it_next_end_step(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_registry_next_end_step_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.action != registry_shapes::RegistryDelayedAction::Sacrifice {
        return Ok(None);
    }
    let filter = if registry_shapes::is_tagged_delayed_object(shape.object_tokens) {
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind())
    } else {
        parse_object_filter(shape.object_tokens, false)?
    };
    let sacrifice = EffectAst::subject_verb_sacrifice(PlayerAst::Implicit, filter, 1, None);
    let delayed_effects = if shape.trailing_tokens.is_empty() {
        vec![sacrifice]
    } else {
        let predicate =
            crate::grammar::structure::parse_trailing_if_predicate_lexed(shape.trailing_tokens)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported delayed sacrifice condition (clause: '{}')",
                        clause.text()
                    ))
                })?;
        vec![EffectAst::TrailingIf {
            predicate,
            effects: vec![sacrifice],
        }]
    };
    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: if shape.your_end_step {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        },
        effects: delayed_effects,
    }]))
}

pub fn parse_sentence_exile_it_next_end_step(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_registry_next_end_step_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.action != registry_shapes::RegistryDelayedAction::Exile {
        return Ok(None);
    }
    let object_clause = SubjectVerbPrimitiveClause::new(shape.object_tokens);
    let plural_demonstrative = shape
        .object_tokens
        .first()
        .is_some_and(|token| token.is_word("those") || token.is_word("them"));
    let exile = if shape.exhaustive {
        let filter = parse_object_filter(shape.object_filter_tokens, false)?;
        EffectAst::subject_verb_exile_all(filter, false)
    } else {
        let target = if registry_shapes::is_tagged_delayed_object(shape.object_tokens) {
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                object_clause.span(),
            )
        } else {
            let mut filter = parse_object_filter(shape.object_tokens, false)?;
            if plural_demonstrative {
                if !filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                }) {
                    filter.tagged_constraints.push(TaggedObjectConstraint {
                        tag: crate::tag::CompilerReferenceTag::It.bind(),
                        relation: TaggedOpbjectRelation::IsTaggedObject,
                    });
                }
                filter.set_plural_object_noun_surface(true);
            }
            TargetAst::Object(filter, None, object_clause.span())
        };
        EffectAst::subject_verb_exile(target, false)
    };
    let delayed_effects = if shape.trailing_tokens.is_empty() {
        vec![exile]
    } else {
        let predicate =
            crate::grammar::structure::parse_trailing_if_predicate_lexed(shape.trailing_tokens)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported delayed exile condition (clause: '{}')",
                        clause.text()
                    ))
                })?;
        vec![EffectAst::TrailingIf {
            predicate,
            effects: vec![exile],
        }]
    };
    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: if shape.your_end_step {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        },
        effects: delayed_effects,
    }]))
}

pub fn parse_sentence_if_tagged_cards_remain_exiled(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if registry_shapes::parse_remain_exiled_tail(clause.tokens()).is_none() {
        return Ok(None);
    }
    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        clause.tokens(),
        parse_effect_chain_lexed,
    )
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EventValueSpec;
    use crate::lexer::lex_line;
    use crate::model::ast::SubjectVerbSubjectAst;

    #[test]
    fn shared_draw_sentence_accepts_that_player() {
        let tokens = lex_line("You and that player each draw that many cards.", 0)
            .expect("xyris-style shared draw clause should lex");

        let parsed = parse_you_and_target_player_each_draw_sentence(
            SubjectVerbPrimitiveClause::new(&tokens),
        )
        .expect("xyris-style shared draw clause should not error")
        .expect("xyris-style shared draw clause should parse");

        assert!(matches!(
            parsed.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::You,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw {
                        count: Value::EventValue(EventValueSpec::Amount),
                    },
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::That,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw {
                        count: Value::EventValue(EventValueSpec::Amount),
                    },
                }),
            ]
        ));

        let public_tokens = lex_line("You and target opponent each draw three cards", 0)
            .expect("shared target-opponent draw should lex");
        let public = crate::effect_sentences::parse_effect_sentences_lexed(&public_tokens)
            .expect("public sentence registry should parse the coordinated draw");
        assert_eq!(
            public.len(),
            2,
            "the public route must retain both coordinated player actions: {public:#?}"
        );
        assert!(matches!(
            public.get(1),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: SubjectVerbSubjectAst {
                    player: PlayerAst::TargetOpponent,
                    ..
                },
                action: SubjectVerbActionAst::Draw {
                    count: Value::Fixed(3),
                },
                ..
            }))
        ));
    }

    #[test]
    fn delayed_sacrifice_retains_resolution_time_condition() {
        let tokens = lex_line(
            "Sacrifice it at the beginning of the next end step if it has mana value 3 or less.",
            0,
        )
        .expect("conditional delayed sacrifice should lex");

        let parsed =
            parse_sentence_sacrifice_it_next_end_step(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("conditional delayed sacrifice should not error")
                .expect("conditional delayed sacrifice should use the registry route");
        let debug = format!("{parsed:#?}");

        assert!(debug.contains("DelayedUntilNextEndStep"), "{debug}");
        assert!(debug.contains("TrailingIf"), "{debug}");
        assert!(debug.contains("mana_value"), "{debug}");
        assert!(debug.contains("LessThanOrEqual"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");

        let full_parse = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
            .expect("the full dispatcher should retain the delayed condition");
        let full_debug = format!("{full_parse:#?}");
        let [EffectAst::DelayedUntilNextEndStep { effects, .. }] = full_parse.as_slice() else {
            panic!("the timing owner must remain outside its condition: {full_parse:#?}");
        };
        assert!(matches!(effects.as_slice(), [EffectAst::TrailingIf { .. }]));
        assert!(full_debug.contains("mana_value"), "{full_debug}");
    }
}
