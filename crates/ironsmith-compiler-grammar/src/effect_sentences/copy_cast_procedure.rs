//! Graveyard copy-cast procedures composed statement by statement.
//!
//! "Exile target instant or sorcery card from your graveyard. Copy it. You
//! may cast the copy." is an exile statement that binds the exiled card,
//! followed by a copy statement over it and a cast statement over the copy.
//! The exile sentence is the ordinary sentence grammar's; this module tags the
//! card it exiles and carries it to the sentences that follow, as
//! [`super::looked_procedure`] carries a viewed group. The copy is spelled as
//! the cast's copy mode — casting a copy of a card in exile — rather than as a
//! stack-spell copy, as the registry programs these statements replace spelled
//! it; how the copy was stated (in its own sentence, as "that card", with the
//! cast in the same sentence) survives as the cast's copy-instruction surface.

use super::dispatch_entry::SentenceInput;
use super::sequence_rules::generic_subject_verb_sequences::exiled_collections::{
    contains_word_phrase, tag_first_exile_in_effects,
};
use super::sequence_rules::generic_subject_verb_sequences::graveyard_copy_cast::{
    exact_single_card_copy_tag, exact_tagged_graveyard_exile_tag, exact_terminal_card_copy_tag,
    is_exact_graveyard_exile, is_exact_single_source_copy, is_exact_tagged_graveyard_exile,
    normalize_shared_graveyard_union_exile, retag_single_card_copy, take_binary_coordination,
};
use crate::activation_and_restrictions::trigger_subject_filters::MayCastTaggedSpec;
use crate::activation_and_restrictions::{
    build_may_cast_tagged_effect, parse_may_cast_it_sentence,
};
use crate::cards::builders::{
    CardTextError, ChoiceCount, ConditionalEffectAst, EffectAst, IfResultPredicate,
    ObjectChoiceEffectAst, ObjectFilter, PermissionEffectAst, PlayerAst,
};
use crate::grammar::effects::{CopyCardReferenceShape, parse_copy_card_reference_shape};
use crate::tag::{CompilerReferenceTag, TagKey};
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::util::helper_tag_for_tokens;
use crate::lexer::OwnedLexToken;
use crate::zone::Zone;
use ironsmith_core::effect::CopyInstructionSurface;

/// How the exiled card's copy was stated.
enum CopyStatement {
    /// In the exile sentence itself: "exile ... and copy it". The surface
    /// applies only when the coordination was a sentence boundary.
    Coordinated {
        surface: Option<CopyInstructionSurface>,
        sentence_boundary: bool,
    },
    /// In its own sentence: "Copy it." / "Copy that card."
    Separate(CopyInstructionSurface),
    /// "If you do, copy it." — the cast is contingent on the exile.
    Gated,
    /// "Copy it, then you may cast the copy." — the cast came with it.
    ThenCast,
    /// "Copy that card three times." — several copies of the one card
    /// (CR 707.12), each cast by the plural cast statement that follows.
    Times(CopyInstructionSurface, u32),
    /// "Copy them." / "Copy those cards." — one copy of each exiled card.
    Each,
    /// "For each card exiled this way, copy it, and you may cast the copy
    /// [without paying its mana cost]." (Mizzix's Mastery) — one copy of each
    /// exiled card, the cast stated with it.
    EachThenCast,
}

enum Exiled {
    /// One card exiled from a graveyard.
    Card { copy: Option<CopyStatement> },
    /// Several cards exiled at random; one chosen from among them is copied.
    Collection {
        chosen: Option<(ObjectFilter, TagKey, u32)>,
    },
}

