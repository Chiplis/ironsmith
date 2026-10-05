//! Named grammar owner for local coin instructions and their receipt consumers.
//! Recognition retains the semantic distinction between faces and wins/losses.
use super::*;
use ironsmith_core::{CoinFlipKind, EffectMetric, EffectMetricSource, PriorEffectAction, PriorEffectMetricQuery};

fn metric_value(metric: EffectMetric) -> Value {
    Value::PendingPriorEffectMetric(
        PriorEffectMetricQuery::new(EffectMetricSource::Outcome, metric)
            .with_action(PriorEffectAction::Flipped),
    )
}

// Word views locate a candidate; the owned clause must also account for
// every punctuation and symbol token before it may become typed semantics.
fn bare_words(tokens: &[OwnedLexToken], trailing: TokenKind) -> bool {
    let tokens = if tokens.last().is_some_and(|token| token.kind == trailing) {
        &tokens[..tokens.len() - 1]
    } else { tokens };
    !tokens.is_empty() && tokens.iter().all(|token| matches!(token.kind, TokenKind::Word | TokenKind::Number))
}

fn malformed_coin_clause() -> CardTextError {
    CardTextError::ParseError("unexpected symbol or punctuation in coin instruction".into())
}

fn flip_instruction(tokens: &[OwnedLexToken], kind: CoinFlipKind) -> Option<EffectAst> {
    if !bare_words(tokens, TokenKind::Period) { return None; }
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let (start, optional) = if words.starts_with(&["you", "may", "flip"]) { (2, true) }
        else if words.starts_with(&["you", "flip"]) { (1, false) }
        else if words.starts_with(&["flip"]) { (0, false) }
        else { return None };
    let rest = &words[start + 1..];
    let count_value = (rest == ["that", "many", "coins"]).then(|| Value::PendingPriorEffectMetric(
        PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count)
            .with_action(PriorEffectAction::ChosenNumber),
    ));
    let (count, repeat_until_loss, opponent_results) = if rest == ["a", "coin", "until", "you", "lose", "a", "flip"] {
        (1, true, None)
    } else if rest == ["a", "coin", "for", "each", "opponent", "you", "have"] {
        (0, false, Some((
            crate::util::helper_tag_for_tokens(tokens, "coin_opponents_won"),
            crate::util::helper_tag_for_tokens(tokens, "coin_opponents_lost"),
        )))
    } else if count_value.is_some() {
        (0, false, None)
    } else {
        let count_token = view.map_word_to_token_start(start + 1)?;
        let number = crate::grammar::leaf::parse_leaf_number_prefix_tokens(&tokens[count_token..])?;
        let (count, consumed) = number.into_fixed()?;
        if crate::lexer::TokenWordView::new(&tokens[count_token + consumed..]).word_refs() != ["coins"] { return None; }
        (count, false, None)
    };
    let flip = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
        SubjectVerbActionAst::Random(RandomActionAst::FlipCoins {
            count, kind: if repeat_until_loss { CoinFlipKind::Called } else { kind }, repeat_until_loss, opponent_results, count_value,
        }),
    );
    Some(if optional { EffectAst::Permissions(PermissionEffectAst::May { effects: vec![flip] }) } else { flip })
}

fn prefix_metric(words: &[&str]) -> Option<(usize, EffectMetric)> {
    for (prefix, metric) in [
        (&["for", "each", "flip", "you", "win"][..], EffectMetric::CoinFlipsWon),
        (&["for", "each", "flip", "you", "won"][..], EffectMetric::CoinFlipsWon),
        (&["for", "each", "flip", "you", "lose"][..], EffectMetric::CoinFlipsLost),
        (&["for", "each", "flip", "you", "lost"][..], EffectMetric::CoinFlipsLost),
    ] {
        if words.starts_with(prefix) { return Some((prefix.len(), metric)); }
    }
    None
}

