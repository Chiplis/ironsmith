//! Readings that run only when no ranked reading claims the input: the source
//! states, turn-history events and graveyard shapes that "Activate only if"
//! gates state (CR 602.5b) and that the ranked readings do not otherwise
//! cover. Running after the ranked readings keeps them from ever competing
//! with an established reading of the same words.

use super::super::*;
use super::Predicate;

/// The input's fallback reading, if a shape reads it.
pub(super) fn read(input: &Predicate<'_>) -> Option<PredicateAst> {
    if let Some(predicate) = negated_mana_spent(input.predicate_tokens) {
        return Some(predicate);
    }
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
    // Negation scopes over the whole copula predicate ("it isn't A or B"),
    // so it is read before the disjunction split below.
    if let Some(predicate) = negated_copula(words) {
        return Some(predicate);
    }
    if let Some(predicate) = possessive_characteristic(words) {
        return Some(predicate);
    }
    if let Some(predicate) = definite_referent_had(words) {
        return Some(predicate);
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

/// Negated copulas (and "doesn't have") and the positive verb each denies.
const NEGATED_COPULAS: &[(&[&str], &str)] = &[
    (&["isnt"], "is"),
    (&["is", "not"], "is"),
    (&["wasnt"], "was"),
    (&["was", "not"], "was"),
    (&["arent"], "are"),
    (&["are", "not"], "are"),
    (&["werent"], "were"),
    (&["were", "not"], "were"),
    (&["doesnt", "have"], "has"),
    (&["does", "not", "have"], "has"),
];

/// Base verbs a "didn't" negation may deny, with the past tense the positive
/// clause is written in.
const DID_NOT_VERBS: &[(&str, &str)] = &[
    ("lose", "lost"),
    ("gain", "gained"),
    ("cast", "cast"),
    ("play", "played"),
    ("attack", "attacked"),
    ("block", "blocked"),
    ("discard", "discarded"),
    ("sacrifice", "sacrificed"),
    ("die", "died"),
];

/// "you didn't lose life this turn" (Luminarch Ascension): the past-tense
/// positive clause of a "didn't" negation over "you" or a short reference.
fn did_not_positive<'a>(words: &[&'a str]) -> Option<Vec<&'a str>> {
    let subject_len = match words {
        ["you", ..] => 1,
        ["it", ..] => 1,
        ["this" | "that", noun, ..] if !is_negation_or_copula(noun) => 2,
        _ => return None,
    };
    let (subject, after) = words.split_at(subject_len);
    let (verb, complement) = match after {
        ["didnt", verb, complement @ ..] | ["did", "not", verb, complement @ ..] => {
            (*verb, complement)
        }
        _ => return None,
    };
    let past = DID_NOT_VERBS
        .iter()
        .find(|(base, _)| *base == verb)
        .map(|(_, past)| *past)?;
    Some(
        subject
            .iter()
            .copied()
            .chain(std::iter::once(past))
            .chain(complement.iter().copied())
            .collect(),
    )
}

/// Words that open a relative or prepositional qualifier: a subject holding
/// one is a noun phrase whose negation may belong to the qualifier.
const QUALIFIER_WORDS: &[&str] = &["that", "which", "who", "with", "without", "you", "of"];

fn is_negation_or_copula(word: &str) -> bool {
    NEGATED_COPULAS
        .iter()
        .any(|(negation, positive)| negation[0] == word || *positive == word)
}

/// Subject words a negated copula may follow: the pronoun "it", or a short
/// demonstrative/definite/target/attachment reference ("this spell", "that
/// creature", "the exiled card", "target player", "enchanted creature").
fn negated_copula_subject_len(words: &[&str]) -> Option<usize> {
    if words.first() == Some(&"it") {
        return Some(1);
    }
    let ["this" | "that" | "the" | "target" | "enchanted" | "equipped", ..] = words else {
        return None;
    };
    (2..=3).find(|len| {
        words.len() > *len
            && words[1..*len]
                .iter()
                .all(|word| !is_negation_or_copula(word) && !QUALIFIER_WORDS.contains(word))
            && NEGATED_COPULAS
                .iter()
                .any(|(negation, _)| words[*len..].starts_with(negation))
    })
}

