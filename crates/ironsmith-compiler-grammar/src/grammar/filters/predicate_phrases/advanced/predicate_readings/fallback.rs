//! Readings that run only when no ranked reading claims the input: the source
//! states, turn-history events and graveyard shapes that "Activate only if"
//! gates state (CR 602.5b) and that the ranked readings do not otherwise
//! cover. Running after the ranked readings keeps them from ever competing
//! with an established reading of the same words.

use super::super::*;
use super::Predicate;

/// The input's fallback reading, if a shape reads it.
pub(super) fn read(input: &Predicate<'_>) -> Option<PredicateAst> {
    let words: Vec<String> = crate::lexer::token_word_refs(input.predicate_tokens)
        .into_iter()
        .map(|word| word.replace(['\'', '’'], ""))
        .collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    read_words(&words)
}

fn read_words(words: &[&str]) -> Option<PredicateAst> {
    for shape in SHAPES {
        if let Some(predicate) = shape(words) {
            return Some(predicate);
        }
    }
    // "an oil counter was removed from a permanent you controlled this turn
    // or a permanent with an oil counter on it was put into a graveyard this
    // turn" (Churning Reservoir): two independent history queries.
    for (index, word) in words.iter().enumerate() {
        if *word != "or" || index == 0 || index + 1 >= words.len() {
            continue;
        }
        let (Some(left), Some(right)) = (
            read_side(&words[..index]),
            read_side(&words[index + 1..]),
        ) else {
            continue;
        };
        return Some(PredicateAst::Or(Box::new(left), Box::new(right)));
    }
    None
}

/// One side of a disjunction: a fallback shape, or any reading of the shared
/// predicate grammar.
fn read_side(words: &[&str]) -> Option<PredicateAst> {
    for shape in SHAPES {
        if let Some(predicate) = shape(words) {
            return Some(predicate);
        }
    }
    let tokens = crate::lexer::synthetic_word_tokens(words.iter().copied());
    parse_predicate(&tokens).ok()
}

type Shape = fn(&[&str]) -> Option<PredicateAst>;

const SHAPES: &[Shape] = &[
    source_state,
    creature_blocking_source,
    cards_above_source,
    opponent_dealt_combat_damage_by,
    cards_left_your_graveyard,
    you_sacrificed_this_turn,
    died_under_your_control,
    you_own_card_in_zone,
    you_discarded_this_turn,
    opponent_dealt_noncombat_damage,
    more_cards_in_hand_than_each_opponent,
    you_were_attacked,
    attacking_opponents,
    keyword_actions_this_turn,
    counter_removed_this_turn,
    permanent_put_into_graveyard_this_turn,
    controlled_source_continuously,
    exact_hand_sizes,
    permanent_types_among_graveyard,
];

const SOURCE_NOUNS: &[&str] = &[
    "creature",
    "artifact",
    "enchantment",
    "land",
    "permanent",
    "card",
    "planeswalker",
    "vehicle",
    "equipment",
    "aura",
    "spell",
];

/// The length of a leading reference to the ability's source: "this",
/// "this creature", "it".
fn source_subject_len(words: &[&str]) -> Option<usize> {
    match words {
        ["it", ..] => Some(1),
        ["this", noun, ..] if SOURCE_NOUNS.contains(noun) => Some(2),
        ["this", ..] => Some(1),
        _ => None,
    }
}

/// Whether the words are exactly a reference to the source.
fn is_source_reference(words: &[&str]) -> bool {
    source_subject_len(words) == Some(words.len())
}

fn number(word: &str) -> Option<u32> {
    crate::util::parse_number_word_u32(word).or_else(|| crate::util::decimal_count(word))
}

fn strip_article<'a, 'b>(words: &'a [&'b str]) -> &'a [&'b str] {
    match words {
        ["a" | "an" | "the", rest @ ..] => rest,
        _ => words,
    }
}

fn filter_from_words(words: &[&str]) -> Option<ObjectFilter> {
    let words = strip_article(words);
    if words.is_empty() {
        return None;
    }
    crate::object_filters::parse_object_filter_words(words, false).ok()
}

fn at_least(left: Value, count: u32) -> PredicateAst {
    PredicateAst::ValueComparison {
        left,
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    }
}

fn history_at_least(query: ironsmith_core::TurnHistoryCount, count: u32) -> PredicateAst {
    at_least(Value::TurnHistoryCount(query), count)
}

fn source_matches(configure: impl FnOnce(&mut ObjectFilter)) -> PredicateAst {
    let mut filter = ObjectFilter::default();
    configure(&mut filter);
    PredicateAst::Source(SourcePredicateAst::SourceMatches(filter))
}

