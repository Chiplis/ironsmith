use super::*;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LibraryActionAst;
use crate::cards::builders::ZoneMoveActionAst;

use crate::diagnostics::TextSpan;
use crate::recognition::ParseOutcome;
#[path = "clause_dispatch_core/clause_readings.rs"]
mod clause_readings;

/// "Each player with exactly 13 life loses the game" (Triskaidekaphobia): the
/// life qualifier gates the action for each iterated player.
fn parse_each_player_with_life_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let word_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.as_word().is_some())
        .map(|(index, _)| index)
        .collect();
    let words: Vec<&str> = word_positions
        .iter()
        .map(|&index| tokens[index].parser_text())
        .collect();
    // The each-player fanout has already restated the subject as "that
    // player" when it reaches a clause.
    let (iterates, qualifier) = match words.as_slice() {
        ["each", "player", "with", qualifier @ ..] => (true, qualifier),
        ["that", "player", "with", qualifier @ ..] => (false, qualifier),
        _ => return Ok(None),
    };
    // "Each player with fewer cards in hand than the player with the most
    // cards in hand draws cards equal to the difference" (Tales of the
    // Ancestors): a comparison against the greatest hand size, whose operands
    // are the antecedent of "the difference".
    const FEWER_THAN_MOST_CARDS: &[&str] = &[
        "fewer", "cards", "in", "hand", "than", "the", "player", "with", "the", "most", "cards",
        "in", "hand",
    ];
    if qualifier.starts_with(FEWER_THAN_MOST_CARDS) {
        let rest_word = 3 + FEWER_THAN_MOST_CARDS.len();
        let Some(&rest_start) = word_positions.get(rest_word) else {
            return Ok(None);
        };
        let mut rebuilt = vec![
            OwnedLexToken::synthetic_word("that"),
            OwnedLexToken::synthetic_word("player"),
        ];
        rebuilt.extend_from_slice(&tokens[rest_start..]);
        let parsed = parse_effect_clause_unstacked(&rebuilt)?;
        let predicate = crate::cards::builders::PredicateAst::ValueComparison {
            left: crate::effect::Value::CardsInHand(crate::target::PlayerFilter::IteratedPlayer),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: crate::effect::Value::MaxCardsInHand(crate::target::PlayerFilter::Any),
        };
        let gated = vec![EffectAst::Conditionals(
            crate::cards::builders::ConditionalEffectAst::Conditional {
                predicate,
                if_true: vec![parsed],
                if_false: Vec::new(),
            },
        )];
        return Ok(Some(if iterates {
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: gated })
        } else {
            gated.into_iter().next().expect("one gated effect")
        }));
    }
    let (operator, amount, used) = match qualifier {
        ["exactly", amount, "life", ..] => {
            (crate::effect::ValueComparisonOperator::Equal, *amount, 3)
        }
        [amount, "or", "less" | "fewer", "life", ..] => (
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            *amount,
            4,
        ),
        [amount, "or", "more", "life", ..] => (
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            *amount,
            4,
        ),
        _ => return Ok(None),
    };
    let Some(amount) =
        crate::util::parse_number_word_u32(amount).or_else(|| crate::util::decimal_count(amount))
    else {
        return Ok(None);
    };
    let rest_word = 3 + used;
    let Some(&rest_start) = word_positions.get(rest_word) else {
        return Ok(None);
    };
    // Each iterated player is "that player" of the gated action.
    let mut rebuilt = vec![
        OwnedLexToken::synthetic_word("that"),
        OwnedLexToken::synthetic_word("player"),
    ];
    rebuilt.extend_from_slice(&tokens[rest_start..]);
    let parsed = parse_effect_clause_unstacked(&rebuilt)?;
    let predicate = crate::cards::builders::PredicateAst::ValueComparison {
        left: crate::effect::Value::LifeTotal(crate::target::PlayerFilter::IteratedPlayer),
        operator,
        right: crate::effect::Value::Fixed(amount as i32),
    };
    let gate = |effects: Vec<EffectAst>| {
        vec![EffectAst::Conditionals(
            crate::cards::builders::ConditionalEffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: effects,
                if_false: Vec::new(),
            },
        )]
    };
    let gated = gate(vec![parsed]);
    Ok(Some(if iterates {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: gated })
    } else {
        gated.into_iter().next().expect("one gated effect")
    }))
}