/// "you're not the monarch" (Court of Vantress), "it wasn't kicked" (Sphinx
/// of Lost Truths), "this spell wasn't cast from your hand" (Twinned Vision),
/// "the exiled card doesn't have suspend" (Gandalf of the Secret Fire): a
/// negated copula denies the positive predicate the shared grammar reads for
/// the same subject and complement. Only a short simple subject is accepted,
/// so the negation is the clause's main verb and never a qualifier inside a
/// noun phrase ("a creature that isn't a Wolf").
fn negated_copula(words: &[&str]) -> Option<PredicateAst> {
    let positive: Vec<&str> = match words {
        // Fused contractions keep their subject: "you're not", "they're not",
        // "it's not".
        [fused @ ("youre" | "theyre"), "not", rest @ ..] if !rest.is_empty() => {
            std::iter::once(*fused).chain(rest.iter().copied()).collect()
        }
        ["its", "not", rest @ ..] if !rest.is_empty() => ["it", "is"]
            .into_iter()
            .chain(rest.iter().copied())
            .collect(),
        _ if did_not_positive(words).is_some() => did_not_positive(words)?,
        _ => {
            let subject_len = negated_copula_subject_len(words)?;
            let (subject, after) = words.split_at(subject_len);
            let (negation, copula) = NEGATED_COPULAS
                .iter()
                .find(|(negation, _)| after.starts_with(negation))?;
            let complement = &after[negation.len()..];
            if complement.is_empty() {
                return None;
            }
            subject
                .iter()
                .copied()
                .chain(std::iter::once(*copula))
                .chain(complement.iter().copied())
                .collect()
        }
    };
    let tokens = crate::lexer::synthetic_word_tokens(positive.iter().copied());
    let predicate = parse_predicate(&tokens).ok()?;
    Some(PredicateAst::Not(Box::new(predicate)))
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
    each_player_has_no_cards_in_hand,
    you_were_attacked,
    attacking_opponents,
    keyword_actions_this_turn,
    counter_removed_this_turn,
    permanent_put_into_graveyard_this_turn,
    controlled_source_continuously,
    exact_hand_sizes,
    permanent_types_among_graveyard,
    graveyard_size_threshold,
    mana_values_among_your_graveyard,
    cards_exiled_with_source,
    you_have_less_life_than_opponent,
    you_committed_crime_this_turn,
    you_played_land_this_turn,
    you_activated_loyalty_ability_this_turn,
    you_gained_and_lost_life_this_turn,
    you_controlled_referenced_object,
    counted_this_way,
    you_acted_this_way,
    no_life_lost_this_way,
    you_didnt_put_onto_battlefield_this_way,
    creatures_attacked_this_turn,
    card_exiled_with_it,
    source_kicked_twice,
    target_player_life_total,
    card_directly_above_source,
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
        SourcePredicateAst::SourceInGraveyardWithCardsAbove {
            filter,
            count,
            directly_above: false,
        },
    ))
}

