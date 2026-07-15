use crate::cards::builders::PlayerAst;
use crate::runtime_backend::grammar::{leaf, permission_shapes};
use crate::runtime_backend::lexer::{OwnedLexToken, TokenWordView};
use crate::static_abilities::StaticAbilityId;
use crate::target::PlayerFilter;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FilterKeywordConstraint {
    Static(StaticAbilityId),
    Marker(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SubjectAst {
    Player(PlayerAst),
    This,
}

pub(crate) fn contains_source_from_your_graveyard(words: &[&str]) -> bool {
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

pub(crate) fn contains_source_from_your_hand(words: &[&str]) -> bool {
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

pub(crate) fn contains_from_command_zone(words: &[&str]) -> bool {
    permission_shapes::find_words(words, &["from", "command", "zone"]).is_some()
}

pub(crate) fn contains_discard_source(words: &[&str]) -> bool {
    permission_shapes::find_words(words, &["discard", "this", "card"]).is_some()
}

pub(crate) fn is_source_from_your_graveyard(words: &[&str]) -> bool {
    words.len() >= 4
        && prefix_one_of(words, &[&["this"], &["thiss"]])
        && permission_shapes::find_words(words, &["from", "your", "graveyard"]).is_some()
        && has_one_of_words(words, &["card", "creature", "permanent"])
}

pub(crate) fn is_source_from_exile(words: &[&str]) -> bool {
    words.len() >= 3
        && prefix_one_of(words, &[&["this"], &["thiss"]])
        && permission_shapes::find_words(words, &["from", "exile"]).is_some()
        && has_one_of_words(words, &["card", "creature", "permanent", "source"])
}

pub(crate) fn parse_subject_tokens(tokens: &[OwnedLexToken]) -> SubjectAst {
    parse_subject_words(&TokenWordView::new(tokens).word_refs())
}

pub(crate) fn parse_subject_words(words: &[&str]) -> SubjectAst {
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
    while prefix_one_of(slice, &[&["then"], &["and"]]) {
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
    if is_that_player_or_object_controller(slice) {
        return SubjectAst::Player(PlayerAst::ThatPlayerOrTargetController);
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
    if permission_shapes::prefix_words(slice, &["its", "controller"])
        || (permission_shapes::prefix_words(slice, &["this"])
            && permission_shapes::suffix_words(slice, &["controller"]))
        || suffix_one_of(slice, &[&["its", "controller"], &["their", "controller"]])
    {
        return SubjectAst::Player(PlayerAst::ItsController);
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

pub(crate) fn parse_filter_keyword_constraint_words(
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

pub(crate) fn cycling_keyword_root(word: &str) -> Option<&str> {
    if permission_shapes::exact_words(&[word], &["cycling"]) {
        return Some("");
    }
    const SUFFIX: &[char] = &['c', 'y', 'c', 'l', 'i', 'n', 'g'];
    if word.chars().count() > SUFFIX.len() && word_has_char_suffix(word, SUFFIX) {
        return word.get(..word.len().saturating_sub(SUFFIX.len()));
    }
    None
}

pub(crate) fn parse_hand_advantage_player(words: &[&str]) -> Option<PlayerFilter> {
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
    let count = leaf::parse_number_complete(words.get(idx)?).ok()?;
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

pub(crate) fn parse_life_advantage_player(words: &[&str]) -> Option<PlayerFilter> {
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

fn filter_keyword_constraint_for_words(words: &[&str]) -> Option<FilterKeywordConstraint> {
    use FilterKeywordConstraint::{Marker, Static};
    let static_id = if permission_shapes::exact_words(words, &["flying"]) {
        Some(StaticAbilityId::Flying)
    } else if permission_shapes::exact_words(words, &["menace"]) {
        Some(StaticAbilityId::Menace)
    } else if permission_shapes::exact_words(words, &["hexproof"]) {
        Some(StaticAbilityId::Hexproof)
    } else if permission_shapes::exact_words(words, &["haste"]) {
        Some(StaticAbilityId::Haste)
    } else if permission_shapes::exact_words(words, &["first", "strike"]) {
        Some(StaticAbilityId::FirstStrike)
    } else if permission_shapes::exact_words(words, &["double", "strike"]) {
        Some(StaticAbilityId::DoubleStrike)
    } else if permission_shapes::exact_words(words, &["deathtouch"]) {
        Some(StaticAbilityId::Deathtouch)
    } else if permission_shapes::exact_words(words, &["lifelink"]) {
        Some(StaticAbilityId::Lifelink)
    } else if permission_shapes::exact_words(words, &["vigilance"]) {
        Some(StaticAbilityId::Vigilance)
    } else if permission_shapes::exact_words(words, &["trample"]) {
        Some(StaticAbilityId::Trample)
    } else if permission_shapes::exact_words(words, &["reach"]) {
        Some(StaticAbilityId::Reach)
    } else if permission_shapes::exact_words(words, &["defender"]) {
        Some(StaticAbilityId::Defender)
    } else if permission_shapes::exact_words(words, &["flash"]) {
        Some(StaticAbilityId::Flash)
    } else if permission_shapes::exact_words(words, &["phasing"]) {
        Some(StaticAbilityId::Phasing)
    } else if permission_shapes::exact_words(words, &["indestructible"]) {
        Some(StaticAbilityId::Indestructible)
    } else if permission_shapes::exact_words(words, &["shroud"]) {
        Some(StaticAbilityId::Shroud)
    } else if permission_shapes::exact_words(words, &["wither"]) {
        Some(StaticAbilityId::Wither)
    } else if permission_shapes::exact_words(words, &["infect"]) {
        Some(StaticAbilityId::Infect)
    } else if permission_shapes::exact_words(words, &["fear"]) {
        Some(StaticAbilityId::Fear)
    } else if permission_shapes::exact_words(words, &["intimidate"]) {
        Some(StaticAbilityId::Intimidate)
    } else if permission_shapes::exact_words(words, &["shadow"]) {
        Some(StaticAbilityId::Shadow)
    } else if permission_shapes::exact_words(words, &["horsemanship"]) {
        Some(StaticAbilityId::Horsemanship)
    } else if permission_shapes::exact_words(words, &["flanking"]) {
        Some(StaticAbilityId::Flanking)
    } else if permission_shapes::exact_words(words, &["skulk"]) {
        Some(StaticAbilityId::Skulk)
    } else if permission_shapes::exact_words(words, &["changeling"]) {
        Some(StaticAbilityId::Changeling)
    } else if permission_shapes::exact_words(words, &["cascade"]) {
        Some(StaticAbilityId::Cascade)
    } else if exact_one_of(
        words,
        &[
            &["landwalk"],
            &["nonbasic", "landwalk"],
            &["artifact", "landwalk"],
        ],
    ) {
        Some(StaticAbilityId::Landwalk)
    } else {
        None
    };
    if let Some(id) = static_id {
        return Some(Static(id));
    }
    if permission_shapes::exact_words(words, &["decayed"]) {
        Some(Marker("decayed"))
    } else if permission_shapes::exact_words(words, &["mutate"]) {
        // Costed keyword markers retain their full printed surface (for
        // example, `Mutate {4}{B}`), while ObjectFilter marker matching is
        // deliberately word-aware.  Keep the semantic marker cost-agnostic.
        Some(Marker("mutate"))
    } else if permission_shapes::exact_words(words, &["toxic"]) {
        Some(Marker("toxic"))
    } else if permission_shapes::exact_words(words, &["islandwalk"]) {
        Some(Marker("islandwalk"))
    } else if permission_shapes::exact_words(words, &["swampwalk"]) {
        Some(Marker("swampwalk"))
    } else if permission_shapes::exact_words(words, &["mountainwalk"]) {
        Some(Marker("mountainwalk"))
    } else if permission_shapes::exact_words(words, &["forestwalk"]) {
        Some(Marker("forestwalk"))
    } else if permission_shapes::exact_words(words, &["plainswalk"]) {
        Some(Marker("plainswalk"))
    } else {
        None
    }
}

fn player_base(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    if prefix_one_of(words, &[&["opponent"], &["opponents"]]) {
        Some((PlayerFilter::Opponent, 1))
    } else if prefix_one_of(words, &[&["player"], &["players"]]) {
        Some((PlayerFilter::Any, 1))
    } else {
        None
    }
}

fn is_that_player_or_object_controller(words: &[&str]) -> bool {
    words.len() >= 6
        && permission_shapes::prefix_words(words, &["that", "player", "or", "that"])
        && is_controlled_object_plural(words[4])
        && permission_shapes::exact_words(&words[5..6], &["controller"])
}

fn is_controlled_object_plural(word: &str) -> bool {
    starts_with_one_of_words(
        &[word],
        0,
        &[
            "creatures",
            "permanents",
            "planeswalkers",
            "sources",
            "spells",
        ],
    )
}

fn is_controller_or_owner(word: &str) -> bool {
    starts_with_one_of_words(&[word], 0, &["controller", "owner"])
}

fn word_has_char_suffix(word: &str, suffix: &[char]) -> bool {
    let mut chars = word.chars().rev();
    suffix
        .iter()
        .rev()
        .all(|expected| chars.next().is_some_and(|ch| ch == *expected))
}

fn find_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::find_words(words, expected).is_some())
}

fn has_one_of_words(words: &[&str], alternatives: &[&str]) -> bool {
    alternatives
        .iter()
        .any(|word| permission_shapes::find_words(words, &[*word]).is_some())
}

fn first_word_offset(words: &[&str], alternatives: &[&str]) -> Option<usize> {
    alternatives
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
}

fn prefix_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::prefix_words(words, expected))
}

fn prefix_at_one_of(words: &[&str], offset: usize, alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::starts_at_words(words, offset, expected))
}

fn suffix_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::suffix_words(words, expected))
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

fn starts_with_one_of_words(words: &[&str], offset: usize, alternatives: &[&str]) -> bool {
    alternatives
        .iter()
        .any(|word| permission_shapes::starts_at_words(words, offset, &[*word]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_subject_and_reference_surfaces() {
        assert_eq!(
            parse_subject_words(&["the", "player", "with", "the", "most", "life"]),
            SubjectAst::Player(PlayerAst::MostLifeTied)
        );
        assert!(contains_source_from_your_hand(&[
            "discard", "this", "card", "from", "your", "hand"
        ]));
        assert!(is_source_from_exile(&["this", "creature", "from", "exile"]));
    }

    #[test]
    fn parses_filter_keyword_and_player_advantage_surfaces() {
        assert_eq!(
            parse_filter_keyword_constraint_words(&["basic", "landcycling"]),
            Some((FilterKeywordConstraint::Marker("cycling"), 2))
        );
        assert_eq!(
            parse_filter_keyword_constraint_words(&["cascade"]),
            Some((FilterKeywordConstraint::Static(StaticAbilityId::Cascade), 1))
        );
        assert_eq!(
            parse_filter_keyword_constraint_words(&["toxic"]),
            Some((FilterKeywordConstraint::Marker("toxic"), 1))
        );
        assert_eq!(
            parse_life_advantage_player(&["opponent", "who", "has", "more", "life", "than", "you"]),
            Some(PlayerFilter::HasMoreLifeThanYou {
                base: Box::new(PlayerFilter::Opponent),
            })
        );
    }
}
