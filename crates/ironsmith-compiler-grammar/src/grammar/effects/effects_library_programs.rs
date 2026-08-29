use super::*;

pub fn parse_search_library_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    subject_starts_effect_lexed: fn(&[OwnedLexToken]) -> bool,
    parse_leading_effects_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
    parse_effect_clause_lexed: fn(&[OwnedLexToken]) -> Result<EffectAst, CardTextError>,
    carry_conjugated_search_player: fn(&[EffectAst], &mut [EffectAst]),
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn has_trailing_that_player_shuffle(tokens: &[OwnedLexToken]) -> bool {
        TRAILING_THAT_PLAYER_SHUFFLE_PHRASES
            .iter()
            .any(|phrase| primitives::find_phrase_start(tokens, phrase).is_some())
    }

    fn nested_iterated_object_filter(effect: &EffectAst) -> Option<ObjectFilter> {
        if let EffectAst::ForEachObject { filter, .. } = effect {
            return Some(filter.clone());
        }
        let mut found = None;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            if found.is_none() {
                found = nested.iter().find_map(nested_iterated_object_filter);
            }
        });
        found
    }

    if parse_each_chosen_player_search_put_top_shape(tokens).is_some() {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Library);
        return Ok(Some(vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::target_player(),
            effects: vec![EffectAst::subject_verb_search_library(
                filter,
                Zone::Library,
                PlayerAst::That,
                PlayerAst::That,
                SearchSelectionMode::Exact,
                false,
                None,
                true,
                ChoiceCount::exactly(1),
                None,
                Some(Value::Fixed(1)),
                crate::effect::SearchResultReferenceSurface::ThatCard,
                false,
                false,
                false,
            )],
        }]));
    }

    let clause_display = render_token_slice(tokens);
    let Some(head_split) = split_search_library_sentence_head_lexed(tokens) else {
        return Ok(None);
    };

    let subject_prelude = parse_search_library_leading_effect_prelude_lexed(
        head_split.subject_tokens,
        subject_starts_effect_lexed,
        parse_leading_effects_lexed,
    )?;
    let subject_tokens = subject_prelude.subject_tokens;
    let sentence_has_direct_may = head_split.sentence_has_direct_may;
    let trailing_that_player_shuffle = has_trailing_that_player_shuffle(tokens);
    let mut leading_effects = subject_prelude.leading_effects;
    let wrap_each_target_player =
        search_library_subject_wraps_each_target_player_lexed(subject_tokens);
    let player_iteration_filter =
        search_library_subject_player_iteration_filter_lexed(subject_tokens);
    let iterated_subject_filter =
        parse_search_library_iterated_object_subject_lexed(subject_tokens)?;
    // A leading `for each <object>` clause is parsed as an effect prelude and
    // therefore removed from `subject_tokens`.  It still supplies the
    // contextual player for possessives in the search body (for example,
    // "that player's library" after iterating permanents a targeted opponent
    // controls).
    let leading_iterated_subject_filter = leading_effects
        .iter()
        .rev()
        .find_map(nested_iterated_object_filter);
    let chooser = if player_iteration_filter.is_some() {
        PlayerAst::That
    } else {
        match parse_subject(subject_tokens) {
            SubjectAst::Player(player) => player,
            _ => PlayerAst::Implicit,
        }
    };
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb=Search subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));

    let search_tokens = head_split.search_tokens;
    if !search_library_starts_with_search_verb_lexed(search_tokens) {
        return Ok(None);
    }
    if LexedClause::new(search_tokens).words().is_empty() {
        return Ok(None);
    }
    let Some(subject_routing) = derive_search_library_subject_routing_lexed(search_tokens, chooser)
    else {
        return Ok(None);
    };
    let player = subject_routing.player;
    let search_player_target = subject_routing.search_player_target;
    let mut forced_library_owner = subject_routing.forced_library_owner;
    // In an object iteration, "that player's library" refers to the
    // controller of the current iterand.  Preserve a statically constrained
    // controller (for example, a targeted opponent) so the collected search
    // results can be followed by one shuffle for that player; otherwise keep
    // the current object identity explicit through the scoped `__it__` tag.
    if let Some(iterated_filter) = iterated_subject_filter
        .as_ref()
        .or(leading_iterated_subject_filter.as_ref())
        && (matches!(forced_library_owner, Some(PlayerFilter::IteratedPlayer))
            || (forced_library_owner.is_none()
                && matches!(player, PlayerAst::That | PlayerAst::ItsController)))
    {
        forced_library_owner = Some(iterated_filter.controller.clone().unwrap_or_else(|| {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        }));
    }
    let search_zones_override = subject_routing.search_zones_override;
    if search_library_has_unsupported_top_position_probe_lexed(search_tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library top-position clause (clause: '{}')",
            clause_display
        )));
    }

    let clause_markers = scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned search-library clause marker scan should produce defaults");
    let for_idx = clause_markers.for_idx;
    let put_idx = clause_markers.put_idx;
    let has_explicit_destination = clause_markers.has_explicit_destination;
    let filter_boundary = clause_markers.filter_boundary;

    let filter_end =
        find_search_library_filter_boundary_lexed(search_tokens, for_idx, filter_boundary)
            .filter_end;

    if filter_end <= for_idx + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing search filter in search-library sentence (clause: '{}')",
            clause_display
        )));
    }

    let count_tokens = &search_tokens[for_idx + 1..filter_end];
    let count_prefix = parse_search_library_count_prefix_lexed(count_tokens);
    let mut count = count_prefix.count;
    let search_mode = count_prefix.search_mode;
    let count_used = count_prefix.count_used;
    let mut prefix_count_value = count_prefix.count_value;

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Err(CardTextError::ParseError(format!(
            "missing object selector in search-library sentence (clause: '{}')",
            clause_display
        )));
    }

    let mut raw_filter_tokens = trim_commas(&search_tokens[filter_start..filter_end]).to_vec();
    if let Some(rest_len) =
        primitives::parse_prefix(&raw_filter_tokens, primitives::phrase(THAT_MANY_PREFIX))
            .map(|(_, rest)| rest.len())
    {
        prefix_count_value.get_or_insert(Value::Count(ObjectFilter::tagged(TagKey::from(IT_TAG))));
        count = if search_mode == SearchSelectionMode::Optional {
            ChoiceCount::up_to_dynamic_x()
        } else {
            ChoiceCount::dynamic_x()
        };
        let consumed = raw_filter_tokens.len().saturating_sub(rest_len);
        raw_filter_tokens.drain(0..consumed);
    }
    let (filter_tokens, count_value) = if let Some((base_filter_tokens, count_value)) =
        split_search_library_count_value_clause_lexed(&raw_filter_tokens)?
    {
        (base_filter_tokens, Some(count_value))
    } else {
        (raw_filter_tokens, prefix_count_value)
    };
    let (filter_tokens, mana_constraint) = if let Some((base_filter_tokens, mana_constraint)) =
        extract_search_library_mana_constraint(&filter_tokens)
    {
        (base_filter_tokens, Some(mana_constraint))
    } else {
        (filter_tokens.to_vec(), None)
    };
    let (filter_tokens, distinct_names) =
        strip_search_library_different_names_clause_lexed(&filter_tokens);
    let mut basic_land_type_slots =
        parse_search_library_basic_land_type_slots_lexed(&filter_tokens);
    let same_name_split = if basic_land_type_slots.is_none() {
        parse_search_library_same_name_reference_lexed(
            &filter_tokens,
            filter_tokens.clone(),
            &clause_display,
        )?
    } else {
        SearchLibrarySameNameSplit {
            filter_tokens: filter_tokens.clone(),
            same_name_reference: None,
            same_name_relation: TaggedOpbjectRelation::SameNameAsTagged,
            same_name_antecedent_surface: None,
        }
    };
    let filter_tokens = same_name_split.filter_tokens;
    let same_name_reference = same_name_split.same_name_reference;
    let same_name_relation = same_name_split.same_name_relation;
    let same_name_antecedent_surface = same_name_split.same_name_antecedent_surface;
    let same_name_reference_requires_setup = matches!(
        same_name_reference,
        Some(SearchLibrarySameNameReference::Target(_))
            | Some(SearchLibrarySameNameReference::Choose { .. })
    );

    let named_filters = if basic_land_type_slots.is_none() && count_used == 0 {
        split_search_named_item_filters_lexed(&filter_tokens, &clause_display)?
    } else {
        None
    };
    let mut filter = if basic_land_type_slots.is_none() {
        parse_search_library_object_filter_lexed(&filter_tokens, &clause_display)?
    } else {
        ObjectFilter::default()
    };
    filter.distinct_names = distinct_names;
    if let Some(same_name_tag) = same_name_reference
        .as_ref()
        .map(|reference| match reference {
            SearchLibrarySameNameReference::Tagged(tag) => tag.clone(),
            SearchLibrarySameNameReference::Target(_) => TagKey::from(IT_TAG),
            SearchLibrarySameNameReference::Choose { tag, .. } => tag.clone(),
        })
    {
        filter.set_same_name_antecedent_surface(same_name_antecedent_surface);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: same_name_tag.clone(),
            relation: same_name_relation,
        });
    }
    if filter.owner.is_none()
        && let Some(owner) = forced_library_owner.clone()
    {
        filter.owner = Some(owner);
    }
    normalize_search_library_filter(&mut filter);
    if let Some(mana_constraint) = mana_constraint {
        apply_search_library_mana_constraint(&mut filter, mana_constraint);
    }
    let search_zones_are_library_only = match search_zones_override.as_ref() {
        None => true,
        Some(zones) => zones.len() == 1 && zones[0] == Zone::Library,
    };
    if search_zones_are_library_only {
        filter.zone = Some(Zone::Library);
    }

    let discard_before_shuffle_followup =
        find_search_library_discard_before_shuffle_followup_lexed(search_tokens, put_idx);
    let discard_after_shuffle_followup =
        find_search_library_discard_after_shuffle_followup_lexed(search_tokens, put_idx);
    let trailing_discard_before_shuffle = discard_before_shuffle_followup.is_some();
    let effect_routing = derive_search_library_effect_routing_lexed(
        tokens,
        search_tokens,
        clause_markers,
        trailing_discard_before_shuffle,
    );
    let destination = effect_routing.destination;
    // A search that reaches another player's library hands the card to you only
    // when the put clause says so; otherwise the move preserves its controller.
    let searched_controller = if effect_routing.enters_under_your_control {
        ReturnControllerAst::You
    } else {
        ReturnControllerAst::Preserve
    };
    let reveal = effect_routing.reveal;
    let face_down_exile = effect_routing.face_down_exile;
    let original_shuffle = effect_routing.shuffle;
    let trailing_create_followup = find_search_library_trailing_create_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    );
    let shuffle = original_shuffle && trailing_create_followup.is_none();
    let split_battlefield_and_hand = effect_routing.split_battlefield_and_hand;
    let library_position_from_top = effect_routing.library_position_from_top.clone();
    let attachment_target = search_put_attachment_target(search_tokens, clause_markers.put_idx)?;
    let mut handled_direct_may_in_iterated_search = false;
    let mut effects = if let Some(mut slots) = basic_land_type_slots.take() {
        if !has_explicit_destination || !search_zones_are_library_only {
            return Err(CardTextError::ParseError(format!(
                "unsupported each-basic-land-type search-library clause (clause: '{}')",
                clause_display
            )));
        }
        for slot in &mut slots {
            if slot.filter.owner.is_none()
                && let Some(owner) = forced_library_owner.clone()
            {
                slot.filter.owner = Some(owner);
            }
        }
        vec![EffectAst::subject_verb_search_library_slots(
            player,
            slots,
            destination,
            reveal,
            crate::tag::CompilerReferenceTag::SearchLibrarySlotsProgress.key(),
        )]
    } else if !effect_routing.battlefield_entry_counters.is_empty() {
        vec![
            EffectAst::subject_verb_search_library(
                filter,
                destination,
                chooser,
                player,
                search_mode,
                reveal,
                effect_routing.reveal_reference_surface,
                shuffle,
                count,
                count_value.clone(),
                library_position_from_top,
                effect_routing.result_reference_surface,
                effect_routing.search_top_in_any_order_surface,
                destination == Zone::Battlefield && effect_routing.has_tapped_modifier,
                effect_routing.enters_under_your_control,
            )
            .with_search_zones(
                search_zones_override
                    .clone()
                    .unwrap_or_else(|| vec![Zone::Library]),
            )
            .with_search_battlefield_entry_counters(
                effect_routing.battlefield_entry_counters.clone(),
            ),
        ]
    } else if let Some(iterated_filter) = iterated_subject_filter.clone()
        && has_explicit_destination
        && named_filters.is_none()
        && !split_battlefield_and_hand
        && !(destination == Zone::Exile && face_down_exile)
    {
        let searched_tag: TagKey = "searched".into();
        let search_zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        // Always use the search subject `player` so the shuffle references
        // the searcher, not the last-referenced player from a preceding effect.
        let shuffle_player = player;

        let mut per_object_effects = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: searched_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if sentence_has_direct_may {
            handled_direct_may_in_iterated_search = true;
            per_object_effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
                EffectAst::May {
                    effects: per_object_effects,
                }
            } else {
                EffectAst::MayByPlayer {
                    player: chooser,
                    effects: per_object_effects,
                }
            }];
        }

        let mut sequence = vec![EffectAst::ForEachObject {
            filter: iterated_filter,
            effects: per_object_effects,
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
        }
        if shuffle && destination == Zone::Library && zones_have(&search_zones, Zone::Library) {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: searched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                destination,
                matches!(destination, Zone::Library),
                searched_controller,
                battlefield_tapped,
                None,
            )],
        });
        if shuffle && !(destination == Zone::Library && zones_have(&search_zones, Zone::Library)) {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if let Some(named_filters) = named_filters {
        let searched_tag: TagKey = "searched_named".into();
        let zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let mut sequence = Vec::new();
        for mut named_filter in named_filters {
            if named_filter.owner.is_none()
                && let Some(owner) = forced_library_owner.clone()
            {
                named_filter.owner = Some(owner);
            }
            normalize_search_library_filter(&mut named_filter);
            sequence.push(EffectAst::ChooseObjectsAcrossZones {
                filter: named_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: chooser,
                tag: searched_tag.clone(),
                zones: zones.clone(),
                search_mode: Some(SearchSelectionMode::Exact),
            });
        }
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
        }
        sequence.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
            destination,
            matches!(destination, Zone::Library),
            searched_controller,
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier,
            None,
        ));
        if shuffle && zones_have(&zones, Zone::Library) {
            sequence.push(EffectAst::IfResult {
                predicate: IfResultPredicate::SearchedLibrary,
                effects: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    player,
                    SubjectVerbActionAst::ShuffleLibrary,
                )],
            });
        }
        sequence
    } else if !has_explicit_destination {
        let chosen_tag: TagKey = "searched".into();
        let search_zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()));
        }
        if shuffle && zones_have(&search_zones, Zone::Library) {
            sequence.push(EffectAst::IfResult {
                predicate: IfResultPredicate::SearchedLibrary,
                effects: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    player,
                    SubjectVerbActionAst::ShuffleLibrary,
                )],
            });
        }
        sequence
    } else if let Some(search_zones) = search_zones_override
        .clone()
        .or_else(|| attachment_target.as_ref().map(|_| vec![Zone::Library]))
    {
        let chosen_tag: TagKey = "searched_multi_zone".into();
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        // Use the search subject `player` (e.g. Implicit/You) rather than
        // PlayerAst::That, which would resolve to the last referenced player
        // in a preceding effect (e.g. "target player" from a damage clause).
        let shuffle_player = player;
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()));
        }
        if shuffle
            && destination == Zone::Library
            && zones_have(&search_zones, Zone::Library)
            && !trailing_that_player_shuffle
        {
            sequence.push(EffectAst::IfResult {
                predicate: IfResultPredicate::SearchedLibrary,
                effects: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    shuffle_player,
                    SubjectVerbActionAst::ShuffleLibrary,
                )],
            });
        }
        let mut per_tag_effects = vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(chosen_tag.clone(), span_from_tokens(tokens)),
            destination,
            matches!(destination, Zone::Library),
            ReturnControllerAst::Preserve,
            battlefield_tapped,
            None,
        )];
        if destination == Zone::Battlefield
            && let Some(target) = attachment_target.clone()
        {
            per_tag_effects.push(EffectAst::subject_verb_attach(
                TargetAst::Tagged(chosen_tag.clone(), span_from_tokens(tokens)),
                target,
            ));
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: per_tag_effects,
        });
        if shuffle
            && !(destination == Zone::Library && zones_have(&search_zones, Zone::Library))
            && !trailing_that_player_shuffle
        {
            sequence.push(EffectAst::IfResult {
                predicate: IfResultPredicate::SearchedLibrary,
                effects: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    shuffle_player,
                    SubjectVerbActionAst::ShuffleLibrary,
                )],
            });
        }
        sequence
    } else if split_battlefield_and_hand {
        let battlefield_tapped = effect_routing.has_tapped_modifier;
        if filter.owner.is_none() && matches!(player, PlayerAst::You | PlayerAst::Implicit) {
            filter.owner = Some(PlayerFilter::You);
        }
        let searched_tag = helper_tag_for_tokens(tokens, "searched_split");
        let battlefield_tag = helper_tag_for_tokens(tokens, "searched_split_battlefield");
        let battlefield_filter = ObjectFilter::tagged(searched_tag.clone()).in_zone(Zone::Library);
        let battlefield_controller = if matches!(player, PlayerAst::You | PlayerAst::Implicit) {
            ReturnControllerAst::You
        } else {
            ReturnControllerAst::Owner
        };
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: searched_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
        }
        sequence.extend([
            EffectAst::ChooseTaggedObjectsInZone {
                filter: battlefield_filter,
                count: ChoiceCount::exactly(1),
                player: chooser,
                tag: battlefield_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::ForEachTagged {
                tag: battlefield_tag.clone(),
                effects: vec![EffectAst::subject_verb_put_onto_battlefield(
                    chooser,
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    battlefield_tapped,
                    battlefield_controller,
                )],
            },
            EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag: searched_tag,
                    keep_tagged: battlefield_tag,
                    zone: Zone::Hand,
                    surface: ironsmith_core::LibraryRemainderSurface::Rest,
                },
            ),
        ]);
        if shuffle {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if destination == Zone::Exile && face_down_exile {
        let searched_tag: TagKey = "searched_face_down".into();
        let mut sequence = vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value: count_value.clone(),
                player: chooser,
                tag: searched_tag.clone(),
                zones: vec![Zone::Library],
                search_mode: Some(search_mode),
            },
            EffectAst::subject_verb_exile(
                TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                true,
            ),
        ];
        if shuffle {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else {
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        vec![
            EffectAst::subject_verb_search_library(
                filter,
                destination,
                chooser,
                player,
                search_mode,
                reveal,
                effect_routing.reveal_reference_surface,
                shuffle,
                count,
                count_value.clone(),
                library_position_from_top,
                effect_routing.result_reference_surface,
                effect_routing.search_top_in_any_order_surface,
                battlefield_tapped,
                effect_routing.enters_under_your_control,
            )
            .with_search_battlefield_entry_counters(
                effect_routing.battlefield_entry_counters.clone(),
            ),
        ]
    };

    if let Some(discard_followup) = discard_before_shuffle_followup {
        let discard_tokens =
            trim_commas(&search_tokens[discard_followup.discard_idx..discard_followup.discard_end]);
        if !discard_tokens.is_empty() {
            effects.push(parse_effect_clause_lexed(&discard_tokens)?);
        }
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    if let Some(discard_tokens) = discard_after_shuffle_followup {
        let mut discard = parse_effect_clause_lexed(discard_tokens)?;
        // The strict boundary helper starts at the authored verb in
        // `..., shuffle, then discard ...`; its omitted subject is the search
        // actor, not a newly selected opponent inherited from an earlier
        // sentence. Preserve that actor explicitly before reference carry can
        // reinterpret the subjectless clause.
        if crate::lexer::parser_token_word_refs(discard_tokens)
            .first()
            .is_some_and(|word| *word == "discard")
            && let EffectAst::SubjectVerb(discard) = &mut discard
            && matches!(discard.action, SubjectVerbActionAst::Discard { .. })
        {
            discard.subject.player = player;
        }
        effects.push(discard);
    }

    if trailing_that_player_shuffle {
        let mut has_existing_shuffle = false;
        for effect in &mut effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect {
                match &subject_verb.action {
                    SubjectVerbActionAst::ShuffleLibrary => {
                        has_existing_shuffle = true;
                        if matches!(
                            subject_verb.subject.player,
                            PlayerAst::You | PlayerAst::Implicit
                        ) {
                            subject_verb.subject.player = PlayerAst::That;
                        }
                    }
                    // A search with `shuffle: true` already lowers to its own
                    // library shuffle. Do not append a second effect merely
                    // because Oracle spells the same shuffle out as a trailing
                    // "Then that player shuffles" sentence.
                    SubjectVerbActionAst::SearchLibrary { shuffle: true, .. } => {
                        has_existing_shuffle = true;
                    }
                    _ => {}
                }
            }
        }
        if !has_existing_shuffle {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if let Some(target) = search_player_target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }

    if let Some(trailing_tokens) = find_search_library_trailing_life_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    ) {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
    }

    if let Some(trailing_tokens) = trailing_create_followup {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
        if original_shuffle {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if let Some(reference) = same_name_reference {
        match reference {
            SearchLibrarySameNameReference::Tagged(_) => {}
            SearchLibrarySameNameReference::Target(target) => {
                effects.insert(0, EffectAst::subject_verb_target_only(target));
            }
            SearchLibrarySameNameReference::Choose { filter, tag } => {
                if same_name_relation == TaggedOpbjectRelation::DifferentNameFromTagged {
                    effects.insert(
                        0,
                        EffectAst::subject_verb_tag_matching_objects(
                            filter,
                            vec![Zone::Battlefield],
                            tag,
                        ),
                    );
                } else {
                    effects.insert(
                        0,
                        EffectAst::ChooseObjects {
                            filter,
                            count: ChoiceCount::exactly(1),
                            count_value: None,
                            player,
                            tag,
                        },
                    );
                }
            }
        }
    }

    if sentence_has_direct_may && !handled_direct_may_in_iterated_search {
        effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
            EffectAst::May { effects }
        } else {
            EffectAst::MayByPlayer {
                player: chooser,
                effects,
            }
        }];
    }

    if let Some(filter) = player_iteration_filter {
        effects = vec![match filter {
            PlayerFilter::Opponent => EffectAst::ForEachOpponent { effects },
            PlayerFilter::Any => EffectAst::ForEachPlayer { effects },
            other => EffectAst::ForEachPlayersFiltered {
                filter: other,
                effects,
            },
        }];
    }

    if let Some(iterated_filter) = iterated_subject_filter
        && !has_explicit_destination
        && !same_name_reference_requires_setup
    {
        effects = vec![EffectAst::ForEachObject {
            filter: iterated_filter,
            effects,
        }];
    }

    if !leading_effects.is_empty() {
        // A conjugated search with no repeated subject belongs to the
        // preceding grammatical subject: "... that player loses 3 life,
        // searches their library ...". The search grammar deliberately
        // leaves that actor implicit, so carry it only for `searches`.
        // Imperative `search your library` starts a new actor and must remain
        // the ability controller even when another player was mentioned by a
        // leading effect.
        if search_tokens
            .first()
            .is_some_and(|token| token.is_word("searches"))
        {
            carry_conjugated_search_player(&leading_effects, &mut effects);
        }
        leading_effects.extend(effects);
        return Ok(Some(leading_effects));
    }

    if wrap_each_target_player {
        effects = vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::target_player(),
            effects,
        }];
    }

    Ok(Some(effects))
}