/// "this creature is attacking", "this is attacking or blocking", "this
/// creature is blocked", "this blocked or was blocked by a blue creature this
/// turn", "this is on the battlefield or in your graveyard".
fn source_state(words: &[&str]) -> Option<PredicateAst> {
    let subject = source_subject_len(words)?;
    let rest = &words[subject..];
    match rest {
        ["is", "attacking"] => Some(PredicateAst::Source(SourcePredicateAst::SourceIsAttacking)),
        ["is", "blocking"] => Some(source_matches(|filter| filter.blocking = true)),
        ["is", "attacking", "or", "blocking"] => Some(PredicateAst::Or(
            Box::new(PredicateAst::Source(SourcePredicateAst::SourceIsAttacking)),
            Box::new(source_matches(|filter| filter.blocking = true)),
        )),
        ["is", "blocked"] => Some(source_matches(|filter| filter.blocked = true)),
        ["is", "unblocked"] => Some(source_matches(|filter| filter.unblocked = true)),
        ["is", "on", "the", "battlefield", "or", "in", "your", "graveyard"] => {
            Some(PredicateAst::Or(
                Box::new(PredicateAst::Source(SourcePredicateAst::SourceIsInZone(
                    Zone::Battlefield,
                ))),
                Box::new(PredicateAst::Source(SourcePredicateAst::SourceIsInZone(
                    Zone::Graveyard,
                ))),
            ))
        }
        ["blocked", "or", "was", "blocked", "by", other @ .., "this", "turn"] => {
            let other = filter_from_words(other)?;
            Some(source_matches(|filter| {
                filter.blocked_or_was_blocked_by_this_turn = Some(Box::new(other));
            }))
        }
        _ => None,
    }
}

/// "at least one creature is blocking this creature" (Grizzled Wolverine).
fn creature_blocking_source(words: &[&str]) -> Option<PredicateAst> {
    let rest = match words {
        ["at", "least", "one", "creature", "is", "blocking", rest @ ..]
        | ["a", "creature", "is", "blocking", rest @ ..] => rest,
        _ => return None,
    };
    if !is_source_reference(rest) {
        return None;
    }
    let mut blockers = ObjectFilter::creature().in_zone(Zone::Battlefield);
    blockers.blocking = true;
    blockers.in_combat_with_source = true;
    Some(at_least(Value::Count(blockers), 1))
}

/// "three or more creature cards are above this card" (Ashen Ghoul): the
/// source is in its owner's graveyard with that many matching cards above it.
fn cards_above_source(words: &[&str]) -> Option<PredicateAst> {
    let [count, "or", "more", rest @ ..] = words else {
        return None;
    };
    let above = rest
        .windows(2)
        .position(|window| window == ["are", "above"])?;
    let (descriptor, source) = (&rest[..above], &rest[above + 2..]);
    if !is_source_reference(source) || descriptor.is_empty() {
        return None;
    }
    let count = number(count)?;
    let mut filter = filter_from_words(descriptor)?;
    if filter.zone == Some(Zone::Battlefield) {
        filter.zone = None;
    }
    if filter.zone.is_some() || filter.controller.is_some() || filter.owner.is_some() {
        return None;
    }
    Some(PredicateAst::Source(
        SourcePredicateAst::SourceInGraveyardWithCardsAbove { filter, count },
    ))
}

/// "an opponent was dealt combat damage by a legendary creature this turn"
/// (Blitzball).
fn opponent_dealt_combat_damage_by(words: &[&str]) -> Option<PredicateAst> {
    let rest = match words {
        ["an", "opponent", "was", "dealt", "combat", "damage", "by", rest @ ..]
        | ["an", "opponent", "has", "been", "dealt", "combat", "damage", "by", rest @ ..] => rest,
        _ => return None,
    };
    let ["this", "turn"] = rest.get(rest.len().checked_sub(2)?..)? else {
        return None;
    };
    let mut sources = filter_from_words(&rest[..rest.len() - 2])?;
    sources.zone = None;
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::PlayersDealtCombatDamageBy {
            players: PlayerFilter::Opponent,
            sources,
        },
        1,
    ))
}

/// "three or more cards left your graveyard this turn" (Bonecache Overseer).
fn cards_left_your_graveyard(words: &[&str]) -> Option<PredicateAst> {
    let count = match words {
        [count, "or", "more", "cards", "left", "your", "graveyard", "this", "turn"] => {
            number(count)?
        }
        ["a", "card", "left", "your", "graveyard", "this", "turn"] => 1,
        _ => return None,
    };
    let mut filter = ObjectFilter::default().owned_by(PlayerFilter::You);
    filter.nontoken = true;
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::MovedZones {
            filter,
            from: Some(Zone::Graveyard),
            to: None,
        },
        count,
    ))
}