/// "reduce that opponent's speed by 1" / "reduce your speed by N"
/// (Spikeshell Harrier): a player's speed decreases (CR 702.179). The floor
/// is 0 unless a following "can't reduce ... below N" sentence raises it.
fn parse_reduce_speed_clause(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let (player, amount_word) = match words.as_slice() {
        ["reduce", "your", "speed", "by", amount] => (PlayerAst::You, *amount),
        [
            "reduce",
            "that",
            "opponent's" | "opponents" | "player's" | "players",
            "speed",
            "by",
            amount,
        ] => (PlayerAst::ThatPlayerOrTargetController, *amount),
        ["reduce", "their", "speed", "by", amount] => (PlayerAst::That, *amount),
        _ => return None,
    };
    let amount = crate::util::parse_number_word_u32(amount_word)
        .or_else(|| crate::util::decimal_count(amount_word))?;
    Some(EffectAst::subject_verb_reduce_speed(
        player,
        crate::effect::Value::Fixed(i32::try_from(amount).ok()?),
        0,
    ))
}

/// "[+2]" / "[−1]" as an instruction inside a resolving loyalty ability
/// (Comet, Stellar Pup's die rows): the printed loyalty symbol adds or removes
/// that many loyalty counters on the source (CR 606.4 symbols used as text).
pub(crate) fn parse_inline_loyalty_symbol_clause(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    use crate::lexer::TokenKind;
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let tokens = match tokens {
        [open, inner @ .., close]
            if open.kind == TokenKind::LBracket && close.kind == TokenKind::RBracket => inner,
        other => other,
    };
    // The lexer retains a signed number as one word. Document bracket
    // processing can also pass the symbol's contents without its brackets.
    let (positive, amount) = match tokens {
        [token] => {
            let text = token.parser_text();
            let (positive, digits) = if let Some(digits) = text.strip_prefix('+') {
                (true, digits)
            } else {
                (false, text.strip_prefix('-').or_else(|| text.strip_prefix('−'))?)
            };
            (positive, digits.parse::<i32>().ok()?)
        }
        [sign, amount] if matches!(sign.kind, TokenKind::Plus | TokenKind::Dash) => {
            (sign.kind == TokenKind::Plus, amount.parser_text().parse::<i32>().ok()?)
        }
        _ => return None,
    };
    if amount <= 0 { return None; }
    let source = crate::cards::builders::TargetAst::Source(None);
    match positive {
        true => Some(EffectAst::subject_verb_put_counters(
            crate::object::CounterType::Loyalty,
            crate::effect::Value::Fixed(amount),
            source,
            None,
            false,
        )),
        false => Some(EffectAst::subject_verb_remove_up_to_any_counters(
            crate::effect::Value::Fixed(amount),
            source,
            Some(crate::object::CounterType::Loyalty),
            false,
        )),
    }
}

