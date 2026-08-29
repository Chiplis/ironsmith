use super::*;

#[inline(never)]
fn parse_complete_conditional_gain_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((predicate_clause, consequence_tokens)) =
        crate::grammar::primitives::split_lexed_once_on_comma(tokens)
    else {
        return Ok(None);
    };
    if !predicate_clause
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        return Ok(None);
    }
    let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(consequence_tokens)?
    else {
        return Ok(None);
    };
    let predicate_tokens = trim_commas(&predicate_clause[1..]);
    let Ok(predicate) =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens)
    else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::Conditional {
        predicate,
        if_true: effects,
        if_false: Vec::new(),
    }]))
}

#[inline(never)]
fn parse_complete_quoted_gain_with_trailing_unless(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !quoted_gain_has_trailing_unless(tokens) {
        return Ok(None);
    }
    let Some(close_quote) =
        crate::slice_primitives::select_last_position(tokens, OwnedLexToken::is_quote)
    else {
        return Ok(None);
    };
    let Some(unless_idx) = tokens
        .iter()
        .enumerate()
        .skip(close_quote + 1)
        .find_map(|(index, token)| token.is_word("unless").then_some(index))
    else {
        return Ok(None);
    };
    let leading_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[..unless_idx]);
    let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(leading_tokens)?
    else {
        return Ok(None);
    };
    let Some(effect) = super::super::subject_verb_primitives::try_build_simple_unless_pays(
        effects,
        &tokens[unless_idx + 1..],
    )?
    else {
        return Ok(None);
    };
    Ok(Some(vec![effect]))
}

#[inline(never)]
pub fn parse_effect_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = super::super::parse_complete_create_statement(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_complete_quoted_gain_with_trailing_unless(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_complete_conditional_gain_ability(tokens)? {
        return Ok(effects);
    }
    dispatch_effect_sentence_lexed_remaining(tokens)
}

#[inline(never)]
fn dispatch_effect_sentence_lexed_remaining(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) =
        super::super::chain_carry::parse_repeated_counter_placement_coordination(tokens)?
    {
        return Ok(effects);
    }
    if super::super::chain_carry::is_atomic_put_counter_for_each_sentence(tokens) {
        return Ok(vec![
            super::super::zone_counter_helpers::parse_put_counters(tokens)?,
        ]);
    }
    // A joint draw has two independent player subjects but one shared verb
    // phrase. Claim the complete grammar shape before broad subject/verb
    // parsing retains only the leading `you` actor.
    if effect_grammar::subject_verb_registry_shapes::parse_joint_draw_shape(tokens).is_some()
        && let Some(effects) =
            super::super::subject_verb_primitives::parse_sentence_you_and_target_player_each_draw(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
    {
        return Ok(effects);
    }
    // The shared target-player subject belongs to the coordinated resource
    // program. Claim this grammar-proven shape before any whole-sentence
    // target or modifier probe can commit to the leading `target` token.
    if super::super::chain_carry::has_target_player_resource_coordination(tokens) {
        return super::super::parse_effect_chain_inner_lexed(tokens);
    }
    // A joint create has two independent actors but one shared verb phrase.
    // Claim the complete grammar shape before the broad imperative-create
    // route can retain only the leading `you` actor.
    if effect_grammar::subject_verb_registry_shapes::parse_joint_create_shape(tokens).is_some()
        && let Some(effects) =
            super::super::subject_verb_primitives::parse_sentence_you_and_player_each_create(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
    {
        return Ok(effects);
    }
    // The complete delayed sentence owns both its trigger and its payload.
    // Claim it before broad payload recognizers (notably quantified player
    // fanout) inspect the outer sentence and try to parse only the trigger
    // header as a recurring ability.
    if effect_grammar::delayed_sentence_shapes::parse_delayed_this_turn_shape(tokens).is_some()
        && let Some(effects) = parse_sentence_delayed_trigger_this_turn(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::super::subject_verb_primitives::parse_sentence_delayed_next_step_unless_pays(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    // A trailing `unless` owns the complete action/payment choice. Claim it
    // before broad tap, damage, exile, and other subject/verb recognizers can
    // accept only the action prefix and silently discard the alternative.
    // The unless parser recurses only on the proven prefix, so this route is
    // terminating and leaves unsupported counter-specific forms to their
    // dedicated grammar.
    let contains_quantified_opponent =
        effect_grammar::for_each_shapes::parse_quantified_opponent_presence(tokens);
    if crate::lexer::split_lexed_sentences(tokens).len() == 1
        && !contains_quantified_opponent
        && !tokens.first().is_some_and(|token| token.is_word("if"))
        && !effect_grammar::chain_splitting::has_authored_comma_then_surface_tokens(tokens)
        && effect_grammar::choice_damage_shapes::parse_unless_sentence_shape(tokens).is_some()
        && let Some(effects) = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }
    if let Some(effects) = super::super::subject_verb_primitives::
        parse_sentence_damage_to_that_player_unless_enchanted_attacked(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    // Predicate-form trailing `unless` clauses (for example, "unless that
    // creature attacked this turn") are control flow rather than a payment
    // alternative. Route a grammar-proven control-flow plan before the broad
    // damage and target primitives can accept only the leading action and
    // silently discard its postcondition.
    if tokens.iter().any(|token| token.is_word("unless"))
        && !effect_grammar::chain_splitting::has_authored_comma_then_surface_tokens(tokens)
        && matches!(
            effect_grammar::control_flow::recognize_control_flow(tokens),
            crate::recognition::ParseOutcome::Match(_)
        )
    {
        return super::super::parse_effect_chain_inner_lexed(tokens);
    }
    if effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(tokens).is_some_and(
        |shape| shape.has_trailing_segment() && tokens.iter().any(OwnedLexToken::is_semicolon),
    ) {
        // This grammar-proven boundary must precede the broad target-gets
        // fast path below; otherwise that path consumes the semicolon tail as
        // part of the where-X value and never reaches inner dispatch.
        return parse_effect_sentence_with_where_x_lexed(tokens);
    }
    if let Some(effects) = parse_sentence_each_player_return_with_additional_counter(
        SubjectVerbPrimitiveClause::new(tokens),
    )? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Return subject=each-player recognizer=return-with-additional-counter",
        );
        return Ok(effects);
    }
    let head_words = crate::lexer::parser_token_word_refs(tokens);
    // Commas between parallel target-state adjectives belong to the object
    // filter (`target nonattacking, nonblocking creature`), not to effect
    // coordination. Prove the complete gets-clause and lower it before a
    // comma segment can reach typed-head diagnostics as a verbless object.
    if head_words.first() == Some(&"target")
        && head_words
            .iter()
            .any(|word| matches!(*word, "get" | "gets"))
        && tokens.iter().any(OwnedLexToken::is_comma)
        && let Some(shape) =
            effect_grammar::clause_dispatch_shapes::parse_clause_subject_verb_shape(tokens)
        && let Some(effect) = super::super::clause_dispatch::parse_get_pump_clause(
            shape.subject_tokens,
            shape.action_tokens,
            tokens,
        )?
    {
        return Ok(vec![effect]);
    }
    if crate::word_primitives::parse_any_sequence_prefix(
        &head_words,
        &[
            &["each", "player", "return"],
            &["each", "player", "returns"],
        ],
    ) {
        // Destination-first returns often carry comma-separated card-type
        // unions. Preserve the quantified actor and complete return operand
        // before generic coordination sees those type-list commas.
        let view = TokenWordView::new(tokens);
        let return_start = view.token_index_after_words(3).unwrap_or(tokens.len());
        let return_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[return_start..]);
        let mut effect = super::super::zone_handlers::parse_return(return_tokens)?;
        super::super::chain_carry::bind_implicit_player_context(&mut effect, PlayerAst::That);
        return Ok(vec![EffectAst::ForEachPlayer {
            effects: vec![effect],
        }]);
    }
    if tokens.first().is_some_and(|token| token.is_word("discard"))
        && !tokens
            .iter()
            .any(|token| token.is_word("if") || token.is_word("unless"))
        && !super::super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(tokens)
    {
        // A sentence-leading bare discard is an imperative controlled by the
        // ability controller. Do not inherit an event participant (such as a
        // damaged player) merely because the command follows a damage trigger.
        let discard_body = crate::util::trim_edge_punctuation_tokens(&tokens[1..]);
        let mut effect = super::super::zone_handlers::parse_discard(discard_body, None)?;
        super::super::chain_carry::bind_implicit_player_context(&mut effect, PlayerAst::You);
        return Ok(vec![effect]);
    }
    if let Some(effects) =
        super::super::chain_carry::parse_conditional_inline_looked_card_partition(tokens)?
    {
        // The consequence owns one typed look/selection/remainder program.
        // Prove that program before the general conditional registry asks a
        // broad `look` verb handler to consume the internal `, then` tail.
        return Ok(effects);
    }
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(comma) =
            crate::slice_primitives::select_position(tokens, OwnedLexToken::is_comma)
        && super::super::fanout_family::parse_compound_damage_fanout_sentence(
            crate::util::trim_edge_punctuation_tokens(&tokens[comma + 1..]),
        )?
        .is_some()
    {
        // The legacy conditional dispatcher probes subject/verb primitives
        // before its consequence callback. A repeated damage head must reach
        // that callback intact or its second amount becomes an orphaned
        // verbless clause. Prove the typed body first, then use the ordinary
        // conditional predicate grammar with the dedicated fanout callback.
        return effect_grammar::parse_conditional_sentence_with_grammar_entrypoint_lexed(
            tokens,
            parse_required_damage_fanout,
        );
    }
    if effect_grammar::chain_carry::parse_tap_or_untap_all_choice_tokens(tokens) {
        let action_tokens = crate::lexer::trim_lexed_commas(&tokens[1..]);
        return Ok(vec![super::super::zone_handlers::parse_tap(action_tokens)?]);
    }
    if let Some(effects) =
        super::super::clause_pattern_helpers::parse_choose_target_prelude_sentence(tokens)?
    {
        return Ok(effects);
    }
    // A coordinated zone-pair declaration also contains the words
    // `target player`, so the generic complete-target preemption below can
    // otherwise claim it and ask the ordinary target parser to interpret the
    // trailing graveyard as part of one object target. Preserve the strict
    // typed two-zone bundle before entering that broader route.
    if let Some(effects) =
        super::super::search_library::parse_exile_hand_and_graveyard_bundle_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::super::fanout_family::parse_compound_damage_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }
    // A token-copy exception is one creation action. Its comma-separated
    // characteristic and ability clauses (`except they're 3/3 ... , and they
    // have flying`) are copiable-value modifiers, not independent effects.
    // Let the complete typed creation grammar prove the sentence before the
    // generic chain splitter can expose the verbless `except` tail.
    if head_words.first() == Some(&"create")
        && crate::word_primitives::sequence_occurs(&head_words, &["except"])
        && head_words
            .iter()
            .any(|word| matches!(*word, "token" | "tokens"))
        && head_words
            .iter()
            .any(|word| matches!(*word, "copy" | "copies"))
        && let Ok(
            effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy { .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                ..
            }),
        ) = super::super::parse_create(tokens, None)
    {
        return Ok(vec![effect]);
    }
    // Triggered, activated, and modal bodies can enter this single-sentence
    // dispatcher without passing through the document-level sentence loop.
    // A complete target declaration containing a relative history clause
    // ("cards ... that were put there") must remain one typed target effect;
    // otherwise the subject/verb planner can reinterpret the relative `put`
    // as a zone-change action.
    if let Some(shape) = effect_grammar::clause_dispatch_shapes::parse_choose_target_shape(tokens)
        && parse_target_phrase(shape.target_tokens).is_ok()
    {
        return Ok(vec![super::super::parse_effect_clause_lexed(tokens)?]);
    }
    // Triggered and activated line parsers can enter this single-sentence
    // dispatcher directly, bypassing the document-level pass that normally
    // hides quoted token rules from outer effect-chain parsing. Keep those
    // rule bodies inside the token blueprint: parse the create action from
    // the stripped surface, then reattach every quoted ability under the
    // token's own source identity.
    let stripped_tokens = strip_embedded_token_rules_text(tokens);
    let has_embedded_token_rules = stripped_tokens.len() != tokens.len();
    let parse_tokens = if has_embedded_token_rules {
        stripped_tokens.as_slice()
    } else {
        tokens
    };
    let mut effects = {
        if let Some(effects) = parse_prefix_then_look_at_top_exile_one(parse_tokens)? {
            Ok(effects)
        } else if let Some(effects) = parse_bounded_x_mana_payment_sentence(parse_tokens) {
            Ok(effects)
        } else {
            parse_effect_sentence_lexed_inner(parse_tokens)
        }
    }?;
    super::super::fanout_family::bind_removed_counter_damage_fanout(&mut effects);
    if has_embedded_token_rules {
        super::super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
            &mut effects,
            tokens,
        );
    }
    if let Some(surface) = parse_set_quantifier_surface(parse_tokens) {
        set_first_continuous_set_quantifier(&mut effects, surface);
    }
    if let Some(surface) = parse_return_set_reference_surface(parse_tokens) {
        set_first_return_set_reference_surface(&mut effects, &surface);
    }
    Ok(crate::effect_sentences::preserve_coordinated_effect_chain_surface(parse_tokens, effects))
}