/// The exiled card (or cards) an exile statement bound, and the statements
/// made over it so far.
pub(super) struct CopyCastGroup {
    /// The exile sentence's effects, the exile tagged with the group's tag.
    exile: Vec<EffectAst>,
    tag: TagKey,
    exiled: Exiled,
    cast: Option<MayCastTaggedSpec>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn is_reference_to(tag: &TagKey, exiled: &TagKey) -> bool {
    tag.as_str() == CompilerReferenceTag::It.as_str()
        || tag.as_str() == CompilerReferenceTag::PriorExiledCard.as_str()
        || tag == exiled
        || crate::util::is_sentence_helper_tag(tag, "exiled")
}

/// "You may cast [any number of] the copies [without paying their mana
/// costs]." — each copy may be cast in turn while the procedure resolves
/// (CR 707.12, CR 608.2g).
fn plural_cast_statement(sentence: &SentenceInput) -> Option<MayCastTaggedSpec> {
    let words = crate::lexer::parser_token_word_refs(sentence.lowered());
    let rest = words.strip_prefix(&["you", "may", "cast"][..])?;
    let rest = rest
        .strip_prefix(&["any", "number", "of"][..])
        .unwrap_or(rest);
    let rest = rest.strip_prefix(&["the", "copies"][..])?;
    let without_paying_mana_cost = match rest {
        [] => false,
        ["without", "paying", "their", "mana", "costs" | "cost"] => true,
        _ => return None,
    };
    Some(MayCastTaggedSpec {
        tag: CompilerReferenceTag::It.bind().key.clone(),
        alternative_cost: None,
        player: PlayerAst::Implicit,
        verb: crate::activation_and_restrictions::trigger_subject_filters::MayCastItVerb::Cast,
        as_copy: true,
        without_paying_mana_cost,
        copy_instruction_surface: None,
        predicate: None,
        cost_reduction: None,
    })
}

/// "Copy it N times." / "Copy that card N times." / "Copy them." /
/// "Copy those cards." over the exiled card or cards.
fn plural_copy_statement(sentence: &SentenceInput) -> Option<CopyStatement> {
    let words = crate::lexer::parser_token_word_refs(sentence.lowered());
    match words.as_slice() {
        ["copy", "them"] | ["copy", "those", "cards"] | ["copy", "those", "exiled", "cards"] => {
            Some(CopyStatement::Each)
        }
        ["copy", reference @ .., count, "times"] => {
            let surface = match reference {
                ["it"] => CopyInstructionSurface::SeparateIt,
                ["that", "card"] => CopyInstructionSurface::SeparateThatCard,
                _ => return None,
            };
            let count = crate::util::parse_number_word_u32(count)?;
            (count >= 2).then_some(CopyStatement::Times(surface, count))
        }
        _ => None,
    }
}

/// "For each card exiled this way, copy it, and you may cast the copy ..."
fn for_each_copy_then_cast(sentence: &SentenceInput) -> Option<MayCastTaggedSpec> {
    let tokens = sentence.lowered();
    let words = crate::lexer::parser_token_word_refs(tokens);
    const HEAD: &[&str] = &["for", "each", "card", "exiled", "this", "way", "copy", "it", "and"];
    if !words.starts_with(HEAD) {
        return None;
    }
    let cast_start = crate::lexer::TokenWordView::new(tokens)
        .token_span_for_words(0, HEAD.len())?
        .end;
    let cast_tokens = crate::util::trim_commas(&tokens[cast_start..]);
    let cast = parse_may_cast_it_sentence(&cast_tokens)?;
    (cast.as_copy && matches!(cast.player, PlayerAst::Implicit | PlayerAst::You)).then_some(cast)
}

/// "Copy each <cards>." / "Copy the <card>." naming the copied cards by a
/// filter rather than by an exile in a preceding sentence (Zethi, Arcane
/// Blademaster; Arcane Bombardment; Spellweaver Volute): the filter tokens
/// and whether the statement copies each matching card.
fn copy_filtered_cards_statement(sentence: &SentenceInput) -> Option<(&[OwnedLexToken], bool)> {
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence.lowered());
    let tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        crate::util::trim_commas(&tokens[1..])
    } else {
        tokens.to_vec()
    };
    let words = crate::lexer::parser_token_word_refs(&tokens);
    let plural = match words.as_slice() {
        ["copy", "each", _, ..] => true,
        ["copy", "the", _, ..] => false,
        _ => return None,
    };
    let start = crate::lexer::TokenWordView::new(sentence.lowered())
        .token_span_for_words(0, 2 + usize::from(sentence_starts_with_then(sentence)))?
        .end;
    let filter_tokens = crate::util::trim_edge_punctuation_tokens(&sentence.lowered()[start..]);
    (!filter_tokens.is_empty()).then_some((filter_tokens, plural))
}

fn sentence_starts_with_then(sentence: &SentenceInput) -> bool {
    sentence
        .lowered()
        .first()
        .is_some_and(|token| token.is_word("then"))
}

