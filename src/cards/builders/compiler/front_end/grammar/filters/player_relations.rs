type GrammarFilterNormalizedWords<'a> = TokenWordView<'a>;

type FilterWordInput<'a> = primitives::WordSliceInput<'a>;

fn synth_words_as_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
}

fn push_unique_filter_value<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    if !items.iter().any(|item| *item == value) {
        items.push(value);
    }
}

fn parse_filter_prefix_words<'a, O>(
    words: &'a [&'a str],
    mut parser: impl Parser<FilterWordInput<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, usize)> {
    let mut input = words;
    let parsed = parser.parse_next(&mut input).ok()?;
    Some((parsed, words.len().saturating_sub(input.len())))
}

#[derive(Clone, Copy)]
enum SpellFilterComparisonAxis {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Clone, Copy)]
enum PlayerRelationVerb {
    Cast,
    Control,
    Own,
}

#[derive(Clone, Copy)]
struct SegmentPhraseVariant {
    words: &'static [&'static str],
    drain_start_offset: usize,
}

impl SpellFilterComparisonAxis {
    fn as_str(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Toughness => "toughness",
            Self::ManaValue => "mana value",
        }
    }

    fn assign(self, filter: &mut ObjectFilter, comparison: crate::target::Comparison) {
        match self {
            Self::Power => filter.power = Some(comparison),
            Self::Toughness => filter.toughness = Some(comparison),
            Self::ManaValue => filter.mana_value = Some(comparison),
        }
    }
}

fn parse_spell_filter_comparison_axis_words(
    words: &[&str],
) -> Option<(SpellFilterComparisonAxis, usize)> {
    parse_filter_prefix_words(
        words,
        alt((
            primitives::word_slice_eq("power").value(SpellFilterComparisonAxis::Power),
            primitives::word_slice_eq("toughness").value(SpellFilterComparisonAxis::Toughness),
            (
                primitives::word_slice_eq("mana"),
                primitives::word_slice_eq("value"),
            )
                .value(SpellFilterComparisonAxis::ManaValue),
        )),
    )
}

fn parse_player_relation_verb(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    parse_filter_prefix_words(
        words,
        alt((
            alt((
                primitives::word_slice_eq("cast"),
                primitives::word_slice_eq("casts"),
            ))
            .value(PlayerRelationVerb::Cast),
            alt((
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("controls"),
            ))
            .value(PlayerRelationVerb::Control),
            alt((
                primitives::word_slice_eq("own"),
                primitives::word_slice_eq("owns"),
            ))
            .value(PlayerRelationVerb::Own),
        )),
    )
}

fn parse_player_relation_subject(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    if let Some((_, consumed)) = parse_filter_prefix_words(words, primitives::word_slice_eq("you"))
    {
        return Some((PlayerFilter::You, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            primitives::word_slice_eq("opponent"),
            primitives::word_slice_eq("opponents"),
        )),
    ) {
        return Some((PlayerFilter::Opponent, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(words, primitives::word_slice_eq("they"))
    {
        return Some((pronoun_player_filter.clone(), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("team"),
        ),
    ) {
        return Some((PlayerFilter::You, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("opponents"),
        ),
    ) {
        return Some((PlayerFilter::Opponent, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::IteratedPlayer, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("target"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::target_player(), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("target"),
            primitives::word_slice_eq("opponent"),
        ),
    ) {
        return Some((PlayerFilter::target_opponent(), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("defending"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::Defending, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("attacking"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::Attacking, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            alt((
                primitives::word_slice_eq("its"),
                primitives::word_slice_eq("their"),
            )),
            alt((
                primitives::word_slice_eq("controller"),
                primitives::word_slice_eq("controllers"),
            )),
        ),
    ) {
        return Some((
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            consumed,
        ));
    }

    None
}

fn apply_player_relation(
    filter: &mut ObjectFilter,
    player: PlayerFilter,
    verb: PlayerRelationVerb,
) {
    match verb {
        PlayerRelationVerb::Cast => filter.cast_by = Some(player),
        PlayerRelationVerb::Control => filter.controller = Some(player),
        PlayerRelationVerb::Own => filter.owner = Some(player),
    }
}

fn try_apply_player_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    let (verb, verb_consumed) = parse_player_relation_verb(&words[subject_consumed..])?;

    if matches!(player, PlayerFilter::Defending | PlayerFilter::Attacking)
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }
    if matches!(player, PlayerFilter::ControllerOf(_))
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }

    apply_player_relation(filter, player, verb);
    Some(subject_consumed + verb_consumed)
}

fn try_apply_negated_you_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            alt((
                primitives::word_slice_eq("dont"),
                primitives::word_slice_eq("don't"),
            )),
            alt((
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("controls"),
            )),
        ),
    ) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            alt((
                primitives::word_slice_eq("dont"),
                primitives::word_slice_eq("don't"),
            )),
            alt((
                primitives::word_slice_eq("own"),
                primitives::word_slice_eq("owns"),
            )),
        ),
    ) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            primitives::word_slice_eq("do"),
            primitives::word_slice_eq("not"),
            alt((
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("controls"),
            )),
        ),
    ) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            primitives::word_slice_eq("do"),
            primitives::word_slice_eq("not"),
            alt((
                primitives::word_slice_eq("own"),
                primitives::word_slice_eq("owns"),
            )),
        ),
    ) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }

    None
}