/// "you've sacrificed an artifact this turn", "you've sacrificed a Food this
/// turn" (Detective's Satchel, Bonecache Overseer).
fn you_sacrificed_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let rest = match words {
        ["youve", "sacrificed", rest @ ..] | ["you", "sacrificed", rest @ ..] => rest,
        _ => return None,
    };
    let ["this", "turn"] = rest.get(rest.len().checked_sub(2)?..)? else {
        return None;
    };
    let object = &rest[..rest.len() - 2];
    let (count, object) = match object {
        [count, "or", "more", object @ ..] => (number(count)?, object),
        object => (1, object),
    };
    let mut filter = filter_from_words(object)?;
    filter.zone = None;
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::Sacrificed {
            player: PlayerFilter::You,
            filter,
        },
        count,
    ))
}

/// "a non-Skeleton creature died under your control this turn" (Cult
/// Conscript).
fn died_under_your_control(words: &[&str]) -> Option<PredicateAst> {
    let ["a" | "an", object @ .., "died", "under", "your", "control", "this", "turn"] = words
    else {
        return None;
    };
    let mut filter = filter_from_words(object)?;
    filter.zone = None;
    filter.controller = Some(PlayerFilter::You);
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::Died {
            filter,
            controller_surface: ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
        },
        1,
    ))
}

/// "you own a card in exile" (Dreadlight Monstrosity).
fn you_own_card_in_zone(words: &[&str]) -> Option<PredicateAst> {
    let ["you", "own", object @ .., "in", zone] = words else {
        return None;
    };
    let zone = match *zone {
        "exile" => Zone::Exile,
        _ => return None,
    };
    let object = strip_article(object);
    let filter = match object {
        ["card"] => ObjectFilter::default(),
        _ => filter_from_words(object)?,
    };
    Some(at_least(
        Value::Count(filter.in_zone(zone).owned_by(PlayerFilter::You)),
        1,
    ))
}

/// "you've discarded a card this turn" (Gilt-Blade Prowler).
fn you_discarded_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let count = match words {
        ["youve" | "you", "discarded", "a", "card", "this", "turn"] => 1,
        ["youve" | "you", "discarded", count, "or", "more", "cards", "this", "turn"] => {
            number(count)?
        }
        _ => return None,
    };
    Some(at_least(Value::CardsDiscardedThisTurn(PlayerFilter::You), count))
}

/// "an opponent has been dealt noncombat damage this turn" (Grim Repriser).
fn opponent_dealt_noncombat_damage(words: &[&str]) -> Option<PredicateAst> {
    match words {
        ["an", "opponent", "has", "been", "dealt", "noncombat", "damage", "this", "turn"]
        | ["an", "opponent", "was", "dealt", "noncombat", "damage", "this", "turn"] => Some(
            at_least(Value::NoncombatDamageDealtToPlayersThisTurn(PlayerFilter::Opponent), 1),
        ),
        _ => None,
    }
}

/// "you have more cards in hand than each opponent" (Kitsune Bonesetter).
fn more_cards_in_hand_than_each_opponent(words: &[&str]) -> Option<PredicateAst> {
    match words {
        ["you", "have", "more", "cards", "in", "hand", "than", "each", "opponent"]
        | ["you", "have", "more", "cards", "in", "your", "hand", "than", "each", "opponent"] => {
            // Each opponent, not each other player: a teammate's hand does
            // not count.
            Some(PredicateAst::ValueComparison {
                left: Value::CardsInHand(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThan,
                right: Value::MaxCardsInHand(PlayerFilter::Opponent),
            })
        }
        _ => None,
    }
}

/// "you've been attacked this step" (Kongming's Contraptions): a creature is
/// attacking you.
fn you_were_attacked(words: &[&str]) -> Option<PredicateAst> {
    match words {
        ["youve", "been", "attacked", "this", "step" | "combat"]
        | ["you", "were", "attacked", "this", "step" | "combat"]
        | ["youre", "being", "attacked"] => {
            let mut attackers = ObjectFilter::creature().in_zone(Zone::Battlefield);
            attackers.attacking = true;
            Some(at_least(
                Value::Count(attackers.attacking_player(PlayerFilter::You)),
                1,
            ))
        }
        _ => None,
    }
}

/// "you're attacking two or more opponents" (Nemesis Phoenix).
fn attacking_opponents(words: &[&str]) -> Option<PredicateAst> {
    let ["youre", "attacking", count, "or", "more", "opponents"] = words else {
        return None;
    };
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::PlayersAttackedThisCombat(PlayerFilter::You),
        number(count)?,
    ))
}

/// "you've scried or surveilled this turn" (Proctor of Potential).
fn keyword_actions_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let ["youve" | "you", actions @ .., "this", "turn"] = words else {
        return None;
    };
    let mut kinds = Vec::new();
    let mut expect_action = true;
    for word in actions {
        if expect_action {
            let kind = match *word {
                "scried" => ironsmith_core::KeywordActionKind::Scry,
                "surveilled" => ironsmith_core::KeywordActionKind::Surveil,
                _ => return None,
            };
            kinds.push(kind);
        } else if *word != "or" {
            return None;
        }
        expect_action = !expect_action;
    }
    if kinds.is_empty() || expect_action {
        return None;
    }
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::KeywordActionsPerformed {
            player: PlayerFilter::You,
            actions: kinds,
        },
        1,
    ))
}