fn is_plural_copy(statement: &CopyStatement) -> bool {
    matches!(statement, CopyStatement::Times(..) | CopyStatement::Each)
}

/// "You may cast the copy [without paying its mana cost]."
fn cast_statement(sentence: &SentenceInput) -> Option<MayCastTaggedSpec> {
    let cast = parse_may_cast_it_sentence(sentence.lowered())?;
    (cast.as_copy && matches!(cast.player, PlayerAst::Implicit | PlayerAst::You)).then_some(cast)
}

/// "Copy it, then you may cast the copy."
fn copy_then_cast(sentence: &SentenceInput) -> Option<MayCastTaggedSpec> {
    let tokens = sentence.lowered();
    let then_idx = crate::slice_primitives::select_position(tokens, |token| token.is_word("then"))?;
    let copy_tokens = crate::util::trim_commas(&tokens[..then_idx]);
    if parse_copy_card_reference_shape(&copy_tokens) != Some(CopyCardReferenceShape::It) {
        return None;
    }
    let cast_tokens = crate::util::trim_commas(&tokens[then_idx + 1..]);
    let cast = parse_may_cast_it_sentence(&cast_tokens)?;
    (cast.as_copy && matches!(cast.player, PlayerAst::Implicit | PlayerAst::You)).then_some(cast)
}

/// "Copy it." / "Copy that card." over the exiled card.
fn separate_copy(sentence: &SentenceInput, exiled: &TagKey) -> Option<CopyInstructionSurface> {
    let effects = crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(
        sentence.lowered(),
    ))?;
    let [copy_effect] = effects.as_slice() else {
        return None;
    };
    let reference = parse_copy_card_reference_shape(sentence.lowered());
    if !(is_exact_single_source_copy(copy_effect) && reference.is_some()) {
        let copy_tag = exact_single_card_copy_tag(copy_effect)?;
        if !is_reference_to(&copy_tag, exiled) {
            return None;
        }
    }
    Some(if reference == Some(CopyCardReferenceShape::ThatCard) {
        CopyInstructionSurface::SeparateThatCard
    } else {
        CopyInstructionSurface::SeparateIt
    })
}

/// "If you do, copy it."
fn gated_copy(sentence: &SentenceInput, exiled: &TagKey) -> bool {
    let Some(effects) = crate::grammar::primitives::probe_shape(
        super::parse_effect_sentence_lexed(sentence.lowered()),
    ) else {
        return false;
    };
    let [
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: copy_effects,
        }),
    ] = effects.as_slice()
    else {
        return false;
    };
    let [copy_effect] = copy_effects.as_slice() else {
        return false;
    };
    exact_single_card_copy_tag(copy_effect).is_some_and(|copy_tag| {
        copy_tag.as_str() == CompilerReferenceTag::It.as_str()
            || copy_tag == *exiled
            || crate::util::is_sentence_helper_tag(&copy_tag, "exiled")
    })
}

/// The copy statement a sentence makes over the exiled card, if any.
fn copy_statement(sentence: &SentenceInput, exiled: &TagKey) -> Option<CopyStatement> {
    if let Some(plural) = plural_copy_statement(sentence) {
        return Some(plural);
    }
    if for_each_copy_then_cast(sentence).is_some() {
        return Some(CopyStatement::EachThenCast);
    }
    if copy_then_cast(sentence).is_some() {
        return Some(CopyStatement::ThenCast);
    }
    if gated_copy(sentence, exiled) {
        return Some(CopyStatement::Gated);
    }
    separate_copy(sentence, exiled).map(CopyStatement::Separate)
}

/// "Choose an instant or sorcery card exiled this way and copy it three
/// times." (Chandra, Pyromaster): the chosen card's filter tokens and the
/// number of copies.
fn choose_exiled_this_way_and_copy(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], u32)> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let exiled = words
        .windows(6)
        .position(|window| window == ["exiled", "this", "way", "and", "copy", "it"])?;
    if words.first() != Some(&"choose") {
        return None;
    }
    let times = match &words[exiled + 6..] {
        [] => 1,
        [count, "times"] => crate::util::parse_number_word_u32(count).filter(|count| *count >= 2)?,
        _ => return None,
    };
    let view = crate::lexer::TokenWordView::new(tokens);
    let span = view.token_span_for_words(1, exiled)?;
    Some((&tokens[span], times))
}