fn try_apply_chosen_player_graveyard_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("chosen"),
                alt((
                    primitives::word_slice_eq("player"),
                    primitives::word_slice_eq("players"),
                )),
                primitives::word_slice_eq("graveyard"),
            )
                .void(),
            (
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("chosen"),
                alt((
                    primitives::word_slice_eq("player"),
                    primitives::word_slice_eq("players"),
                )),
                primitives::word_slice_eq("graveyard"),
            )
                .void(),
        )),
    ) {
        filter.owner = Some(PlayerFilter::ChosenPlayer);
        filter.zone = Some(Zone::Graveyard);
        return Some(consumed);
    }

    None
}

fn try_apply_joint_owner_controller_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    let (_, consumed) = parse_filter_prefix_words(
        &words[subject_consumed..],
        alt((
            (
                primitives::word_slice_eq("both"),
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
                primitives::word_slice_eq("and"),
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
            ),
            (
                primitives::word_slice_eq("both"),
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
                primitives::word_slice_eq("and"),
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
            ),
        )),
    )?;
    filter.owner = Some(player.clone());
    filter.controller = Some(player);
    Some(subject_consumed + consumed)
}

fn parse_owner_or_controller_disjunction_player(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    if matches!(
        player,
        PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::ControllerOf(_)
    ) {
        return None;
    }
    let (_, consumed) = parse_filter_prefix_words(
        &words[subject_consumed..],
        alt((
            (
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
                primitives::word_slice_eq("or"),
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
            ),
            (
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
                primitives::word_slice_eq("or"),
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
            ),
        )),
    )?;
    Some((player, subject_consumed + consumed))
}

fn find_filter_prefix_consumed<F>(words: &[&str], parser: F) -> Option<(usize, usize)>
where
    F: Fn(&[&str]) -> Option<usize>,
{
    words
        .iter()
        .enumerate()
        .find_map(|(idx, _)| parser(&words[idx..]).map(|consumed| (idx, consumed)))
}

fn drain_segment_phrase_variants(
    segment_tokens: &mut Vec<OwnedLexToken>,
    variants: &[SegmentPhraseVariant],
) {
    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let segment_match = variants.iter().find_map(|variant| {
        find_word_slice_phrase_start(&segment_words, variant.words).map(|seg_start| {
            (
                seg_start + variant.drain_start_offset,
                seg_start + variant.words.len(),
            )
        })
    });
    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(start_token_idx) =
            normalized_token_index_for_word_index(segment_tokens.as_slice(), start_word_idx)
    {
        let end_token_idx =
            normalized_token_index_after_words(segment_tokens.as_slice(), end_word_idx)
                .unwrap_or(segment_tokens.len());
        segment_tokens.drain(start_token_idx..end_token_idx);
    }
}

fn parse_put_there_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            alt((
                primitives::word_slice_eq("was"),
                primitives::word_slice_eq("were"),
            )),
            primitives::word_slice_eq("put"),
            primitives::word_slice_eq("there"),
            primitives::word_slice_eq("from"),
            primitives::word_slice_eq("battlefield"),
            primitives::word_slice_eq("this"),
            primitives::word_slice_eq("turn"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn parse_put_there_from_anywhere_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            alt((
                primitives::word_slice_eq("was"),
                primitives::word_slice_eq("were"),
            )),
            primitives::word_slice_eq("put"),
            primitives::word_slice_eq("there"),
            primitives::word_slice_eq("from"),
            primitives::word_slice_eq("anywhere"),
            primitives::word_slice_eq("this"),
            primitives::word_slice_eq("turn"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn parse_graveyard_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            alt((
                primitives::word_slice_eq("graveyard"),
                primitives::word_slice_eq("graveyards"),
            )),
            primitives::word_slice_eq("from"),
            primitives::word_slice_eq("battlefield"),
            primitives::word_slice_eq("this"),
            primitives::word_slice_eq("turn"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn parse_entered_battlefield_this_turn_words(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                primitives::word_slice_eq("your"),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                primitives::word_slice_eq("your"),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
        )),
    ) {
        return Some((Some(PlayerFilter::You), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                alt((
                    primitives::word_slice_eq("opponent"),
                    primitives::word_slice_eq("opponents"),
                )),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                alt((
                    primitives::word_slice_eq("opponent"),
                    primitives::word_slice_eq("opponents"),
                )),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
        )),
    ) {
        return Some((Some(PlayerFilter::Opponent), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
        )),
    ) {
        return Some((None, consumed));
    }

    None
}