/// "an oil counter was removed from a permanent you controlled this turn"
/// (Churning Reservoir).
fn counter_removed_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let ["a" | "an", rest @ ..] = words else {
        return None;
    };
    let removed = rest
        .windows(4)
        .position(|window| window == ["counter", "was", "removed", "from"])?;
    let (counter_words, rest) = (&rest[..removed], &rest[removed + 4..]);
    let ["this", "turn"] = rest.get(rest.len().checked_sub(2)?..)? else {
        return None;
    };
    let counter_type = match counter_words {
        [] => None,
        [counter] => Some(crate::grammar::filters::parse_counter_type_word(counter)?),
        _ => return None,
    };
    let mut object = strip_article(&rest[..rest.len() - 2]);
    let mut controller = None;
    if let [head @ .., "you", "controlled" | "control"] = object {
        object = head;
        controller = Some(PlayerFilter::You);
    }
    let mut filter = filter_from_words(object)?;
    filter.zone = None;
    if controller.is_some() {
        filter.controller = controller;
    }
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::CountersRemovedFrom {
            counter_type,
            filter,
        },
        1,
    ))
}

/// "a permanent with an oil counter on it was put into a graveyard this turn"
/// (Churning Reservoir).
fn permanent_put_into_graveyard_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let ["a" | "an", object @ .., "was", "put", "into", "a", "graveyard", "this", "turn"] = words
    else {
        return None;
    };
    if object.first() != Some(&"permanent") {
        return None;
    }
    let mut filter = filter_from_words(object)?;
    filter.zone = None;
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::MovedZones {
            filter,
            from: Some(Zone::Battlefield),
            to: Some(Zone::Graveyard),
        },
        1,
    ))
}

/// "you've controlled this artifact continuously since the beginning of your
/// most recent turn" (Rocket Launcher): the source has been under your
/// control since your most recent turn began, which is exactly when it is
/// free of summoning sickness (CR 302.6).
fn controlled_source_continuously(words: &[&str]) -> Option<PredicateAst> {
    let ["youve" | "you", "controlled", rest @ ..] = words else {
        return None;
    };
    const TAIL: &[&str] = &[
        "continuously",
        "since",
        "the",
        "beginning",
        "of",
        "your",
        "most",
        "recent",
        "turn",
    ];
    let subject_end = rest.len().checked_sub(TAIL.len())?;
    if rest[subject_end..] != *TAIL || !is_source_reference(&rest[..subject_end]) {
        return None;
    }
    Some(PredicateAst::Not(Box::new(source_matches(|filter| {
        filter.entered_since_your_last_turn_ended = true;
    }))))
}

/// "you have exactly zero or seven cards in hand" (The Biblioplex).
fn exact_hand_sizes(words: &[&str]) -> Option<PredicateAst> {
    let (first, second) = match words {
        ["you", "have", "exactly", first, "or", second, "cards", "in", "hand"]
        | ["you", "have", "exactly", first, "or", second, "cards", "in", "your", "hand"] => {
            (number(first)?, number(second)?)
        }
        _ => return None,
    };
    let exactly = |count: u32| PredicateAst::ValueComparison {
        left: Value::CardsInHand(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Fixed(count as i32),
    };
    Some(PredicateAst::Or(
        Box::new(exactly(first)),
        Box::new(exactly(second)),
    ))
}

/// "there are four or more permanent types among cards in your graveyard"
/// (Matzalantli): each permanent type (CR 110.4a) present among the cards
/// counts once.
fn permanent_types_among_graveyard(words: &[&str]) -> Option<PredicateAst> {
    let [
        "there",
        "are",
        count,
        "or",
        "more",
        "permanent",
        "types",
        "among",
        "cards",
        "in",
        "your",
        "graveyard",
    ] = words
    else {
        return None;
    };
    let count = number(count)?;
    let present = |card_type: crate::types::CardType| {
        let mut filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You);
        filter.card_types = vec![card_type];
        Value::Min(Box::new(Value::Count(filter)), Box::new(Value::Fixed(1)))
    };
    let total = [
        crate::types::CardType::Artifact,
        crate::types::CardType::Battle,
        crate::types::CardType::Creature,
        crate::types::CardType::Enchantment,
        crate::types::CardType::Land,
        crate::types::CardType::Planeswalker,
    ]
    .into_iter()
    .map(present)
    .reduce(|left, right| Value::Add(Box::new(left), Box::new(right)))?;
    Some(at_least(total, count))
}