/// "Choose a noncreature, nonland card from among them and copy it."
fn choose_and_copy(
    sentence: &SentenceInput,
    exiled: &TagKey,
) -> Option<(ObjectFilter, TagKey, u32)> {
    let tokens = sentence.lowered();
    if !contains_word_phrase(tokens, &["and", "copy", "it"]) {
        return None;
    }
    let (filter_tokens, times) =
        if let Some((filter_tokens, times)) = choose_exiled_this_way_and_copy(tokens) {
            (filter_tokens, times)
        } else {
            let shape =
                crate::grammar::effects::control_copy_attach_shapes::parse_from_among_them_shape(
                    tokens,
                )?;
            (
                crate::util::strip_leading_token_words_any(shape.filter_tokens, &["choose"]),
                1,
            )
        };
    let mut filter = super::looked_cards_family::parse_looked_card_choice_filter(filter_tokens)?;
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: exiled.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    Some((
        filter,
        helper_tag_for_tokens(tokens, "chosen_exiled").into(),
        times,
    ))
}

/// The free cast that follows a chosen exiled card's copies: singular for one
/// copy, plural for several.
fn chosen_copy_cast(sentence: &SentenceInput, times: u32) -> Option<MayCastTaggedSpec> {
    let cast = if times > 1 {
        plural_cast_statement(sentence)
    } else {
        cast_statement(sentence)
    };
    cast.filter(|cast| cast.without_paying_mana_cost)
}

/// Tag the exile of one card from a graveyard, minting a tag when the
/// sentence grammar left it untagged.
fn tag_card_exile(exile: &mut EffectAst, sentence: &SentenceInput) -> Option<TagKey> {
    normalize_shared_graveyard_union_exile(exile);
    if let Some(tag) = exact_tagged_graveyard_exile_tag(exile) {
        return Some(tag);
    }
    if !is_exact_graveyard_exile(exile) {
        return None;
    }
    let tag = helper_tag_for_tokens(sentence.lowered(), "exiled");
    let plain = exile.clone();
    *exile = EffectAst::TagAffected {
        effect: Box::new(plain),
        tag: crate::tag::TagRef::of(tag.clone()),
    };
    Some(tag.key.clone())
}