pub(super) fn parse_effect_clause_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    if let Some(effects) = crate::effect_sentences::life_unit_programs::parse_prefix(tokens)? {
        return Ok(EffectAst::Sequence { effects });
    }
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }
    if let Some(effect) = parse_reduce_speed_clause(tokens) {
        return Ok(effect);
    }
    if let Some(effect) = parse_inline_loyalty_symbol_clause(tokens) {
        return Ok(effect);
    }
    // "choose a letter" (_____ _____ Rocketship): a named option among the
    // 26 letters, remembered on the source.
    if matches!(
        crate::lexer::token_word_refs(crate::util::trim_edge_punctuation_tokens(tokens)).as_slice(),
        ["choose", "a", "letter"] | ["you", "choose", "a", "letter"]
    ) {
        return Ok(EffectAst::subject_verb_choose_named_option(
            PlayerAst::You,
            ('A'..='Z').map(|letter| letter.to_string()).collect(),
        ));
    }
    // "claim the prize!" (Attractions): the claim keyword action; the
    // document splices the card's "Prize —" instructions after it.
    if matches!(
        crate::lexer::token_word_refs(crate::util::trim_edge_punctuation_tokens(tokens)).as_slice(),
        ["claim", "the", "prize"] | ["you", "claim", "the", "prize"]
    ) {
        return Ok(EffectAst::subject_verb_emit_keyword_action(
            crate::events::KeywordActionKind::ClaimAttractionPrize,
            1,
        ));
    }
    let stripped_instead = super::super::strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);
    let tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    // "That creature phases out until you roll a 3 or less while rolling to
    // visit your Attractions" (Ferris Wheel): the duration names a roll, so
    // the generic verb search would otherwise read "roll" as the action.
    {
        let mut terminated = tokens.to_vec();
        if !terminated.last().is_some_and(|token| token.is_period()) {
            terminated.push(OwnedLexToken::period(TextSpan::synthetic()));
        }
        if matches!(
            crate::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(
                &terminated
            ),
            Some(crate::grammar::effects::clause_pattern_shapes::KeywordMechanicShape::Phase {
                until_visit_roll_at_most: Some(_),
                ..
            })
        ) && let Some(effect) =
            super::super::clause_pattern_helpers::parse_keyword_mechanic_clause(&terminated)?
        {
            return Ok(effect);
        }
    }
    // "the owner of each card exiled with this Saga puts that card on the
    // bottom of their library": the whole source-exiled set moves, whatever
    // procedure (a vote result, a condition) reached this clause.
    if let Some(effect) =
        crate::effect_sentences::dispatch_inner::parse_source_exiled_owner_library_bottom_subject_verb(
            tokens,
        )
    {
        return Ok(effect);
    }
    if let Some(shape) = crate::grammar::effects::parse_shuffle_object_shape_lexed(tokens)
        && shape.owner_subject_target_tokens.is_some()
        && let Some(effects) =
            super::super::search_library::parse_shuffle_object_into_library_sentence(tokens)?
    {
        return Ok(EffectAst::Sequence { effects });
    }
    if let Some(effect) = super::super::gain_ability::parse_copular_base_characteristic_assignment(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_each_player_with_life_clause(tokens)? {
        return Ok(effect);
    }
    // "it deals 2 damage to you unless it came under your control this turn"
    // (Erg Raiders): a resolution-time state exception, not a payment.
    if let Some(unless) = tokens.iter().position(|token| token.is_word("unless"))
        && unless > 0
        && crate::lexer::token_word_refs(&tokens[unless + 1..]).as_slice()
            == ["it", "came", "under", "your", "control", "this", "turn"]
    {
        let effect = parse_effect_clause_unstacked(&tokens[..unless])?;
        return Ok(EffectAst::Conditionals(
            crate::cards::builders::ConditionalEffectAst::TrailingUnless {
                predicate: crate::cards::builders::PredicateAst::Source(
                    crate::cards::builders::SourcePredicateAst::SourceCameUnderYourControlThisTurn,
                ),
                effects: vec![effect],
            },
        ));
    }
    if let Some(effect) = crate::effect_sentences::clause_pattern_helpers::parse_can_attack_as_though_no_defender_clause(tokens)? {
        return Ok(effect);
    }
    let restriction_head = matches!(
        crate::grammar::effects::typed_clause_heads::classify_typed_clause_head(tokens),
        ParseOutcome::Match(matched)
            if matched.value.form
                == crate::grammar::effects::typed_clause_heads::ClauseHeadFormAst::Restriction
    );
    if restriction_head
        && let Some(effects) = crate::grammar::effects::parse_cant_effect_sentence(tokens)?
    {
        // Trigger-facts and nested consequence readers may enter at the
        // clause boundary, bypassing the sentence primitive index entirely.
        return Ok(EffectAst::Sequence { effects });
    }
    let input = clause_readings::Clause {
        tokens,
        read_by_cache: Default::default(),
    };
    match clause_readings::read_clause(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => clause_readings::diagnose(&input)?,
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    if restriction_head {
        return Err(CardTextError::ParseError(format!(
            "unsupported complete negated restriction clause (clause: '{}')",
            render_lower_words(tokens)
        )));
    }
    // "... and it's a 3/3 Robot artifact creature with flying" (Brilliance
    // Unleashed): a contracted pronoun copula split off as its own clause
    // states the object's new characteristics, read as "becomes".
    // A descriptor the become grammar cannot read keeps the clause's other
    // readings.
    if let Some((subject, animation)) =
        clause_grammar::parse_contracted_pronoun_copula_shape(tokens)
        && let Ok(effect) = parse_become_clause(&subject, &animation)
    {
        return Ok(effect);
    }
    // "it's goaded" / "it's goaded for as long as they control it" (Vislor
    // Turlough): the passive form of goading the referenced creature; the
    // control-bounded duration names the player who controls it as it is
    // goaded (CR 701.15, 611.2b).
    {
        let words = crate::lexer::token_word_refs(tokens);
        let tail = match words.as_slice() {
            ["it's" | "its", crate::keyword_forms::GOADED, tail @ ..] | ["it", "is", crate::keyword_forms::GOADED, tail @ ..] => Some(tail),
            // "that creature is also goaded" (Hot Pursuit): the same passive
            // goad of the referenced creature; "also" adds to its other state.
            ["that", "creature", "is", crate::keyword_forms::GOADED, tail @ ..]
            | ["that", "creature", "is", "also", crate::keyword_forms::GOADED, tail @ ..]
            | ["it", "is", "also", crate::keyword_forms::GOADED, tail @ ..] => Some(tail),
            // "They're goaded for the rest of the game." (Dack Fayden,
            // Helping Hand): the plural referent, goaded until the game ends.
            ["they're" | "theyre", crate::keyword_forms::GOADED, tail @ ..] | ["they", "are", crate::keyword_forms::GOADED, tail @ ..]
                if matches!(tail, [] | ["for", "the", "rest", "of", "the" | "this", "game"]) =>
            {
                Some(tail)
            }
            _ => None,
        };
        if let Some(tail) = tail {
            let it = crate::cards::builders::TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                None,
            );
            match tail {
                [] => return Ok(EffectAst::subject_verb_goad(it)),
                ["for", "the", "rest", "of", "the" | "this", "game"] => {
                    return Ok(EffectAst::subject_verb_goad_for(it, Until::Forever));
                }
                ["for", "as", "long", "as", "they" | "that" | "its", .., "control" | "controls", "it"] => {
                    use ironsmith_core::{
                        ContinuousDurationObject as Object, ContinuousDurationPlayer as Player,
                        ContinuousDurationPredicate as Predicate,
                    };
                    return Ok(EffectAst::subject_verb_goad_for(
                        it,
                        Until::ForAsLongAs(Predicate::ObjectControlledBy {
                            object: Object::AffectedObject,
                            player: Player::ControllerOf(Object::AffectedObject),
                        }),
                    ));
                }
                _ => {}
            }
        }
    }
    let (verb, _) = find_verb(tokens).ok_or_else(|| {
        let clause = render_lower_words(tokens);
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "convert",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "unattach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "detain",
            "goad",
            "suspect",
            "note",
            "end",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    let verb_shape = clause_grammar::parse_clause_subject_verb_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "could not split subject and verb in effect clause (clause: '{}')",
            render_lower_words(tokens)
        ))
    })?;
    let mut subject_tokens_storage = trim_commas(verb_shape.subject_tokens);
    // "It also gets +1/+0 ..." (Might of the Meek): the adverb sits between
    // the subject and its verb and is not part of the subject phrase.
    while subject_tokens_storage.len() > 1
        && subject_tokens_storage
            .last()
            .is_some_and(|token| token.is_word("also"))
    {
        subject_tokens_storage.pop();
    }
    let subject_tokens = subject_tokens_storage.as_slice();
    let rest = verb_shape.action_tokens;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));
    // "Gogo and that creature each get +2/+0 ... and attack this turn if
    // able": one predicate for two independent recipients.
    if matches!(verb, Verb::Get | Verb::Gain)
        && let Some(sentences) =
            super::super::gain_ability::split_anaphor_pair_subject_sentences(tokens)
    {
        // "Alandra and Drakes you control each get +X/+X": the source is one
        // of two recipients of a single coordinated predicate.
        let source_recipient = {
            let words = crate::lexer::token_word_refs(&sentences[0]);
            let verb = words
                .iter()
                .position(|word| matches!(*word, "gain" | "gains" | "get" | "gets"))
                .unwrap_or(words.len());
            crate::util::is_source_reference_words(&words[..verb])
        };
        let mut effects = Vec::new();
        for sentence in sentences {
            effects.push(parse_effect_clause_unstacked(&sentence)?);
        }
        if source_recipient {
            return Ok(EffectAst::Coordinated {
                effects,
                leading_duration: false,
                result_conjunction: false,
            });
        }
        return Ok(EffectAst::Sequence { effects });
    }
    // The verb names the clause family; each family's typed shape reads before
    // the general verb dispatch below.
    match verb {
        Verb::Counter => {
            if let Some(entry) = tokens
                .windows(2)
                .position(|pair| pair[0].is_word("enters") && pair[1].is_word("with"))
                && crate::util::is_source_reference_words(&crate::lexer::token_word_refs(
                    &tokens[..entry],
                ))
            {
                return parse_put_counters(&tokens[entry + 2..]);
            }
            if !subject_tokens.is_empty()
                && !tokens
                    .first()
                    .is_some_and(|token| token.is_any_word(&["if", "unless", "when", "whenever"]))
                && contains_token_word(tokens, "on")
                && let Ok(effect) = parse_put_counters(tokens)
            {
                parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
                return Ok(effect);
            }
        }
        Verb::Get => {
            if let Some(effect) = parse_get_pump_clause(subject_tokens, rest, tokens)? {
                return Ok(effect);
            }
        }
        Verb::Sacrifice => {
            if let Some((subject, target)) =
                parse_controller_or_owner_of_target_subject(subject_tokens)
            {
                return parse_sacrifice(rest, Some(subject), Some(target));
            }
        }
        Verb::Put => {
            if let Some((SubjectAst::Player(PlayerAst::ItsOwner), target)) =
                parse_controller_or_owner_of_target_subject(subject_tokens)
            {
                // "... puts it on their choice of the top or bottom of their
                // library" / "... puts it into their library second from the
                // top or on the bottom".
                let top_position = if is_pronoun_top_or_bottom_library_choice_put_tail(rest) {
                    Some(0)
                } else {
                    let words = crate::lexer::parser_token_word_refs(rest);
                    (words.as_slice()
                        == [
                            "it", "into", "their", "library", "second", "from", "the", "top", "or",
                            "on", "the", "bottom",
                        ])
                    .then_some(1)
                };
                if let Some(top_position) = top_position {
                    return Ok(EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::ItsOwner,
                        SubjectVerbActionAst::Library(
                            LibraryActionAst::MoveToLibraryTopOrBottomChoice {
                                target,
                                top_position,
                            },
                        ),
                    ));
                }
            }
        }
        _ => {}
    }
    let subject_word_view = ClauseDispatchCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    // An indicative control clause is not a player subject for a later verb.
    // Do not discard an unsupported static clause while keeping its "and draw"
    // tail as an unrelated spell instruction.
    if subject_words.starts_with(&["you", "control"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported complete control clause (clause: '{}')",
            render_lower_words(tokens)
        )));
    }
    if matches!(verb, Verb::Gain)
        && !subject_tokens.is_empty()
        && let Some(shape) = clause_grammar::parse_protection_choice_shape(rest)
    {
        let chooser = match shape.chooser {
            clause_grammar::ProtectionChoiceChooserShape::You => PlayerAst::You,
            clause_grammar::ProtectionChoiceChooserShape::TargetController => {
                PlayerAst::ItsController
            }
        };
        // "you and each permanent you control gain protection from the color
        // of your choice" (Faith's Shield): one choice, granted to the player
        // and to each member of the object set.
        if let Some((player, also_each)) =
            crate::effect_sentences::parse_player_and_each_object_recipients(subject_tokens)?
        {
            return Ok(EffectAst::subject_verb_grant_protection_choice_with_each(
                player,
                also_each,
                chooser,
                shape.includes_colorless,
                shape.includes_artifacts,
                shape.chooses_card_type,
            ));
        }
        let target = parse_target_phrase(subject_tokens)?;
        return Ok(EffectAst::subject_verb_grant_protection_choice(
            target,
            chooser,
            shape.includes_colorless,
            shape.includes_artifacts,
            shape.chooses_card_type,
        ));
    }
    if matches!(verb, Verb::Gain)
        && let Some(effects) =
            super::super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(EffectAst::Sequence { effects });
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Gain) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let parsed_actions = parse_ability_line(tail.ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(tail.ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !tail.ability_tokens.is_empty()
            && tail.trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_grant_abilities_to_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = super::super::gain_ability::parse_lose_family_types_and_abilities(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose) && clause_grammar::parse_shared_ability_gain_shape(rest).is_some()
    {
        let target = match clause_grammar::parse_reference_subject_shape(subject_tokens) {
            clause_grammar::ReferenceSubjectShape::Source => {
                TargetAst::Source(span_from_tokens(subject_tokens))
            }
            clause_grammar::ReferenceSubjectShape::Tagged => TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(subject_tokens),
            ),
            clause_grammar::ReferenceSubjectShape::Other => parse_target_phrase(subject_tokens)?,
        };
        return Ok(EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            Until::EndOfTurn,
        ));
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let ability_tokens = trim_edge_punctuation(tail.ability_tokens);
        let trailing_tokens = trim_edge_punctuation(tail.trailing_tokens);
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Deal)
        && let Some(effect) = parse_explicit_target_object_damage_source(subject_tokens, rest)?
    {
        return Ok(effect);
    }
    // "it deals 1 damage to each opponent": the per-player fan-out keeps the
    // anaphoric damage source (CR 120.3); the generic route below would drop
    // the subject and make the ability's source deal the damage.
    if matches!(verb, Verb::Deal)
        && let Some(effect @ EffectAst::ForEach(_)) =
            crate::effect_sentences::parse_anaphoric_object_deals_damage_clause(tokens)?
    {
        return Ok(effect);
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let subject_words = crate::lexer::parser_token_word_refs(subject_tokens);
    let each_other_player = crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[&["each"], &["other"], &["player", "players"]],
    );
    let another_target_player = crate::word_primitives::parse_sequence_complete(
        &subject_words,
        &["another", "target", "player"],
    );
    let optional_target_player = if crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[
            &["up"],
            &["to"],
            &["one"],
            &["target"],
            &["player", "players"],
        ],
    ) {
        Some(TargetAst::WithCount(
            Box::new(TargetAst::Player(
                PlayerFilter::Any,
                span_from_tokens(subject_tokens),
            )),
            ChoiceCount::up_to(1),
        ))
    } else if crate::word_primitives::parse_sequence_prefix(&subject_words, &["up", "to"]) {
        let target = parse_target_phrase(subject_tokens)?;
        let is_optional_player = matches!(
            &target,
            TargetAst::WithCount(inner, count)
                if matches!(inner.as_ref(), TargetAst::Player(_, _))
                    && count.min == 0
                    && count.max == Some(1)
        );
        is_optional_player.then_some(target)
    } else {
        None
    };
    if matches!(verb, Verb::Return)
        && clause_grammar::is_return_tagged_reference_shape(subject_tokens)
    {
        let mut return_tokens = subject_tokens.to_vec();
        return_tokens.extend(rest.iter().cloned());
        return parse_effect_with_verb(verb, Some(SubjectAst::This), &return_tokens);
    }
    if matches!(verb, Verb::Put)
        && clause_grammar::is_exiled_cards_to_hand_shape(subject_tokens, rest)
    {
        let filter = parse_object_filter(subject_tokens, false)?;
        return Ok(EffectAst::subject_verb_return_all_to_hand(filter));
    }
    let relative_player_subject = if subject_tokens
        .first()
        .is_some_and(|token| token.is_word(TARGET_WORD))
    {
        match parse_target_phrase(subject_tokens) {
            Ok(target) => match &target {
                TargetAst::Player(filter, _)
                    if !matches!(filter, PlayerFilter::Any | PlayerFilter::Opponent) =>
                {
                    Some(target)
                }
                _ => None,
            },
            Err(_) => None,
        }
    } else {
        None
    };
    let mut effect = if let Some(target) = optional_target_player {
        let action = parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), action],
        }
    } else if another_target_player {
        let target = TargetAst::Player(
            PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::target_player()),
            span_from_tokens(subject_tokens),
        );
        let action = parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), action],
        }
    } else if let Some(target) = relative_player_subject {
        let source_relative_target = target_player_mentions_source_object(&target);
        let mut action =
            parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        if source_relative_target && matches!(verb, Verb::Gain) {
            bind_gain_control_pronoun_to_source(&mut action);
        }
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), action],
        }
    } else if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = if each_other_player {
            SubjectAst::Player(PlayerAst::That)
        } else {
            parse_subject(subject_tokens)
        };
        if let Some(clause) = CommonPlayerActionClause::recognize(subject, verb, rest) {
            clause.lower()?
        } else {
            parse_effect_with_verb(verb, Some(subject), rest)?
        }
    };
    let authored_control_pronoun = {
        let rest_words = ClauseDispatchCompatWords::new(rest).to_word_refs();
        crate::word_primitives::sequence_occurs(&rest_words, &["they", "control"])
    };
    if matches!(verb, Verb::Return)
        && (crate::word_primitives::parse_sequence_complete(&subject_words, &["they"])
            || authored_control_pronoun)
        && let EffectAst::SubjectVerb(subject_verb) = &mut effect
        && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. }) =
            &mut subject_verb.action
    {
        fn mark_iterated_actor_pronoun(target: &mut TargetAst) {
            match target {
                TargetAst::Object(filter, ..) => {
                    filter.set_iterated_actor_pronoun_surface(true);
                }
                TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
                    mark_iterated_actor_pronoun(inner);
                }
                _ => {}
            }
        }
        mark_iterated_actor_pronoun(target);
    }
    // "Each land of the first chosen type becomes the second chosen type":
    // a become-with-choice picks once for the whole affected set, so it must
    // not be repeated (with a fresh choice) for each object.
    let choice_applies_to_whole_set = matches!(
        &effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Characteristics(
                crate::cards::builders::CharacteristicActionAst::BecomeBasicLandTypeChoice { .. }
            ),
            ..
        })
    );
    if let Some(filter) = for_each_subject_filter
        && !choice_applies_to_whole_set
    {
        if matches!(verb, Verb::Deal) {
            bind_quantified_damage_actor(&mut effect, span_from_tokens(subject_tokens));
        }
        if let EffectAst::SubjectVerb(subject_verb) = &mut effect
            && let SubjectVerbActionAst::StatChanges(
                crate::cards::builders::StatChangeActionAst::Pump { target, .. },
            ) = &mut subject_verb.action
            && matches!(target, TargetAst::Source(_))
        {
            *target = TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(subject_tokens),
            );
        }
        effect = EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        });
    }
    if each_other_player {
        effect = EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
            filter: PlayerFilter::NotYou,
            effects: vec![effect],
        });
    }
    Ok(effect)
}

