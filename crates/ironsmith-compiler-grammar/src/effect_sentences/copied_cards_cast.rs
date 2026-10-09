//! "Copy that card three times. You may cast the copies without paying their
//! mana costs." (Mnemonic Deluge) / "Then copy each card exiled with this
//! enchantment. You may cast any number of the copies without paying their
//! mana costs." (Arcane Bombardment) / "Copy them. You may cast any number of
//! the copies." (The Tale of Tamiyo).
//! Copying a card makes a copy that exists only to be cast (CR 707.12): each
//! copy is cast through the tagged-cast copy mode, once per copy made, and
//! each cast is optional. The single-copy form ("You may cast the copy") stays
//! with the copy-cast procedure; this reader owns the plural "copies".
use crate::cards::builders::{
    CardTextError, EffectAst, ForEachEffectAst, OwnedLexToken, PermissionEffectAst, PlayerAst,
};
use crate::lexer::parser_token_word_refs;
use crate::tag::{CompilerReferenceTag, TagRef};

/// The copied cards and how many copies each gets.
fn copy_statement(sentence: &[OwnedLexToken]) -> Option<(TagRef, u32)> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let words = parser_token_word_refs(sentence);
    let words = words.strip_prefix(&["then"]).unwrap_or(&words);
    let rest = words.strip_prefix(&["copy"])?;
    let (tag, rest) = if let Some(rest) = rest
        .strip_prefix(&["that", "card"])
        .or_else(|| rest.strip_prefix(&["it"]))
        .or_else(|| rest.strip_prefix(&["them"]))
        .or_else(|| rest.strip_prefix(&["those", "cards"]))
        .or_else(|| rest.strip_prefix(&["those", "exiled", "cards"]))
    {
        (CompilerReferenceTag::It.bind(), rest)
    } else {
        let source = rest.strip_prefix(&["each", "card", "exiled", "with"])?;
        if !crate::util::is_source_reference_words(source) {
            return None;
        }
        (CompilerReferenceTag::SourceExiled.bind(), &[][..])
    };
    let times = match rest {
        [] => 1,
        ["twice"] => 2,
        [count, "times"] => crate::util::parse_number_word_u32(count)?,
        _ => return None,
    };
    (1..=8).contains(&times).then_some((tag, times))
}

/// "You may cast [any number of] the copies [without paying their mana
/// costs]" → whether the casts are free.
fn cast_statement(sentence: &[OwnedLexToken]) -> Option<bool> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let words = parser_token_word_refs(sentence);
    let rest = words.strip_prefix(&["you", "may", "cast"])?;
    let rest = rest.strip_prefix(&["any", "number", "of"]).unwrap_or(rest);
    let rest = rest.strip_prefix(&["the", "copies"])?;
    match rest {
        [] => Some(false),
        ["without", "paying", "their", "mana", "costs"] => Some(true),
        _ => None,
    }
}

/// "Choose an instant or sorcery card exiled this way and copy it three
/// times." (Chandra, Pyromaster): one card chosen from the cards just exiled.
fn choose_then_copy(
    sentence: &[OwnedLexToken],
) -> Result<Option<(EffectAst, TagRef, u32)>, CardTextError> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    if !sentence.first().is_some_and(|token| token.is_word("choose")) {
        return Ok(None);
    }
    let Some(and) = (1..sentence.len().saturating_sub(1))
        .find(|&index| sentence[index].is_word("and") && sentence[index + 1].is_word("copy"))
    else {
        return Ok(None);
    };
    let Some((_, times)) = copy_statement(&sentence[and + 1..]) else {
        return Ok(None);
    };
    let described = &sentence[1..and];
    let words = parser_token_word_refs(described);
    let Some(noun_words) = words.len().checked_sub(3).filter(|_| {
        words.ends_with(&["exiled", "this", "way"])
    }) else {
        return Ok(None);
    };
    let noun_tokens = &described[..described.len() - 3];
    let noun_tokens = match noun_tokens.first() {
        Some(token) if token.is_any_word(&["a", "an"]) => &noun_tokens[1..],
        _ => noun_tokens,
    };
    if noun_words == 0 || noun_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = crate::object_filters::parse_object_filter(noun_tokens, false)?;
    filter.zone = Some(crate::zone::Zone::Exile);
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: CompilerReferenceTag::It.bind().into(),
        relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
    });
    filter.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Exiled));
    let chosen = crate::util::helper_tag_for_tokens(sentence, "copied_choice");
    Ok(Some((
        EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: crate::effect::ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen.clone(),
        }),
        chosen,
        times,
    )))
}

fn copy_cast(free: bool) -> EffectAst {
    EffectAst::Permissions(PermissionEffectAst::May {
        effects: vec![EffectAst::subject_verb_cast_tagged(
            CompilerReferenceTag::It.bind(),
            PlayerAst::You,
            false,
            true,
            free,
            None,
        )],
    })
}

