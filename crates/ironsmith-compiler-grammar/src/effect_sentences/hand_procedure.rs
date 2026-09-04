//! Revealed- and looked-hand procedures composed statement by statement.
//!
//! "Target opponent reveals their hand. You choose a nonland card from it or a
//! card from their graveyard." is a reveal statement that binds the revealed
//! hand, followed by a statement over it. The reveal (or "look at that player's
//! hand") is the ordinary sentence grammar's; this module carries the hand's
//! owner to the sentences that follow, as [`super::looked_procedure`] carries a
//! viewed group: a choice from it, a cast from among those cards, a draw for
//! each matching card in it, or exiling every noncreature, nonland card from
//! that hand and graveyard.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ObjectFilter, PlayerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TargetAst, Value,
};
use crate::target::{PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

/// How the hand was shown.
enum Shown {
    /// "Target player/opponent reveals their hand."
    Revealed(PlayerAst),
    /// "Look at that player's hand." — whose hand, as the look names them.
    Looked(PlayerFilter),
}

pub(super) struct HandGroup {
    effects: Vec<EffectAst>,
    shown: Shown,
    closed: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn shown_hand(sentence: &SentenceInput) -> Option<(EffectAst, Shown)> {
    if let Ok(Some(look_effects)) = super::parse_look_at_hand_sentence(sentence.lexed())
        && let [look_effect] = look_effects.as_slice()
        && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtHand {
                    target: TargetAst::Player(hand_owner, _),
                },
            ..
        }) = look_effect
        && matches!(
            hand_owner,
            PlayerFilter::DamagedPlayer
                | PlayerFilter::IteratedPlayer
                | PlayerFilter::Target(_)
                | PlayerFilter::AliasedTarget(_)
        )
    {
        return Some((look_effect.clone(), Shown::Looked(hand_owner.clone())));
    }
    let effects =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(sentence.lowered()))?;
    let [effect] = effects.as_slice() else {
        return None;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::RevealHand,
    }) = effect
    else {
        return None;
    };
    matches!(player, PlayerAst::Target | PlayerAst::TargetOpponent)
        .then(|| (effect.clone(), Shown::Revealed(*player)))
}

/// "You choose a nonland card from it or a card from their graveyard."
fn choose_from_it_or_graveyard(sentence: &SentenceInput) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    if !crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &[
                "you", "choose", "a", "nonland", "card", "from", "it", "or", "a", "card", "from",
                "their", "graveyard",
            ],
            &[
                "you", "choose", "a", "nonland", "card", "from", "it", "or", "a", "card", "from",
                "that", "players", "graveyard",
            ],
        ],
    ) {
        return None;
    }
    let mut hand = ObjectFilter::default();
    hand.zone = Some(Zone::Hand);
    hand.excluded_card_types = vec![CardType::Land];
    hand.tagged_constraints.push(TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::It.key(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let mut graveyard = ObjectFilter::default();
    graveyard.zone = Some(Zone::Graveyard);
    graveyard.owner = Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)));
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![hand, graveyard];
    Some(EffectAst::ChooseObjects {
        filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::You,
        tag: crate::tag::CompilerReferenceTag::It.key(),
    })
}

/// "You may cast an instant or sorcery spell/card from among those cards
/// without paying its mana cost." over a revealed opponent's hand.
fn may_cast_instant_or_sorcery_from_among(sentence: &SentenceInput) -> Option<EffectAst> {
    let words = crate::lexer::parser_token_word_refs(sentence.lowered());
    let exact_spell_surface = [
        "you", "may", "cast", "an", "instant", "or", "sorcery", "spell", "from", "among", "those",
        "cards", "without", "paying", "its", "mana", "cost",
    ];
    let exact_card_surface = [
        "you", "may", "cast", "an", "instant", "or", "sorcery", "card", "from", "among", "those",
        "cards", "without", "paying", "its", "mana", "cost",
    ];
    if words.as_slice() != exact_spell_surface && words.as_slice() != exact_card_surface {
        return None;
    }
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "chosen_revealed_spell");
    let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::RevealedThisWay.key());
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)));
    filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    Some(EffectAst::May {
        effects: vec![
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
                zone: Zone::Hand,
            },
            EffectAst::subject_verb_cast_tagged(chosen_tag, PlayerAst::You, false, false, true, None),
        ],
    })
}

