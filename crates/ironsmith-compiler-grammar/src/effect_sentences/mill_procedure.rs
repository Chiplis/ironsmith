//! Milled-group procedures composed statement by statement.
//!
//! "Mill three cards. You may put a creature card from among them into your
//! hand." is a mill statement that binds the milled cards, followed by a
//! statement over that group. The mill sentence is the ordinary sentence
//! grammar's; this module tags the cards it mills and carries that group to
//! the sentences that follow, as [`super::looked_procedure`] carries a viewed
//! group. Where the mill sentence was itself a result branch ("If you do, mill
//! three cards"), an unconditional follow-up joins that branch, as the registry
//! program it replaces had it.

use crate::cards::builders::ForEachEffectAst;
use super::dispatch_entry::{
    SentenceInput, parse_if_you_cant_sentence, parse_if_you_dont_sentence,
};
use super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::{
    compose_choose_from_looked_cards_into_hand_rest_into_graveyard, parse_optional_payment_sentence,
};
use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::{
    append_to_outer_if_result, parse_may_put_filtered_card_from_among_into_hand,
    parse_put_from_milled_cards_followup, tag_single_mill_effect,
};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IfResultPredicate, ObjectFilter, PlayerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, Value, LibraryActionAst, CharacteristicActionAst, ObjectChoiceEffectAst, ConditionalEffectAst, PermissionEffectAst,
};
use ironsmith_core::CardType;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::tag::TagKey;
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

/// The milled cards a mill statement bound, and the statements made over them.
pub(super) struct MilledGroup {
    /// The mill sentence's effect, its mill tagged with the group's tag.
    mill: EffectAst,
    /// The same effect untagged: the hand selection that waits for "if you
    /// don't" reads the milled cards through the prior-object reference, as
    /// its program did, and spells the mill as written.
    plain_mill: EffectAst,
    /// Whether the mill was a bare subject-verb sentence rather than nested in
    /// a "may" or a result branch.
    bare_mill: bool,
    player: PlayerAst,
    tag: TagKey,
    /// A selection into the hand that waits for the "if you don't" sentence
    /// before it is spelled.
    pending_hand: Option<(PlayerAst, ObjectFilter, Vec<OwnedTokens>)>,
    /// The hand selection and its "if you don't" were made: the mill is
    /// spelled untagged and the selection reads the prior object.
    hand_with_if_not: bool,
    followups: Vec<EffectAst>,
    /// The follow-up was itself under "if you do,".
    conditional_followup: bool,
    /// "Then you may pay {1} and 3 life." was made; "If you do, put a card
    /// from among those cards into your hand." follows.
    payment_made: bool,
    /// "Exile up to two creature cards put into graveyards this way." was made;
    /// the next sentence's total power reads the exiled cards.
    exiled_creatures: Option<TagKey>,
    /// The mill was "each player mills": only the exiled-creatures statement
    /// reads that group; the selections into one player's hand do not.
    per_player: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

type OwnedTokens = crate::lexer::OwnedLexToken;

fn mill_effect(
    sentence: &SentenceInput,
    tag: &TagKey,
) -> Option<(EffectAst, EffectAst, PlayerAst, bool, bool)> {
    let Ok(effects) = super::parse_effect_sentence_lexed(
        crate::util::trim_edge_punctuation_tokens(sentence.lowered()),
    ) else {
        return None;
    };
    let [effect] = effects.as_slice() else {
        return None;
    };
    let bare_mill = matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Library(LibraryActionAst::Mill { .. }),
            ..
        })
    );
    let plain = effect.clone();
    let mut effect = effect.clone();
    // "Each player mills three cards." mills once per player; the group is
    // every card milled, tagged on the mill inside the iteration.
    let (player, per_player) = if let EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) = &mut effect
        && let [inner] = effects.as_mut_slice()
    {
        (tag_single_mill_effect(inner, tag)?, true)
    } else {
        (tag_single_mill_effect(&mut effect, tag)?, false)
    };
    Some((effect, plain, player, bare_mill, per_player))
}