pub fn parse_effect_sentence_lexed_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let authored_surface = crate::util::authored_named_source_reference_surface(context, tokens);
    let leading_normalized =
        crate::util::normalize_leading_named_source_reference_tokens_with_context(context, tokens);
    let normalized = if leading_normalized == tokens {
        super::super::normalize_source_references_with_context(context, tokens)?
    } else {
        leading_normalized
    };
    let mut effects = if authored_surface.is_some() {
        match super::super::clause_primitives::parse_anaphoric_object_deals_damage_clause(
            &normalized,
        )? {
            Some(effect) => vec![effect],
            None => parse_effect_sentence_lexed(&normalized)?,
        }
    } else {
        parse_effect_sentence_lexed(&normalized)?
    };
    if let Some(surface) = authored_surface {
        restore_authored_damage_source_surface(&mut effects, &surface);
    }
    Ok(effects)
}

pub(super) fn has_unrecognized_leading_effect_label(tokens: &[OwnedLexToken]) -> bool {
    if crate::grammar::structure::split_leading_result_prefix_lexed(tokens).is_some() {
        return false;
    }
    effect_grammar::labeled_dispatch::parse_leading_effect_label_tokens(tokens).is_some_and(
        |shape| shape.kind == effect_grammar::labeled_dispatch::LeadingEffectLabelKind::Unknown,
    )
}

pub(super) fn parse_effect_sentence_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_sentence_lexed_inner_unstacked(tokens)
}

