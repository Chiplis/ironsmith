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
    if let Some(group) = open_counted_battlefield_partition(sentences, sentence_idx, &search)? {
        return Ok(Some(group));
    }
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
            optional_search_zones: false,
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

/// Who controls one counted part of a searched group as it enters.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PartitionController {
    Default,
    You,
    ChosenOpponent,
}

/// "put <n> of them onto the battlefield [tapped] [under your control | under
/// an opponent's control] [and the rest into your hand] [if <predicate>]"
struct CountedBattlefieldPart {
    count: usize,
    tapped: bool,
    controller: PartitionController,
    rest_to_hand: bool,
    predicate: Option<crate::cards::builders::PredicateAst>,
}

fn counted_battlefield_part(sentence: &SentenceInput) -> Option<CountedBattlefieldPart> {
    let tokens = trim(sentence.lexed());
    let (body, predicate) = match crate::grammar::structure::split_trailing_if_clause_lexed(tokens) {
        Some(spec) => (trim(spec.leading_tokens), Some(spec.predicate)),
        None => (tokens, None),
    };
    let words = crate::lexer::parser_token_word_refs(body)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .filter(|word| !matches!(word.as_str(), "," | "."))
        .collect::<Vec<_>>();
    let words = words.iter().map(String::as_str).collect::<Vec<_>>();
    let ["put", count, "of", "them", "onto", "the", "battlefield", rest @ ..] = words.as_slice()
    else {
        return None;
    };
    let count = crate::util::parse_number_word_u32(count)? as usize;
    let mut rest = rest;
    let mut tapped = false;
    if let ["tapped", tail @ ..] = rest {
        tapped = true;
        rest = tail;
    }
    let mut controller = PartitionController::Default;
    for (phrase, parsed) in [
        (&["under", "your", "control"][..], PartitionController::You),
        (&["under", "an", "opponent's", "control"][..], PartitionController::ChosenOpponent),
        (&["under", "an", "opponents", "control"][..], PartitionController::ChosenOpponent),
        (&["under", "an", "opponent", "s", "control"][..], PartitionController::ChosenOpponent),
    ] {
        if let Some(tail) = rest.strip_prefix(phrase) {
            controller = parsed;
            rest = tail;
            break;
        }
    }
    if !tapped && let ["tapped", tail @ ..] = rest {
        tapped = true;
        rest = tail;
    }
    let rest_to_hand = match rest {
        [] => false,
        ["and", "the", "rest" | "other", "into", "your", "hand"] => true,
        _ => return None,
    };
    Some(CountedBattlefieldPart {
        count,
        tapped,
        controller,
        rest_to_hand,
        predicate,
    })
}

/// "Search your library for up to four basic land cards and reveal them. Put
/// one of them onto the battlefield tapped under an opponent's control if the
/// {3}{G} cost was paid. Put two of them onto the battlefield tapped under
/// your control and the rest into your hand. Then shuffle." (Verdant Mastery)
///
/// Each counted part is chosen from the searched cards no earlier part took
/// and enters under its own controller (a chosen opponent, CR 110.2); a part
/// gated by a trailing "if" is skipped when its predicate is false, so its
/// cards stay in the group for the later parts and the rest.
fn open_counted_battlefield_partition(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    search: &SearchReveal,
) -> Result<Option<SearchPartitionGroup>, CardTextError> {
    let mut parts = Vec::new();
    let mut closed_by_rest = false;
    for sentence in sentences.iter().skip(sentence_idx + 1) {
        let Some(part) = counted_battlefield_part(sentence) else {
            break;
        };
        let rest_to_hand = part.rest_to_hand;
        parts.push((sentence, part));
        if rest_to_hand {
            closed_by_rest = true;
            break;
        }
    }
    // A single counted part is read by the ordinary partition grammar.
    if !closed_by_rest || parts.len() < 2 {
        return Ok(None);
    }
    let searched = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "searched_partition_pool");
    let remainder = helper_tag_for_tokens(parts[parts.len() - 1].0.lowered(), "searched_partition_rest");
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
    let mut effects = vec![
        // CR 701.23: the searched cards are found and revealed as one group.
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            optional_search_zones: false,
            filter: search.filter.clone(),
            count: search.count.clone(),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(searched.clone()),
            zones: vec![Zone::Library],
            search_mode: Some(search_mode),
        }),
        EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(searched.clone())),
    ];
    let mut taken: Vec<crate::tag::TagRef> = Vec::new();
    for (index, (sentence, part)) in parts.into_iter().enumerate() {
        let tag = crate::tag::TagRef::of(helper_tag_for_tokens(
            sentence.lowered(),
            &format!("searched_partition_part_{index}"),
        ));
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: pool(&taken),
            count: ChoiceCount::exactly(part.count),
            player: PlayerAst::You,
            tag: tag.clone(),
            zone: Zone::Library,
        });
        let target = TargetAst::Tagged(tag.clone(), None);
        let enter = match part.controller {
            PartitionController::ChosenOpponent => EffectAst::subject_verb_put_onto_battlefield(
                PlayerAst::Opponent,
                target,
                part.tapped,
                ReturnControllerAst::Preserve,
            ),
            PartitionController::You | PartitionController::Default => {
                EffectAst::subject_verb_move_to_zone(
                    target,
                    Zone::Battlefield,
                    false,
                    if part.controller == PartitionController::You {
                        ReturnControllerAst::You
                    } else {
                        ReturnControllerAst::Preserve
                    },
                    part.tapped,
                    None,
                )
            }
        };
        let program = vec![choose, enter];
        match part.predicate {
            Some(predicate) => effects.push(EffectAst::Conditionals(
                crate::cards::builders::ConditionalEffectAst::TrailingIf {
                    predicate,
                    effects: program,
                },
            )),
            None => effects.extend(program),
        }
        taken.push(tag);
    }
    effects.push(EffectAst::subject_verb_tag_matching_objects(
        pool(&taken),
        vec![Zone::Library],
        crate::tag::TagRef::of(remainder.clone()),
    ));
    effects.push(move_group(
        crate::tag::TagRef::of(remainder),
        effect_grammar::LookedPartitionDestination::Hand,
    ));
    let consumed = 1 + taken.len();
    Ok(Some(SearchPartitionGroup {
        effects,
        stage: Stage::AwaitShuffle,
        first_sentence: sentence_idx,
        consumed,
    }))
}