/// "You may cast an instant or sorcery spell [with mana value X or less] from
/// among them without paying its mana cost": the maximum mana value, if any.
fn may_cast_from_among(sentence: &SentenceInput) -> Option<Option<Value>> {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "you", "may", "cast", "an", "instant", "or", "sorcery", "spell", "from", "among",
            "them", "without", "paying", "its", "mana", "cost",
        ],
    ) {
        return Some(None);
    }
    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "you", "may", "cast", "an", "instant", "or", "sorcery", "spell", "with", "mana",
            "value", "x", "or", "less", "from", "among", "them", "without", "paying", "its",
            "mana", "cost",
        ],
    ) {
        return Some(Some(Value::X));
    }
    None
}

/// Choose an instant or sorcery card among the milled cards and cast it
/// without paying its mana cost.
fn cast_from_among(group: &MilledGroup, sentence: &SentenceInput, maximum: Option<Value>) -> Vec<EffectAst> {
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "chosen_milled_castable");
    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    if let Some(maximum) = maximum {
        let mut instant = ObjectFilter::default();
        instant.card_types = vec![CardType::Instant];
        let mut sorcery = ObjectFilter::default();
        sorcery.card_types = vec![CardType::Sorcery];
        let comparison = crate::filter::Comparison::LessThanOrEqualExpr(Box::new(maximum));
        instant.mana_value = Some(comparison.clone());
        sorcery.mana_value = Some(comparison);
        filter.any_of = vec![instant, sorcery];
    } else {
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    }
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: ChoiceCount::up_to(1),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            zone: Zone::Graveyard,
        }),
        EffectAst::subject_verb_cast_tagged(crate::tag::TagRef::of(chosen_tag), PlayerAst::You, false, false, true, None),
    ]
}

/// "If you do, put a card from among those cards into your hand."
fn if_you_do_put_from_among_into_hand(
    sentence: &SentenceInput,
    player: PlayerAst,
) -> Option<(PlayerAst, ObjectFilter, Vec<OwnedTokens>)> {
    let followup = crate::grammar::sentence_markers::parse_conditional_followup_tokens(sentence.lowered())?;
    if followup.actor != crate::grammar::sentence_markers::ConditionalFollowupActor::You {
        return None;
    }
    let tail = crate::util::trim_commas(followup.tail_tokens);
    let (chooser, filter) = crate::grammar::primitives::probe_shape(
        parse_may_put_filtered_card_from_among_into_hand(&tail, player, Zone::Graveyard),
    )??;
    Some((chooser, filter, tail))
}

