//! "Search your library for up to two basic land cards and reveal them. Put
//! one into your hand and the other into your graveyard. Then shuffle."
//! (Fork in the Road, Jarad's Orders)
//!
//! The searched set is one group: one chosen card of it goes to the hand and
//! every other searched card to the graveyard (or the library bottom), both
//! moved by the same instruction. Reading the partition here keeps the second
//! destination: the generic one-sentence readers would otherwise collapse the
//! group to "put it into your hand".

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ObjectChoiceEffectAst, PlayerAst,
    ReturnControllerAst, TargetAst,
};
use crate::grammar::effects as effect_grammar;
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;
use winnow::prelude::*;

pub(super) struct SearchPartitionGroup {
    effects: Vec<EffectAst>,
    stage: Stage,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

#[derive(PartialEq, Eq)]
enum Stage {
    AwaitShuffle,
    Closed,
}

struct SearchReveal {
    count: ChoiceCount,
    filter: ObjectFilter,
}

fn trim(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    trim_lexed_commas(crate::util::trim_edge_punctuation_tokens(tokens))
}

/// "search your library for <count> <filter> and reveal them"
fn search_reveal(sentence: &SentenceInput) -> Option<SearchReveal> {
    // The authored surface: normalization may rewrite the pronoun "them".
    let tokens = trim(sentence.lexed());
    let mut input = LexStream::new(tokens);
    primitives::phrase(&["search", "your", "library", "for"])
        .parse_next(&mut input)
        .ok()?;
    let count = crate::grammar::leaf::parse_leaf_choice_count_prefix_lexed
        .parse_next(&mut input)
        .ok()?;
    if count.max.is_none_or(|max| max < 2) {
        return None;
    }
    let rest_start = tokens.len() - input.len();
    let rest = &tokens[rest_start..];
    let reveal_at = rest.windows(3).position(|window| {
        window[0].is_word("and") && window[1].is_word("reveal") && window[2].is_word("them")
    })?;
    if reveal_at + 3 != rest.len() || reveal_at == 0 {
        return None;
    }
    let mut filter = crate::grammar::primitives::probe_shape(
        crate::object_filters::parse_object_filter_lexed(&rest[..reveal_at], false),
    )?;
    filter.zone = Some(Zone::Library);
    filter.owner = Some(PlayerFilter::You);
    Some(SearchReveal { count, filter })
}

fn partition_destination(
    sentence: &SentenceInput,
) -> Option<effect_grammar::LookedPartitionDestination> {
    match effect_grammar::parse_looked_card_disposition(trim(sentence.lowered()))? {
        effect_grammar::LookedCardDisposition::HandAndGraveyard => {
            Some(effect_grammar::LookedPartitionDestination::Graveyard)
        }
        effect_grammar::LookedCardDisposition::HandAndExile => {
            Some(effect_grammar::LookedPartitionDestination::Exile)
        }
        effect_grammar::LookedCardDisposition::HandAndLibraryBottom(order) => {
            Some(effect_grammar::LookedPartitionDestination::LibraryBottom(order))
        }
    }
}

fn is_then_shuffle(sentence: &SentenceInput) -> bool {
    let tokens = trim(sentence.lowered());
    primitives::parse_all(
        tokens,
        (
            winnow::combinator::opt(primitives::kw("then")),
            primitives::kw("shuffle"),
            winnow::combinator::opt(primitives::phrase(&["your", "library"])),
        )
            .void(),
        "then-shuffle",
    )
    .is_ok()
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<SearchPartitionGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let Some(search) = search_reveal(sentence) else {
        return Ok(None);
    };
    let Some(remainder_destination) = partition_destination(next) else {
        return Ok(None);
    };
    let searched = helper_tag_for_tokens(sentence.lowered(), "searched_partition_pool");
    let hand = helper_tag_for_tokens(next.lowered(), "searched_partition_hand");
    let remainder = helper_tag_for_tokens(next.lowered(), "searched_partition_rest");
    let pool = |excluded: &[crate::tag::TagRef]| {
        let mut filter = ObjectFilter::tagged(searched.clone()).in_zone(Zone::Library);
        for tag in excluded {
            filter = filter.not_tagged(tag.clone());
        }
        filter
    };
    let search_mode = if search.count.min == 0 {
        crate::effect::SearchSelectionMode::Optional
    } else {
        crate::effect::SearchSelectionMode::Exact
    };
    let effects = vec![
        // CR 701.23: the searched cards are found and revealed as one group.
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter: search.filter,
            count: search.count,
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(searched.clone()),
            zones: vec![Zone::Library],
            search_mode: Some(search_mode),
        }),
        EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(searched.clone())),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: pool(&[]),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(hand.clone()),
            zone: Zone::Library,
        }),
        EffectAst::subject_verb_tag_matching_objects(
            pool(&[crate::tag::TagRef::of(hand.clone())]),
            vec![Zone::Library],
            crate::tag::TagRef::of(remainder.clone()),
        ),
        move_group(hand, effect_grammar::LookedPartitionDestination::Hand),
        move_group(remainder, remainder_destination),
    ];
    Ok(Some(SearchPartitionGroup {
        effects,
        stage: Stage::AwaitShuffle,
        first_sentence: sentence_idx,
        consumed: 2,
    }))
}

fn move_group(
    tag: crate::tag::TagRef,
    destination: effect_grammar::LookedPartitionDestination,
) -> EffectAst {
    let (zone, order) = match destination {
        effect_grammar::LookedPartitionDestination::Hand => (Zone::Hand, None),
        effect_grammar::LookedPartitionDestination::Graveyard => (Zone::Graveyard, None),
        effect_grammar::LookedPartitionDestination::Exile => (Zone::Exile, None),
        effect_grammar::LookedPartitionDestination::LibraryTop(order)
        | effect_grammar::LookedPartitionDestination::LibraryBottom(order) => {
            (Zone::Library, Some(order))
        }
    };
    let to_top = matches!(
        destination,
        effect_grammar::LookedPartitionDestination::LibraryTop(_)
    );
    EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(tag, None),
        zone,
        to_top,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
    .with_library_order(order, PlayerAst::You)
}

pub(super) fn continue_with(
    group: &mut SearchPartitionGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.stage != Stage::AwaitShuffle || !is_then_shuffle(sentence) {
        return Ok(false);
    }
    // The trailing shuffle is the search's own (CR 701.23a) and is read by
    // the ordinary sentence grammar.
    group
        .effects
        .extend(super::parse_effect_sentence_lexed(sentence.lexed())?);
    group.stage = Stage::Closed;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: SearchPartitionGroup) -> Vec<EffectAst> {
    group.effects
}
