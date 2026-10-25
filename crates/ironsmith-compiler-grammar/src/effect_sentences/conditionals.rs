use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::{
    CounterSpellConditionalKind, ForEachPlayerKind, split_for_each_opponent_doesnt_clause_lexed,
    split_for_each_player_doesnt_clause_lexed, split_negated_who_this_way_filter_tokens_lexed,
};
use super::super::grammar::values as shared_values;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    parse_card_type, parse_subtype_word as parse_shared_subtype_word,
    parse_supertype_word as parse_shared_supertype_word, parse_target_phrase, span_from_tokens,
    trim_commas,
};
use super::parse_effect_chain_inner;
#[cfg(test)]
use super::parse_effect_chain_lexed;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, PlayerAst, PlayerPredicateAst, PredicateAst,
    TagKey, TargetAst,
};
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

#[cfg(test)]
pub fn parse_conditional_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    super::super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed(
        tokens,
        parse_effect_chain_lexed,
    )
}

pub fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    shared_values::parse_scryfall_mana_cost(raw)
}

pub fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    shared_values::parse_mana_symbol_group(raw)
}

pub fn parse_type_line(
    raw: &str,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    let (supertypes, card_types, mut subtypes) = shared_values::parse_type_line_with(
        raw,
        parse_supertype_word,
        |word| {
            let lower = word.to_ascii_lowercase();
            // CR 309.1 / 311.1 / 312.1 / 313.1 / 314.1 / 315.2: the
            // nontraditional card types are card types only on a printed type
            // line; in rules text these words name the objects, not filters.
            parse_card_type(&lower).or_else(|| match lower.as_str() {
                "dungeon" => Some(crate::types::CardType::Dungeon),
                "plane" => Some(crate::types::CardType::Plane),
                "phenomenon" => Some(crate::types::CardType::Phenomenon),
                "vanguard" => Some(crate::types::CardType::Vanguard),
                "scheme" => Some(crate::types::CardType::Scheme),
                "conspiracy" => Some(crate::types::CardType::Conspiracy),
                _ => None,
            })
        },
        parse_type_line_subtype_word,
    )?;
    // CR 311.3: all words after the dash of a plane's type line are,
    // collectively, a single planar type. CR 205.3p: Undercity is the one
    // dungeon type.
    let front_face = raw.split("//").next().unwrap_or(raw);
    let subtype_text = front_face
        .split_once('—')
        .or_else(|| front_face.split_once(" - "))
        .map(|(_, rest)| rest.trim());
    if let Some(subtype_text) = subtype_text.filter(|text| !text.is_empty()) {
        if card_types.contains(&crate::types::CardType::Plane)
            && let Some(planar) = Subtype::planar_type_from_name(subtype_text)
        {
            subtypes = vec![planar];
        } else if card_types.contains(&crate::types::CardType::Dungeon)
            && subtype_text.eq_ignore_ascii_case("Undercity")
        {
            subtypes = vec![Subtype::Undercity];
        }
    }
    Ok((supertypes, card_types, subtypes))
}

/// A printed type line is an unambiguous subtype context, so it reads the full
/// subtype vocabulary. The rules-text classifier behind `parse_subtype_word`
/// rejects subtypes that double as English nouns (`Mine`, `Tower`, `Sphere`,
/// `Omen`, ...), which would silently drop them from the card's own type line
/// and break every "if you control an Urza's Mine" check against it.
fn parse_type_line_subtype_word(word: &str) -> Option<Subtype> {
    super::super::grammar::leaf::classify_token_definition_subtype(word)
}

pub fn parse_supertype_word(word: &str) -> Option<Supertype> {
    parse_shared_supertype_word(word)
}

pub fn parse_subtype_word(word: &str) -> Option<Subtype> {
    parse_shared_subtype_word(word)
}

pub fn parse_for_each_opponent_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, true)? {
        return Ok(Some(effect));
    }
    let Some(split) = split_for_each_opponent_doesnt_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each opponent who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(split.effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(split.inner_tokens)?;
    Ok(Some(EffectAst::ForEach(
        ForEachEffectAst::ForEachOpponentDoesNot { effects, predicate },
    )))
}

pub fn parse_for_each_player_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, false)? {
        return Ok(Some(effect));
    }
    let Some(split) = split_for_each_player_doesnt_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each player who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(split.effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(split.inner_tokens)?;
    Ok(Some(EffectAst::ForEach(
        ForEachEffectAst::ForEachPlayerDoesNot { effects, predicate },
    )))
}

pub fn parse_for_each_doesnt_control_lose_game(
    tokens: &[OwnedLexToken],
    opponent: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_for_each_no_control_lose_game_tokens(tokens) else {
        return Ok(None);
    };
    let expected_kind = if opponent {
        ForEachPlayerKind::Opponent
    } else {
        ForEachPlayerKind::Player
    };
    if shape.player_kind != expected_kind {
        return Ok(None);
    }
    let filter_tokens = trim_commas(shape.filter_tokens);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false)?;

    let effect = EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::Player(PlayerPredicateAst::PlayerControlsNo {
            player: PlayerAst::That,
            filter,
        }),
        if_true: vec![EffectAst::subject_verb_lose_game(PlayerAst::That)],
        if_false: Vec::new(),
    });

    Ok(Some(if opponent {
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![effect],
        })
    } else {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![effect],
        })
    }))
}

fn parse_negated_who_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(filter_tokens) = split_negated_who_this_way_filter_tokens_lexed(inner_tokens) else {
        return Ok(None);
    };

    let mut filter = match parse_object_filter(filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };
    // A "card" noun names a card, never a permanent on the battlefield.
    if filter.zone == Some(crate::zone::Zone::Battlefield)
        && filter_tokens
            .iter()
            .any(|token| token.is_word("card") || token.is_word("cards"))
    {
        filter.zone = None;
    }

    Ok(Some(PredicateAst::Player(
        PlayerPredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::That,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            filter,
            mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
        },
    )))
}

pub fn parse_sentence_counter_target_spell_if_it_was_kicked(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_counter_spell_conditional_tokens(tokens) else {
        return Ok(None);
    };
    if shape.kind != CounterSpellConditionalKind::IfKicked {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(shape.target_tokens));
    let counter = EffectAst::subject_verb_counter(target);
    let effect = EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::TargetWasKicked,
        if_true: vec![counter],
        if_false: Vec::new(),
    });
    Ok(Some(vec![effect]))
}

pub fn parse_sentence_counter_target_spell_thats_second_cast_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_counter_spell_conditional_tokens(tokens) else {
        return Ok(None);
    };
    if shape.kind != CounterSpellConditionalKind::SecondCastThisTurn {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(shape.target_tokens));
    let counter = EffectAst::subject_verb_counter(target);
    let effect = EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::TargetSpellCastOrderThisTurn(2),
        if_true: vec![counter],
        if_false: Vec::new(),
    });
    Ok(Some(vec![effect]))
}

pub fn parse_sentence_exile_target_creature_with_greatest_power(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_exile_greatest_power_creature_tokens(tokens) else {
        return Ok(None);
    };

    let target_tokens = trim_commas(shape.target_tokens);
    let target = parse_target_phrase(&target_tokens)?;
    let exile = EffectAst::subject_verb_exile(target.clone(), false);
    let effect = EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::TargetHasGreatestPowerAmongCreatures,
        if_true: vec![exile],
        if_false: Vec::new(),
    });
    Ok(Some(vec![effect]))
}