/// "You may cast a spell from among those cards without paying its mana
/// cost." over a looked-at hand. The authored collection phrase is kept: the
/// generic reference normalization would rewrite it to "it" and lose both the
/// optional choice and the hand's provenance.
fn may_cast_spell_from_among(sentence: &SentenceInput) -> Option<EffectAst> {
    let words = crate::lexer::parser_token_word_refs(sentence.lexed());
    let exact_surface = [
        "you", "may", "cast", "a", "spell", "from", "among", "those", "cards", "without", "paying",
        "its", "mana", "cost",
    ];
    if words.as_slice() != exact_surface {
        return None;
    }
    Some(EffectAst::may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
        PlayerAst::You,
        PlayerAst::That,
        ObjectFilter::nonland().in_zone(Zone::Hand),
        Zone::Hand,
    ))
}

/// "You draw a card for each Mountain and red card in it."
fn draw_for_each_in_it(sentence: &SentenceInput, revealed: PlayerFilter) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    if words.len() < 10
        || words.get(..6) != Some(["you", "draw", "a", "card", "for", "each"].as_slice())
        || words.get(words.len() - 2..) != Some(["in", "it"].as_slice())
    {
        return None;
    }
    let filter_tokens = crate::lexer::synthetic_word_tokens(&words[6..words.len() - 2]);
    let mut filter =
        crate::grammar::filters::parse_subtype_color_shared_card_union_lexed(&filter_tokens, false)?;
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::AliasedTarget(Box::new(revealed)));
    let count = Value::Count(filter).with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
    Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::Draw { count },
    ))
}

/// "Exile all noncreature, nonland cards from that player's hand and graveyard."
fn exile_noncreature_nonland_hand_and_graveyard(sentence: &SentenceInput) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    if !crate::word_primitives::parse_sequence_prefix(
        &words,
        &["exile", "all", "noncreature", "nonland", "cards", "from"],
    ) || !crate::slice_primitives::contains_all(&words, &["that", "hand", "graveyard"])
        || !crate::slice_primitives::contains_any(&words, &["player", "players", "player's"])
    {
        return None;
    }
    let mut hand = ObjectFilter::default();
    hand.zone = Some(Zone::Hand);
    let mut graveyard = ObjectFilter::default();
    graveyard.zone = Some(Zone::Graveyard);
    let mut union = ObjectFilter::default();
    union.owner = Some(PlayerFilter::target_opponent());
    union.excluded_card_types = vec![CardType::Creature, CardType::Land];
    union.any_of = vec![hand, graveyard];
    Some(EffectAst::subject_verb_exile_all(union, false))
}

/// The statement a sentence makes over the shown hand, if any.
fn statement(shown: &Shown, sentence: &SentenceInput) -> Option<EffectAst> {
    match shown {
        Shown::Looked(_) => may_cast_spell_from_among(sentence),
        Shown::Revealed(player) => {
            let revealed = match player {
                PlayerAst::Target => PlayerFilter::Any,
                _ => PlayerFilter::Opponent,
            };
            (*player == PlayerAst::TargetOpponent)
                .then(|| choose_from_it_or_graveyard(sentence))
                .flatten()
                .or_else(|| {
                    (*player == PlayerAst::TargetOpponent)
                        .then(|| may_cast_instant_or_sorcery_from_among(sentence))
                        .flatten()
                })
                .or_else(|| draw_for_each_in_it(sentence, revealed))
                .or_else(|| exile_noncreature_nonland_hand_and_graveyard(sentence))
        }
    }
}

/// Open a procedure at a reveal-hand or look-at-hand sentence when the next
/// sentence makes a statement over the hand.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<HandGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let Some((effect, shown)) = shown_hand(sentence) else {
        return Ok(None);
    };
    if statement(&shown, next).is_none() {
        return Ok(None);
    }
    Ok(Some(HandGroup {
        effects: vec![effect],
        shown,
        closed: false,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// Continue with the statement over the hand; false for anything else.
pub(super) fn continue_with(
    group: &mut HandGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.closed {
        return Ok(false);
    }
    let Some(effect) = statement(&group.shown, sentence) else {
        return Ok(false);
    };
    group.effects.push(effect);
    group.closed = true;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: HandGroup) -> Vec<EffectAst> {
    group.effects
}