/// Open a procedure at a mill sentence when the next sentence selects from
/// the milled cards.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<MilledGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let tag = helper_tag_for_tokens(sentence.lowered(), "milled");
    let Some((mill, plain_mill, player, bare_mill, per_player)) = mill_effect(sentence, &tag) else {
        return Ok(None);
    };
    let exiles_milled_creatures =
        crate::grammar::effects::triple_sequence_shapes::is_milled_creature_exile_shape(
            crate::util::trim_commas(next.lowered()).as_slice(),
        );
    if per_player && !exiles_milled_creatures {
        return Ok(None);
    }
    let continues = parse_put_from_milled_cards_followup(next.lowered(), player, tag.clone().into())?
        .is_some()
        || may_cast_from_among(next).is_some()
        || exiles_milled_creatures
        || (parse_optional_payment_sentence(next.lowered(), player)?.is_some()
            && sentences
                .get(sentence_idx + 2)
                .is_some_and(|third| if_you_do_put_from_among_into_hand(third, player).is_some()))
        || (bare_mill
            && parse_may_put_filtered_card_from_among_into_hand(
                next.lowered(),
                player,
                Zone::Graveyard,
            )?
            .is_some()
            && sentences.get(sentence_idx + 2).is_some_and(|third| {
                matches!(parse_if_you_dont_sentence(third.lowered()), Ok(Some(_)))
                    || matches!(parse_if_you_cant_sentence(third.lowered()), Ok(Some(_)))
            }));
    if !continues {
        return Ok(None);
    }
    Ok(Some(MilledGroup {
        mill,
        plain_mill,
        bare_mill,
        player,
        tag: tag.key.clone(),
        pending_hand: None,
        hand_with_if_not: false,
        followups: Vec::new(),
        conditional_followup: false,
        payment_made: false,
        exiled_creatures: None,
        per_player,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// Continue an open procedure with the next sentence.
pub(super) fn continue_with(
    group: &mut MilledGroup,
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
) -> Result<bool, CardTextError> {
    // "If you don't, ..." / "If you can't, ..." closes a pending hand selection.
    if let Some((chooser, filter, tokens)) = group.pending_hand.take() {
        let (if_not_chosen, count) =
            if let Some(effects) = parse_if_you_dont_sentence(sentence.lowered())? {
                (effects, ChoiceCount::up_to(1))
            } else if let Some(effects) = parse_if_you_cant_sentence(sentence.lowered())? {
                (effects, ChoiceCount::exactly(1))
            } else {
                return Ok(false);
            };
        let chosen_tag = helper_tag_for_tokens(&tokens, "chosen");
        group.hand_with_if_not = true;
        group.followups.extend(
            compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
                chooser,
                filter,
                (crate::tag::CompilerReferenceTag::It.bind()).into(),
                chosen_tag.key.clone(),
                Zone::Graveyard,
                false,
                if_not_chosen,
                count,
            ),
        );
        group.consumed += 1;
        return Ok(true);
    }
    if let Some(exiled_tag) = group.exiled_creatures.take() {
        // The sentence after the exile: its total power reads the exiled cards.
        let mut effects = super::parse_effect_sentence_lexed(sentence.lowered())?;
        if effects.is_empty() {
            return Ok(false);
        }
        for effect in &mut effects {
            rewrite_total_power_effect(effect, &exiled_tag);
        }
        group.followups.extend(effects);
        group.consumed += 1;
        return Ok(true);
    }
    if group.followups.is_empty()
        && crate::grammar::effects::triple_sequence_shapes::is_milled_creature_exile_shape(
            crate::util::trim_commas(sentence.lowered()).as_slice(),
        )
    {
        let exiled_tag = helper_tag_for_tokens(
            crate::util::trim_commas(sentence.lowered()).as_slice(),
            "exiled",
        );
        let mut milled_creature_filter =
            ObjectFilter::tagged(group.tag.clone()).in_zone(Zone::Graveyard);
        milled_creature_filter
            .card_types
            .push(crate::types::CardType::Creature);
        group.followups.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: milled_creature_filter,
            count: ChoiceCount::up_to(2),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(exiled_tag.clone()),
            zone: Zone::Graveyard,
        }));
        group.followups.push(EffectAst::subject_verb_exile(
            crate::cards::builders::TargetAst::Tagged(crate::tag::TagRef::of(exiled_tag.clone()), None),
            false,
        ));
        group.exiled_creatures = Some(exiled_tag.key.clone());
        group.consumed += 1;
        return Ok(true);
    }
    if group.payment_made {
        let Some((chooser, filter, tail)) = if_you_do_put_from_among_into_hand(sentence, group.player)
        else {
            return Ok(false);
        };
        let chosen_tag = helper_tag_for_tokens(&tail, "chosen");
        group.followups.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
                chooser,
                filter,
                (crate::tag::CompilerReferenceTag::It.bind()).into(),
                chosen_tag.key.clone(),
                Zone::Graveyard,
                false,
                Vec::new(),
                ChoiceCount::exactly(1),
            ),
        }));
        // The selection reads the milled cards through the prior-object
        // reference, as its program did; the mill is spelled as written.
        group.hand_with_if_not = true;
        group.payment_made = false;
        group.consumed += 1;
        return Ok(true);
    }
    if !group.followups.is_empty() {
        return Ok(false);
    }
    // A hand selection followed by "if you don't" is the three-sentence
    // program; it is read here and spelled when that sentence arrives.
    if group.bare_mill
        && let Some((chooser, filter)) = parse_may_put_filtered_card_from_among_into_hand(
            sentence.lowered(),
            group.player,
            Zone::Graveyard,
        )?
        && following.is_some_and(|third| {
            matches!(parse_if_you_dont_sentence(third.lowered()), Ok(Some(_)))
                || matches!(parse_if_you_cant_sentence(third.lowered()), Ok(Some(_)))
        })
    {
        group.pending_hand = Some((chooser, filter, sentence.lowered().to_vec()));
        group.consumed += 1;
        return Ok(true);
    }
    if let Some(maximum) = may_cast_from_among(sentence) {
        group.followups = cast_from_among(group, sentence, maximum);
        group.consumed += 1;
        return Ok(true);
    }
    if group.followups.is_empty()
        && let Some(payment) = parse_optional_payment_sentence(sentence.lowered(), group.player)?
        && following.is_some_and(|third| if_you_do_put_from_among_into_hand(third, group.player).is_some())
    {
        group.followups.push(EffectAst::Permissions(PermissionEffectAst::May { effects: payment }));
        group.payment_made = true;
        group.consumed += 1;
        return Ok(true);
    }
    if let Some((followup, conditional)) =
        parse_put_from_milled_cards_followup(sentence.lowered(), group.player, group.tag.clone())?
    {
        group.followups = followup;
        group.conditional_followup = conditional;
        group.consumed += 1;
        return Ok(true);
    }
    Ok(false)
}