/// Open a procedure at an exile sentence when the sentences that follow copy
/// the exiled card and cast the copy.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<CopyCastGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let following = sentences.get(sentence_idx + 2);
    // "Copy each exiled card you own with a kick counter on it. You may cast
    // the copies." — the copied cards are named by a filter (CR 707.12).
    if let Some((filter_tokens, plural)) = copy_filtered_cards_statement(sentence) {
        let cast = if plural {
            plural_cast_statement(next)
        } else {
            cast_statement(next)
        };
        if cast.is_some()
            && let Ok(filter) = crate::object_filters::parse_object_filter(filter_tokens, false)
            && let Some(zone) = filter.zone.filter(|zone| *zone != Zone::Battlefield)
        {
            let tag = helper_tag_for_tokens(sentence.lowered(), "copied");
            let select = EffectAst::subject_verb_tag_matching_objects(
                filter,
                vec![zone],
                crate::tag::TagRef::of(tag.clone()),
            );
            return Ok(Some(CopyCastGroup {
                exile: vec![select],
                tag: tag.key.clone(),
                exiled: Exiled::Card {
                    copy: Some(if plural {
                        CopyStatement::Each
                    } else {
                        CopyStatement::Separate(CopyInstructionSurface::SeparateThatCard)
                    }),
                },
                cast: None,
                first_sentence: sentence_idx,
                consumed: 1,
            }));
        }
    }
    let Some(effects) = crate::grammar::primitives::probe_shape(
        super::parse_effect_sentence_lexed(sentence.lowered()),
    ) else {
        return Ok(None);
    };
    let group = |exile, tag, exiled| CopyCastGroup {
        exile,
        tag,
        exiled,
        cast: None,
        first_sentence: sentence_idx,
        consumed: 1,
    };

    // One card exiled, its copy stated by the next sentence.
    if let [exile] = effects.as_slice() {
        let mut exile = exile.clone();
        if let Some(tag) = tag_card_exile(&mut exile, sentence) {
            let continues = match copy_statement(next, &tag) {
                Some(CopyStatement::ThenCast | CopyStatement::EachThenCast) => true,
                Some(statement) if is_plural_copy(&statement) => {
                    following.is_some_and(|third| plural_cast_statement(third).is_some())
                }
                Some(_) => following.is_some_and(|third| cast_statement(third).is_some()),
                None => false,
            };
            if continues {
                return Ok(Some(group(vec![exile], tag, Exiled::Card { copy: None })));
            }
        }
    }

    // One card exiled and copied in the same sentence; the cast follows.
    if cast_statement(next).is_some()
        && let Some((mut exile, mut copy, operator)) = take_binary_coordination(effects.clone())
    {
        normalize_shared_graveyard_union_exile(&mut exile);
        let bound = if let Some(tag) = exact_terminal_card_copy_tag(&copy)
            && is_exact_tagged_graveyard_exile(&exile, &tag)
        {
            Some((
                crate::tag::TagRef::of(tag),
                Some(CopyInstructionSurface::SeparateThatCard),
            ))
        } else if let Some(copy_tag) = exact_single_card_copy_tag(&copy)
            && (copy_tag.as_str() == CompilerReferenceTag::It.as_str()
                || copy_tag.as_str() == CompilerReferenceTag::PriorExiledCard.as_str())
            && is_exact_graveyard_exile(&exile)
        {
            let surface = if copy_tag.as_str() == CompilerReferenceTag::PriorExiledCard.as_str() {
                CopyInstructionSurface::SeparateThatCard
            } else {
                CopyInstructionSurface::SeparateIt
            };
            let tag = helper_tag_for_tokens(sentence.lowered(), "exiled");
            exile = EffectAst::TagAffected {
                effect: Box::new(exile),
                tag: crate::tag::TagRef::of(tag.clone()),
            };
            retag_single_card_copy(&mut copy, tag.clone().into()).then_some((tag, Some(surface)))
        } else {
            None
        };
        if let Some((tag, surface)) = bound {
            return Ok(Some(group(
                vec![exile],
                tag.key.clone(),
                Exiled::Card {
                    copy: Some(CopyStatement::Coordinated {
                        surface,
                        sentence_boundary: operator
                            == Some(crate::model::CoordinationOperatorAst::SentenceBoundary),
                    }),
                },
            )));
        }
    }

    // Several cards exiled at random; one is chosen and copied, then cast.
    let mut collection = effects;
    let tag = helper_tag_for_tokens(sentence.lowered(), "exiled");
    if tag_first_exile_in_effects(&mut collection, &tag)
        && let Some((_, _, times)) = choose_and_copy(next, &tag)
        && following.is_some_and(|third| chosen_copy_cast(third, times).is_some())
    {
        return Ok(Some(group(
            collection,
            tag.key.clone(),
            Exiled::Collection { chosen: None },
        )));
    }
    Ok(None)
}

/// Continue an open procedure with the next sentence. Returns false, leaving
/// the group untouched, when the sentence is not one of its statements.
pub(super) fn continue_with(
    group: &mut CopyCastGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.cast.is_some() {
        return Ok(false);
    }
    match &mut group.exiled {
        Exiled::Card { copy: None } => {
            let Some(statement) = copy_statement(sentence, &group.tag) else {
                return Ok(false);
            };
            if matches!(statement, CopyStatement::ThenCast) {
                group.cast = copy_then_cast(sentence);
            }
            if matches!(statement, CopyStatement::EachThenCast) {
                group.cast = for_each_copy_then_cast(sentence);
            }
            group.exiled = Exiled::Card {
                copy: Some(statement),
            };
        }
        Exiled::Card { copy: Some(statement) } => {
            let cast = if is_plural_copy(statement) {
                plural_cast_statement(sentence)
            } else {
                cast_statement(sentence)
            };
            let Some(cast) = cast else {
                return Ok(false);
            };
            group.cast = Some(cast);
        }
        Exiled::Collection { chosen: None } => {
            let Some(chosen) = choose_and_copy(sentence, &group.tag) else {
                return Ok(false);
            };
            group.exiled = Exiled::Collection {
                chosen: Some(chosen),
            };
        }
        Exiled::Collection {
            chosen: Some((_, _, times)),
        } => {
            let Some(cast) = chosen_copy_cast(sentence, *times) else {
                return Ok(false);
            };
            group.cast = Some(cast);
        }
    }
    group.consumed += 1;
    Ok(true)
}