fn try_apply_put_there_from_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_put_there_from_battlefield_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    filter.entered_graveyard_from_battlefield_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "was",
                    "put",
                    "there",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "was",
                    "put",
                    "there",
                    "from",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "were",
                    "put",
                    "there",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "were",
                    "put",
                    "there",
                    "from",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

fn try_apply_put_there_from_anywhere_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_put_there_from_anywhere_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "that", "was", "put", "there", "from", "anywhere", "this", "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that", "were", "put", "there", "from", "anywhere", "this", "turn",
                ],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

fn try_apply_graveyard_from_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_graveyard_from_battlefield_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_from_battlefield_this_turn = true;
    all_words.drain(word_start + 1..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &["graveyard", "from", "the", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyard", "from", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyards", "from", "the", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyards", "from", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
        ],
    );
    true
}

fn try_apply_entered_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, (controller, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_entered_battlefield_this_turn_words(&all_words[idx..])
                .map(|matched| (idx, matched))
        })
    else {
        return false;
    };
    filter.entered_battlefield_this_turn = true;
    filter.entered_battlefield_controller = controller;
    filter.zone = Some(Zone::Battlefield);
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "the", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

fn push_it_tagged_object_constraint(filter: &mut ObjectFilter) {
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
}

fn try_apply_leading_tagged_reference_prefix(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    if all_words.len() >= 2 && matches!(all_words[0], "that" | "those" | "chosen") {
        let noun_idx = if all_words.get(1).is_some_and(|word| *word == "other") {
            2
        } else {
            1
        };
        if all_words
            .get(noun_idx)
            .is_some_and(|word| is_demonstrative_object_head(word))
        {
            push_it_tagged_object_constraint(filter);
            all_words.remove(0);
            return true;
        }
    }

    if all_words
        .first()
        .is_some_and(|word| matches!(*word, "it" | "them"))
    {
        push_it_tagged_object_constraint(filter);
        all_words.remove(0);
        return true;
    }

    false
}

fn is_name_clause_boundary(word: &str) -> bool {
    matches!(
        word,
        "in" | "from"
            | "with"
            | "without"
            | "that"
            | "which"
            | "who"
            | "whose"
            | "under"
            | "among"
            | "on"
            | "you"
            | "your"
            | "opponent"
            | "opponents"
            | "their"
            | "its"
            | "controller"
            | "controllers"
            | "owner"
            | "owners"
    )
}

fn find_name_clause_end(all_words: &[&str], name_start: usize) -> usize {
    let mut name_end = all_words.len();
    for idx in (name_start + 1)..all_words.len() {
        if is_name_clause_boundary(all_words[idx]) {
            name_end = idx;
            break;
        }
    }
    name_end
}

fn extract_name_clause_text<'a, F, G>(
    all_words: &[&'a str],
    all_words_with_articles: &[&'a str],
    marker_idx: usize,
    marker_len: usize,
    map_non_article_index: &F,
    map_non_article_end: &G,
    error_label: &str,
) -> Result<(String, usize), CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let name_start = marker_idx + marker_len;
    let name_end = find_name_clause_end(all_words, name_start);
    let full_marker_idx = map_non_article_index(marker_idx).unwrap_or(marker_idx);
    let full_name_end = map_non_article_end(name_end).unwrap_or(name_end);
    let name_words = if full_marker_idx + marker_len <= full_name_end
        && full_name_end <= all_words_with_articles.len()
    {
        &all_words_with_articles[full_marker_idx + marker_len..full_name_end]
    } else {
        &all_words[name_start..name_end]
    };
    if name_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing card name in {error_label} object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }

    Ok((name_words.join(" "), name_end))
}