/// The grammatical actor of "each [object] deals ..." remains the damage
/// source inside recipient fanout. The superficially similar "for each
/// [object], this spell deals ..." is parsed through a different scope and
/// must retain the spell as source. Already explicit damage sources are not
/// rebound by this helper.
fn bind_quantified_damage_actor(effect: &mut EffectAst, span: Option<TextSpan>) {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(action),
        ..
    }) = effect
        && let DamageActionAst::DealDamage {
            amount,
            target,
            unpreventable,
        } = action
    {
        *action = DamageActionAst::DealDamageEqualToPower {
            source: TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span),
            amount: amount.clone(),
            target: target.clone(),
            unpreventable: *unpreventable,
        };
        return;
    }
    // "Each creature you control deals damage equal to its power to each
    // opponent" (Lukka, Coppercoat Outcast): the per-opponent fanout already
    // built the power-damage action with the quantified subject as a fresh
    // (non-target) object source. Inside the per-object loop the source is
    // the iterated object, not a new choice among the subject set.
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { source, .. }),
        ..
    }) = effect
        && matches!(source, TargetAst::Object(filter, None, _) if !filter.source)
    {
        *source = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span);
        return;
    }
    crate::model::visit::for_each_nested_effects_mut(effect, false, |effects| {
        for child in effects {
            bind_quantified_damage_actor(child, span);
        }
    });
}

