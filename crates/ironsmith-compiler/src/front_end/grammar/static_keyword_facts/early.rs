use std::ops::Range;

use crate::front_end::lexer::{OwnedLexToken, TokenWordView};

use crate::grammar::{
    keyword_static_lines, primitives, static_keyword_line_shapes,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifeTotalCostConditionShape {
    pub(crate) quantity_tokens: Range<usize>,
    pub(crate) quantity_words: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EarlyKeywordMarkerKind {
    ToughnessCrewsVehicles,
    GreaterPowerCrewsVehicles,
    LoyaltyCounterPaysCrewCost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DamageDoublingManaValueMarker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PregameMulliganRedraw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WardDiscardHandTail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntersTappedChoiceShape {
    pub(crate) tapped_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseCardNameTail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NoteLifeTotalTail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NamedChoiceAlternativesShape {
    pub(crate) choice_word: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CountAsCardNamedShape {
    pub(crate) spell_name_words: Range<usize>,
    pub(crate) counted_name_words: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerDuplicationSourceOrOwnedEmblem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerDuplicationPlayerSubject {
    Any,
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerDuplicationEventKind {
    DealsCombatDamageToPlayer,
    BecomesTargeted,
    IsDealtDamage,
    EntersOrLeavesBattlefield,
    EntersBattlefield,
    LeavesBattlefield,
    DrawsCard(Option<TriggerDuplicationPlayerSubject>),
    Attacks,
    Dies,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TriggerDuplicationEventShape {
    TurningFaceUp {
        subject_tokens: Range<usize>,
    },
    YouCastingOrCopying {
        subject_tokens: Range<usize>,
    },
    SubjectEvent {
        subject_tokens: Range<usize>,
        kind: TriggerDuplicationEventKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerDuplicationEventSyntaxError {
    MissingTurnedFaceUpSubject,
    MissingSpellSubject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerDuplicationCoreShape<'a> {
    AbilityTriggers {
        source_tokens: &'a [OwnedLexToken],
        condition_tokens: Option<&'a [OwnedLexToken]>,
    },
    EventCausesAbility {
        event_tokens: &'a [OwnedLexToken],
        source_tokens: &'a [OwnedLexToken],
    },
}

pub(crate) fn parse_life_total_cost_condition_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LifeTotalCostConditionShape> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let quantity_words = if words.len() >= 4
        && prefix(&words, &["you", "have"])
        && matches!(words.last().copied(), Some("life"))
    {
        2..words.len().saturating_sub(1)
    } else if words.len() >= 6 && prefix(&words, &["your", "life", "total", "is"]) {
        4..words.len()
    } else {
        return None;
    };
    Some(LifeTotalCostConditionShape {
        quantity_words: quantity_words.len(),
        quantity_tokens: token_range_for_words(tokens, &view, quantity_words)?,
    })
}

pub(crate) fn parse_early_keyword_marker_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EarlyKeywordMarkerKind> {
    let words = TokenWordView::new(tokens).word_refs();
    if exact_any(
        &words,
        &[
            &[
                "this",
                "creature",
                "crews",
                "vehicles",
                "using",
                "its",
                "toughness",
                "rather",
                "than",
                "its",
                "power",
            ],
            &[
                "this",
                "creature",
                "saddles",
                "mounts",
                "and",
                "crews",
                "vehicles",
                "using",
                "its",
                "toughness",
                "rather",
                "than",
                "its",
                "power",
            ],
        ],
    ) {
        return Some(EarlyKeywordMarkerKind::ToughnessCrewsVehicles);
    }
    if prefix_any(
        &words,
        &[
            &[
                "this", "creature", "crews", "vehicles", "as", "though", "its", "power", "were",
            ],
            &[
                "this", "creature", "saddles", "mounts", "and", "crews", "vehicles", "as",
                "though", "its", "power", "were",
            ],
            &[
                "this", "token", "saddles", "mounts", "and", "crews", "vehicles", "as", "though",
                "its", "power", "were",
            ],
        ],
    ) && suffix(&words, &["greater"])
    {
        return Some(EarlyKeywordMarkerKind::GreaterPowerCrewsVehicles);
    }
    (prefix(
        &words,
        &[
            "you",
            "may",
            "remove",
            "a",
            "loyalty",
            "counter",
            "from",
            "a",
            "planeswalker",
            "you",
            "control",
            "rather",
            "than",
            "pay",
        ],
    ) && suffix(&words, &["crew", "cost"]))
    .then_some(EarlyKeywordMarkerKind::LoyaltyCounterPaysCrewCost)
}

pub(crate) fn parse_count_as_card_named_shape_words(
    words: &[&str],
) -> Option<CountAsCardNamedShape> {
    if !prefix_any(
        words,
        &[
            &["if", "this", "card", "is", "in", "a", "graveyard"],
            &["if", "this", "card", "is", "in", "your", "graveyard"],
        ],
    ) {
        return None;
    }
    let effects =
        primitives::parse_word_sequence_span(words, &["effects", "from", "spells", "named"])?;
    let spell_name_start = effects.start + effects.len;
    let count_word = static_keyword_line_shapes::parse_count_as_card_count_word(
        words.get(spell_name_start..).unwrap_or_default(),
    )?
    .word
        + spell_name_start;
    if count_word <= spell_name_start
        || !words
            .get(count_word..count_word + 6)
            .is_some_and(|tail| exact(tail, &["count", "it", "as", "a", "card", "named"]))
        || count_word + 6 >= words.len()
    {
        return None;
    }
    Some(CountAsCardNamedShape {
        spell_name_words: spell_name_start..count_word,
        counted_name_words: count_word + 6..words.len(),
    })
}

pub(crate) fn parse_damage_doubling_mana_value_marker_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DamageDoublingManaValueMarker> {
    let words = TokenWordView::new(tokens).word_refs();
    (prefix(&words, &["if", "a", "source", "you", "control", "with"])
        && suffix(&words, &["instead"])
        && contains(&words, &["mana"])
        && contains(&words, &["value"])
        && contains(&words, &["double"])
        && (contains(&words, &["would", "deal", "damage", "to", "a"])
            || contains(&words, &["would", "deal", "damage", "to", "target"])))
    .then_some(DamageDoublingManaValueMarker)
}

pub(crate) fn parse_pregame_mulligan_redraw_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PregameMulliganRedraw> {
    let words = TokenWordView::new(tokens).word_refs();
    [
        &["any", "time", "you", "could", "mulligan"][..],
        &["is", "in", "your", "hand"][..],
        &[
            "you", "may", "exile", "all", "the", "cards", "from", "your", "hand",
        ][..],
        &["then", "draw", "that", "many", "cards"][..],
    ]
    .iter()
    .all(|phrase| contains(&words, phrase))
    .then_some(PregameMulliganRedraw)
}

pub(crate) fn parse_ward_discard_hand_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WardDiscardHandTail> {
    let words = TokenWordView::new(tokens).word_refs();
    exact(&words, &["your", "hand"]).then_some(WardDiscardHandTail)
}

pub(crate) fn parse_enters_tapped_choice_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersTappedChoiceShape> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !prefix(&words, &["this"])
        || !contains(&words, &["enters"])
        || !contains(&words, &["tapped"])
    {
        return None;
    }
    let tapped_word = primitives::parse_word_sequence_span(&words, &["tapped"])?.start;
    Some(EntersTappedChoiceShape {
        tapped_token: view.token_start_indices().get(tapped_word).copied()?,
    })
}

pub(crate) fn parse_choose_card_name_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChooseCardNameTail> {
    let words = TokenWordView::new(tokens).word_refs();
    exact(&words, &["choose", "a", "card", "name"]).then_some(ChooseCardNameTail)
}

pub(crate) fn parse_note_life_total_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NoteLifeTotalTail> {
    let words = TokenWordView::new(tokens).word_refs();
    exact(&words, &["note", "your", "life", "total"]).then_some(NoteLifeTotalTail)
}

pub(crate) fn parse_named_choice_alternatives_shape_words(
    words: &[&str],
) -> Option<NamedChoiceAlternativesShape> {
    let choice_word = static_keyword_line_shapes::parse_choice_word(words)?.word;
    let choice_words = words.get(choice_word..)?;
    (choice_words.len() >= 4 && contains(choice_words, &["or"]))
        .then_some(NamedChoiceAlternativesShape { choice_word })
}

pub(crate) fn parse_trigger_duplication_source_or_owned_emblem_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TriggerDuplicationSourceOrOwnedEmblem> {
    let words = TokenWordView::new(trim_punctuation_edges(tokens)).word_refs();
    exact_any(
        &words,
        &[
            &["this", "creature", "or", "an", "emblem", "you", "own"],
            &["this", "creature", "or", "emblem", "you", "own"],
        ],
    )
    .then_some(TriggerDuplicationSourceOrOwnedEmblem)
}

pub(crate) fn parse_trigger_duplication_event_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerDuplicationEventShape>, TriggerDuplicationEventSyntaxError> {
    let tokens = trim_punctuation_edges(tokens);
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();

    if prefix(&words, &["turning"]) && suffix(&words, &["face", "up"]) {
        if words.len() <= 3 {
            return Err(TriggerDuplicationEventSyntaxError::MissingTurnedFaceUpSubject);
        }
        return Ok(Some(TriggerDuplicationEventShape::TurningFaceUp {
            subject_tokens: token_range_for_words(tokens, &view, 1..words.len() - 2)
                .ok_or(TriggerDuplicationEventSyntaxError::MissingTurnedFaceUpSubject)?,
        }));
    }
    if prefix(&words, &["you", "casting", "or", "copying"]) {
        if words.len() <= 4 {
            return Err(TriggerDuplicationEventSyntaxError::MissingSpellSubject);
        }
        return Ok(Some(TriggerDuplicationEventShape::YouCastingOrCopying {
            subject_tokens: token_range_for_words(tokens, &view, 4..words.len())
                .ok_or(TriggerDuplicationEventSyntaxError::MissingSpellSubject)?,
        }));
    }

    let suffixes: &[(&[&str], TriggerDuplicationEventKind)] = &[
        (
            &["dealing", "combat", "damage", "to", "a", "player"],
            TriggerDuplicationEventKind::DealsCombatDamageToPlayer,
        ),
        (
            &[
                "becoming", "the", "target", "of", "a", "spell", "or", "ability",
            ],
            TriggerDuplicationEventKind::BecomesTargeted,
        ),
        (
            &["being", "dealt", "damage"],
            TriggerDuplicationEventKind::IsDealtDamage,
        ),
        (
            &["entering", "or", "leaving", "the", "battlefield"],
            TriggerDuplicationEventKind::EntersOrLeavesBattlefield,
        ),
        (
            &["entering", "the", "battlefield"],
            TriggerDuplicationEventKind::EntersBattlefield,
        ),
        (
            &["leaving", "the", "battlefield"],
            TriggerDuplicationEventKind::LeavesBattlefield,
        ),
        (
            &["drawing", "a", "card"],
            TriggerDuplicationEventKind::DrawsCard(None),
        ),
        (&["attacking"], TriggerDuplicationEventKind::Attacks),
        (&["dying"], TriggerDuplicationEventKind::Dies),
        (
            &["entering"],
            TriggerDuplicationEventKind::EntersBattlefield,
        ),
    ];

    for (event_suffix, mut kind) in suffixes.iter().copied() {
        if !suffix(&words, event_suffix) || words.len() <= event_suffix.len() {
            continue;
        }
        let subject_words = 0..words.len() - event_suffix.len();
        if matches!(kind, TriggerDuplicationEventKind::DrawsCard(_)) {
            let subject = words.get(subject_words.clone()).unwrap_or_default();
            kind = TriggerDuplicationEventKind::DrawsCard(
                parse_trigger_duplication_player_subject_words(subject),
            );
        }
        return Ok(Some(TriggerDuplicationEventShape::SubjectEvent {
            subject_tokens: token_range_for_words(tokens, &view, subject_words)
                .expect("nonempty trigger-duplication subject range"),
            kind,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_trigger_duplication_core_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TriggerDuplicationCoreShape<'_>> {
    let tokens = trim_punctuation_edges(tokens);
    let segments = primitives::split_lexed_slices_on_comma(tokens);
    if segments.len() != 2 {
        return None;
    }
    let head_tokens = trim_punctuation_edges(segments[0]);
    let tail_tokens = trim_punctuation_edges(segments[1]);
    if head_tokens.is_empty()
        || tail_tokens.is_empty()
        || !keyword_static_lines::parse_trigger_duplication_tail_tokens(tail_tokens)
    {
        return None;
    }

    let head_view = TokenWordView::new(head_tokens);
    let head_words = head_view.word_refs();
    if !prefix(&head_words, &["if"]) || head_words.len() < 2 {
        return None;
    }
    let body_range = token_range_for_words(head_tokens, &head_view, 1..head_words.len())?;
    let body_tokens = &head_tokens[body_range];
    let body_view = TokenWordView::new(body_tokens);
    let body_words = body_view.word_refs();
    let ability_prefixes: &[&[&str]] = &[
        &["a", "triggered", "ability", "of"],
        &["an", "ability", "of"],
    ];

    for ability_prefix in ability_prefixes {
        if !prefix(&body_words, ability_prefix) || body_words.len() <= ability_prefix.len() + 1 {
            continue;
        }
        let Some(triggers_word) =
            static_keyword_line_shapes::parse_trigger_duplication_triggers_word(&body_words)
                .map(|boundary| boundary.word)
        else {
            continue;
        };
        if triggers_word <= ability_prefix.len() {
            continue;
        }
        let condition_tokens = if body_words
            .get(triggers_word + 1..)
            .is_some_and(|tail| prefix(tail, &["while"]))
        {
            let range = token_range_for_words(
                body_tokens,
                &body_view,
                triggers_word + 2..body_words.len(),
            )?;
            Some(&body_tokens[range])
        } else if triggers_word + 1 == body_words.len() {
            None
        } else {
            continue;
        };
        let source_range =
            token_range_for_words(body_tokens, &body_view, ability_prefix.len()..triggers_word)?;
        return Some(TriggerDuplicationCoreShape::AbilityTriggers {
            source_tokens: &body_tokens[source_range],
            condition_tokens,
        });
    }

    let causes_word =
        static_keyword_line_shapes::parse_trigger_duplication_causes_word(&body_words)?.word;
    let event_range = token_range_for_words(body_tokens, &body_view, 0..causes_word)?;
    let source_body_range =
        token_range_for_words(body_tokens, &body_view, causes_word + 1..body_words.len())?;
    let source_body_tokens = &body_tokens[source_body_range];
    let source_view = TokenWordView::new(source_body_tokens);
    let source_words = source_view.word_refs();
    for ability_prefix in ability_prefixes {
        if !prefix(&source_words, ability_prefix)
            || source_words.len() <= ability_prefix.len() + 2
            || !suffix(&source_words, &["to", "trigger"])
        {
            continue;
        }
        let source_range = token_range_for_words(
            source_body_tokens,
            &source_view,
            ability_prefix.len()..source_words.len() - 2,
        )?;
        return Some(TriggerDuplicationCoreShape::EventCausesAbility {
            event_tokens: &body_tokens[event_range],
            source_tokens: &source_body_tokens[source_range],
        });
    }
    None
}

fn parse_trigger_duplication_player_subject_words(
    words: &[&str],
) -> Option<TriggerDuplicationPlayerSubject> {
    if exact_any(words, &[&["a", "player"], &["player"]]) {
        Some(TriggerDuplicationPlayerSubject::Any)
    } else if exact(words, &["you"]) {
        Some(TriggerDuplicationPlayerSubject::You)
    } else if exact_any(words, &[&["an", "opponent"], &["opponent"]]) {
        Some(TriggerDuplicationPlayerSubject::Opponent)
    } else {
        None
    }
}

fn exact(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_complete(words, expected).is_some()
}

fn exact_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| exact(words, expected))
}

fn prefix(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_prefix(words, expected).is_some()
}

fn prefix_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| prefix(words, expected))
}

fn suffix(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_suffix(words, expected).is_some()
}

fn contains(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_span(words, expected).is_some()
}

fn token_range_for_words(
    tokens: &[OwnedLexToken],
    view: &TokenWordView<'_>,
    words: Range<usize>,
) -> Option<Range<usize>> {
    if words.start >= words.end {
        return None;
    }
    let start = view.token_start_indices().get(words.start).copied()?;
    let end = view
        .token_index_after_words(words.end)
        .unwrap_or(tokens.len());
    (start <= end).then_some(start..end)
}

fn trim_punctuation_edges(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens
        .first()
        .is_some_and(|token| token.is_comma() || token.is_period())
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| token.is_comma() || token.is_period())
    {
        tokens = &tokens[..tokens.len().saturating_sub(1)];
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{TokenWordView, lex_line};
    use super::*;

    #[test]
    fn early_marker_and_life_total_shapes_are_typed() {
        let marker = lex_line(
            "This creature crews Vehicles using its toughness rather than its power.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_early_keyword_marker_tokens(&marker),
            Some(EarlyKeywordMarkerKind::ToughnessCrewsVehicles)
        );

        let life = lex_line("You have 5 or less life.", 0).unwrap();
        let shape = parse_life_total_cost_condition_shape_tokens(&life).unwrap();
        assert_eq!(
            TokenWordView::new(&life[shape.quantity_tokens]).word_refs(),
            ["5", "or", "less"]
        );
    }

    #[test]
    fn count_as_named_and_choice_tail_shapes_preserve_boundaries() {
        let words = [
            "if",
            "this",
            "card",
            "is",
            "in",
            "a",
            "graveyard",
            "effects",
            "from",
            "spells",
            "named",
            "alpha",
            "count",
            "it",
            "as",
            "a",
            "card",
            "named",
            "beta",
        ];
        let shape = parse_count_as_card_named_shape_words(&words).unwrap();
        assert_eq!(&words[shape.spell_name_words], ["alpha"]);
        assert_eq!(&words[shape.counted_name_words], ["beta"]);

        assert!(
            parse_named_choice_alternatives_shape_words(&["choose", "sun", "or", "moon"]).is_some()
        );
    }

    #[test]
    fn pregame_damage_and_enters_tapped_shapes_are_typed() {
        let pregame = lex_line(
            "Any time you could mulligan and this is in your hand, you may exile all the cards from your hand, then draw that many cards.",
            0,
        )
        .unwrap();
        assert!(parse_pregame_mulligan_redraw_tokens(&pregame).is_some());

        let damage = lex_line(
            "If a source you control with mana value 4 would deal damage to a target, double that damage instead.",
            0,
        )
        .unwrap();
        assert!(parse_damage_doubling_mana_value_marker_tokens(&damage).is_some());

        let tapped = lex_line("This enters tapped, choose a color.", 0).unwrap();
        let shape = parse_enters_tapped_choice_shape_tokens(&tapped).unwrap();
        assert_eq!(tapped[shape.tapped_token].as_word(), Some("tapped"));
    }

    #[test]
    fn trigger_duplication_event_and_core_shapes_are_typed() {
        let event = lex_line("a creature entering the battlefield", 0).unwrap();
        assert!(matches!(
            parse_trigger_duplication_event_shape_tokens(&event),
            Ok(Some(TriggerDuplicationEventShape::SubjectEvent {
                kind: TriggerDuplicationEventKind::EntersBattlefield,
                ..
            }))
        ));

        let core = lex_line(
            "If an ability of a creature you control triggers, that ability triggers an additional time.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_trigger_duplication_core_shape_tokens(&core),
            Some(TriggerDuplicationCoreShape::AbilityTriggers { .. })
        ));
    }
}