/// The feature the registry programs these statements replace reported, which
/// the line grammar reads to admit the copy-cast reminder text and a cast
/// result tail after the procedure.
pub(super) fn feature_tag(group: &CopyCastGroup) -> &'static str {
    match group.exiled {
        Exiled::Card {
            copy: Some(CopyStatement::Gated),
        } => "conditional-graveyard-card-copy-cast",
        Exiled::Card { .. } => "graveyard-card-copy-cast",
        Exiled::Collection { .. } => "exiled-collection-copy-cast",
    }
}

/// Close the procedure: the tagged exile, then the cast of the copy, spelled
/// with the copy statement's surface and gated when the copy was.
pub(super) fn finish(group: CopyCastGroup) -> Vec<EffectAst> {
    let mut effects = group.exile;
    let Some(mut cast) = group.cast else {
        // The opener saw the cast statement ahead; a group closes without one
        // only when an intervening statement failed to continue it.
        return effects;
    };
    match group.exiled {
        Exiled::Card { copy } => {
            cast.tag = group.tag.clone();
            match copy {
                Some(CopyStatement::Coordinated {
                    surface,
                    sentence_boundary,
                }) => {
                    if sentence_boundary {
                        cast.copy_instruction_surface = surface;
                    }
                    effects.push(build_may_cast_tagged_effect(&cast));
                }
                Some(CopyStatement::Separate(surface)) => {
                    cast.copy_instruction_surface = Some(surface);
                    effects.push(build_may_cast_tagged_effect(&cast));
                }
                Some(CopyStatement::ThenCast) => {
                    cast.copy_instruction_surface = Some(CopyInstructionSurface::SeparateItThen);
                    effects.push(build_may_cast_tagged_effect(&cast));
                }
                Some(CopyStatement::Gated) => {
                    effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                        predicate: IfResultPredicate::Did,
                        effects: vec![build_may_cast_tagged_effect(&cast)],
                    }))
                }
                Some(CopyStatement::Times(surface, count)) => {
                    cast.copy_instruction_surface = Some(surface);
                    effects.push(EffectAst::ForEach(
                        crate::cards::builders::ForEachEffectAst::RepeatEffects {
                            count: crate::effect::Value::Fixed(count as i32),
                            effects: vec![build_may_cast_tagged_effect(&cast)],
                        },
                    ));
                }
                Some(CopyStatement::Each | CopyStatement::EachThenCast) => {
                    // Each exiled card is copied, and each copy may be cast
                    // in turn; the iteration binds the current card as "it".
                    cast.tag = CompilerReferenceTag::It.bind().key.clone();
                    effects.push(EffectAst::ForEach(
                        crate::cards::builders::ForEachEffectAst::ForEachTagged {
                            tag: crate::tag::TagRef::of(group.tag.clone()),
                            effects: vec![build_may_cast_tagged_effect(&cast)],
                        },
                    ));
                }
                None => effects.push(build_may_cast_tagged_effect(&cast)),
            }
        }
        Exiled::Collection { chosen } => {
            let Some((filter, chosen_tag, times)) = chosen else {
                return effects;
            };
            effects.push(EffectAst::ObjectChoices(
                ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                    filter,
                    count: ChoiceCount::exactly(1),
                    player: PlayerAst::You,
                    tag: crate::tag::TagRef::of(chosen_tag.clone()),
                    zone: Zone::Exile,
                },
            ));
            let may_cast = EffectAst::Permissions(PermissionEffectAst::May {
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    crate::tag::TagRef::of(chosen_tag),
                    cast.player,
                    false,
                    true,
                    true,
                    cast.cost_reduction,
                )],
            });
            effects.push(if times > 1 {
                // "copy it three times. You may cast the copies" (CR 707.12).
                EffectAst::ForEach(crate::cards::builders::ForEachEffectAst::RepeatEffects {
                    count: crate::effect::Value::Fixed(times as i32),
                    effects: vec![may_cast],
                })
            } else {
                may_cast
            });
        }
    }
    effects
}