pub(super) fn parse_effect_sentence_lexed_inner_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    // A terminal win-game action has an explicit player subject but no
    // ordinary controller verb. Claim it before the typed subject/verb head
    // registry commits on `you` and reports an unsupported action. Trigger
    // splitting commonly removes the sentence-end token, so this route must
    // accept both standalone and embedded effect surfaces.
    if let Some(effect) = super::super::clause_pattern_helpers::parse_win_the_game_clause(tokens)? {
        return Ok(vec![effect]);
    }
    // This joint source-and-blocked-object move is one typed iteration over
    // two object filters, followed by an owner-relative shuffle. Claim the
    // complete grammar-proven sentence before broad `put` and coordination
    // routes can commit on only `this creature` and discard the second set.
    if let Some(effect) = parse_source_and_blocked_creatures_top_library_shuffle_sentence(tokens) {
        return Ok(vec![effect]);
    }
    // The final `and planeswalker it has dealt damage to this game` is the
    // object half of one historical damage-recipient union, not a second
    // executable action. Prove that the damage body lowers to the typed
    // player/object history sequence before the generic action-chain splitter
    // sees the embedded `has dealt` verb.
    if let Some(verb_word_idx) =
        crate::slice_primitives::select_position(&words, |word| matches!(*word, "deal" | "deals"))
    {
        let word_view = TokenWordView::new(tokens);
        let body_start = word_view
            .map_word_or_end_to_token_boundary(verb_word_idx + 1)
            .unwrap_or(tokens.len());
        let body = trim_edge_punctuation(&tokens[body_start..]);
        let historical_union =
            super::super::verb_handlers::is_historical_player_object_damage_recipient_clause(&body);
        if historical_union {
            crate::parse_trace::event(
                "effect-route: subject-verb verb=Deal subject=source recognizer=historical-player-object-union",
            );
            return Ok(vec![super::super::verb_handlers::parse_deal_damage(&body)?]);
        }
    }
    // A copy exception is part of the `becomes a copy` action, even though
    // its comma-separated tail has an ordinary `has` verb. Parse the complete
    // typed animation before generic coordination can reinterpret
    // `except it has ...` as an independent battlefield anthem.
    if super::super::parse_leading_player_may_lexed(tokens).is_none()
        && let Some(become_word_idx) =
            crate::slice_primitives::select_last_position(&words, |word| {
                matches!(*word, "become" | "becomes")
            })
    {
        let view = TokenWordView::new(tokens);
        let subject_end = view
            .map_word_or_end_to_token_boundary(become_word_idx)
            .unwrap_or(tokens.len());
        let body_start = view
            .map_word_or_end_to_token_boundary(become_word_idx + 1)
            .unwrap_or(tokens.len());
        let subject = trim_edge_punctuation(&tokens[..subject_end]);
        let body = trim_edge_punctuation(&tokens[body_start..]);
        if !subject.is_empty()
            && !body.is_empty()
            && effect_grammar::become_shapes::parse_become_rest_shape(&body)
                .copy_exception
                .is_some()
        {
            return Ok(vec![super::super::clause_dispatch::parse_become_clause(
                &subject, &body,
            )?]);
        }
    }
    // A trailing subjectless permission belongs to the same subject as the
    // preceding action: "this creature gets ... and can attack ... as though
    // it didn't have defender."  The standalone permission recognizer cannot
    // treat the whole prefix as an object subject, while the broad effect
    // chain otherwise mistakes the comparison's `have defender` for an
    // ability grant. Split only the grammar-proven final `and can attack`
    // clause, parse the prefix normally, and reattach the typed permission to
    // the prefix's explicit subject.
    if let Some(and_word_idx) =
        crate::word_primitives::parse_last_sequence_start(&words, &["and", "can", "attack"])
    {
        let word_view = TokenWordView::new(tokens);
        let and_token_idx = word_view
            .map_word_or_end_to_token_boundary(and_word_idx)
            .unwrap_or(tokens.len());
        let can_token_idx = word_view
            .map_word_or_end_to_token_boundary(and_word_idx + 1)
            .unwrap_or(tokens.len());
        let prefix = trim_edge_punctuation(&tokens[..and_token_idx]);
        if !prefix.is_empty()
            && let Some((_, verb_word_idx)) =
                super::super::lex_chain_helpers::find_verb_lexed(&prefix)
        {
            let prefix_words = TokenWordView::new(&prefix);
            let subject_end = prefix_words
                .map_word_or_end_to_token_boundary(verb_word_idx)
                .unwrap_or(prefix.len());
            let subject = trim_edge_punctuation(&prefix[..subject_end]);
            if !subject.is_empty() {
                // The trailing clause is deliberately subjectless. Bind it to
                // the preceding effect-chain result (`it`) rather than
                // reparsing the original `target ...` phrase as an untargeted
                // all-objects filter.
                if let Some(permission) =
                    parse_can_attack_as_though_no_defender_clause(&tokens[can_token_idx..])?
                {
                    let parsed_prefix = parse_effect_sentence_lexed(&prefix)?;
                    if !parsed_prefix.is_empty() {
                        let mut coordinated = Vec::new();
                        for effect in parsed_prefix {
                            match effect {
                                EffectAst::Coordinated { effects, .. } => {
                                    coordinated.extend(effects);
                                }
                                effect => coordinated.push(effect),
                            }
                        }
                        coordinated.push(permission);
                        return Ok(vec![EffectAst::Coordinated {
                            effects: coordinated,
                            leading_duration: false,
                            result_conjunction: false,
                        }]);
                    }
                }
            }
        }
    }
    // This permission contains the words `have defender`, but those words are
    // inside an `as though` comparison rather than an ability to grant. Claim
    // the complete typed combat-permission clause before the broad gain-
    // ability routes can reduce it to granting defender itself.
    if let Some(effect) = parse_can_attack_as_though_no_defender_clause(tokens)? {
        return Ok(vec![effect]);
    }
    // Keep a demonstrative per-object reward ahead of the broad gain-life
    // sentence parser. The latter can otherwise reduce "the controller of
    // each of those artifacts" to the ability controller and discard both
    // the iteration and prior-result provenance.
    if let Some(effect) =
        super::super::chain_carry::parse_each_prior_affected_object_controller_mana_value_life(
            tokens,
        )?
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_destroy_attached_object_then_source_damage_to_controller(tokens)? {
        return Ok(effects);
    }
    // This grammar-proven cast-origin grant must own the complete sentence.
    // Later gain-ability routes permissively accept the leading `as you cast`
    // phrase as an object-filter subject and lose both the hand provenance
    // and authored duration surface.
    if let Some(effect) = parse_as_you_cast_from_zone_this_turn_grant(tokens)? {
        return Ok(vec![effect]);
    }
    // Delayed payment clauses can name any supported next step. Route the
    // complete sentence before broad subject/verb parsing splits at `unless`;
    // otherwise an action-first draw-step clause is reduced to a life-loss
    // action with an unsupported timing tail. Parsing the action prefix
    // recurses with the timing marker removed, so this route terminates.
    if let Some(effects) =
        parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }
    if let Some(effects) = parse_attacking_doesnt_tap_if_source_untapped(tokens)? {
        return Ok(effects);
    }
    // A trailing condition on a mass-destruction instruction governs whether
    // that instruction happens. Route the grammar-proven condition before the
    // broad destroy subject/verb primitive can consume only its leading
    // action and silently discard the predicate.
    if crate::grammar::structure::split_trailing_if_clause_lexed(tokens).is_some()
        && let Ok(effect) =
            super::super::chain_carry::parse_effect_clause_with_trailing_if_lexed(tokens)
        && matches!(
            &effect,
            EffectAst::TrailingIf {
                effects,
                ..
            } if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::DestroyAll { .. },
                    ..
                })]
            )
        )
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) =
        super::super::player_subject_sequences::parse_each_player_exile_sacrifice_return_exiled(
            tokens,
        )?
    {
        return Ok(effects);
    }
    if let Some(effect) =
        super::super::chain_carry::parse_may_have_any_number_tagged_phase_out_lexed(tokens)
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) = super::super::dispatch_entry::parse_if_you_dont_sentence(tokens)? {
        return Ok(vec![EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::ExplicitDidNot,
            effects,
        }]);
    }
    if let Some(diag) =
        super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens)
    {
        return Err(diag);
    }

    // A causative `unless its controller has <source> deal ...` clause is one
    // action choice. The broad subject/verb recognizer can otherwise claim
    // only the embedded damage phrase and discard both the primary action
    // and the `unless` relationship.
    if let Some(effects) = super::super::subject_verb_primitives::
        parse_sentence_damage_unless_controller_has_source_deal_damage(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    // Shared-characteristic fanouts are one linked target set. In particular,
    // a broad destroy parser must not reduce `target enchantment and each
    // other enchantment that shares a color with it` to two unrelated
    // targets before the typed relation is recorded.
    if let Some(effects) =
        super::super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }

    // A keyword-bundle pump contains an authored `and so on for ...` list,
    // not a conjunction of executable actions. Route that complete typed
    // shape before the broad leading-duration chain predicate can split it
    // into only the first two conditional pump clauses.
    if let Some(effects) =
        super::super::subject_verb_special_recognizers::parse_keyword_bundle_pump_sentence(tokens)?
    {
        return Ok(effects);
    }

    // A genuine top-level conjunction with a leading duration needs chain
    // carry before broad gain/subject recognizers see an isolated arm. The
    // grammar predicate rejects quoted/list conjunctions and `then` chains.
    if effect_grammar::chain_carry::coordinated_effect_chain_leading_duration(tokens) == Some(true)
    {
        return super::super::parse_effect_chain_lexed(tokens);
    }

    let quoted_ability_shape = sentence_shapes::parse_quoted_ability_sentence_tokens(tokens);
    // A quoted restriction is payload of the outer gain, not a top-level
    // restriction. Keep its duration and trailing `unless` together before
    // the broad `can't` route sees the nested negation.
    if quoted_ability_shape.is_some()
        && quoted_gain_has_trailing_unless(tokens)
        && let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) = parse_explicit_assign_no_combat_damage_followup(tokens)? {
        return Ok(effects);
    }

    // A source pump followed by `can't be blocked this turn` is one shared-
    // subject program. Preserve both typed effects before any generic chain
    // or prohibition route can reinterpret the leading source/pump words as
    // the blocked-object filter and silently retain only the restriction.
    if let Some(effects) = parse_source_gets_unblockable_subject_verb(tokens)? {
        return Ok(effects);
    }
    // The target variant is the same atomic program: one explicit target,
    // one P/T modification, and a same-target blocking restriction. Give it
    // the same early ownership as the source form so the broad pump route
    // cannot accept only the leading `gets ...` clause and discard the
    // coordinated `can't be blocked this turn` tail.
    if let Some(effects) = parse_target_gets_unblockable_subject_verb(tokens)? {
        return Ok(effects);
    }

    if super::super::parse_leading_player_may_lexed(tokens).is_some()
        && let Some(spec) = parse_may_cast_it_sentence(tokens)
    {
        return Ok(vec![build_may_cast_tagged_effect(&spec)]);
    }
    if super::super::parse_leading_player_may_lexed(tokens).is_some()
        && let Some(effect) = crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)?
    {
        // The any-color mana rider repeats the complete `you may` subject,
        // which makes both halves look independently executable to the
        // generic conjunction preemption below. The tagged-permission grammar
        // has already proved that the rider modifies the cast grant, so keep
        // the pair atomic here.
        return Ok(vec![effect]);
    }

    // A looked-card cloak partition owns the apparent `and put` action: the
    // cloaked subset and the library-bottom complement are two dispositions
    // of one looked collection. Claim the fully consumed typed shape before
    // generic coordination splits off the remainder and drops the cloak.
    if let Some(effects) = parse_generic_top_cards_cloak_counted_rest_bottom_subject_verb(tokens) {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Look subject=implicit recognizer=cloak-looked-partition",
        );
        return Ok(effects);
    }

    // An explicit subject and executable verb after a coordinated `and`
    // starts a new action, even when the leading action has a broad target-
    // list recognizer. Without this proof, phrases such as `destroy target
    // artifact ... and this creature assigns no combat damage` can be
    // swallowed as one malformed destroy-target union. The semantic splitter
    // keeps type/color conjunctions inside their filters, while the explicit
    // effect-head requirement excludes ordinary target lists.
    let explicit_action_segments =
        super::super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens);
    if explicit_action_segments.len() >= 2
        && !matches!(
            sentence_shapes::parse_leading_if_sentence_tokens(tokens),
            Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
        )
        && explicit_action_segments
            .iter()
            .all(|segment| super::super::lex_chain_helpers::segment_has_effect_head_lexed(segment))
    {
        if sentence_shapes::parse_where_x_sentence_tokens(tokens).is_some() {
            return parse_effect_sentence_with_where_x_lexed(tokens);
        }
        return super::super::parse_effect_chain_lexed(tokens);
    }

    // A mixed restriction/action conjunction belongs to the shared-subject
    // chain. The broad top-level `can't` recognizer can otherwise accept the
    // first arm and silently discard a following affirmative action such as
    // `becomes ...`. Prove both halves independently before preempting it:
    // every negated arm must be a complete typed restriction, every other
    // arm must begin with an executable effect head, and both kinds must be
    // present. This deliberately excludes pure coordinated restrictions.
    // Use the semantic chain splitter rather than a raw `and` split so an
    // internal characteristic list such as `blue and black` remains inside
    // the animation arm.
    if let Some(effects) = parse_fully_typed_mixed_restriction_action_chain(tokens)? {
        return Ok(effects);
    }

    // Pure coordinated restrictions are already fully understood by the
    // cant grammar. Route them before a broad subject parser can claim the
    // object of the first restriction (for example, `life`) as a new subject.
    // Requiring negation in every top-level arm leaves mixed `can't ... and
    // gain ...` clauses to the coordinated chain route.
    let cant_segments = split_lexed_slices_on_and(tokens);
    if !cant_segments.is_empty()
        && cant_segments.iter().all(|segment| {
            super::super::super::activation_and_restrictions::find_negation_span(segment).is_some()
        })
        && let Some(effects) = parse_cant_effect_sentence_lexed(tokens)?
    {
        return Ok(effects);
    }

    // These shapes must be recognized before the broad sentence-shape
    // predicates below. Otherwise a result-prefixed sentence can be claimed
    // by generic target parsing, and a leading roll clause can be reduced to
    // the unsupported `two d6` fragment.
    if let Some(effect_grammar::SentencePreludeShape::RollDiceChooseOneResult {
        count,
        sides,
        surface,
    }) = effect_grammar::parse_sentence_prelude_shape_tokens(tokens)
    {
        return Ok(vec![
            EffectAst::subject_verb_roll_dice_choose_result_with_surface(
                PlayerAst::Implicit,
                count,
                sides,
                Some(surface),
            ),
        ]);
    }

    // A result gate can govern an action that is scheduled for a later step,
    // as in "If you do, unattach it at the beginning of the next end step."
    // Preserve that timing before the broad result-prefix route strips the
    // suffix and parses only the immediate action. The delayed parser keeps
    // the result gate outside the scheduled payload, so it can still bind to
    // the preceding optional effect.
    if let Some(effects) =
        parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }

    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let mut trailing_effects =
            super::super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?;
        if matches!(
            &prefix.predicate,
            crate::cards::builders::IfResultPredicate::Value(_)
        ) {
            bind_numeric_result_counter_amounts(&mut trailing_effects);
        }
        let mut result = vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
        }];
        super::super::preserve_leading_result_coordination_lexed(tokens, &mut result);
        return Ok(result);
    }

    // Explicit player offers must retain both their actor and optionality
    // before broad subject/verb parsing claims the action. This is especially
    // important for split actors such as "that player or that permanent's
    // controller may ...", whose second branch is otherwise discarded.
    if super::super::parse_leading_player_may_lexed(tokens).is_some() {
        // A singular immediate "you may cast it" instruction is a choice
        // made during resolution, not a persistent cast permission. Keep the
        // explicit May wrapper before the broader tagged-permission parser
        // below gets a chance to lower only the cast action.
        if let Some(spec) = parse_may_cast_it_sentence(tokens) {
            return Ok(vec![build_may_cast_tagged_effect(&spec)]);
        }
        // A tagged play/cast permission may itself contain a second authored
        // "you may" in its mana-spending rider. Preserve that complete typed
        // permission before generic chain splitting treats the rider as an
        // unrelated `spend` action.
        if let Some(effect) = crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)? {
            return Ok(vec![effect]);
        }
        return super::super::parse_effect_chain_lexed(tokens);
    }

    fn search_followup_shuffle_player(effect: &EffectAst) -> Option<PlayerAst> {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SearchLibrary { player, .. },
                ..
            }) => Some(*player),
            _ => None,
        }
    }

    fn normalize_search_followup_shuffles(effects: &mut [EffectAst]) {
        for idx in 0..effects.len() {
            let is_default_shuffle = matches!(
                effects.get(idx),
                Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::ShuffleLibrary,
                }))
                    if matches!(subject.player, PlayerAst::You | PlayerAst::Implicit)
            );
            if !is_default_shuffle {
                continue;
            }
            let Some(search_player) = effects[..idx]
                .iter()
                .rev()
                .find_map(search_followup_shuffle_player)
            else {
                continue;
            };
            if !matches!(search_player, PlayerAst::You | PlayerAst::Implicit)
                && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::ShuffleLibrary,
                }) = &mut effects[idx]
            {
                subject.player = search_player;
            }
        }
    }

    // A duration-scoped trigger may itself contain a damage action. Preserve
    // the grammar-proven outer `Until ..., whenever ...` scope before the
    // broad damage recognizers examine the whole sentence as a direct action.
    // The trigger parser recursively dispatches only the smaller payload, so
    // this route cannot claim an ordinary leading-duration continuous effect.
    if let Some(effect) =
        super::super::clause_primitives::parse_until_duration_triggered_clause(tokens)?
    {
        return Ok(vec![effect]);
    }

    // A delayed trigger may contain a compound damage fanout as its payload.
    // Preserve the outer `whenever ... this turn` scope before the broad
    // fanout recognizer examines the whole sentence as a direct action.
    if let Some(effects) = parse_sentence_delayed_trigger_this_turn(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) =
        super::super::fanout_family::parse_compound_damage_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }

    // A comma-following consequence is sometimes presented as `Then if ...`
    // when it is parsed in isolation from the preceding sentence. Treat the
    // sequencing marker as surface glue before the conditional grammar runs;
    // otherwise the generic dispatcher detaches the iterator subject from
    // its comma-delimited effect payload.
    let conditional_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    if let Some(effects) = parse_player_villainous_choice_statement(conditional_tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = super::super::bundle_rules::parse_consult_disposition_bundle(tokens) {
        return Ok(effects);
    }
    if let Some(effect) =
        super::super::dispatch_entry::future_zone_replacement_from_sentence_tokens(tokens)
    {
        return Ok(vec![effect]);
    }
    if let Some(schedule) =
        effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens)
    {
        let effects = parse_effect_sentence_lexed_inner(schedule.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "delayed schedule sentence missing effect payload".to_string(),
            ));
        }
        let delayed = match schedule.step {
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::UntapStep => {
                EffectAst::DelayedUntilNextUntapStep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::Upkeep => {
                EffectAst::DelayedUntilNextUpkeep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::DrawStep => {
                EffectAst::DelayedUntilNextDrawStep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::MainPhase => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextMainPhase { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::FirstMainPhase => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextFirstMainPhase { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::EndStep
                if schedule.start_next_turn =>
            {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerAst::You,
                    PlayerAst::That => PlayerAst::That,
                    PlayerAst::Target => PlayerAst::Target,
                    PlayerAst::TargetOpponent => PlayerAst::TargetOpponent,
                    _ => PlayerAst::Any,
                };
                EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::EndStep => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextEndStep { player, effects }
            }
        };
        return Ok(vec![delayed]);
    }
    if let Some(effects) =
        super::super::subject_verb_primitives::parse_sentence_you_and_attacking_player_each_draw_and_lose(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    if conditional_tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(effects) = super::super::subject_verb_primitives::
            parse_if_any_tagged_cards_share_card_type_with_triggering_spell(
                SubjectVerbPrimitiveClause::new(conditional_tokens),
            )?
    {
        return Ok(effects);
    }
    if conditional_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && let Some(effects) =
            super::super::subject_verb_primitives::parse_if_enters_with_additional_counter_sentence(
                SubjectVerbPrimitiveClause::new(conditional_tokens),
            )?
    {
        return Ok(effects);
    }
    // The damage-replacement counter form begins with `If`, but its leading
    // clause describes an event rather than a state predicate. Route the
    // typed subject/verb recognizer before the generic conditional parser
    // attempts to interpret that event as a predicate.
    if let Some(effect) = parse_generic_damage_replacement_counters_subject_verb(tokens)? {
        return Ok(vec![effect]);
    }
    if conditional_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && let Some(effects) =
            parse_conditional_sentence_family_lexed(conditional_tokens, parse_effect_chain_lexed)?
    {
        return Ok(effects);
    }

    // Redirect clauses begin with an affected-object phrase rather than a
    // normal subject/verb pair (`All damage ... is dealt ...`). Dispatch the
    // typed redirect grammar before the generic extension parser reports a
    // missing verb.
    if let Some(effects) =
        super::super::clause_pattern_helpers::parse_redirect_next_damage_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::super::clause_pattern_helpers::parse_prevent_next_time_damage_sentence(tokens)?
    {
        return Ok(effects);
    }
    // Choice-complement sentences also look like ordinary subject/verb
    // clauses. Route the typed grammar before generic subject recognition can
    // interpret the `then` complement as a separate mechanic marker.
    let dispatch_shape = effect_grammar::labeled_dispatch::parse_labeled_dispatch_shape(tokens);
    if dispatch_shape.each_player_choose
        && let Some(effect) = parse_choice_complement_subject_verb(tokens)?
    {
        return Ok(vec![effect]);
    }

    if let Some(effect) = crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)? {
        return Ok(vec![effect]);
    }

    // This complete producer/copy chain must precede the intentionally
    // tolerant standalone copy parser below. That parser can locate a later
    // `copy that spell` clause inside surrounding text, but claiming this
    // exact chain there would discard the token-producing first arm.
    if let Some(effects) = parse_create_token_then_copy_spell_chain(tokens)? {
        crate::parse_trace::event(
            "effect-route: create-token-then-copy after punctuation normalization",
        );
        return Ok(effects);
    }

    // A complete spell-copy clause can contain an `except that the copy is`
    // characteristic modifier.  Route that typed action before broad
    // subject/verb recognition sees only the modifier's final `is` verb and
    // turns the source spell itself into a continuous color-setting effect.
    if let Some(effect) = super::super::clause_pattern_helpers::parse_copy_spell_clause(tokens)? {
        return Ok(vec![effect]);
    }

    if let Some(effects) =
        super::super::subject_verb_special_recognizers::parse_keyword_bundle_pump_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) =
        super::super::subject_verb_special_recognizers::parse_scaled_target_power_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(effects);
    }

    // Matching-spell cost reductions can also be phrased with a leading
    // duration ("Until your next turn, ... spells ... cost ... less"). Give
    // that typed shape precedence over the generic chain parser, which can
    // otherwise reinterpret the spell restriction as a static ability grant
    // to an inferred object.
    if let Some(effect) = lower_matching_spell_cost_reduction_sentence(tokens) {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Cost subject=spell recognizer=matching-spell-reduction",
        );
        return Ok(vec![effect]);
    }

    if let Some(effects) = parse_manifest_dread_graveyard_card_to_hand(tokens) {
        return Ok(effects);
    }

    if let Some(effects) =
        parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }

    if let Some(shape) = effect_grammar::parse_spell_cast_this_way_tax_tokens(tokens) {
        let mut spell_filter = ObjectFilter::spell().without_type(crate::types::CardType::Land);
        spell_filter.zone = None;
        if let Some(caster) = shape.taxed_caster {
            spell_filter.cast_by = Some(caster);
        }
        return Ok(vec![EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
            crate::model::CompilerGrantableCore::Ability(
                crate::model::CompilerStaticAbilityCore::new(
                    crate::static_abilities::CostIncreaseManaCost::new(
                        spell_filter,
                        shape.additional_cost,
                    ),
                ),
            ),
            crate::grant::GrantDuration::Forever,
        )]);
    }

    if let Some(effects) = parse_attack_or_block_then_prohibition_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) =
        super::super::optional_companion_fanout::parse_optional_companion_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) =
        super::super::player_subject_sequences::parse_controller_and_defending_player_discard_or_sacrifice(
            tokens,
        )
    {
        return Ok(effects);
    }

    if let Some(clauses) =
        super::super::player_subject_sequences::split_explicit_player_subject_clauses(tokens)
    {
        let mut effects = Vec::new();
        for clause in clauses {
            effects.extend(parse_effect_sentence_lexed_inner(clause)?);
        }
        return Ok(effects);
    }

    if let Some(effects) = parse_target_relative_combat_set_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) = parse_conjoined_must_be_blocked_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) =
        super::super::parse_destroy_then_temporary_cant_attack_block_chain_lexed(tokens)?
    {
        return Ok(effects);
    }
    // "If <player refs> would gain life this turn, that player gains no life
    // instead." == a can't-gain-life window for those players (Flames of the
    // Blood Hand). Intercept before leading-if splitting since the would-gain
    // predicate isn't a state condition.
    if sentence_shapes::parses_cant_gain_life_replacement_tokens(tokens) {
        return Ok(vec![EffectAst::subject_verb_cant(
            crate::effect::Restriction::gain_life(crate::target::PlayerFilter::DamagedPlayer),
            crate::effect::Until::EndOfTurn,
            None,
        )]);
    }
    if let Some(effects) = parse_reveal_source_exiled_permanents_sentence_lexed(tokens) {
        return Ok(effects);
    }
    if let Some(effect) =
        parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(tokens)
    {
        return Ok(vec![effect]);
    }
    // Preserve voter-relative player predicates before the generic player
    // subject machinery rewrites `each opponent` to an iterated `that
    // player` action and discards the qualifying vote relationship.
    if let Some(effects) = parse_vote_affinity_subject_verb(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Vote subject=explicit recognizer=vote-affinity",
        );
        return Ok(effects);
    }
    if let Some(effect) = parse_vote_subject_verb(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Vote subject=explicit recognizer=vote-procedure",
        );
        return Ok(vec![effect]);
    }
    // Numeric die-result branches also have the surface shape
    // "for each <noun phrase>, <effect>".  Route the typed keyword shape
    // before the generic object iterator so "odd/even result" is not sent to
    // object-filter lowering.
    if matches!(
        effect_grammar::clause_pattern_shapes::parse_keyword_mechanic_tokens(tokens),
        Some(effect_grammar::clause_pattern_shapes::KeywordMechanicShape::OddEvenResult { .. })
    ) && let Some(effect) = parse_keyword_mechanic_clause(tokens)?
    {
        return Ok(vec![effect]);
    }
    // Counter-result clauses also have the generic surface shape
    // `for each <noun phrase>, <effect>`. Route their typed grammar shapes
    // first so `counter(s) removed this way` is not treated as an object
    // filter or target phrase.
    if let Some(effect) = parse_for_each_counter_removed_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) =
        super::super::clause_dispatch::parse_for_each_counter_group_removed_this_way_clause(tokens)?
    {
        return Ok(vec![effect]);
    }
    if let Some(effect) =
        super::super::clause_dispatch::parse_for_each_prevent_damage_clause(tokens)?
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) =
        super::super::search_library::parse_for_each_destroyed_this_way_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::super::search_library::parse_for_each_sacrificed_this_way_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::super::search_library::parse_for_each_put_into_graveyard_this_way_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::super::search_library::parse_for_each_exiled_this_way_sentence(tokens)?
    {
        return Ok(effects);
    }
    // This typed search sequence contains an internal `then` chain. Route it
    // before the generic object iterator can interpret "each of them" as an
    // object filter and detach the final put-on-top clause.
    if effect_grammar::parse_each_chosen_player_search_put_top_shape(tokens).is_some()
        && let Some(effects) = parse_search_library_sentence_lexed(tokens)?
    {
        return Ok(effects);
    }
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_mana_symbol_spent_effect_shape(tokens)
    {
        let base = Value::ManaSymbolSpentToCastThisSpell {
            symbol: shape.symbol,
            reference: shape.reference,
        };
        let count = if shape.group_size == 1 {
            base
        } else {
            Value::DividedRoundedDown(Box::new(base), shape.group_size as i32)
        }
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each mana-symbol clause has no effect payload".to_string(),
            ));
        }
        return Ok(vec![EffectAst::RepeatEffects { count, effects }]);
    }
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_spent_mana_effect_shape(tokens)
    {
        let source_words = crate::lexer::token_word_refs(shape.source_tokens);
        let count = crate::grammar::shared_util::count_shapes::mana_from_source_spent_to_cast_value_with_reference(
            &source_words,
            shape.reference,
        )
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported for-each spent-mana source (source: '{}')",
                render_token_slice(shape.source_tokens).trim()
            ))
        })?
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "for-each spent-mana clause has no effect payload (effect: '{}')",
                render_token_slice(shape.effect_tokens).trim()
            )));
        }
        return Ok(vec![EffectAst::RepeatEffects { count, effects }]);
    }
    if let Some(shape) = effect_grammar::for_each_shapes::parse_for_each_object_effect_shape(tokens)
    {
        let mut count_words = vec!["for", "each"];
        count_words.extend(crate::lexer::token_word_refs(shape.filter_tokens));
        if let Some((count, used)) = crate::util::parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
            && !matches!(count.unhinted(), Value::Count(_))
        {
            let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "for-each scalar sentence missing effect payload".to_string(),
                ));
            }
            return Ok(vec![EffectAst::RepeatEffects {
                count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
                effects,
            }]);
        }
    }
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_dynamic_target_effect_shape(tokens)
    {
        let mut filter = parse_object_filter_lexed(shape.filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each dynamic target sentence missing effect payload".to_string(),
            ));
        }
        let tag = TagKey::from(IT_TAG);
        return Ok(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::dynamic_x(),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ForEachTagged { tag, effects },
        ]);
    }
    if let Some(shape) = effect_grammar::for_each_shapes::parse_for_each_object_effect_shape(tokens)
    {
        let filter =
            super::super::for_each_helpers::parse_for_each_object_filter(shape.filter_tokens)?;
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each object sentence missing effect payload".to_string(),
            ));
        }
        return Ok(vec![EffectAst::ForEachObject { filter, effects }]);
    }
    if let Some(effects) = super::super::bundle_rules::parse_consult_disposition_bundle(tokens) {
        return Ok(effects);
    }
    let delayed_shape = sentence_shapes::parse_delayed_sentence_tokens(tokens);
    if matches!(
        delayed_shape,
        Some(sentence_shapes::DelayedSentenceShape::NextEndStep)
    ) && let Some(effects) = parse_delayed_until_next_end_step_sentence(tokens)?
    {
        return Ok(effects);
    }
    if matches!(
        delayed_shape,
        Some(sentence_shapes::DelayedSentenceShape::NextCombat)
    ) && let Some(effects) = parse_delayed_next_combat_phase_this_turn_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) = parse_it_is_aura_enchantment_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    let quoted_animation_grant = tokens
        .iter()
        .filter(|token| token.kind == crate::lexer::TokenKind::Quote)
        .count()
        >= 2
        && tokens.iter().any(|token| token.is_word("becomes"))
        && tokens.iter().any(|token| token.is_word("gains"));
    if quoted_ability_shape.is_some()
        && let Some(effects) =
            super::super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }
    // Preserve the chooser on optional quoted restrictions. The broad quoted
    // grant parser can otherwise consume the whole sentence before the chain
    // parser turns the leading "you may have" into a MayByPlayer node.
    if quoted_ability_shape.is_some()
        && super::super::parse_leading_player_may_lexed(tokens).is_some()
    {
        return super::super::parse_effect_chain_lexed(tokens);
    }
    // A leading conditional owns the whole sentence. Do not let a quoted
    // ability's inner verbs make the broad gain parser consume the unsplit
    // condition and body; the conditional route below parses the body with
    // this same gain parser after removing the predicate.
    if (quoted_ability_shape.is_some() || quoted_animation_grant)
        && !matches!(
            sentence_shapes::parse_leading_if_sentence_tokens(tokens),
            Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
        )
        && let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }
    if effect_grammar::gain_ability_shapes::parse_source_tapped_gain_duration_shape(tokens)
        .is_some()
        && let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }
    if sentence_shapes::parse_immediate_sacrifice_sentence_tokens(tokens).is_some() {
        let mut effects = super::super::parse_effect_chain_inner_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(sentence_shapes::DelayedSentenceShape::EndOfCombat { remainder_tokens }) =
        delayed_shape
    {
        let remainder = trim_commas(remainder_tokens);
        if remainder.is_empty() {
            return Err(CardTextError::ParseError(
                "end-of-combat delayed trigger missing effect payload".to_string(),
            ));
        }
        let effects = parse_effect_sentence_lexed_inner(&remainder)?;
        return Ok(vec![EffectAst::DelayedUntilEndOfCombat { effects }]);
    }

    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(vec![effect]);
    }

    // Future replacement clauses use an `If ... would ... instead` surface,
    // but their condition is an event predicate rather than an ordinary
    // state predicate.  Recognize the typed replacement before the generic
    // leading-if splitter asks the predicate grammar to parse `would die`.
    if let Some(effect) =
        crate::effect_sentences::dispatch_entry::future_zone_replacement_from_sentence_tokens(
            tokens,
        )
    {
        return Ok(vec![effect]);
    }

    if let Some(effect) = parse_triggering_object_had_counters_create_tokens(tokens)? {
        return Ok(vec![effect]);
    }

    let leading_if_shape = sentence_shapes::parse_leading_if_sentence_tokens(tokens);
    if matches!(
        leading_if_shape,
        Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
    ) {
        // A quoted ability can contain its own verbs. Parse the conditional
        // body as an outer gain grant first so a nested trigger such as
        // `"At the beginning of the end step, sacrifice this permanent."`
        // cannot steal dispatch from `the copy gains ...`.
        let conditional = if quoted_ability_shape.is_some() {
            parse_conditional_sentence_family_lexed(
                tokens,
                parse_gain_ability_before_effect_chain,
            )
        } else if effect_grammar::control_copy_attach_shapes::contains_source_exiled_owner_library_bottom_shape(tokens)
        {
            parse_conditional_sentence_family_lexed(
                tokens,
                parse_effect_chain_preserving_source_exiled_owner_library_bottom,
            )
        } else {
            parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)
        };
        let Some(mut effects) = conditional? else {
            return Err(CardTextError::InvariantViolation(
                "recognized leading-if shape was not claimed by conditional grammar".to_string(),
            ));
        };
        if matches!(effects.as_slice(), [EffectAst::Conditional { .. }]) {
            apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
            normalize_search_followup_shuffles(&mut effects);
            return Ok(effects);
        }
        if matches!(effects.as_slice(), [EffectAst::IfResult { .. }]) {
            super::super::preserve_leading_result_coordination_lexed(tokens, &mut effects);
            normalize_search_followup_shuffles(&mut effects);
            return Ok(effects);
        }
        return Err(CardTextError::InvariantViolation(
            "leading-if grammar returned a non-conditional effect program".to_string(),
        ));
    }

    if has_unrecognized_leading_effect_label(tokens) {
        return Err(CardTextError::ParseError(
            "unknown labeled effect prefix".to_string(),
        ));
    }

    if let Some(effects) =
        parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Reveal subject=each-player recognizer=top-count-permanents-rest-graveyard",
        );
        return Ok(effects);
    }

    // Preserve an inline continuation after a reveal-until traversal before
    // the broad subject/verb recognizer claims only the leading reveal.
    if let Some(effects) =
        super::super::consult_family::parse_consult_traversal_with_inline_followup(tokens)?
    {
        return Ok(effects);
    }

    if effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(tokens).is_some_and(
        |shape| {
            shape.comma_tail_has_effect_clause
                || (shape.has_trailing_segment() && tokens.iter().any(OwnedLexToken::is_semicolon))
        },
    ) {
        // A semicolon/comma after the where-X binding begins another effect
        // clause. Route the grammar-confirmed layout before broad gain and
        // subject/verb probes can absorb the trailing clause's subject into
        // the first `gets` modifier and report a malformed binding.
        crate::parse_trace::event("effect-route: where-x binding with trailing effect clause");
        let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
        apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
        return Ok(effects);
    }

    // A three-arm continuous clause has one grammatical subject even though
    // its comma before `becomes` also looks like an ordinary effect-chain
    // boundary. Preserve the grammar-confirmed coordinated model before the
    // fallback chain splitter expands the middle arm and treats its subtype
    // payload as a new object-filter subject.
    if let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
        && is_loss_become_base_pt_coordinated_chain(&effects)
    {
        return Ok(effects);
    }

    // Same-object exile/return programs own their complete `, then` clause.
    // In particular, a timing suffix on the exile action scopes both actions:
    // "exile it at end of combat, then return it ..." is one delayed program.
    // Route that typed shape before the general comma-then splitter turns it
    // into two immediate zone changes and loses the timing wrapper.
    if let Some(effects) = parse_exile_then_return_same_object_sentence(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Exile subject=explicit recognizer=exile-return-same-object",
        );
        return Ok(effects);
    }

    // A looked-card partition owns its internal `, then` boundary.  Route the
    // grammar-proven full program before the generic chain splitter; otherwise
    // the leading look/exile actions can be mistaken for additional trigger
    // text and only the remainder move survives (for example, Clone Shell's
    // "look ..., exile one face down, then put the rest ..." trigger).
    if let Some(effects) =
        parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(tokens)
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Look subject=implicit recognizer=face-down-looked-partition",
        );
        return Ok(effects);
    }

    // The comma-then boundary in the each-player exile-top/cast program is
    // internal to one collection-producing effect.  Its typed recognizer
    // accumulates every iterated player's exiled card under one tag before
    // granting the trailing cast permissions.  Generic chain splitting would
    // instead lower the leading library object as one unowned card and lose
    // both the player loop and the collection relationship.
    if let Some(effects) =
        parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(tokens)?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Exile subject=each-player recognizer=exile-top-cast",
        );
        return Ok(effects);
    }

    // Once the specialist whole-sentence shapes above have had a chance to
    // claim the clause, an authored `, then` boundary must be parsed as an
    // executable chain before the broad subject/verb recognizer runs.  Broad
    // action parsers deliberately accept descriptive suffixes, so asking one
    // of them to parse the whole clause can otherwise keep only the leading
    // action (for example, `create a token, then copy that spell`) and silently
    // discard the follow-up.
    if super::super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(tokens) {
        // A where-X binding scopes the complete ordered program. Strip and
        // parse that binding before handing the action body to the chain
        // parser; otherwise both actions survive but the later X remains
        // unbound because the generic chain route never sees the value tail.
        if has_where_x_value_binding(tokens) {
            let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
            apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
            return Ok(effects);
        }
        return super::super::parse_effect_chain_lexed(tokens);
    }

    // `Put ... or remove ... counter` is a single typed counter operation,
    // not the generic action-choice form represented by `UnlessAction`.
    // Let the counter verb handler confirm the complete shape before the
    // broad top-level `or` splitter examines the sentence.
    if tokens.first().is_some_and(|token| token.is_word("put"))
        && let Ok(effect) = super::super::verb_dispatch::parse_effect_with_verb(
            super::super::Verb::Put,
            None,
            &tokens[1..],
        )
        && matches!(
            &effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutOrRemoveCounters { .. },
                ..
            })
        )
    {
        return Ok(vec![effect]);
    }

    // A serial negative keyword predicate is part of the mass-damage object
    // filter, not a top-level alternative-action list. Prove the complete
    // direct clause first and only preempt when it yields one damage sweep
    // with multiple excluded static abilities.
    let words = crate::lexer::parser_token_word_refs(tokens);
    let has_negative_ability_predicate = crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["doesn't", "have"],
            &["doesnt", "have"],
            &["does", "not", "have"],
        ],
    );
    if has_negative_ability_predicate
        && let Some(each_idx) =
            crate::slice_primitives::select_position(tokens, |token| token.is_word("each"))
        && let Some(filter_tokens) = tokens.get(each_idx + 1..)
        && let Ok(serial_filter) =
            crate::object_filters::parse_object_filter_lexed(filter_tokens, false)
        && serial_filter.excluded_static_abilities.len() >= 2
        && let Ok(mut effect) = super::super::clause_dispatch::parse_effect_clause_lexed(tokens)
    {
        // The broad damage primitive deliberately accepts the first complete
        // object-filter prefix.  Reattach the grammar-proven full serial
        // predicate before the later top-level `or` dispatcher can interpret
        // the final keyword as an alternative executable action.
        let repaired = match &mut effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDamageEach { filter, .. },
                ..
            }) => {
                *filter = serial_filter;
                true
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DealDamageEqualToPower {
                        target: crate::model::TargetAst::Object(filter, _, _),
                        ..
                    },
                ..
            }) => {
                *filter = serial_filter;
                true
            }
            _ => false,
        };
        if repaired {
            return Ok(vec![effect]);
        }
    }

    // A bounded target-player fanout has an explicit plural target set whose
    // members each perform the trailing action (`Two target players each
    // draw ...`). The broad subject/verb recognizer can parse its final verb
    // while collapsing the counted target phrase to one player, so give the
    // grammar-proven iterator ownership before that fallback.
    if let Some(effect) = super::super::parse_for_each_target_players_clause(tokens)? {
        return Ok(vec![effect]);
    }

    // An explicit top-level action choice must be split before the broad
    // subject/verb recognizer. Otherwise a later gain/lose verb can accept
    // the complete leading action as an object-filter subject and silently
    // retain only the final ability-grant branch.
    if let Some(unless_action) = super::super::parse_or_action_clause_lexed(tokens)? {
        return Ok(vec![unless_action]);
    }

    if let Some((route, mut effects)) = parse_top_level_subject_verb_recognition(tokens)? {
        crate::parse_trace::event(format!("effect-route: {route}"));
        normalize_search_followup_shuffles(&mut effects);
        return Ok(effects);
    }
    // The sentence dispatcher has exhausted its specialized routes here.
    // Delegate to the lower-level chain parser; calling this dispatcher again
    // with the same tokens recurses forever for ordinary subject/verb clauses.
    let mut effects = super::super::parse_effect_chain_inner_lexed(tokens)?;
    apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
    normalize_search_followup_shuffles(&mut effects);
    Ok(effects)
}