/// "copy each exiled card [you own] <qualifier>" → the exiled-card filter.
fn described_exiled_pool(
    sentence: &[OwnedLexToken],
) -> Result<Option<crate::target::ObjectFilter>, CardTextError> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let words = parser_token_word_refs(sentence);
    if !words.starts_with(&["copy", "each", "exiled", "card"]) || words.len() < 5 {
        return Ok(None);
    }
    // Tokens after "copy each exiled": "card you own with a kick counter on it".
    let described = &sentence[3..];
    let mut filter = crate::object_filters::parse_object_filter(described, false)?;
    filter.zone = Some(crate::zone::Zone::Exile);
    Ok(Some(filter))
}

pub(crate) fn read(
    copy: &[OwnedLexToken],
    cast: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "..., then exile up to that many target instant and/or sorcery cards
    // ... and copy them." (Bloodthirsty Adversary): the copy closes a longer
    // instruction. The lead is read as its own sentence; when it is a result
    // conditional ("When you pay this cost one or more times, ..."), the copy
    // casts belong inside that result.
    if copy_statement(copy).is_none()
        && choose_then_copy(copy)?.is_none()
        && let Some(and) = (1..copy.len()).rev().find(|&index| {
            copy[index].is_word("and")
                && copy.get(index + 1).is_some_and(|token| token.is_word("copy"))
        })
        && copy_statement(&copy[and + 1..]).is_some()
    {
        let Some(pair) = read(&copy[and + 1..], cast)? else {
            return Ok(None);
        };
        let lead = crate::util::trim_commas(&copy[..and]);
        let mut effects = super::parse_effect_sentence_lexed(&lead)?;
        match effects.last_mut() {
            Some(EffectAst::Conditionals(
                crate::cards::builders::ConditionalEffectAst::WhenResult { effects: inner, .. }
                | crate::cards::builders::ConditionalEffectAst::IfResult { effects: inner, .. },
            )) => inner.extend(pair),
            _ => effects.extend(pair),
        }
        return Ok(Some(effects));
    }
    // "Copy each exiled card you own with a kick counter on it." (Zethi): a
    // described pool of exiled cards, iterated by filter.
    if let Some(filter) = described_exiled_pool(copy)? {
        let Some(free) = cast_statement(cast) else {
            return Ok(None);
        };
        return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter,
            effects: vec![copy_cast(free)],
        })]));
    }
    let (choice, (tag, times)) = if let Some((choice, tag, times)) = choose_then_copy(copy)? {
        (Some(choice), (tag, times))
    } else {
        let Some(statement) = copy_statement(copy) else {
            return Ok(None);
        };
        (None, statement)
    };
    let Some(free) = cast_statement(cast) else {
        return Ok(None);
    };
    let one_cast = copy_cast(free);
    let mut effects: Vec<EffectAst> = choice.into_iter().collect();
    let casts = vec![one_cast; times as usize];
    if tag.as_str() == CompilerReferenceTag::SourceExiled.as_str() {
        // "each card exiled with this enchantment" is the whole linked set
        // (CR 607.2a). Within this resolution the source-exiled TAG names
        // only what this resolution exiled; a filter over the link set reads
        // every card ever exiled with the source.
        let mut linked = crate::target::ObjectFilter::default().in_zone(crate::zone::Zone::Exile);
        linked.tagged_constraints.push(crate::target::TaggedObjectConstraint {
            tag: tag.key.clone(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
        effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter: linked,
            effects: casts,
        }));
    } else {
        effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag, effects: casts }));
    }
    Ok(Some(effects))
}

/// "You may copy an instant or sorcery card in it. If you do, you may cast
/// the copy without paying its mana cost." (Reversal of Fortune, after a
/// revealed hand): choosing the card is the optional copy; casting the copy
/// is a second, independent option (CR 707.12).
pub(crate) fn read_optional_copy_from_revealed(
    copy: &[OwnedLexToken],
    cast: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let copy = crate::util::trim_edge_punctuation_tokens(copy);
    let words = parser_token_word_refs(copy);
    if !words.starts_with(&["you", "may", "copy"]) || !words.ends_with(&["in", "it"]) {
        return Ok(None);
    }
    let cast_words =
        parser_token_word_refs(crate::util::trim_edge_punctuation_tokens(cast));
    let free = match cast_words.as_slice() {
        ["if", "you", "do", "you", "may", "cast", "the", "copy"] => false,
        [
            "if", "you", "do", "you", "may", "cast", "the", "copy", "without", "paying", "its",
            "mana", "cost",
        ] => true,
        _ => return Ok(None),
    };
    // Read the described card as the existing "you choose <card> from it"
    // clause over the revealed hand.
    let described = &copy[3..copy.len() - 2];
    let mut choose = crate::lexer::synthetic_word_tokens(["you", "choose"]);
    choose.extend_from_slice(described);
    choose.extend(crate::lexer::synthetic_word_tokens(["from", "it"]));
    let Some((_, filter, _, _)) =
        crate::activation_and_restrictions::parse_you_choose_objects_clause_with_count_value(
            &choose,
        )?
    else {
        return Ok(None);
    };
    let chosen = crate::util::helper_tag_for_tokens(copy, "copied_from_hand");
    Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::May {
        effects: vec![
            EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
                filter,
                count: crate::effect::ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            }),
            EffectAst::Permissions(PermissionEffectAst::May {
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    chosen,
                    PlayerAst::You,
                    false,
                    true,
                    free,
                    None,
                )],
            }),
        ],
    })]))
}
