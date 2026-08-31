use crate::cards::builders::PlayerAst;
use crate::grammar::{leaf, permission_shapes};
use crate::lexer::{OwnedLexToken, TokenWordView};
use crate::static_abilities::StaticAbilityId;
use crate::target::PlayerFilter;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterKeywordConstraint {
    Static(StaticAbilityId),
    Marker(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubjectAst {
    Player(PlayerAst),
    /// The controller of the source object from the enclosing trigger event.
    ///
    /// This remains distinct from `PlayerAst::ItsController`: the latter
    /// resolves through ordinary last-object antecedent memory, which may be
    /// the object damaged by the triggering source.
    TriggeringSourceController,
    This,
}

pub fn contains_source_from_your_graveyard(words: &[&str]) -> bool {
    find_one_of(
        words,
        &[
            &["this", "card", "from", "your", "graveyard"],
            &["thiss", "card", "from", "your", "graveyard"],
            &["this", "creature", "from", "your", "graveyard"],
            &["thiss", "creature", "from", "your", "graveyard"],
            &["this", "permanent", "from", "your", "graveyard"],
            &["thiss", "permanent", "from", "your", "graveyard"],
        ],
    )
}

pub fn contains_source_from_your_hand(words: &[&str]) -> bool {
    find_one_of(
        words,
        &[
            &["this", "card", "from", "your", "hand"],
            &["thiss", "card", "from", "your", "hand"],
            &["this", "creature", "from", "your", "hand"],
            &["thiss", "creature", "from", "your", "hand"],
            &["this", "permanent", "from", "your", "hand"],
            &["thiss", "permanent", "from", "your", "hand"],
            &["this", "from", "your", "hand"],
            &["thiss", "from", "your", "hand"],
        ],
    )
}

pub fn contains_from_command_zone(words: &[&str]) -> bool {
    permission_shapes::find_words(words, &["from", "command", "zone"]).is_some()
}

/// Returns true only when the object whose ability is being parsed is the
/// object moving from the command zone.  A command-zone phrase in the effect's
/// object set (for example, "commanders you own from the command zone") must
/// not change the functional zone of the source ability.
pub fn contains_source_from_command_zone(words: &[&str]) -> bool {
    find_one_of(
        words,
        &[
            &["this", "from", "command", "zone"],
            &["thiss", "from", "command", "zone"],
            &["this", "card", "from", "command", "zone"],
            &["thiss", "card", "from", "command", "zone"],
            &["this", "creature", "from", "command", "zone"],
            &["thiss", "creature", "from", "command", "zone"],
            &["this", "permanent", "from", "command", "zone"],
            &["thiss", "permanent", "from", "command", "zone"],
        ],
    )
}

pub fn contains_discard_source(words: &[&str]) -> bool {
    permission_shapes::find_words(words, &["discard", "this", "card"]).is_some()
}

pub fn is_source_from_your_graveyard(words: &[&str]) -> bool {
    words.len() >= 4
        && prefix_one_of(words, &[&["this"], &["thiss"]])
        && permission_shapes::find_words(words, &["from", "your", "graveyard"]).is_some()
        && has_one_of_words(words, &["card", "creature", "permanent"])
}

pub fn is_source_from_exile(words: &[&str]) -> bool {
    words.len() >= 3
        && prefix_one_of(words, &[&["this"], &["thiss"]])
        && permission_shapes::find_words(words, &["from", "exile"]).is_some()
        && has_one_of_words(words, &["card", "creature", "permanent", "source"])
}

pub fn parse_subject_tokens(tokens: &[OwnedLexToken]) -> SubjectAst {
    parse_subject_words(&TokenWordView::new(tokens).word_refs())
}

pub fn parse_subject_words(words: &[&str]) -> SubjectAst {
    if words.is_empty() {
        return SubjectAst::This;
    }
    let mut start = 0usize;
    if permission_shapes::prefix_words(words, &["any", "number"]) {
        start = if permission_shapes::starts_at_words(words, 2, &["of"]) {
            3
        } else {
            2
        };
    }

    let mut slice = words.get(start..).unwrap_or_default();
    while prefix_one_of(slice, &[&["then"], &["and"], &["instead"]]) {
        slice = &slice[1..];
    }
    while permission_shapes::prefix_words(slice, &["each"]) {
        slice = &slice[1..];
    }
    if slice
        .first()
        .is_some_and(|word| leaf::parse_number_complete(word).is_ok())
    {
        slice = &slice[1..];
    }

    if prefix_one_of(
        slice,
        &[
            &[
                "the", "player", "who", "has", "the", "most", "cards", "in", "hand",
            ],
            &["player", "who", "has", "the", "most", "cards", "in", "hand"],
            &[
                "the", "player", "with", "the", "most", "cards", "in", "hand",
            ],
            &["player", "with", "the", "most", "cards", "in", "hand"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::MostCardsInHand);
    }
    if prefix_one_of(
        slice,
        &[
            &["the", "player", "who", "has", "the", "most", "life"],
            &["player", "who", "has", "the", "most", "life"],
            &["the", "player", "with", "the", "most", "life"],
            &["player", "with", "the", "most", "life"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::MostLifeTied);
    }
    if prefix_one_of(
        slice,
        &[
            &[
                "the", "player", "who", "has", "the", "lowest", "life", "total",
            ],
            &["player", "who", "has", "the", "lowest", "life", "total"],
            &["the", "player", "with", "the", "lowest", "life", "total"],
            &["player", "with", "the", "lowest", "life", "total"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::LowestLifeTied);
    }

    if let Some(have_idx) = first_word_offset(slice, &["has", "have"])
        && have_idx + 1 < slice.len()
    {
        slice = &slice[have_idx + 1..];
    }
    if prefix_one_of(slice, &[&["you"], &["your"]]) {
        return SubjectAst::Player(PlayerAst::You);
    }
    if prefix_one_of(slice, &[&["target", "opponent"], &["target", "opponents"]]) {
        return SubjectAst::Player(PlayerAst::TargetOpponent);
    }
    if prefix_one_of(slice, &[&["target", "player"], &["target", "players"]]) {
        return SubjectAst::Player(PlayerAst::Target);
    }
    if prefix_one_of(
        slice,
        &[
            &["a", "player", "of", "your", "choice"],
            &["player", "of", "your", "choice"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::Chosen);
    }
    if prefix_one_of(slice, &[&["opponent"], &["opponents"], &["an", "opponent"]]) {
        return SubjectAst::Player(PlayerAst::Opponent);
    }
    if prefix_one_of(
        slice,
        &[&["active", "player"], &["the", "active", "player"]],
    ) {
        return SubjectAst::Player(PlayerAst::Active);
    }
    if prefix_one_of(
        slice,
        &[
            &["enchanted", "player"],
            &["enchanted", "players"],
            &["enchanted", "opponent"],
            &["enchanted", "opponents"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::Enchanted);
    }
    if prefix_one_of(slice, &[&["other", "player"], &["other", "players"]]) {
        return SubjectAst::Player(PlayerAst::NotYou);
    }
    if permission_shapes::prefix_words(slice, &["defending", "player"])
        || permission_shapes::suffix_words(slice, &["defending", "player"])
    {
        return SubjectAst::Player(PlayerAst::Defending);
    }
    if prefix_one_of(
        slice,
        &[&["attacking", "player"], &["the", "attacking", "player"]],
    ) || permission_shapes::suffix_words(slice, &["attacking", "player"])
    {
        return SubjectAst::Player(PlayerAst::Attacking);
    }
    if prefix_one_of(
        slice,
        &[
            &["the", "player", "to", "your", "left"],
            &["player", "to", "your", "left"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::PlayerToYourLeft);
    }
    if prefix_one_of(
        slice,
        &[
            &["the", "player", "to", "your", "right"],
            &["player", "to", "your", "right"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::PlayerToYourRight);
    }
    if is_that_player_or_object_controller(slice) {
        return SubjectAst::Player(PlayerAst::ThatPlayerOrTargetController);
    }
    if prefix_one_of(slice, &[&["they"], &["that", "player"], &["the", "player"]])
        || prefix_one_of(slice, &[&["the", "voter"], &["voter"]])
    {
        return SubjectAst::Player(PlayerAst::That);
    }
    if prefix_one_of(
        slice,
        &[
            &["the", "chosen", "player"],
            &["chosen", "player"],
            &["the", "chosen", "players"],
            &["chosen", "players"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::Chosen);
    }
    if prefix_one_of(slice, &[&["that", "players"], &["their"]]) {
        return SubjectAst::Player(PlayerAst::That);
    }
    if prefix_one_of(
        slice,
        &[
            &["the", "owners", "of", "those", "cards"],
            &["owners", "of", "those", "cards"],
            &["the", "owners", "of", "those", "objects"],
            &["owners", "of", "those", "objects"],
        ],
    ) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if prefix_one_of(
        slice,
        &[
            &["that", "source's", "controller"],
            &["that", "source", "s", "controller"],
            &["that", "sources", "controller"],
            &["that", "spell", "or", "ability's", "controller"],
            &["that", "spell", "or", "ability", "s", "controller"],
            &["that", "spell", "or", "abilitys", "controller"],
        ],
    ) {
        return SubjectAst::TriggeringSourceController;
    }
    // "That artifact's controller gains …" — possessive singular subject.
    if slice.len() >= 3
        && permission_shapes::exact_words(&slice[..1], &["that"])
        && matches!(
            slice[1],
            "artifact's"
                | "creature's"
                | "permanent's"
                | "enchantment's"
                | "land's"
                | "planeswalker's"
                | "spell's"
                | "token's"
                | "card's"
        )
        && is_controller_or_owner(slice[2])
    {
        return SubjectAst::Player(
            if permission_shapes::exact_words(&slice[2..3], &["owner"]) {
                PlayerAst::ItsOwner
            } else {
                PlayerAst::ItsController
            },
        );
    }
    // The same possessive with the apostrophe split into ["artifact", "s"].
    if slice.len() >= 4
        && permission_shapes::exact_words(&slice[..1], &["that"])
        && matches!(
            slice[1],
            "artifact"
                | "creature"
                | "permanent"
                | "enchantment"
                | "land"
                | "planeswalker"
                | "spell"
                | "token"
                | "card"
        )
        && slice[2] == "s"
        && is_controller_or_owner(slice[3])
    {
        return SubjectAst::Player(
            if permission_shapes::exact_words(&slice[3..4], &["owner"]) {
                PlayerAst::ItsOwner
            } else {
                PlayerAst::ItsController
            },
        );
    }
    if slice.len() >= 3
        && permission_shapes::exact_words(&slice[..1], &["that"])
        && is_controlled_object_plural(slice[1])
        && is_controller_or_owner(slice[2])
    {
        return SubjectAst::Player(
            if permission_shapes::exact_words(&slice[2..3], &["owner"]) {
                PlayerAst::ItsOwner
            } else {
                PlayerAst::ItsController
            },
        );
    }
    if prefix_one_of(
        slice,
        &[
            &["its", "controller"],
            &["her", "controller"],
            &["his", "controller"],
        ],
    ) || (permission_shapes::prefix_words(slice, &["this"])
        && permission_shapes::suffix_words(slice, &["controller"]))
        || suffix_one_of(slice, &[&["its", "controller"], &["their", "controller"]])
    {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    // Named possessives are normalized into parser words such as
    // `hold for ransoms controller`. In a clause subject, the terminal
    // controller/owner relation is the semantic fact; the authored name is
    // retained by the enclosing ability surface.
    if slice.len() >= 2 && is_controller_or_owner(slice[slice.len() - 1]) {
        return SubjectAst::Player(if slice.last() == Some(&"owner") {
            PlayerAst::ItsOwner
        } else {
            PlayerAst::ItsController
        });
    }
    if prefix_one_of(slice, &[&["its", "owner"], &["their", "owner"]])
        || (permission_shapes::prefix_words(slice, &["this"])
            && permission_shapes::suffix_words(slice, &["owner"]))
        || suffix_one_of(slice, &[&["its", "owner"], &["their", "owner"]])
    {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if prefix_one_of(slice, &[&["this"], &["thiss"]]) {
        return SubjectAst::This;
    }
    SubjectAst::This
}

pub fn parse_filter_keyword_constraint_words(
    words: &[&str],
) -> Option<(FilterKeywordConstraint, usize)> {
    if words.is_empty() {
        return None;
    }
    if prefix_one_of(words, &[&["mana", "ability"], &["mana", "abilities"]]) {
        return Some((FilterKeywordConstraint::Marker("mana ability"), 2));
    }
    if cycling_keyword_root(words[0]).is_some() {
        return Some((FilterKeywordConstraint::Marker("cycling"), 1));
    }
    if permission_shapes::prefix_words(words, &["basic", "landcycling"]) {
        return Some((FilterKeywordConstraint::Marker("cycling"), 2));
    }
    let max_len = words.len().min(4);
    for len in (1..=max_len).rev() {
        if let Some(constraint) = filter_keyword_constraint_for_words(&words[..len]) {
            return Some((constraint, len));
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKeywordListConnective {
    And,
    Or,
    AndOr,
}

/// Parse a serial keyword list ("first strike, double strike, vigilance, or
/// haste"). Commas are elided from the word view, so adjacent keywords with no
/// separator word are accepted as list items; the final connective word picks
/// the surface ("or" vs "and/or" vs "and").
pub fn parse_filter_keyword_constraint_list_words(
    words: &[&str],
) -> Option<(
    Vec<FilterKeywordConstraint>,
    FilterKeywordListConnective,
    usize,
)> {
    let (first, first_used) = parse_filter_keyword_constraint_words(words)?;
    let mut constraints = vec![first];
    let mut consumed = first_used;
    let mut saw_or = false;
    let mut saw_and_or = false;
    loop {
        let rest = &words[consumed..];
        let (separator, connective) = match rest {
            ["and", "or", ..] => (2, Some(FilterKeywordListConnective::AndOr)),
            ["and/or", ..] => (1, Some(FilterKeywordListConnective::AndOr)),
            ["or", ..] => (1, Some(FilterKeywordListConnective::Or)),
            ["and", ..] => (1, Some(FilterKeywordListConnective::And)),
            _ => (0, None),
        };
        let Some((next, used)) = parse_filter_keyword_constraint_words(&rest[separator..]) else {
            break;
        };
        match connective {
            Some(FilterKeywordListConnective::Or) => saw_or = true,
            Some(FilterKeywordListConnective::AndOr) => saw_and_or = true,
            Some(FilterKeywordListConnective::And) | None => {}
        }
        constraints.push(next);
        consumed += separator + used;
    }
    let connective = if saw_and_or {
        FilterKeywordListConnective::AndOr
    } else if saw_or {
        FilterKeywordListConnective::Or
    } else {
        FilterKeywordListConnective::And
    };
    if std::env::var_os("IRONSMITH_CHOICE_TRACE").is_some() {
        eprintln!(
            "keyword-list: words={words:?} consumed={consumed} constraints={}",
            constraints.len()
        );
    }
    Some((constraints, connective, consumed))
}

pub fn cycling_keyword_root(word: &str) -> Option<&str> {
    if permission_shapes::exact_words(&[word], &["cycling"]) {
        return Some("");
    }
    const SUFFIX: &[char] = &['c', 'y', 'c', 'l', 'i', 'n', 'g'];
    if word.chars().count() > SUFFIX.len() && word_has_char_suffix(word, SUFFIX) {
        return word.get(..word.len().saturating_sub(SUFFIX.len()));
    }
    None
}

pub fn parse_hand_advantage_player(words: &[&str]) -> Option<PlayerFilter> {
    let (base, mut idx) = player_base(words)?;
    if !starts_with_one_of_words(words, idx, &["who", "that"])
        || !starts_with_one_of_words(words, idx + 1, &["has", "have"])
    {
        return None;
    }
    idx += 2;
    if permission_shapes::starts_at_words(words, idx, &["at", "least"]) {
        idx += 2;
    }
    let count =
        crate::grammar::primitives::probe_shape(leaf::parse_number_complete(words.get(idx)?))?;
    idx += 1;
    if !prefix_at_one_of(
        words,
        idx,
        &[&["more", "card", "in"], &["more", "cards", "in"]],
    ) {
        return None;
    }
    idx += 3;
    if permission_shapes::starts_at_words(words, idx, &["their"]) {
        idx += 1;
    }
    if !permission_shapes::starts_at_words(words, idx, &["hand"])
        || !permission_shapes::starts_at_words(words, idx + 1, &["than", "you"])
    {
        return None;
    }
    idx += 3;
    if permission_shapes::starts_at_words(words, idx, &["do"]) {
        idx += 1;
    }
    if idx < words.len()
        && !permission_shapes::exact_words(
            &words[idx..],
            &["as", "you", "activate", "this", "ability"],
        )
    {
        return None;
    }
    Some(PlayerFilter::CardsInHandAtLeastMoreThanYou {
        base: Box::new(base),
        count,
    })
}

pub fn parse_life_advantage_player(words: &[&str]) -> Option<PlayerFilter> {
    if permission_shapes::exact_words(
        words,
        &[
            "player", "with", "most", "life", "or", "tied", "for", "most", "life",
        ],
    ) || permission_shapes::exact_words(
        words,
        &[
            "the", "player", "with", "the", "most", "life", "or", "tied", "for", "most", "life",
        ],
    ) || permission_shapes::exact_words(
        words,
        &[
            "a", "player", "with", "the", "most", "life", "or", "tied", "for", "most", "life",
        ],
    ) {
        return Some(PlayerFilter::MostLifeTied);
    }
    let (base, mut idx) = player_base(words)?;
    if !starts_with_one_of_words(words, idx, &["who", "that"])
        || !starts_with_one_of_words(words, idx + 1, &["has", "have"])
    {
        return None;
    }
    idx += 2;
    if !permission_shapes::starts_at_words(words, idx, &["more", "life", "than", "you"]) {
        return None;
    }
    idx += 4;
    if permission_shapes::starts_at_words(words, idx, &["do"]) {
        idx += 1;
    }
    if idx < words.len()
        && !permission_shapes::exact_words(
            &words[idx..],
            &["as", "you", "activate", "this", "ability"],
        )
    {
        return None;
    }
    Some(PlayerFilter::HasMoreLifeThanYou {
        base: Box::new(base),
    })
}

#[cfg(test)]
#[path = "reference_shapes_inline_tests.rs"]
mod tests;

#[path = "reference_shapes/core.rs"]
mod core_programs;
use core_programs::{
    exact_one_of, find_one_of, first_word_offset, has_one_of_words, prefix_at_one_of,
    prefix_one_of, starts_with_one_of_words, suffix_one_of, word_has_char_suffix,
};
#[path = "reference_shapes/object_action.rs"]
mod object_action_programs;
use object_action_programs::is_controller_or_owner;
#[path = "reference_shapes/reference.rs"]
mod reference_programs;
use reference_programs::{
    filter_keyword_constraint_for_words, is_controlled_object_plural,
    is_that_player_or_object_controller, player_base,
};