pub(super) fn parse_effect_sentence_with_where_x_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let view = crate::rule_engine::LexClauseView::from_tokens(tokens);
    match crate::effect_sentences::subject_verb_special_recognizers::parse_cross_zone_where_x_fanout_rule_lexed(&view) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }

    fn replace_search_filter_x(effect: &mut EffectAst, replacement: &Value) {
        let (filter, count, count_value) = match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::SearchLibrary {
                        filter,
                        count,
                        count_value,
                        ..
                    },
                ..
            }) => (filter, count, count_value),
            EffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                ..
            }
            | EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value,
                ..
            } => (filter, count, count_value),
            _ => return,
        };

        if count.dynamic_x && count_value.is_none() {
            *count_value = Some(replacement.clone());
        }
        if let Some(mana_value) = filter.mana_value.as_mut() {
            use crate::filter::Comparison;

            match mana_value {
                Comparison::EqualExpr(value)
                | Comparison::NotEqualExpr(value)
                | Comparison::LessThanExpr(value)
                | Comparison::LessThanOrEqualExpr(value)
                | Comparison::GreaterThanExpr(value)
                | Comparison::GreaterThanOrEqualExpr(value)
                    if matches!(value.as_ref(), Value::X) =>
                {
                    **value = replacement.clone();
                }
                _ => {}
            }
        }
    }

    fn bind_dynamic_target_count(target: &mut TargetAst, replacement: &Value) {
        fn bind_comparison_x(
            comparison: &mut Option<crate::filter::Comparison>,
            replacement: &Value,
        ) {
            let Some(
                crate::filter::Comparison::EqualExpr(value)
                | crate::filter::Comparison::NotEqualExpr(value)
                | crate::filter::Comparison::LessThanExpr(value)
                | crate::filter::Comparison::LessThanOrEqualExpr(value)
                | crate::filter::Comparison::GreaterThanExpr(value)
                | crate::filter::Comparison::GreaterThanOrEqualExpr(value),
            ) = comparison
            else {
                return;
            };
            if matches!(value.as_ref(), Value::X) {
                **value = replacement.clone();
            }
        }

        fn bind_filter_x(filter: &mut crate::target::ObjectFilter, replacement: &Value) {
            bind_comparison_x(&mut filter.power, replacement);
            bind_comparison_x(&mut filter.toughness, replacement);
            bind_comparison_x(&mut filter.mana_value, replacement);
            if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
                bind_filter_x(attached_to, replacement);
            }
            for branch in &mut filter.any_of {
                bind_filter_x(branch, replacement);
            }
        }

        match target {
            TargetAst::Object(filter, _, _) => bind_filter_x(filter, replacement),
            TargetAst::WithCount(inner, count) => {
                bind_dynamic_target_count(inner, replacement);
                if count.is_dynamic_x() {
                    let old = std::mem::replace(target, TargetAst::Source(None));
                    if let TargetAst::WithCount(inner, count) = old {
                        *target = TargetAst::WithCountValue(inner, count, replacement.clone());
                    }
                }
            }
            TargetAst::WithCountValue(inner, _, value) => {
                bind_dynamic_target_count(inner, replacement);
                if matches!(value, Value::X) {
                    *value = replacement.clone();
                }
            }
            _ => {}
        }
    }

    fn bind_dynamic_target_counts(effect: &mut EffectAst, replacement: &Value) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return;
        };
        match action {
            SubjectVerbActionAst::Explore { target }
            | SubjectVerbActionAst::Endure { target, .. }
            | SubjectVerbActionAst::Connive { target, .. }
            | SubjectVerbActionAst::ExchangeTextBoxes { target }
            | SubjectVerbActionAst::Attach { target, .. }
            | SubjectVerbActionAst::Unattach { object: target }
            | SubjectVerbActionAst::ReturnToHand { target, .. }
            | SubjectVerbActionAst::MayMoveToZone { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. }
            | SubjectVerbActionAst::MoveToZone { target, .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
            | SubjectVerbActionAst::TargetOnly { target, .. }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::SetBasePowerToughness { target, .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
            | SubjectVerbActionAst::SetBasePower { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::PumpByLastEffect { target, .. }
            | SubjectVerbActionAst::AddCardTypes { target, .. }
            | SubjectVerbActionAst::SetCardTypes { target, .. }
            | SubjectVerbActionAst::RemoveCardTypes { target, .. }
            | SubjectVerbActionAst::AddSubtypes { target, .. }
            | SubjectVerbActionAst::RemoveSubtypes { target, .. }
            | SubjectVerbActionAst::AddColors { target, .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandType { target, .. }
            | SubjectVerbActionAst::SetColors { target, .. }
            | SubjectVerbActionAst::MakeColorless { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeColorChoice { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { target, .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            }
            | SubjectVerbActionAst::RetargetStackObject { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDistributedDamage { target, .. }
            | SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target } => {
                bind_dynamic_target_count(target, replacement)
            }
            SubjectVerbActionAst::Destroy { target, .. } => {
                bind_dynamic_target_count(target, replacement)
            }
            SubjectVerbActionAst::PutCounters {
                target,
                target_count,
                ..
            } => {
                bind_dynamic_target_count(target, replacement);
                if let Some(count) = target_count
                    .as_ref()
                    .copied()
                    .filter(|count| count.is_dynamic_x())
                    && !matches!(target, TargetAst::WithCountValue(_, _, _))
                {
                    let inner = std::mem::replace(target, TargetAst::Source(None));
                    *target =
                        TargetAst::WithCountValue(Box::new(inner), count, replacement.clone());
                }
            }
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            } => {
                if let Some(target) = protected_target {
                    bind_dynamic_target_count(target, replacement);
                }
                if let Some(target) = destination_target {
                    bind_dynamic_target_count(target, replacement);
                }
            }
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
                ..
            }
            | SubjectVerbActionAst::DealDamageEqualToPower {
                source: creature1,
                target: creature2,
                ..
            }
            | SubjectVerbActionAst::BecomeCopy {
                target: creature1,
                source: creature2,
                ..
            } => {
                bind_dynamic_target_count(creature1, replacement);
                bind_dynamic_target_count(creature2, replacement);
            }
            SubjectVerbActionAst::CreateTokenCopyFromSource { source, .. } => {
                bind_dynamic_target_count(source, replacement);
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                attached_to: Some(target),
                ..
            } => bind_dynamic_target_count(target, replacement),
            _ => {}
        }
    }

    let clause_display = render_token_slice(tokens).trim().to_string();
    let Some(where_shape) = sentence_shapes::parse_where_x_sentence_tokens(tokens) else {
        return parse_effect_sentence_inner_lexed(tokens);
    };
    let aggregate_where =
        crate::keyword_static::parse_where_x_is_aggregate_filter_value(where_shape.where_tokens);
    let turn_history_where = aggregate_where
        .is_none()
        .then(|| {
            crate::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
                where_shape.where_tokens,
            )
        })
        .flatten();
    let has_semicolon_tail =
        where_shape.has_trailing_segment() && tokens.iter().any(OwnedLexToken::is_semicolon);
    let full_where_is_count_value = !where_shape.comma_tail_has_effect_clause
        && !has_semicolon_tail
        && (turn_history_where.is_some()
            || crate::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                where_shape.where_tokens,
            )
            .is_some()
            || crate::keyword_static::parse_where_x_is_number_of_filter_value(
                where_shape.where_tokens,
            )
            .is_some());
    let layout = where_shape.layout(full_where_is_count_value);
    let primary_where_tokens = layout.primary_where_tokens;
    let trailing_after_where = layout.trailing_after_where;
    let stripped = trim_edge_punctuation(where_shape.stripped_tokens);

    if let Some(effects) = parse_target_deals_power_damage_to_other_and_self_where_x(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) =
        parse_tap_then_damage_for_number_tapped_this_way(&stripped, primary_where_tokens)?
    {
        return Ok(effects);
    }

    let mut prelude_effects = Vec::new();
    // Only the action before the where-X binding determines what a possessive
    // reference denotes. A later effect clause is dispatched independently
    // and cannot turn "target creature ... where X is its power" back into a
    // source-relative value.
    let typed_where_references_target = where_shape.stripped_references_target
        && !sentence_shapes::starts_with_source_deals_x_tokens(&stripped);
    // Prefer the complete number-of family before the generic typed value
    // shape. The latter can correctly find the trailing object scope while
    // still losing the aggregate being measured, as in "the number of
    // abilities from among ... found among creatures you control."
    // A player-comparison value ends in an object noun ("more lands than
    // you"), but its cardinality is the number of qualifying players. Parse
    // that participant domain before the generic number-of-filter family can
    // collapse it to a battlefield-object count.
    let participant_comparison_where = turn_history_where
        .is_none()
        .then(|| {
            crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                primary_where_tokens,
            )
        })
        .flatten();
    // The complete value-expression parser treats a bare possessive such as
    // `its power` as source-relative. When the action before the where-X
    // clause introduced a target, the typed reference shape owns that
    // pronoun (`target creature ... where X is its power`). Explicit `this
    // creature's` remains source-relative through the shape's own reference
    // classification.
    let targeted_reference_where = typed_where_references_target
        .then(|| sentence_shapes::parse_where_x_value_shape_tokens(primary_where_tokens, true))
        .flatten()
        .filter(|shape| {
            matches!(
                shape,
                sentence_shapes::WhereXValueShape::ReferenceMetric { .. }
            )
        })
        .and_then(lower_where_x_shape);
    let exact_where_value = (turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && targeted_reference_where.is_none())
    .then(|| {
        super::super::dispatch_entry::parse_exact_where_x_value_expression(primary_where_tokens)
    })
    .flatten();
    let complete_number_where = (turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && exact_where_value.is_none())
    .then(|| crate::keyword_static::parse_where_x_is_number_of_filter_value(primary_where_tokens))
    .flatten();
    let typed_where_value = if targeted_reference_where.is_some() {
        targeted_reference_where
    } else if turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && exact_where_value.is_none()
        && complete_number_where.is_none()
    {
        sentence_shapes::parse_where_x_value_shape_tokens(
            primary_where_tokens,
            typed_where_references_target,
        )
        .and_then(lower_where_x_shape)
    } else {
        None
    };
    let where_value = if let Some(value) = aggregate_where {
        value
    } else if let Some(value) = turn_history_where {
        value
    } else if let Some(value) = participant_comparison_where {
        value
    } else if let Some(value) = exact_where_value {
        value
    } else if let Some(value) = complete_number_where {
        value
    } else if let Some((prelude, value)) = typed_where_value {
        if let Some(prelude) = prelude {
            prelude_effects.push(prelude);
        }
        value
    } else {
        let activation_time_trimmed =
            sentence_shapes::parse_before_activation_time_tokens(primary_where_tokens)
                .map(trim_edge_punctuation);
        let specific_where_value =
            super::super::dispatch_entry::parse_exact_where_x_value_expression(
                primary_where_tokens,
            )
            .or_else(|| {
                crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_different_powers_filter_value(
                    primary_where_tokens,
                )
            });
        let number_of_filter_value = specific_where_value
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_colored_mana_symbols_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_filter_value(primary_where_tokens)
            })
            .or_else(|| {
                activation_time_trimmed
                    .as_deref()
                    .and_then(crate::keyword_static::parse_where_x_is_number_of_filter_value)
            });
        if let Some(value) = number_of_filter_value {
            value
        } else if let Some(trimmed) = activation_time_trimmed.as_deref() {
            parse_value_binding_clause_lexed(trimmed).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x clause (clause: '{}')",
                    &clause_display
                ))
            })?
        } else {
            parse_value_binding_clause_lexed(primary_where_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x clause (clause: '{}')",
                    &clause_display
                ))
            })?
        }
    };
    let where_value = crate::effect_sentences::dispatch_entry::with_where_x_surface_hints(
        where_value,
        primary_where_tokens,
    );

    let search_like = where_shape.stripped_starts_search;
    let granted_entry_static = if crate::word_primitives::any_sequence_occurs(
        &crate::lexer::parser_token_word_refs(&stripped),
        &[&["enters", "with"], &["enter", "with"]],
    ) {
        let words = crate::lexer::parser_token_word_refs(&stripped);
        let explicit_source_subject = words.first() == Some(&"this")
            && words.get(2) == Some(&"enters")
            && words.get(1).is_some_and(|subject| {
                matches!(
                    *subject,
                    "artifact"
                        | "battle"
                        | "card"
                        | "creature"
                        | "enchantment"
                        | "land"
                        | "permanent"
                        | "planeswalker"
                        | "source"
                )
            });
        let entry_abilities = if explicit_source_subject {
            crate::keyword_static::parse_enters_with_counters_line(&stripped)?
        } else {
            parse_enters_with_additional_counter_for_filter_line(&stripped)?
                .map(|ability| vec![ability])
        };
        entry_abilities
            .filter(|abilities| !abilities.is_empty())
            .map(|abilities| {
                EffectAst::subject_verb_grant_abilities_to_target(
                    if explicit_source_subject {
                        TargetAst::Source(None)
                    } else {
                        TargetAst::Tagged(TagKey::from(IT_TAG), None)
                    },
                    abilities
                        .into_iter()
                        .map(|ability| {
                            GrantedAbilityAst::StaticAbility(Box::new(
                                crate::cards::builders::StaticAbilityAst::Static(ability),
                            ))
                        })
                        .collect(),
                    Until::Forever,
                )
            })
    } else {
        None
    };
    let mut effects = if search_like && !trailing_after_where.is_empty() {
        let mut recombined = stripped.clone();
        recombined.extend(trailing_after_where.clone());
        parse_effect_sentence_lexed(&recombined)?
    } else if let Some(grant) = granted_entry_static {
        let mut parsed = vec![grant];
        if !trailing_after_where.is_empty() {
            let mut trailing_effects = parse_effect_sentence_lexed(&trailing_after_where)?;
            parsed.append(&mut trailing_effects);
        }
        parsed
    } else {
        // The terminal where-X owner strips its binding before inner
        // dispatch. Preserve explicit player-subject boundaries at that
        // point too; otherwise `each opponent ... and you ...` is accepted
        // as one participant body and the controller action is repeated for
        // every opponent. The shared where value is rebound across both
        // resulting effects below.
        let mut parsed = if let Some(clauses) =
            super::super::player_subject_sequences::split_explicit_player_subject_clauses(&stripped)
        {
            let mut split_effects = Vec::new();
            for clause in clauses {
                split_effects.extend(parse_effect_sentence_inner_lexed(clause)?);
            }
            split_effects
        } else {
            parse_effect_sentence_inner_lexed(&stripped)?
        };
        if parsed.is_empty() && !stripped.is_empty() {
            parsed.push(super::super::parse_effect_clause_lexed(&stripped)?);
        }
        if !trailing_after_where.is_empty() {
            let mut trailing_effects = parse_effect_sentence_lexed(&trailing_after_where)?;
            parsed.append(&mut trailing_effects);
        }
        parsed
    };
    rebind_plural_create_followup_damage_source(&mut effects);
    replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_display)?;
    for effect in &mut effects {
        replace_search_filter_x(effect, &where_value);
        bind_dynamic_target_counts(effect, &where_value);
    }
    if !prelude_effects.is_empty() {
        prelude_effects.append(&mut effects);
        return Ok(prelude_effects);
    }
    Ok(effects)
}
