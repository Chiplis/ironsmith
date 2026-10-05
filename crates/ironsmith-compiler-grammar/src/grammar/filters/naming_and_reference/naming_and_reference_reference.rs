use super::*;

/// "another target creature card with equal or lesser power" (Body
/// Launderer), "other target creature card with lesser toughness" (Colfenor):
/// the comparison is against the clause's object antecedent (`it`), which
/// reference resolution binds to the trigger object or source.
fn apply_relative_lesser_power_toughness(filter: &mut ObjectFilter, all_words: &[&str]) {
    // "exile target creature an opponent controls with greater power" (Eowyn,
    // Fearless Knight) compares upward against the same antecedent.
    for (axis, is_power) in [("power", true), ("toughness", false)] {
        for greater in [false, true] {
            let comparative = if greater { "greater" } else { "lesser" };
            let or_equal =
                find_phrase_start(all_words, &["with", "equal", "or", comparative, axis]);
            let strict = find_phrase_start(all_words, &["with", comparative, axis]);
            let Some(start) = or_equal.or(strict) else {
                continue;
            };
            // "with lesser power than <something>" names its own comparand.
            let phrase_len = if or_equal.is_some() { 5 } else { 3 };
            if all_words.get(start + phrase_len) == Some(&"than") {
                continue;
            }
            let slot = if is_power {
                &mut filter.power
            } else {
                &mut filter.toughness
            };
            if slot.is_some() {
                continue;
            }
            let spec = Box::new(crate::ChooseSpec::Tagged(
                (crate::tag::CompilerReferenceTag::It.bind()).into(),
            ));
            let value = Box::new(if is_power {
                Value::PowerOf(spec)
            } else {
                Value::ToughnessOf(spec)
            });
            *slot = Some(match (greater, or_equal.is_some()) {
                (false, true) => crate::filter::Comparison::LessThanOrEqualExpr(value),
                (false, false) => crate::filter::Comparison::LessThanExpr(value),
                (true, true) => crate::filter::Comparison::GreaterThanOrEqualExpr(value),
                (true, false) => crate::filter::Comparison::GreaterThanExpr(value),
            });
        }
    }
}