/// "this card is in your graveyard with a creature card directly above it"
/// (Death Spark, Krovikan Horror): the card immediately above the source in
/// its owner's ordered graveyard matches (CR 404.1).
fn card_directly_above_source(words: &[&str]) -> Option<PredicateAst> {
    let rest = match words {
        ["this", "card", "is", "in", "your", "graveyard", "with", rest @ ..] => rest,
        _ => return None,
    };
    let [descriptor @ .., "directly", "above", "it"] = rest else {
        return None;
    };
    let descriptor = strip_article(descriptor);
    if descriptor.is_empty() {
        return None;
    }
    let mut filter = filter_from_words(descriptor)?;
    if filter.zone == Some(Zone::Battlefield) {
        filter.zone = None;
    }
    if filter.zone.is_some() || filter.controller.is_some() || filter.owner.is_some() {
        return None;
    }
    Some(PredicateAst::Source(
        SourcePredicateAst::SourceInGraveyardWithCardsAbove {
            filter,
            count: 1,
            directly_above: true,
        },
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
        // "a player discarded a card this turn" (The Raven Man): the turn's
        // discards summed over every player.
        ["a", "player", "discarded", "a", "card", "this", "turn"] => {
            return Some(at_least(Value::CardsDiscardedThisTurn(PlayerFilter::Any), 1));
        }
        ["an", "opponent", "discarded", "a", "card", "this", "turn"] => {
            return Some(at_least(
                Value::CardsDiscardedThisTurn(PlayerFilter::Opponent),
                1,
            ));
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

/// "each player has no cards in hand" (Howltooth Hollow): the largest hand
/// among all players is empty.
fn each_player_has_no_cards_in_hand(words: &[&str]) -> Option<PredicateAst> {
    match words {
        ["each", "player", "has", "no", "cards", "in", "hand"]
        | ["each", "player", "has", "no", "cards", "in", "their", "hand"] => {
            Some(PredicateAst::ValueComparison {
                left: Value::MaxCardsInHand(PlayerFilter::Any),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(0),
            })
        }
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

/// "a graveyard has twenty or more cards in it" (Visions of Beyond, Jace, the
/// Perfected Mind): some player's graveyard meets the size threshold.
fn graveyard_size_threshold(words: &[&str]) -> Option<PredicateAst> {
    let ["a", "graveyard", "has", minimum, "or", "more", "cards", "in", "it"] = words else {
        return None;
    };
    let minimum = number(minimum)?;
    Some(at_least(
        Value::CountPlayersWithCardsInGraveyardAtLeast(PlayerFilter::Any, minimum),
        1,
    ))
}

/// "there are five or more mana values among cards in your graveyard"
/// (Sanguine Spy, Tainted Indulgence): distinct mana values (CR 202.3) of the
/// cards you own in your graveyard.
fn mana_values_among_your_graveyard(words: &[&str]) -> Option<PredicateAst> {
    let [
        "there",
        "are",
        count,
        "or",
        "more",
        "mana",
        "values",
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
    let cards = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    Some(at_least(Value::DistinctManaValues(cards), count))
}

/// "there are four or more creature cards exiled with this artifact" (Negative
/// Zone Portal, River Song's Diary, Profane Procession): cards in exile linked
/// to the source by its own exile instructions (CR 607.2a).
fn cards_exiled_with_source(words: &[&str]) -> Option<PredicateAst> {
    let ["there", "are", count, "or", "more", rest @ ..] = words else {
        return None;
    };
    let exiled = rest.windows(3).position(|window| window == ["exiled", "with", "this"])?;
    let (descriptor, source) = (&rest[..exiled], &rest[exiled + 3..]);
    match source {
        [] => {}
        [noun] if SOURCE_NOUNS.contains(noun) => {}
        _ => return None,
    }
    let count = number(count)?;
    let mut filter = match descriptor {
        ["cards"] => ObjectFilter::default(),
        [.., "cards"] => {
            let mut filter = filter_from_words(descriptor)?;
            if filter.zone.is_some() || filter.controller.is_some() || filter.owner.is_some() {
                return None;
            }
            filter
        }
        _ => return None,
    };
    filter = filter
        .match_tagged(
            crate::tag::CompilerReferenceTag::SourceExiled.bind(),
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        )
        .in_zone(Zone::Exile);
    Some(at_least(Value::Count(filter), count))
}

/// "you have less life than an opponent" (Timely Reinforcements): some
/// opponent has more life than you.
fn you_have_less_life_than_opponent(words: &[&str]) -> Option<PredicateAst> {
    let ["you", "have", "less", "life", "than", "an", "opponent"] = words else {
        return None;
    };
    Some(PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreLifeThanYou {
        player: PlayerAst::Opponent,
    }))
}

/// "target player has exactly 10 life" (Hidetsugu's Second Rite): a life-total
/// comparison of a player the condition itself targets. The comparison reads
/// as for "you have ..."; the player is the ability's target, which the
/// conditional announces before it resolves (CR 601.2c, 608.2b).
fn target_player_life_total(words: &[&str]) -> Option<PredicateAst> {
    let (player, rest) = match words {
        ["target", "player", "has", rest @ ..] => (PlayerFilter::target_player(), rest),
        ["target", "opponent", "has", rest @ ..] => (PlayerFilter::target_opponent(), rest),
        _ => return None,
    };
    // "... has exactly 10 life", "... has fewer than nine poison counters".
    if !matches!(rest.last(), Some(&("life" | "counters"))) {
        return None;
    }
    let tokens =
        crate::lexer::synthetic_word_tokens(["you", "have"].into_iter().chain(rest.iter().copied()));
    let PredicateAst::ValueComparison {
        left,
        operator,
        right,
    } = parse_predicate(&tokens).ok()?
    else {
        return None;
    };
    let left = match left.unhinted() {
        Value::LifeTotal(PlayerFilter::You) => Value::LifeTotal(player),
        Value::PlayerCounters(PlayerFilter::You, counter_type) => {
            Value::PlayerCounters(player, *counter_type)
        }
        _ => return None,
    };
    Some(PredicateAst::ValueComparison {
        left,
        operator,
        right,
    })
}

/// "you've committed a crime this turn" (Servant of the Stinger, Oko, the
/// Ringleader): you targeted an opponent, or something they control, this
/// turn (CR 700.13).
fn you_committed_crime_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let ["youve" | "you", "committed", "a", "crime", "this", "turn"] = words else {
        return None;
    };
    Some(PredicateAst::Player(PlayerPredicateAst::PlayerCommittedCrimeThisTurn {
        player: PlayerAst::You,
    }))
}

/// "you played a land this turn" (River of Tears).
fn you_played_land_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let ["youve" | "you", "played", "a", "land", "this", "turn"] = words else {
        return None;
    };
    Some(PredicateAst::TurnHistory(
        TurnHistoryPredicateAst::PlayerPlayedLandThisTurn(PlayerAst::You),
    ))
}

/// "you've activated a loyalty ability this turn" (Kiora of Salt and Sand).
fn you_activated_loyalty_ability_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let ["youve" | "you", "activated", "a", "loyalty", "ability", "this", "turn"] = words else {
        return None;
    };
    Some(PredicateAst::TurnHistory(
        TurnHistoryPredicateAst::PlayerActivatedLoyaltyAbilityThisTurn(PlayerAst::You),
    ))
}

/// "you gained and lost life this turn" (Lunar Convocation): both turn
/// facts, each read as its own clause.
fn you_gained_and_lost_life_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let ["you", "gained", "and", "lost", "life", "this", "turn"] = words else {
        return None;
    };
    let gained = crate::lexer::synthetic_word_tokens(["you", "gained", "life", "this", "turn"]);
    let lost = crate::lexer::synthetic_word_tokens(["you", "lost", "life", "this", "turn"]);
    Some(PredicateAst::And(
        Box::new(parse_predicate(&gained).ok()?),
        Box::new(parse_predicate(&lost).ok()?),
    ))
}

/// "enchanted creature's power is 4 or greater" (Arachnus Web, Domestication):
/// the possessive spelling of "enchanted creature has power 4 or greater",
/// read by the shared grammar for that spelling.
fn possessive_characteristic(words: &[&str]) -> Option<PredicateAst> {
    let [
        determiner @ ("enchanted" | "equipped" | "target" | "that"),
        possessor,
        characteristic @ ("power" | "toughness"),
        "is",
        comparison @ ..,
    ] = words
    else {
        return None;
    };
    let noun = match *possessor {
        "creatures" => "creature",
        "permanents" => "permanent",
        _ => return None,
    };
    if comparison.is_empty() {
        return None;
    }
    let rewritten: Vec<&str> = [*determiner, noun, "has", *characteristic]
        .into_iter()
        .chain(comparison.iter().copied())
        .collect();
    parse_predicate(&crate::lexer::synthetic_word_tokens(rewritten)).ok()
}

/// "If the creature had power 4 or greater" (Anax, Hardened in the Forge):
/// a definite description of the referenced object in a past-tense
/// (last-known) characteristic check reads as "that creature had ...".
fn definite_referent_had(words: &[&str]) -> Option<PredicateAst> {
    let ["the", noun @ ("creature" | "permanent"), "had", rest @ ..] = words else {
        return None;
    };
    if rest.is_empty() {
        return None;
    }
    let rewritten: Vec<&str> = ["that", *noun, "had"]
        .into_iter()
        .chain(rest.iter().copied())
        .collect();
    parse_predicate(&crate::lexer::synthetic_word_tokens(rewritten)).ok()
}

/// "If you controlled it" (Hotshot Investigators, Unyielding Gatekeeper), "If
/// you controlled that artifact" (Gleeful Demolition): the referenced object's
/// last-known controller (CR 608.2h) was you.
fn you_controlled_referenced_object(words: &[&str]) -> Option<PredicateAst> {
    match words {
        ["you", "controlled", "it"]
        | [
            "you",
            "controlled",
            "that",
            "artifact" | "creature" | "permanent" | "enchantment" | "land" | "planeswalker",
        ] => Some(PredicateAst::ItMatchedLastKnown(
            ObjectFilter::default().controlled_by(PlayerFilter::You),
        )),
        _ => None,
    }
}

/// The count of objects a prior instruction acted on, read from the shared
/// "<filter> <action> this way" metric grammar (bound to its exact producer
/// by reference resolution).
fn this_way_count(object_words: &[&str]) -> Option<Value> {
    crate::grammar::shared_util::value_semantics::parse_prior_effect_aggregate_metric_value(
        ironsmith_core::EffectMetric::Count,
        object_words,
    )
}

/// "If two or more cards are exiled this way" (Mysterious Stranger), "If
/// eight or more cards were returned to your hand this way" (Long Rest).
fn counted_this_way(words: &[&str]) -> Option<PredicateAst> {
    let [count, "or", "more", rest @ ..] = words else {
        return None;
    };
    if !rest.ends_with(&["this", "way"]) {
        return None;
    }
    Some(at_least(this_way_count(rest)?, number(count)?))
}

/// "If you return four or more nontoken permanents you control this way"
/// (Flood of Tears), "If you return a nonland card to your hand this way"
/// (Vengeful Rebirth), "If you draw one or more cards this way" (Transcendent
/// Archaic), "If you didn't draw cards this way" (Mr. Foxglove): the active
/// spelling of the passive count.
fn you_acted_this_way(words: &[&str]) -> Option<PredicateAst> {
    let (negated, rest) = match words {
        ["you", "didnt" | "dont", rest @ ..] => (true, rest),
        ["you", "did", "not", rest @ ..] => (true, rest),
        ["you", rest @ ..] => (false, rest),
        _ => return None,
    };
    let (participle, rest): (&[&str], &[&str]) = match rest {
        ["return" | "returned", rest @ ..] => (&["returned"], rest),
        ["draw" | "drew", rest @ ..] => (&["drawn"], rest),
        _ => return None,
    };
    let [object @ .., "this", "way"] = rest else {
        return None;
    };
    let (minimum, object) = match object {
        [count, "or", "more", object @ ..] => (number(count)?, object),
        ["a" | "an", object @ ..] => (1, object),
        object => (1, object),
    };
    if object.is_empty() {
        return None;
    }
    // "a nonland card to your hand": the destination follows the object.
    let (object, destination): (&[&str], &[&str]) = match object {
        [object @ .., "to", "your", "hand"] => (object, &["to", "your", "hand"]),
        object => (object, &[]),
    };
    let object_words: Vec<&str> = object
        .iter()
        .copied()
        .chain(participle.iter().copied())
        .chain(destination.iter().copied())
        .chain(["this", "way"])
        .collect();
    let predicate = at_least(this_way_count(&object_words)?, minimum);
    Some(if negated {
        PredicateAst::Not(Box::new(predicate))
    } else {
        predicate
    })
}

/// "If no life is lost this way" (Blitzwing, Cruel Tormentor): the prior
/// life-loss instruction's actual loss was zero.
fn no_life_lost_this_way(words: &[&str]) -> Option<PredicateAst> {
    let ["no", "life", "is" | "was", "lost", "this", "way"] = words else {
        return None;
    };
    Some(PredicateAst::ValueComparison {
        left: Value::PendingPriorEffectMetric(ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::Outcome,
            ironsmith_core::EffectMetric::LifeLost,
        )),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Fixed(0),
    })
}

/// "If you didn't put a card onto the battlefield this way" (Rulik Mons),
/// "If you didn't put the revealed card onto the battlefield this way" (Break
/// Out): the referenced card is not (or was not, last known) on the
/// battlefield after the optional put.
fn you_didnt_put_onto_battlefield_this_way(words: &[&str]) -> Option<PredicateAst> {
    let rest = match words {
        ["you", "didnt", "put", rest @ ..] | ["you", "did", "not", "put", rest @ ..] => rest,
        _ => return None,
    };
    let reference = match rest {
        [reference @ .., "onto", "the", "battlefield", "this", "way"]
        | [reference @ .., "onto", "battlefield", "this", "way"] => reference,
        _ => return None,
    };
    if !matches!(
        reference,
        ["it"] | ["a", "card"] | ["the", "card"] | ["that", "card"] | ["the", "revealed", "card"]
    ) {
        return None;
    }
    Some(PredicateAst::Not(Box::new(PredicateAst::Player(
        PlayerPredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            filter: ObjectFilter::default().in_zone(Zone::Battlefield),
            mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
        },
    ))))
}