fn suffix_metric(words: &[&str]) -> Option<(usize, EffectMetric)> {
    for (suffix, metric) in [
        (&["for", "each", "flip", "you", "won"][..], EffectMetric::CoinFlipsWon),
        (&["for", "each", "flip", "you", "win"][..], EffectMetric::CoinFlipsWon),
        (&["for", "each", "coin", "that", "comes", "up", "heads"][..], EffectMetric::CoinHeads),
        (&["for", "each", "coin", "that", "comes", "up", "tails"][..], EffectMetric::CoinTails),
    ] {
        if words.ends_with(suffix) { return Some((words.len() - suffix.len(), metric)); }
    }
    None
}

fn quantified_effect(mut effects: Vec<EffectAst>, metric: EffectMetric) -> Vec<EffectAst> {
    // Putting N counters or creating N tokens is one event batch, so preserve
    // the atomic operation. RepeatEffects is appropriate for extra turns and
    // instructions which have no simultaneous count slot.
    fn scale(effect: &mut EffectAst, metric: EffectMetric) -> bool {
        if let EffectAst::SubjectVerb(subject) = effect {
            let count = match &mut subject.action {
                SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count, .. })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { count, .. })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { count, .. }) => Some(count),
                _ => None,
            };
            if let Some(count) = count {
                if count.unhinted() == &Value::Fixed(1) { *count = metric_value(metric); return true; }
            }
            return false;
        }
        let mut scaled = false;
        crate::model::visit::for_each_nested_effects_mut(effect, false, |nested| {
            if let [only] = nested { scaled |= scale(only, metric); }
        });
        scaled
    }
    if let [only] = effects.as_mut_slice() {
        if scale(only, metric) { return effects; }
    }
    vec![EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count: metric_value(metric), effects })]
}

fn receipt_consumer(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    if let Some((word, metric)) = prefix_metric(&words) {
        let start = view.map_word_to_token_start(word).ok_or_else(|| CardTextError::ParseError("coin count has no following action".into()))?;
        if !bare_words(&tokens[..start], TokenKind::Comma) { return Err(malformed_coin_clause()); }
        let effects = parse_effect_sentences_lexed(&tokens[start..])?;
        return Ok(Some(quantified_effect(effects, metric)));
    }
    if let Some((word, metric)) = suffix_metric(&words) {
        let end = view.map_word_to_token_start(word).expect("matched suffix has tokens");
        if !bare_words(&tokens[end..], TokenKind::Period) { return Err(malformed_coin_clause()); }
        let effects = parse_effect_sentences_lexed(&tokens[..end])?;
        return Ok(Some(quantified_effect(effects, metric)));
    }
    if words.starts_with(&["if", "you", "won"])
        && words.get(4..7).is_some_and(|tail| tail == ["flips", "this", "way"])
    {
        let number_start = view.map_word_to_token_start(3).ok_or_else(malformed_coin_clause)?;
        let Some((count, consumed)) = crate::grammar::leaf::parse_leaf_number_prefix_tokens(&tokens[number_start..])
            .and_then(|number| number.into_fixed()) else { return Err(malformed_coin_clause()); };
        let flips_start = view.map_word_to_token_start(4).ok_or_else(malformed_coin_clause)?;
        if number_start + consumed != flips_start || count > i32::MAX as u32 { return Err(malformed_coin_clause()); }
        let start = view.map_word_to_token_start(7).ok_or_else(malformed_coin_clause)?;
        if !bare_words(&tokens[..start], TokenKind::Comma) { return Err(malformed_coin_clause()); }
        return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: metric_value(EffectMetric::CoinFlipsWon), operator: ironsmith_core::ValueComparisonOperator::Equal,
                right: Value::Fixed(count as i32),
            },
            if_true: parse_effect_sentences_lexed(&tokens[start..])?, if_false: Vec::new(),
        })]));
    }
    let metric = if words.starts_with(&["if", "both", "coins", "come", "up", "heads"]) { Some(EffectMetric::CoinHeads) }
        else if words.starts_with(&["if", "both", "coins", "come", "up", "tails"]) { Some(EffectMetric::CoinTails) }
        else { None };
    if let Some(metric) = metric {
        let start = view.map_word_to_token_start(6).ok_or_else(|| CardTextError::ParseError("coin predicate has no following action".into()))?;
        if !bare_words(&tokens[..start], TokenKind::Comma) { return Err(malformed_coin_clause()); }
        return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: metric_value(metric), operator: ironsmith_core::ValueComparisonOperator::Equal,
                right: Value::Fixed(2),
            },
            if_true: parse_effect_sentences_lexed(&tokens[start..])?, if_false: Vec::new(),
        })]));
    }
    Ok(None)
}