/// Close the procedure: the tagged mill, then its follow-up, joined into the
/// mill's own result branch when the follow-up is unconditional and the mill
/// sat in one.
pub(super) fn finish(mut group: MilledGroup) -> Vec<EffectAst> {
    if group.pending_hand.is_some() {
        // No "if you don't" arrived: the hand selection stands on its own.
        let (chooser, filter, tokens) = group.pending_hand.take().expect("checked");
        let chosen_tag = helper_tag_for_tokens(&tokens, "chosen");
        group.followups.extend(
            compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
                chooser,
                filter,
                (crate::tag::CompilerReferenceTag::It.bind()).into(),
                chosen_tag.key.clone(),
                Zone::Graveyard,
                false,
                Vec::new(),
                ChoiceCount::up_to(1),
            ),
        );
    }
    let mut mill = if group.hand_with_if_not {
        group.plain_mill
    } else {
        group.mill
    };
    let mut followups = group.followups;
    if !group.conditional_followup && append_to_outer_if_result(&mut mill, &mut followups) {
        return vec![mill];
    }
    let mut effects = vec![mill];
    if group.conditional_followup {
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followups,
        }));
    } else {
        effects.extend(followups);
    }
    effects
}

fn rewrite_total_power_value(value: &mut Value, tag: &TagKey) {
    match value {
        Value::TotalPower(filter) => {
            *filter = ObjectFilter::tagged(tag.clone()).in_zone(Zone::Exile);
        }
        Value::SurfaceHinted { value, .. } => rewrite_total_power_value(value, tag),
        _ => {}
    }
}

/// "Create an X/X blue Zombie creature token, where X is the total power of
/// the cards exiled this way.": the total power reads the exiled cards.
fn rewrite_total_power_effect(effect: &mut EffectAst, tag: &TagKey) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                    power, toughness, ..
                }),
            ..
        }) => {
            rewrite_total_power_value(power, tag);
            rewrite_total_power_value(toughness, tag);
        }
        EffectAst::Sequence { effects }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. }) => {
            for effect in effects {
                rewrite_total_power_effect(effect, tag);
            }
        }
        _ => {}
    }
}
