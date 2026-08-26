use super::*;

pub fn parse_generic_top_cards_cloak_counted_rest_bottom_subject_verb(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentence_tokens = trim_commas(tokens);
    let shape = effect_grammar::parse_look_cloak_partition_shape(&sentence_tokens)?;
    let look_tokens = trim_commas(&sentence_tokens[shape.look]);
    let (player, count, reveal) =
        super::super::dispatch_entry::parse_top_cards_view_sentence(&look_tokens)?;
    if reveal {
        return None;
    }

    let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked_cloak");
    let selected_tag = crate::util::helper_tag_for_tokens(tokens, "cloaked_selection");
    let mut selected_filter = ObjectFilter::tagged(looked_tag.clone());
    selected_filter.zone = Some(Zone::Library);

    Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: shape.selected_count,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_cloak_onto_battlefield(
            PlayerAst::You,
            TargetAst::Tagged(selected_tag.clone(), None),
            false,
            ReturnControllerAst::You,
            false,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            shape.remainder_order,
            player,
        ),
    ])
}


pub fn parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_clause = LexedClause::new(&sentence_tokens).trimmed();
    let matched = LOOK_EXILE_COUNTED_FACE_DOWN_PATTERN.parse_full(sentence_clause)?;
    let look_clause = matched
        .capture_clause("look_clause", sentence_clause)?
        .trimmed();
    let look_effect = super::super::verb_handlers::parse_look(look_clause.tokens(), None).ok()?;
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
    }) = look_effect
    else {
        return None;
    };

    let count_clause = matched
        .capture_clause("exile_count", sentence_clause)?
        .trimmed();
    let count_tokens = trim_commas(count_clause.tokens());
    let (exile_count, _used) =
        crate::util::parse_choice_count_token_prefix_consumed(&count_tokens)?;

    let remainder_clause = matched
        .capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, sentence_clause)?
        .trimmed();
    let counter_modifier = if remainder_clause.word_refs().first() == Some(&"with") {
        let modifier_match =
            EXILE_FACE_DOWN_COUNTER_MODIFIER_PATTERN.parse_full(remainder_clause)?;
        let descriptor_clause = modifier_match
            .capture_clause("counter_descriptor", remainder_clause)?
            .trimmed();
        Some(
            super::super::super::grammar::effects::zone_counter_shapes::parse_counter_descriptor_shape(
                descriptor_clause.tokens(),
            )?,
        )
    } else {
        None
    };
    if EXILE_FACE_DOWN_REST_BOTTOM_PATTERN
        .locate_in(remainder_clause)
        .is_none()
        || EXILE_FACE_DOWN_REST_LIBRARY_PATTERN
            .locate_in(remainder_clause)
            .is_none()
    {
        return None;
    }
    let singleton_remainder = matches!(count.unhinted(), Value::Fixed(2))
        && exile_count.min == 1
        && exile_count.max == Some(1)
        && !exile_count.dynamic_x
        && !exile_count.up_to_x
        && !exile_count.random;
    let bottom_order = if EXILE_FACE_DOWN_REST_RANDOM_ORDER_PATTERN
        .locate_in(remainder_clause)
        .is_some()
    {
        crate::cards::builders::LibraryBottomOrderAst::Random
    } else if EXILE_FACE_DOWN_REST_ANY_ORDER_PATTERN
        .locate_in(remainder_clause)
        .is_some()
    {
        crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
    } else if singleton_remainder {
        // Ordering a one-card complement is meaningless, so Oracle omits an
        // order clause ("put the other on the bottom of that library").  The
        // runtime still uses the ordinary chooser-order primitive; with one
        // card it has exactly one legal ordering.
        crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
    } else {
        return None;
    };

    let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked");
    let exiled_tag = TagKey::from(IT_TAG);
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    let mut effects = vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: choice_filter,
            count: exile_count,
            count_value: None,
            player: PlayerAst::You,
            tag: exiled_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
    ];
    if let Some(descriptor) = counter_modifier {
        effects.push(EffectAst::subject_verb_put_counters(
            descriptor.counter_type,
            Value::Fixed(descriptor.count as i32),
            TargetAst::Tagged(exiled_tag.clone(), None),
            None,
            false,
        ));
    }
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag),
            bottom_order,
            player,
        ),
    );
    Some(effects)
}