/// "Three or more creatures attacked this turn" (Case of the Gateway
/// Express): distinct creatures any player attacked with this turn.
fn creatures_attacked_this_turn(words: &[&str]) -> Option<PredicateAst> {
    let [count, "or", "more", "creatures", "attacked", "this", "turn"] = words else {
        return None;
    };
    Some(history_at_least(
        ironsmith_core::TurnHistoryCount::CreaturesAttackedWith {
            player: PlayerFilter::Any,
            filter: ObjectFilter::creature(),
        },
        number(count)?,
    ))
}

/// "if a card is exiled with it" (Smirking Spelljacker): the source has a
/// linked exiled card, exactly as "a card is exiled with this creature".
fn card_exiled_with_it(words: &[&str]) -> Option<PredicateAst> {
    let ["a", "card", "is", "exiled", "with", "it"] = words else {
        return None;
    };
    let exiled_with_source =
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind())
            .in_zone(Zone::Exile);
    Some(PredicateAst::CountComparison {
        count: ironsmith_core::AnthemCountExpression::MatchingFilter(exiled_with_source),
        comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
        display: Some("a card is exiled with it".to_string()),
    })
}

/// "if it was kicked twice" (Archangel of Wrath): the source's kicker was
/// paid at least twice (CR 702.33c).
fn source_kicked_twice(words: &[&str]) -> Option<PredicateAst> {
    let subject = words.strip_suffix(&["was", "kicked", "twice"])?;
    if !is_source_reference(subject) {
        return None;
    }
    Some(at_least(Value::KickCount, 2))
}

/// "if {C} wasn't spent to cast it" (Wumpus Aberration): the denial of the
/// mana-spent reading, rebuilt from the authored tokens (the mana symbol stays
/// a real mana-group token) with the positive copula.
fn negated_mana_spent(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let negation = tokens
        .iter()
        .position(|token| token.is_word("wasn't") || token.is_word("wasnt"))?;
    if negation == 0 || !tokens.get(negation + 1)?.is_word("spent") {
        return None;
    }
    let mut positive = tokens.to_vec();
    positive[negation] = OwnedLexToken::synthetic_word("was");
    let predicate = parse_predicate(&positive).ok()?;
    Some(PredicateAst::Not(Box::new(predicate)))
}