pub(in super::super) fn apply_reference_and_tag_stage(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> ReferenceTagStageResult {
    if all_words.first().is_some_and(|word| *word == EQUIPPED_WORD)
        && !all_words
            .get(1)
            .is_some_and(|noun| matches!(*noun, "creatures" | "permanents"))
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::Equipped.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    } else if all_words
        .first()
        .is_some_and(|word| *word == ENCHANTED_WORD)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    }

    if let Some((source_word_len, surface)) =
        source_reference_prefix_surface(all_words, segment_tokens)
    {
        filter.source = true;
        filter.source_surface = Some(surface);
        all_words.drain(..source_word_len);
        drain_source_reference_prefix_tokens(segment_tokens, source_word_len);
    }

    if let Some(its_attached_idx) = find_phrase_start(all_words, ITS_ATTACHED_TO_PHRASE) {
        let mut normalized = Vec::with_capacity(all_words.len() + 1);
        normalized.extend_from_slice(&all_words[..its_attached_idx]);
        normalized.extend(["attached", "to", "it"]);
        normalized.extend_from_slice(&all_words[its_attached_idx + 3..]);
        *all_words = normalized;
    }

    if let Some(attached_idx) = word_index_for_exact(all_words, ATTACHED_WORD)
        && all_words
            .get(attached_idx + 1)
            .is_some_and(|word| *word == TO_WORD)
    {
        let attached_to_words = &all_words[attached_idx + 2..];
        if words_start_with_phrase(attached_to_words, ENCHANTED_PLAYER_PREFIX) {
            let trim_start = if attached_idx >= 2
                && all_words[attached_idx - 2] == THAT_WORD
                && word_is_any(all_words[attached_idx - 1], BE_VERB_WORDS)
            {
                attached_idx - 2
            } else {
                attached_idx
            };
            all_words.truncate(trim_start);
            filter.attached_to_player = Some(PlayerFilter::TaggedPlayer(
                (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
            ));
            return ReferenceTagStageResult {
                source_linked_exile_reference: false,
                early_return: false,
            };
        }
        let references_it =
            words_start_with_any_phrase(attached_to_words, ATTACHED_TO_TAGGED_OBJECT_PREFIXES)
                .is_some();
        if references_it {
            let relation = if attached_idx >= 2
                && all_words[attached_idx - 2] == THAT_WORD
                && matches!(all_words[attached_idx - 1], "was" | "were")
            {
                TaggedOpbjectRelation::WasAttachedToTaggedObject
            } else {
                TaggedOpbjectRelation::AttachedToTaggedObject
            };
            let trim_start = if attached_idx >= 2
                && all_words[attached_idx - 2] == THAT_WORD
                && word_is_any(all_words[attached_idx - 1], BE_VERB_WORDS)
            {
                attached_idx - 2
            } else {
                attached_idx
            };
            // The attachment host is an independently referenced object, not
            // another characteristic of the attachment being selected. Keep
            // the token-backed characteristic pass in sync with `all_words`;
            // otherwise its later scan can re-add the host noun (for example,
            // turning "Equipment attached to that creature" into an
            // Equipment creature filter).
            let segment_words = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
            let segment_word_refs = segment_words.to_word_refs();
            if let Some(segment_attached_idx) =
                crate::word_primitives::parse_sequence_start(&segment_word_refs, &[ATTACHED_WORD])
            {
                let segment_trim_start = if segment_attached_idx >= 2
                    && segment_word_refs[segment_attached_idx - 2] == THAT_WORD
                    && word_is_any(segment_word_refs[segment_attached_idx - 1], BE_VERB_WORDS)
                {
                    segment_attached_idx - 2
                } else {
                    segment_attached_idx
                };
                if let Some(token_start) =
                    segment_words.map_word_or_end_to_token_boundary(segment_trim_start)
                {
                    segment_tokens.truncate(token_start);
                }
            }
            all_words.truncate(trim_start);
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                relation,
            });
        }
    }

    if let Some(relation_idx) = find_blocking_or_blocked_by_source_phrase(all_words) {
        filter.in_combat_with_source = true;
        all_words.truncate(relation_idx);
    }

    if let Some(relation_idx) = find_blocking_source_phrase(all_words) {
        filter.blocking = true;
        filter.in_combat_with_source = true;
        all_words.truncate(relation_idx);
    }

    // "a Vehicle crewed by this creature this turn" (Balthier and Fran): the
    // source tapped to pay that Vehicle's crew cost this turn.
    if let Some(relation_idx) = find_crewed_by_source_this_turn_phrase(all_words) {
        filter.crewed_by_source_this_turn = true;
        // Keep the token-backed characteristic pass in sync: the crewer's
        // noun ("this creature") is not a characteristic of the Vehicle.
        let segment_words = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
        let segment_word_refs = segment_words.to_word_refs();
        if let Some(segment_idx) = find_crewed_by_source_this_turn_phrase(&segment_word_refs)
            && let Some(token_start) = segment_words.map_word_or_end_to_token_boundary(segment_idx)
        {
            segment_tokens.truncate(token_start);
        }
        all_words.truncate(relation_idx);
    }

    // "that weren't put there this way" — exclude the objects the previous
    // effect just moved to this zone (the sacrificed set).
    for phrase in [
        ["that", "weren't", "put", "there", "this", "way"],
        ["that", "werent", "put", "there", "this", "way"],
    ] {
        if let Some(idx) = crate::word_primitives::parse_sequence_start(all_words, &phrase) {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::ThisWaySacrificed.bind()).into(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
            all_words.drain(idx..idx + 6);
            break;
        }
    }

    // "…you controlled that/attached…" — past-tense controller in
    // last-known-information relative clauses. The active relation verbs
    // stay present-tense (a broad "controlled" verb breaks as-you-cast
    // predicates).
    if let Some(idx) =
        crate::word_primitives::parse_sequence_start(all_words, &["you", "controlled"])
    {
        let next = all_words.get(idx + 2).copied();
        if matches!(next, None | Some("that") | Some("attached")) {
            filter.controller = Some(PlayerFilter::You);
            all_words.drain(idx..idx + 2);
        }
    }

    // "creature this (creature) is blocking" — the filtered object is an
    // attacker currently blocked by the source.
    if let Some(relation_idx) = find_any_filter_phrase_start(
        all_words,
        &[
            &["this", "creature", "is", "blocking"],
            &["this", "permanent", "is", "blocking"],
            &["this", "source", "is", "blocking"],
            &["this", "is", "blocking"],
        ],
    ) {
        filter.blocked_by_source = true;
        all_words.truncate(relation_idx);
    }

    // "a creature it's paired with" (Donna Noble): the source's soulbond
    // partner (CR 702.95). The relation is read from the filter's source.
    if let Some(relation_idx) = find_any_filter_phrase_start(
        all_words,
        &[
            &["it's", "paired", "with"],
            &["its", "paired", "with"],
            &["it", "is", "paired", "with"],
            &["this", "creature", "is", "paired", "with"],
        ],
    ) && all_words.last() == Some(&"with")
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::SourceObject.bind()).into(),
            relation: TaggedOpbjectRelation::SoulbondPartnerOfTagged,
        });
        all_words.truncate(relation_idx);
    }

    // "creatures banded with it" (Icatian Skirmishers, Urza's Engine): the
    // source's attacking band (CR 702.22).
    if let Some(relation_idx) = find_any_filter_phrase_start(
        all_words,
        &[
            &["banded", "with", "it"],
            &["banded", "with", "this", "creature"],
            &["banded", "with", "this", "permanent"],
        ],
    ) && (all_words[relation_idx..] == ["banded", "with", "it"]
        || all_words[relation_idx..] == ["banded", "with", "this", "creature"]
        || all_words[relation_idx..] == ["banded", "with", "this", "permanent"])
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::SourceObject.bind()).into(),
            relation: TaggedOpbjectRelation::BandedWithTagged,
        });
        all_words.truncate(relation_idx);
    }

    let starts_with_exiled_card =
        words_start_with_any_phrase(all_words, EXILED_CARD_PREFIXES).is_some();
    if starts_with_exiled_card {
        filter.zone.get_or_insert(Zone::Exile);
    }
    let has_exiled_with_phrase = find_phrase_start(all_words, EXILED_WITH_PHRASE).is_some();
    let source_exiled_is_same_name_antecedent =
        find_any_phrase_start(all_words, SAME_NAME_AS_TAGGED_OBJECT_PHRASES)
            .zip(find_phrase_start(all_words, EXILED_WITH_PHRASE))
            .is_some_and(|(same_name_idx, exiled_with_idx)| same_name_idx < exiled_with_idx);
    let has_used_to_craft_phrase = find_phrase_start(all_words, USED_TO_CRAFT_PHRASE).is_some();
    let is_source_linked_exile_reference = has_exiled_with_phrase
        || (starts_with_exiled_card && (all_words.len() == 2 || has_used_to_craft_phrase));
    let mut source_linked_exile_reference = false;
    if is_source_linked_exile_reference {
        source_linked_exile_reference = true;
        if !source_exiled_is_same_name_antecedent {
            filter.zone = Some(Zone::Exile);
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        }
        if let Some(exiled_with_idx) = find_phrase_start(all_words, EXILED_WITH_PHRASE) {
            let mut reference_end = exiled_with_idx + 2;
            let mut has_reference_head = false;
            if all_words
                .get(reference_end)
                .is_some_and(|word| word_is_any(word, REFERENCE_HEAD_WORDS))
            {
                reference_end += 1;
                has_reference_head = true;
            }
            if all_words
                .get(reference_end)
                .is_some_and(|word| is_exiled_with_reference_noun(word, has_reference_head))
            {
                reference_end += 1;
            }
            if reference_end > exiled_with_idx + 1 {
                all_words.drain(exiled_with_idx + 1..reference_end);
            }
        }
        if let Some(used_to_craft_idx) = find_phrase_start(all_words, USED_TO_CRAFT_PHRASE) {
            let mut reference_end = used_to_craft_idx + 3;
            if all_words
                .get(reference_end)
                .is_some_and(|word| word_is_any(word, REFERENCE_HEAD_WORDS))
            {
                reference_end += 1;
            }
            if all_words
                .get(reference_end)
                .is_some_and(|word| word_is_any(word, REFERENCE_OBJECT_NOUN_WORDS))
            {
                reference_end += 1;
            }
            all_words.drain(used_to_craft_idx..reference_end);
        }
        let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
        let segment_words = segment_words_view.to_word_refs();
        if let Some(exiled_with_idx) = find_phrase_start(&segment_words, EXILED_WITH_PHRASE) {
            let mut reference_end_word = exiled_with_idx + EXILED_WITH_PHRASE.len();
            let mut has_reference_head = false;
            if segment_words
                .get(reference_end_word)
                .is_some_and(|word| word_is_any(word, REFERENCE_HEAD_WORDS))
            {
                reference_end_word += 1;
                has_reference_head = true;
            }
            if segment_words
                .get(reference_end_word)
                .is_some_and(|word| is_exiled_with_reference_noun(word, has_reference_head))
            {
                reference_end_word += 1;
            }
            if reference_end_word > exiled_with_idx + 1
                && let Some(token_range) =
                    segment_words_view.token_span_for_words(exiled_with_idx + 1, reference_end_word)
            {
                segment_tokens.drain(token_range);
            }
        }
        let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
        let segment_words = segment_words_view.to_word_refs();
        if let Some(used_to_craft_idx) = find_phrase_start(&segment_words, USED_TO_CRAFT_PHRASE) {
            let mut reference_end_word = used_to_craft_idx + USED_TO_CRAFT_PHRASE.len();
            if segment_words
                .get(reference_end_word)
                .is_some_and(|word| word_is_any(word, REFERENCE_HEAD_WORDS))
            {
                reference_end_word += 1;
            }
            if segment_words
                .get(reference_end_word)
                .is_some_and(|word| word_is_any(word, REFERENCE_OBJECT_NOUN_WORDS))
            {
                reference_end_word += 1;
            }
            if let Some(token_range) =
                segment_words_view.token_span_for_words(used_to_craft_idx, reference_end_word)
            {
                segment_tokens.drain(token_range);
            }
        }
    }

    if all_words
        .first()
        .is_some_and(|word| word_is_any(word, IT_OR_THEM_WORDS))
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        if all_words.len() == 1 {
            return ReferenceTagStageResult {
                source_linked_exile_reference,
                early_return: true,
            };
        }
        all_words.remove(0);
    }

    if words_start_with_any_phrase(all_words, REVEALED_CARD_PREFIXES).is_some() {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.drain(..2);
    }

    let additional_cost_surface = additional_cost_object_surface(all_words);
    let references_additional_cost_object = additional_cost_surface.is_some()
        || find_any_phrase_start(all_words, ADDITIONAL_COST_OBJECT_REFERENCE_PHRASES).is_some();
    if references_additional_cost_object {
        filter.set_additional_cost_object_surface(additional_cost_surface);
    }
    let has_share_card_type = find_any_phrase_start(all_words, SHARED_CARD_TYPE_PHRASES).is_some()
        && words_contain_any_word(all_words, SHARE_WORDS)
        && (words_contain_any_word(all_words, IT_OR_THEM_WORDS)
            || references_additional_cost_object);
    let has_share_color = (words_contain_any_word(all_words, SHARE_WORDS)
        && words_contain_any_word(all_words, COLOR_OR_COLORS_WORDS)
        && words_contain_any_word(all_words, IT_OR_THEM_WORDS))
        || (references_additional_cost_object
            && words_contain_any_word(all_words, SHARE_WORDS)
            && words_contain_any_word(all_words, COLOR_OR_COLORS_WORDS));
    let references_tapped_cost_objects =
        find_phrase_start(all_words, TAPPED_THIS_WAY_PHRASE).is_some();
    let references_each_tapped_cost_object =
        find_phrase_start(all_words, EACH_CREATURE_TAPPED_THIS_WAY_PHRASE).is_some();
    // "a creature that shares a creature type with that creature" (Reins of
    // the Vinesteed, Conjurer's Mantle): the demonstrative names the creature
    // whose event triggered the ability.
    let shares_with_triggering_creature = !words_contain_any_word(all_words, IT_OR_THEM_WORDS)
        && find_any_phrase_start(
            all_words,
            &[
                &["with", "that", "creature"],
                &["with", "that", "permanent"],
            ],
        )
        .is_some();
    let has_share_creature_type = find_any_phrase_start(all_words, CREATURE_TYPE_PHRASES).is_some()
        && words_contain_any_word(all_words, SHARE_WORDS)
        && (words_contain_any_word(all_words, IT_OR_THEM_WORDS)
            || shares_with_triggering_creature
            || references_tapped_cost_objects
            || references_additional_cost_object);
    let has_same_mana_value = find_phrase_start(all_words, SAME_MANA_VALUE_AS_PHRASE).is_some();
    let has_equal_or_lesser_mana_value =
        find_phrase_start(all_words, EQUAL_OR_LESSER_MANA_VALUE_PHRASE).is_some();
    let has_lte_mana_value_than_that_spell =
        find_any_phrase_start(all_words, LTE_MANA_VALUE_THAN_THAT_SPELL_PHRASES).is_some();
    let has_lte_mana_value_as_tagged =
        find_any_phrase_start(all_words, LTE_MANA_VALUE_AS_TAGGED_PHRASES).is_some()
            || has_equal_or_lesser_mana_value;
    let has_lt_mana_value_as_tagged = find_phrase_start(all_words, LESSER_MANA_VALUE_PHRASE)
        .is_some()
        && !has_equal_or_lesser_mana_value;
    let references_it_for_mana_value = words_contain_any_word(all_words, IT_OR_ITS_REFERENCE_WORDS)
        || find_any_phrase_start(all_words, TAGGED_OBJECT_REFERENCE_FOR_MANA_VALUE_PHRASES)
            .is_some();
    let has_same_name_as_tagged_object =
        find_any_phrase_start(all_words, SAME_NAME_AS_TAGGED_OBJECT_PHRASES).is_some();
    let same_name_references_source_object =
        find_any_phrase_start(all_words, SAME_NAME_AS_SOURCE_OBJECT_PHRASES).is_some();

    if has_share_card_type {
        let tag = if references_additional_cost_object {
            crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()
        } else {
            crate::tag::CompilerReferenceTag::It.bind()
        };
        let constraint = TaggedObjectConstraint {
            tag: tag.key.clone(),
            relation: shared_type_relation(all_words),
        };
        if !filter
            .tagged_constraints
            .iter()
            .any(|existing| existing == &constraint)
        {
            filter.tagged_constraints.push(constraint);
        }
    }
    if has_share_color {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: if references_additional_cost_object {
                (crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()).into()
            } else {
                (crate::tag::CompilerReferenceTag::It.bind()).into()
            },
            relation: TaggedOpbjectRelation::SharesColorWithTagged,
        });
    }
    if has_share_creature_type {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: if references_additional_cost_object {
                (crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()).into()
            } else if shares_with_triggering_creature {
                (crate::tag::CompilerReferenceTag::Triggering.bind()).into()
            } else {
                (crate::tag::CompilerReferenceTag::It.bind()).into()
            },
            relation: if references_each_tapped_cost_object {
                TaggedOpbjectRelation::SharesSubtypeWithEachTagged
            } else {
                TaggedOpbjectRelation::SharesSubtypeWithTagged
            },
        });
    }
    let references_sacrificed_cost_object = crate::word_primitives::any_sequence_occurs(
        all_words,
        &[
            &["sacrificed", "creature"],
            &["sacrificed", "artifact"],
            &["sacrificed", "enchantment"],
            &["sacrificed", "permanent"],
        ],
    );
    if has_same_mana_value
        && (references_additional_cost_object || references_sacrificed_cost_object)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()).into(),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    } else if has_same_mana_value && references_it_for_mana_value {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    }
    if has_equal_or_lesser_mana_value {
        filter.union_surface = filter.union_surface.with_equal_or_lesser_mana_value(true);
    }
    if has_lte_mana_value_as_tagged
        && (references_it_for_mana_value || has_equal_or_lesser_mana_value)
    {
        let tag = if has_lte_mana_value_than_that_spell {
            crate::tag::CompilerReferenceTag::Triggering.bind()
        } else {
            crate::tag::CompilerReferenceTag::It.bind()
        };
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.key.clone(),
            relation: TaggedOpbjectRelation::ManaValueLteTagged,
        });
    }
    if has_lt_mana_value_as_tagged {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::ManaValueLtTagged,
        });
    }
    apply_relative_lesser_power_toughness(filter, all_words);
    // "it fights up to one target creature you don't control with the same
    // mana value" (Boxing Ring): the comparand is the clause's antecedent.
    if !has_same_mana_value
        && filter.mana_value.is_none()
        && (all_words.ends_with(&["with", "same", "mana", "value"])
            || all_words.ends_with(&["with", "the", "same", "mana", "value"]))
    {
        filter.mana_value = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::ManaValueOf(Box::new(crate::ChooseSpec::Tagged(
                (crate::tag::CompilerReferenceTag::It.bind()).into(),
            ))),
        )));
    }
    if has_same_name_as_tagged_object {
        filter.set_same_name_antecedent_surface(same_name_antecedent_surface(all_words));
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: if source_exiled_is_same_name_antecedent {
                (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into()
            } else if same_name_references_source_object {
                (crate::tag::CompilerReferenceTag::SourceObject.bind()).into()
            } else {
                (crate::tag::CompilerReferenceTag::It.bind()).into()
            },
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    }

    if find_any_phrase_start(all_words, CONVOKED_THIS_SPELL_TAG_PHRASES).is_some() {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::ConvokedThisSpell.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if find_phrase_start(all_words, CREWED_IT_THIS_TURN_TAG_PHRASE).is_some() {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::CrewedItThisTurn.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if find_phrase_start(all_words, SADDLED_IT_THIS_TURN_TAG_PHRASE).is_some() {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::SaddledItThisTurn.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if find_any_phrase_start(all_words, AMASSED_ARMY_TAG_PHRASES).is_some() {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if let Some(became_creature_idx) = find_any_phrase_start(
        all_words,
        &[
            &["that", "became", "a", "creature", "this", "way"],
            &["that", "became", "creature", "this", "way"],
            &["that", "became", "creatures", "this", "way"],
            &["which", "became", "a", "creature", "this", "way"],
            &["which", "became", "creature", "this", "way"],
            &["which", "became", "creatures", "this", "way"],
        ],
    ) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.truncate(became_creature_idx);
    }
    let references_type_chosen_this_way =
        crate::word_primitives::sequence_occurs(all_words, &["type", "chosen", "this", "way"]);
    if !references_each_tapped_cost_object
        && !references_type_chosen_this_way
        && let Some(this_way_idx) = find_phrase_start(all_words, &["this", "way"])
        && let Some((action, action_start)) =
            crate::grammar::shared_util::value_helper_shapes::parse_prior_effect_action(
                &all_words[..this_way_idx],
            )
    {
        // Tapping is only meaningful for objects on the battlefield. Preserve
        // that semantic zone when a later clause refers to the objects tapped
        // this way, even though the reference surface itself omits the zone.
        if action == ironsmith_core::PriorEffectAction::Tapped {
            filter.zone.get_or_insert(Zone::Battlefield);
        }
        // Milling moves cards from a library to a graveyard. Establish that
        // zone before the creature noun's default battlefield inference.
        if matches!(
            action,
            ironsmith_core::PriorEffectAction::Milled
                | ironsmith_core::PriorEffectAction::PutIntoGraveyard
        ) {
            filter.zone.get_or_insert(Zone::Graveyard);
        }
        filter.set_prior_effect_action_surface(Some(action));
        // "For each opponent, exile a creature card they revealed this way"
        // (Valki): each iteration chooses among the cards that player
        // revealed, from wherever they revealed them.
        if action == ironsmith_core::PriorEffectAction::Revealed
            && action_start
                .checked_sub(1)
                .and_then(|idx| all_words.get(idx))
                .is_some_and(|word| *word == "they")
        {
            filter.owner.get_or_insert(PlayerFilter::IteratedPlayer);
        }
        let relation = if action_start
            .checked_sub(1)
            .and_then(|idx| all_words.get(idx))
            .is_some_and(|word| {
                matches!(
                    *word,
                    "not"
                        | "isnt"
                        | "isn't"
                        | "arent"
                        | "aren't"
                        | "wasnt"
                        | "wasn't"
                        | "werent"
                        | "weren't"
                        | "doesnt"
                        | "doesn't"
                        | "didnt"
                        | "didn't"
                )
            }) {
            TaggedOpbjectRelation::IsNotTaggedObject
        } else {
            TaggedOpbjectRelation::IsTaggedObject
        };
        // "creatures tapped this way" names the objects a preceding "tap
        // all" actually tapped; normalization binds the marker (or falls
        // back to the ordinary `it` antecedent).
        let reference = if action == ironsmith_core::PriorEffectAction::Tapped
            && relation == TaggedOpbjectRelation::IsTaggedObject
        {
            crate::tag::CompilerReferenceTag::TappedThisWay
        } else {
            crate::tag::CompilerReferenceTag::It
        };
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (reference.bind()).into(),
            relation,
        });
    }

    ReferenceTagStageResult {
        source_linked_exile_reference,
        early_return: false,
    }
}

/// The noun naming the source in `exiled with this <noun>`. After a reference
/// head the source may be named by its subtype (`this Saga`, `this
/// Vehicle`); that subtype names the source, not the exiled card.
fn is_exiled_with_reference_noun(word: &str, has_reference_head: bool) -> bool {
    word_is_any(word, REFERENCE_OBJECT_NOUN_WORDS)
        || (has_reference_head && crate::util::parse_subtype_word(word).is_some())
}