pub(super) fn parse_document(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    // Face-only is proved by this instruction's own consumers. Stop at the
    // next coin instruction so another instruction cannot change its kind.
    let mut parsed = Vec::new();
    let mut recognized = false;
    let mut opponent_loss_tag = None;
    for (index, sentence) in sentences.iter().enumerate() {
        let words = crate::lexer::TokenWordView::new(sentence).word_refs();
        let coin_head = words.starts_with(&["flip"])
            || words.starts_with(&["you", "flip"])
            || words.starts_with(&["you", "may", "flip"]);
        if coin_head && (words.contains(&"coins") || words.contains(&"coin"))
            && !bare_words(sentence, TokenKind::Period)
        { return Err(malformed_coin_clause()); }
        let mut kind = CoinFlipKind::Called;
        let mut observes_winner = false;
        for following in sentences.iter().skip(index + 1) {
            if flip_instruction(following, CoinFlipKind::Called).is_some() { break; }
            let words = crate::lexer::TokenWordView::new(following).word_refs();
            observes_winner |= prefix_metric(&words).is_some()
                || suffix_metric(&words).is_some_and(|(_, metric)| matches!(metric, EffectMetric::CoinFlipsWon | EffectMetric::CoinFlipsLost))
                || words.starts_with(&["if", "you", "win"])
                || words.starts_with(&["if", "you", "lose"])
                || words.starts_with(&["if", "you", "won"]);
            if suffix_metric(&words).is_some_and(|(_, metric)| matches!(metric, EffectMetric::CoinHeads | EffectMetric::CoinTails))
                || words.starts_with(&["if", "both", "coins", "come", "up"])
            { kind = CoinFlipKind::FaceOnly; }
        }
        if observes_winner { kind = CoinFlipKind::Called; }
        let effects = if let Some(flip) = flip_instruction(sentence, kind) {
            opponent_loss_tag = match &flip {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Random(RandomActionAst::FlipCoins { opponent_results: Some((_, lost)), .. }), ..
                }) => Some(lost.clone()),
                _ => None,
            };
            Some(vec![flip])
        } else if let Some(tag) = opponent_loss_tag.clone()
            && prefix_metric(&words).is_some_and(|(_, metric)| metric == EffectMetric::CoinFlipsLost)
        {
            let view = crate::lexer::TokenWordView::new(sentence);
            let start = view.map_word_to_token_start(5).ok_or_else(malformed_coin_clause)?;
            if !bare_words(&sentence[..start], TokenKind::Comma) { return Err(malformed_coin_clause()); }
            Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer {
                require_evidence: true,
                tag, effects: parse_effect_sentences_lexed(&sentence[start..])?,
            })])
        } else {
            if coin_head { opponent_loss_tag = None; }
            receipt_consumer(sentence)?
        };
        recognized |= effects.is_some();
        parsed.push(effects);
    }
    if !recognized { return Ok(None); }
    let mut effects = Vec::new();
    for (sentence, parsed) in sentences.iter().zip(parsed) {
        let words = crate::lexer::TokenWordView::new(sentence).word_refs();
        let token_followup = words == ["those", "tokens", "gain", "haste"]
            || words == ["exile", "them", "at", "the", "beginning", "of", "the", "next", "end", "step"];
        if parsed.is_none() && token_followup {
            if !bare_words(sentence, TokenKind::Period) { return Err(malformed_coin_clause()); }
            if let Some(followup) = parse_token_copy_modifier_sentence(sentence)
                && try_apply_token_copy_followup(&mut effects, followup)?
            {
                continue;
            }
        }
        effects.extend(match parsed { Some(effects) => effects, None => parse_effect_sentences_lexed(sentence)? });
    }
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<EffectAst> {
        parse_document(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().unwrap()
    }
    fn flip(effects: &[EffectAst]) -> (u32, CoinFlipKind, bool) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Random(RandomActionAst::FlipCoins { count, kind, repeat_until_loss, .. }), ..
        }) = &effects[0] else { panic!("expected a grouped flip: {effects:?}") };
        (*count, *kind, *repeat_until_loss)
    }

    #[test]
    fn fixed_faces_and_called_results_are_distinct_typed_producers() {
        assert_eq!(flip(&parse("Flip five coins. Take an extra turn after this one for each coin that comes up heads.")), (5, CoinFlipKind::FaceOnly, false));
        assert_eq!(flip(&parse("Flip two coins. If both coins come up heads, draw a card.")), (2, CoinFlipKind::FaceOnly, false));
        assert_eq!(flip(&parse("Flip three coins. For each flip you win, draw a card.")), (3, CoinFlipKind::Called, false));
        assert_eq!(flip(&parse("Flip a coin until you lose a flip.")), (1, CoinFlipKind::Called, true));
    }

    #[test]
    fn fixed_group_kind_does_not_leak_from_a_later_coin_instruction() {
        let parsed = parse("Flip two coins. For each flip you win, draw a card. Flip five coins. Take an extra turn after this one for each coin that comes up heads.");
        assert_eq!(flip(&parsed), (2, CoinFlipKind::Called, false));
    }

    #[test]
    fn a_winner_consumer_keeps_calls_even_when_the_same_batch_also_counts_faces() {
        assert_eq!(flip(&parse("Flip two coins. For each flip you win, draw a card. Take an extra turn after this one for each coin that comes up heads.")), (2, CoinFlipKind::Called, false));
    }

    #[test]
    fn counters_and_tokens_keep_one_atomic_action_with_a_receipt_count() {
        for text in [
            "Flip a coin until you lose a flip. Put a +1/+1 counter on this creature for each flip you won.",
            "Flip three coins. For each flip you win, create a 1/1 red Goblin creature token that's tapped and attacking.",
        ] {
            let parsed = parse(text);
            let debug = format!("{parsed:?}");
            assert!(debug.contains("CoinFlipsWon"), "{debug}");
            assert!(!debug.contains("RepeatEffects"), "{debug}");
        }
    }

    #[test]
    fn extra_unknown_tail_is_never_accepted_as_the_known_coin_instruction() {
        let tokens = crate::lexer::lex_line("Flip two coins and quietly ignore unknown instructions.", 0).unwrap();
        assert!(flip_instruction(&tokens, CoinFlipKind::Called).is_none());
        let tokens = crate::lexer::lex_line("Flip a coin until you lose a flip and draw seventeen moons.", 0).unwrap();
        assert!(flip_instruction(&tokens, CoinFlipKind::Called).is_none());
    }
}

#[cfg(test)]
mod dynamic_tests {
    use super::*;
    #[test]
    fn chosen_count_and_five_win_gate_keep_distinct_typed_receipts() {
        let tokens = crate::lexer::lex_line("Choose a number between 1 and 5. Flip that many coins. For each flip you win, draw a card. If you won five flips this way, you may cast spells from your hand this turn without paying their mana costs.", 0).unwrap();
        let effects = parse_document(&tokens).unwrap().unwrap();
        let debug = format!("{effects:?}");
        assert!(debug.contains("ChosenNumber"), "{debug}");
        assert!(debug.contains("CoinFlipsWon"), "{debug}");
        assert!(debug.contains("GrantBySpec"), "{debug}");
        assert!(!debug.contains("MayCastMatchingSpellWithoutPayingManaCost"), "the reward grants a duration, not an immediate cast");
    }
}