pub(super) fn parse_passive_goad_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_passive_goad_shape(tokens) else {
        return Ok(None);
    };
    let target = match shape.target {
        clause_grammar::GoadTargetShape::TaggedToken => TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(tokens),
        ),
        clause_grammar::GoadTargetShape::Target(target_tokens) => {
            parse_target_phrase(target_tokens)?
        }
    };
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    if shape.no_longer {
        return Ok(Some(EffectAst::subject_verb_clear_goad(Some(target))));
    }
    let duration = if shape.for_rest_of_game {
        Until::Forever
    } else {
        Until::YourNextTurn
    };
    Ok(Some(EffectAst::subject_verb_goad_for(target, duration)))
}

pub fn parse_effect_clause_lexed(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_effect_clause(tokens)
}

/// "The tokens are goaded for as long as this Saga remains on the
/// battlefield." (The War Games): the definite description names the tokens
/// just created, never every token on the battlefield, and the goad lasts
/// for the source-lifetime duration (CR 701.15, 611.2b).
pub(crate) fn parse_referenced_tokens_goaded_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let subject_len = match words.as_slice() {
        ["the" | "that", "token", "is", crate::keyword_forms::GOADED, ..] => 4,
        ["the" | "those", "tokens", "are", crate::keyword_forms::GOADED, ..] => 4,
        _ => return None,
    };
    let goaded = tokens
        .iter()
        .position(|token| token.is_word(crate::keyword_forms::GOADED))?;
    let tail = crate::lexer::trim_lexed_commas(&tokens[goaded + 1..]);
    let tail_words = &words[subject_len..];
    // Undurationed and rest-of-game forms keep their existing readings.
    if tail_words.is_empty() {
        return None;
    }
    let duration = crate::grammar::effects::parse_source_lifetime_duration(tail)?;
    let it = crate::cards::builders::TargetAst::Tagged(
        crate::tag::CompilerReferenceTag::It.bind(),
        None,
    );
    Some(EffectAst::subject_verb_goad_for(it, duration))
}
