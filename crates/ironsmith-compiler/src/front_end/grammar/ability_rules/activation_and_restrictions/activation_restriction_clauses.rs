use super::*;
use crate::grammar::activation_restrictions as restriction_grammar;

const DAMAGED_THIS_WAY_TAG: &str = "damaged_0";

fn player_negated_restriction_subject(words: &[&str]) -> Option<PlayerFilter> {
    restriction_grammar::parse_player_negated_subject_words(words)
}

fn simple_negated_object_restriction(
    words: &[&str],
    filter: &ObjectFilter,
) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    let kind = restriction_grammar::parse_simple_object_restriction_words(words)?;
    use restriction_grammar::SimpleObjectRestrictionKind;
    Some(match kind {
        SimpleObjectRestrictionKind::Attack => Restriction::attack(filter.clone()),
        SimpleObjectRestrictionKind::AttackAlone => Restriction::attack_alone(filter.clone()),
        SimpleObjectRestrictionKind::AttackOrBlock => Restriction::attack_or_block(filter.clone()),
        SimpleObjectRestrictionKind::AttackOrBlockAlone => {
            Restriction::attack_or_block_alone(filter.clone())
        }
        SimpleObjectRestrictionKind::Block => Restriction::block(filter.clone()),
        SimpleObjectRestrictionKind::BlockAlone => Restriction::block_alone(filter.clone()),
        SimpleObjectRestrictionKind::BeBlocked => Restriction::be_blocked(filter.clone()),
        SimpleObjectRestrictionKind::BeDestroyed => Restriction::be_destroyed(filter.clone()),
        SimpleObjectRestrictionKind::BeRegenerated => Restriction::be_regenerated(filter.clone()),
        SimpleObjectRestrictionKind::BeSacrificed => Restriction::be_sacrificed(filter.clone()),
        SimpleObjectRestrictionKind::BeCountered => Restriction::be_countered(filter.clone()),
        SimpleObjectRestrictionKind::Transform => Restriction::transform(filter.clone()),
        SimpleObjectRestrictionKind::PhaseOut => Restriction::phase_out(filter.clone()),
        SimpleObjectRestrictionKind::PhaseIn => Restriction::phase_in(filter.clone()),
        SimpleObjectRestrictionKind::BeTargeted => Restriction::be_targeted(filter.clone()),
    })
}

fn source_filtered_target_restriction(
    tokens: &[OwnedLexToken],
    target_filter: &ObjectFilter,
) -> Result<Option<crate::effect::Restriction>, CardTextError> {
    use restriction_grammar::TargetRestrictionEnvelope;

    let Some(envelope) = restriction_grammar::parse_target_restriction_envelope_tokens(tokens)
    else {
        return Ok(None);
    };
    let error = || {
        CardTextError::ParseError(format!(
            "unsupported source-filtered target restriction tail (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))
    };

    let source_filter = match envelope {
        TargetRestrictionEnvelope::FilteredSources {
            spell_descriptor_tokens,
            source_descriptor_tokens,
        } => {
            let spell_filter = if let Some(range) = spell_descriptor_tokens {
                let spell_tokens = trim_commas(&tokens[range]);
                Some(
                    match parse_object_filter(&spell_tokens, false).ok() {
                        Some(filter) => Some(filter),
                        None => parse_subject_object_filter(&spell_tokens)?,
                    }
                    .ok_or_else(&error)?,
                )
            } else {
                None
            };
            let source_tokens = trim_commas(&tokens[source_descriptor_tokens]);
            let source_filter = match parse_object_filter(&source_tokens, false).ok() {
                Some(filter) => Some(filter),
                None => parse_subject_object_filter(&source_tokens)?,
            }
            .ok_or_else(&error)?;
            if spell_filter
                .as_ref()
                .is_some_and(|filter| filter != &source_filter)
            {
                return Err(error());
            }
            source_filter
        }
        TargetRestrictionEnvelope::SourceSpell {
            full_source_tokens,
            descriptor_tokens,
        } => {
            let source_tokens = trim_commas(&tokens[full_source_tokens]);
            let descriptor_tokens = trim_commas(&tokens[descriptor_tokens]);
            let mut source_filter = match parse_object_filter(&source_tokens, false).ok() {
                Some(filter) => Some(filter),
                None => parse_subject_object_filter(&source_tokens)?,
            }
            .or_else(
                || match parse_object_filter(&descriptor_tokens, false).ok() {
                    Some(filter) => Some(filter),
                    None => parse_subject_object_filter(&descriptor_tokens)
                        .ok()
                        .flatten(),
                },
            )
            .ok_or_else(error)?;
            source_filter.zone = Some(crate::zone::Zone::Stack);
            source_filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            source_filter
        }
    };

    Ok(Some(crate::effect::Restriction::be_targeted_from(
        target_filter.clone(),
        source_filter,
    )))
}

fn player_negated_restriction_from_tail(
    words: &[&str],
    player: PlayerFilter,
) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    if let Some(spell_filter) = parse_cast_restriction_tail_filter(words) {
        Some(Restriction::cast_spells_matching(player, spell_filter))
    } else {
        use restriction_grammar::PlayerRestrictionTailKind;
        Some(
            match restriction_grammar::parse_player_restriction_tail_words(words)? {
                PlayerRestrictionTailKind::GainLife => Restriction::gain_life(player),
                PlayerRestrictionTailKind::SearchLibraries => Restriction::search_libraries(player),
                PlayerRestrictionTailKind::LoseGame => Restriction::lose_game(player),
                PlayerRestrictionTailKind::LoseLife => Restriction::lose_life(player),
                PlayerRestrictionTailKind::WinGame => Restriction::win_game(player),
                PlayerRestrictionTailKind::DrawCards => Restriction::draw_cards(player),
                PlayerRestrictionTailKind::DrawExtraCards => Restriction::draw_extra_cards(player),
                PlayerRestrictionTailKind::PoisonCounters => Restriction::poison_counters(player),
                PlayerRestrictionTailKind::CastMoreThanOneSpellEachTurn => {
                    Restriction::cast_more_than_one_spell_each_turn(player)
                }
                PlayerRestrictionTailKind::CastSpells => {
                    Restriction::cast_spells_matching(player, ObjectFilter::spell())
                }
            },
        )
    }
}

fn damage_cause_life_loss_restriction_from_tail(
    words: &[&str],
) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    let player = match restriction_grammar::parse_damage_life_loss_tail_words(words)? {
        restriction_grammar::DamageLifeLossSubject::You => PlayerFilter::You,
        restriction_grammar::DamageLifeLossSubject::AnyPlayer => PlayerFilter::Any,
        restriction_grammar::DamageLifeLossSubject::IteratedPlayer => PlayerFilter::IteratedPlayer,
    };
    Some(Restriction::damage_cause_life_loss(player))
}

pub fn format_negated_restriction_display(tokens: &[OwnedLexToken]) -> String {
    let authored_self_surface = crate::slice_primitives::select_position(tokens, |token| {
        token.is_word("can't")
            || token.is_word("cant")
            || token.is_word("cannot")
            || token.is_word("can")
    })
    .and_then(|negation| {
        let subject_words = words(&tokens[..negation]);
        source_reference_surface_for_words(&subject_words)
    });
    let words = crate::lexer::token_word_refs(tokens);
    let mut out = Vec::with_capacity(words.len());
    let mut idx = 0usize;
    while idx < words.len() {
        match (words[idx], words.get(idx + 1).copied()) {
            ("cant", _) => {
                out.push("can't".to_string());
                idx += 1;
            }
            ("can", Some("not")) => {
                out.push("can't".to_string());
                idx += 2;
            }
            ("does", Some("not")) => {
                out.push("doesn't".to_string());
                idx += 2;
            }
            ("do", Some("not")) => {
                out.push("don't".to_string());
                idx += 2;
            }
            ("non", Some("phyrexian")) => {
                out.push("non-phyrexian".to_string());
                idx += 2;
            }
            ("aura", _) => {
                out.push("Aura".to_string());
                idx += 1;
            }
            _ => {
                out.push(words[idx].to_string());
                idx += 1;
            }
        }
    }
    let rendered = out.join(" ");
    if let Some(surface) = authored_self_surface
        && let Some((_, tail)) = crate::string_primitives::split_once(&rendered, " can't ")
    {
        return format!("{} can't {tail}", surface.display_text());
    }
    rendered
}

pub fn parse_cant_restrictions(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<ParsedCantRestriction>>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if matches!(
        restriction_grammar::parse_global_cant_restriction_words(&normalized),
        Some(restriction_grammar::GlobalCantRestrictionFact::PlayersLoseOrWin)
    ) {
        return Ok(Some(vec![
            ParsedCantRestriction {
                restriction: crate::effect::Restriction::lose_game(PlayerFilter::Any),
                target: None,
            },
            ParsedCantRestriction {
                restriction: crate::effect::Restriction::win_game(PlayerFilter::Any),
                target: None,
            },
        ]));
    }

    let words = crate::lexer::token_word_refs(tokens);
    if is_mana_retention_negated_clause(&words) {
        return Ok(None);
    }

    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    let segments = grammar::split_lexed_slices_on_and(tokens);
    if segments.len() > 1 {
        // A conjunction before the clause's only negation belongs to the
        // subject ("each attacking creature and each blocking creature
        // doesn't ..."), rather than separating multiple restrictions.
        if segments
            .first()
            .is_some_and(|segment| find_negation_span(segment).is_none())
        {
            return parse_cant_restriction_clause(tokens)
                .map(|restriction| restriction.map(|parsed| vec![parsed]));
        }

        let shared_subject = find_negation_span(segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut restrictions = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.to_vec();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().cloned());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && let Some((neg_start, _)) = find_negation_span(segment)
                && matches!(
                    trim_commas(&segment[..neg_start]).as_slice(),
                    [token] if token.is_word("it") || token.is_word("they")
                )
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment[neg_start..].iter().cloned());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(restriction) = parse_cant_restriction_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant restriction segment (clause: '{}')",
                    crate::lexer::token_word_refs(segment).join(" ")
                )));
            };
            let segment_words = normalize_cant_words(segment);
            let segment_word_refs = segment_words.iter().map(String::as_str).collect::<Vec<_>>();
            let has_or_win_tail =
                restriction_grammar::parse_or_win_game_tail_words(&segment_word_refs).is_some();
            if has_or_win_tail
                && let crate::effect::Restriction::LoseGame(player_filter) =
                    restriction.restriction.clone()
            {
                restrictions.push(ParsedCantRestriction {
                    restriction: crate::effect::Restriction::win_game(player_filter),
                    target: None,
                });
            }
            restrictions.push(restriction);
        }

        if restrictions.is_empty() {
            return Ok(None);
        }
        return Ok(Some(restrictions));
    }

    parse_cant_restriction_clause(tokens).map(|restriction| restriction.map(|r| vec![r]))
}

pub fn parse_cant_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let words = crate::lexer::token_word_refs(tokens);
    if is_mana_retention_negated_clause(&words) {
        return Ok(None);
    }

    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && !remainder.is_empty()
        && remainder.len() < tokens.len()
    {
        return parse_cant_restriction_clause(&remainder);
    }

    if let Some(parsed) = parse_player_negated_restriction_clause(tokens)? {
        return Ok(Some(parsed));
    }

    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let restriction = if let Some(parsed) = parse_cant_cast_restriction_words(&normalized) {
        parsed
    } else if let Some(fact) = restriction_grammar::parse_global_cant_restriction_words(&normalized)
    {
        use restriction_grammar::GlobalCantRestrictionFact;
        match fact {
            GlobalCantRestrictionFact::OpponentsBlockManaValueParity(parity) => Restriction::block(
                ObjectFilter::creature()
                    .opponent_controls()
                    .with_mana_value_parity(parity),
            ),
            GlobalCantRestrictionFact::GainLife(player) => Restriction::gain_life(player),
            GlobalCantRestrictionFact::SearchLibraries(player) => {
                Restriction::search_libraries(player)
            }
            GlobalCantRestrictionFact::DrawCards(player) => Restriction::draw_cards(player),
            GlobalCantRestrictionFact::DrawExtraCards(player) => {
                Restriction::draw_extra_cards(player)
            }
            GlobalCantRestrictionFact::PreventDamage => Restriction::prevent_damage(),
            GlobalCantRestrictionFact::LoseGame(player) => Restriction::lose_game(player),
            GlobalCantRestrictionFact::WinGame(player) => Restriction::win_game(player),
            GlobalCantRestrictionFact::ChangeLifeTotal(player) => {
                Restriction::change_life_total(player)
            }
            GlobalCantRestrictionFact::BecomeMonarch(player) => Restriction::become_monarch(player),
            GlobalCantRestrictionFact::PlayersLoseOrWin => {
                return parse_negated_object_restriction_clause(tokens);
            }
        }
    } else {
        return parse_negated_object_restriction_clause(tokens);
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target: None,
    }))
}

fn is_mana_retention_negated_clause(words: &[&str]) -> bool {
    restriction_grammar::parse_mana_retention_negated_clause_words(words).is_some()
}

fn is_mana_retention_tail(words: &[&str]) -> bool {
    restriction_grammar::parse_mana_retention_tail_words(words).is_some()
}

/// Parse "lose unspent [color] mana as steps [and phases end]" tails.
/// Returns `Some(color)` on a match; the inner option is the retained color
/// scope (`None` retains the whole pool).
pub fn parse_unspent_mana_retention_tail(words: &[&str]) -> Option<Option<crate::color::Color>> {
    crate::grammar::activation_restrictions::parse_unspent_mana_retention_tail_words(words)
        .map(|parsed| parsed.color)
}

pub fn parse_cant_cast_restriction_words(words: &[&str]) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;
    use restriction_grammar::CantCastRestrictionFact;

    Some(
        match restriction_grammar::parse_cant_cast_restriction_fact_words(words)? {
            CantCastRestrictionFact::CastSpells(player) => Restriction::cast_spells(player),
            CantCastRestrictionFact::CastCreatureSpells(player) => {
                Restriction::cast_creature_spells(player)
            }
            CantCastRestrictionFact::CastSpellsMatching { player, filter } => {
                Restriction::cast_spells_matching(player, filter)
            }
            CantCastRestrictionFact::CastMoreThanOneMatching { player, filter } => {
                restriction_from_cast_limit_filter(player, filter)
            }
        },
    )
}

pub fn strip_static_restriction_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::ConditionExpr, Vec<OwnedLexToken>)>, CardTextError> {
    use crate::grammar::activation_restrictions::{
        StaticRestrictionConditionKind, StaticRestrictionConditionShape,
        parse_source_attached_to_creature_condition_tokens,
        parse_static_restriction_condition_shape_tokens,
    };

    let Some(shape) = parse_static_restriction_condition_shape_tokens(tokens) else {
        return Ok(None);
    };
    match shape {
        StaticRestrictionConditionShape::Timing {
            timing,
            remainder_first,
            remainder_end,
        } => Ok(Some((
            crate::ConditionExpr::ActivationTiming(timing),
            trim_commas(&tokens[remainder_first..remainder_end]).to_vec(),
        ))),
        StaticRestrictionConditionShape::Condition {
            kind,
            condition,
            remainder_first,
        } => {
            let condition_tokens = trim_commas(&tokens[condition.first..condition.end]);
            let condition = match parse_static_condition_clause(&condition_tokens) {
                Ok(condition) => condition,
                Err(_) if kind == StaticRestrictionConditionKind::If => return Ok(None),
                Err(_) if parse_source_attached_to_creature_condition_tokens(&condition_tokens) => {
                    crate::ConditionExpr::SourceIsEquipped
                }
                Err(_) => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported static condition clause (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    )));
                }
            };
            Ok(Some((
                condition,
                trim_commas(&tokens[remainder_first..]).to_vec(),
            )))
        }
        StaticRestrictionConditionShape::ExtraTurn {
            remainder_first,
            remainder_end,
        } => Ok(Some((
            crate::ConditionExpr::CurrentTurnIsExtra,
            trim_commas(&tokens[remainder_first..remainder_end]).to_vec(),
        ))),
    }
}

pub fn parse_player_negated_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let Some((player, target)) = parse_player_restriction_subject(&subject_tokens)? else {
        return Ok(None);
    };
    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Ok(None);
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    use restriction_grammar::PlayerActivationRestrictionTailFact;
    let Some(fact) =
        restriction_grammar::parse_player_activation_restriction_tail_words(&remainder_words)
    else {
        return Ok(None);
    };
    let restriction = match fact {
        PlayerActivationRestrictionTailFact::CastSpellsMatching(filter) => {
            Restriction::cast_spells_matching(player, filter)
        }
        PlayerActivationRestrictionTailFact::CastSpells => Restriction::cast_spells(player),
        PlayerActivationRestrictionTailFact::ActivateNonManaAbilities => {
            Restriction::activate_non_mana_abilities(player)
        }
        PlayerActivationRestrictionTailFact::ActivateAbilitiesOf {
            mut filter,
            non_mana_only,
        } => {
            filter.controller = Some(player);
            if non_mana_only {
                Restriction::activate_non_mana_abilities_of(filter)
            } else {
                Restriction::activate_abilities_of(filter)
            }
        }
    };
    Ok(Some(ParsedCantRestriction {
        restriction,
        target,
    }))
}

pub fn parse_player_restriction_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerFilter, Option<TargetAst>)>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    if starts_with_target_indicator(subject_tokens) {
        let target = parse_target_phrase(subject_tokens)?;
        if let TargetAst::Player(player, span) = &target {
            return Ok(Some((
                target_ast_player_filter(player.clone(), *span),
                Some(target),
            )));
        }
        return Ok(None);
    }

    let normalized_storage = normalize_cant_words(subject_tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some(player) = restriction_grammar::parse_player_restriction_subject_words(&normalized) {
        return Ok(Some((player, None)));
    }

    let player = match parse_subject(subject_tokens) {
        crate::cards::builders::SubjectAst::Player(PlayerAst::You | PlayerAst::Implicit) => {
            PlayerFilter::You
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Opponent) => PlayerFilter::Opponent,
        crate::cards::builders::SubjectAst::Player(PlayerAst::That) => PlayerFilter::IteratedPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Defending) => PlayerFilter::Defending,
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsController) => {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsOwner) => {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Chosen) => PlayerFilter::ChosenPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Attacking) => PlayerFilter::Attacking,
        crate::cards::builders::SubjectAst::Player(PlayerAst::MostLifeTied) => {
            PlayerFilter::MostLifeTied
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::LowestLifeTied) => {
            PlayerFilter::LowestLifeTied
        }
        _ => return Ok(None),
    };
    Ok(Some((player, None)))
}

pub fn target_ast_player_filter(player: PlayerFilter, span: Option<TextSpan>) -> PlayerFilter {
    if span.is_some() {
        match player {
            PlayerFilter::Any => PlayerFilter::target_player(),
            PlayerFilter::Opponent => PlayerFilter::target_opponent(),
            other => other,
        }
    } else {
        player
    }
}

pub fn parse_cast_restriction_tail_filter(words: &[&str]) -> Option<ObjectFilter> {
    restriction_grammar::parse_cast_restriction_tail_filter_words(words)
}

fn parse_and_or_disjunction_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(separator_facts) = restriction_grammar::parse_and_or_separator_facts_tokens(tokens)
    else {
        return Ok(None);
    };

    let mut segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    let mut start = 0usize;
    for separator in separator_facts.separators {
        let segment = trim_commas(&tokens[start..separator.start]);
        if !segment.is_empty() {
            segments.push(segment.to_vec());
        }
        start = separator.end;
    }
    let tail = trim_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail.to_vec());
    }

    if segments.len() < 2 {
        return Ok(None);
    }

    let mut filters = Vec::with_capacity(segments.len());
    for segment in segments {
        let Some(filter) = parse_subject_object_filter(&segment)?
            .or_else(|| parse_object_filter(&segment, false).ok())
        else {
            return Ok(None);
        };
        filters.push(filter);
    }

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = filters;
    disjunction.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    Ok(Some(disjunction))
}

fn parse_distributive_compound_subject_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let separators = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let separator = token.is_word("and") || token.is_word("or");
            let starts_distributive_arm = tokens
                .get(index + 1)
                .is_some_and(|token| token.is_word("each") || token.is_word("every"));
            (separator && starts_distributive_arm).then_some(index)
        })
        .collect::<Vec<_>>();
    if separators.is_empty() {
        return Ok(None);
    }

    let mut filters = Vec::with_capacity(separators.len() + 1);
    let mut start = 0usize;
    for end in separators.into_iter().chain(std::iter::once(tokens.len())) {
        let segment = trim_commas(&tokens[start..end]);
        let filter_tokens = segment
            .first()
            .filter(|token| token.is_word("each") || token.is_word("every"))
            .map(|_| &segment[1..])
            .unwrap_or(segment.as_slice());
        let Some(filter) = parse_subject_object_filter(filter_tokens)? else {
            return Ok(None);
        };
        filters.push(filter);
        start = end.saturating_add(1);
    }

    let mut compound = ObjectFilter::default();
    compound.any_of = filters;
    Ok(Some(compound))
}

fn token_is_type_adjective(token: &OwnedLexToken) -> bool {
    token.as_word().is_some_and(|word| {
        let singular = crate::word_primitives::strip_word_suffix(word, "s").unwrap_or(word);
        crate::util::parse_card_type(word).is_some()
            || crate::util::parse_card_type(singular).is_some()
    })
}

fn token_is_type_or_subtype_noun(token: &OwnedLexToken) -> bool {
    token_is_type_adjective(token)
        || token
            .as_word()
            .is_some_and(|word| crate::util::parse_subtype_flexible(word).is_some())
}

fn token_is_comma(token: &OwnedLexToken) -> bool {
    token.kind == crate::lexer::TokenKind::Comma
}

/// Split a pre-conjunction segment into its comma-separated noun runs; each
/// run must consist entirely of type or subtype words.
fn type_noun_runs(segment: &[OwnedLexToken]) -> Option<Vec<Vec<OwnedLexToken>>> {
    let trimmed = trim_commas(segment);
    if trimmed.is_empty() {
        return None;
    }
    let mut runs = Vec::new();
    let mut run: Vec<OwnedLexToken> = Vec::new();
    for token in &trimmed {
        if token_is_comma(token) {
            if run.is_empty() {
                return None;
            }
            runs.push(std::mem::take(&mut run));
        } else if token_is_type_or_subtype_noun(token) {
            run.push(token.clone());
        } else {
            return None;
        }
    }
    if run.is_empty() {
        return None;
    }
    runs.push(run);
    Some(runs)
}

/// "Creature and enchantment spells you control" conjoins type adjectives
/// before a shared head noun and tail; parse it as one union filter with the
/// tail distributed over each adjective ("creature spells you control" or
/// "enchantment spells you control"). "Krakens, Leviathans, Octopuses, and
/// Serpents you control" distributes a bare qualifier tail over subtype
/// nouns the same way, and a bare list ("Goblins and Elves") unions with no
/// tail at all.
pub fn parse_type_adjective_conjunction_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    // "instant and/or sorcery cards" is an inclusive type list, not a
    // distributive conjunction; the and-splitter would orphan the or-half.
    let words = crate::lexer::token_word_refs(tokens);
    if crate::slice_primitives::contains(&words, &"and/or")
        || crate::word_primitives::sequence_occurs(&words, &["and", "or"])
    {
        return Ok(None);
    }
    let segments = grammar::split_lexed_slices_on_and(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }
    let (last, noun_segments) = segments.split_last().expect("checked len >= 2");
    let mut branch_heads: Vec<Vec<OwnedLexToken>> = Vec::new();
    for segment in noun_segments {
        let Some(mut runs) = type_noun_runs(segment) else {
            return Ok(None);
        };
        branch_heads.append(&mut runs);
    }

    let last = trim_commas(last);
    let mut head_types = 0usize;
    while head_types < last.len() && token_is_type_or_subtype_noun(&last[head_types]) {
        head_types += 1;
    }
    if head_types == 0 {
        return Ok(None);
    }
    let tail = &last[head_types..];
    // A card-type adjective before a shared head noun needs the tail to
    // carry that noun ("creature and enchantment SPELLS ..."); a bare
    // subtype union ("Goblins and Elves") stands alone.
    if tail.is_empty()
        && branch_heads
            .iter()
            .any(|head| head.iter().any(token_is_type_adjective))
    {
        return Ok(None);
    }

    let mut branches = Vec::with_capacity(branch_heads.len() + 1);
    for head in &branch_heads {
        let mut branch_tokens = head.clone();
        branch_tokens.extend_from_slice(tail);
        branches.push(parse_object_filter_lexed(&branch_tokens, false)?);
    }
    branches.push(parse_object_filter_lexed(&last, false)?);

    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Ok(Some(filter))
}

fn invert_except_by_blocker_filter(allowed: &ObjectFilter) -> Option<ObjectFilter> {
    let clauses: Vec<&ObjectFilter> = if allowed.any_of.is_empty() {
        vec![allowed]
    } else {
        allowed.any_of.iter().collect()
    };
    if clauses.is_empty() {
        return None;
    }

    let mut disallowed = ObjectFilter::creature();
    disallowed.set_union_connective(allowed.union_connective());
    for clause in clauses {
        if !clause.any_of.is_empty() {
            return None;
        }

        // `artifact creatures` carries both the shared blocker domain
        // (`Creature`) and the actual allowed qualifier (`Artifact`). Invert
        // every non-Creature type qualifier while leaving the implicit
        // blocker domain intact.
        for card_type in clause
            .card_types
            .iter()
            .chain(clause.all_card_types.iter())
            .copied()
            .filter(|card_type| *card_type != CardType::Creature)
        {
            disallowed = disallowed.without_type(card_type);
        }

        for subtype in &clause.subtypes {
            disallowed = disallowed.without_subtype(*subtype);
        }
        for ability in &clause.static_abilities {
            disallowed = disallowed.without_static_ability(*ability);
        }
        if let Some(colors) = clause.colors {
            disallowed = disallowed.without_colors(colors);
        }
    }

    Some(disallowed)
}

pub fn restriction_from_cast_limit_filter(
    player: PlayerFilter,
    spell_filter: ObjectFilter,
) -> crate::effect::Restriction {
    crate::effect::Restriction::cast_more_than_one_spell_each_turn_matching(player, spell_filter)
}

pub fn parse_negated_object_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let words = crate::lexer::token_word_refs(tokens);
    if restriction_grammar::parse_mana_retention_negated_clause_words(&words).is_some() {
        return Ok(None);
    }

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let subject_words_storage = normalize_cant_words(&subject_tokens);
    let subject_words = subject_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let bare_other_choice = crate::grammar::choices::parse_chosen_cant_block_shape(tokens)
        .ok()
        .flatten()
        .is_some_and(|shape| shape.bare_other_reference);
    if restriction_grammar::parse_leading_if_restriction_subject_words(&subject_words).is_some() {
        return Ok(None);
    }

    let (mut filter, mut target, ability_scope) =
        if let Some(parsed) = parse_activated_ability_subject(&subject_tokens)? {
            (parsed.filter, parsed.target, Some(parsed.scope))
        } else if starts_with_target_indicator(&subject_tokens) {
            let target = parse_target_phrase(&subject_tokens)?;
            let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported target restriction subject (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                ))
            })?;
            ensure_it_tagged_constraint(&mut filter);
            (filter, Some(target), None)
        } else if subject_tokens.is_empty() {
            // Supports carried clauses like "... and can't be blocked this turn."
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
            (
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                Some(target),
                None,
            )
        } else if crate::word_primitives::parse_any_sequence_complete(
            &subject_words,
            &[
                &["it"],
                &["that", "creature"],
                &["that", "permanent"],
                &["them"],
                &["those", "creatures"],
            ],
        ) {
            // A pronoun/demonstrative subject back-references the object the
            // trigger introduced (e.g. "Whenever this blocks or becomes
            // blocked, it can't be regenerated this turn"), not a filter over
            // every creature. target=None keeps it on the plain
            // cant-restriction path (no spurious "choose it").
            (ObjectFilter::tagged(TagKey::from(IT_TAG)), None, None)
        } else if bare_other_choice {
            (
                ObjectFilter::creature().not_tagged(TagKey::from(IT_TAG)),
                None,
                None,
            )
        } else if matches!(
            restriction_grammar::parse_restriction_subject_surface_words(&subject_words),
            Some(restriction_grammar::RestrictionSubjectSurface::Player)
        ) {
            (ObjectFilter::default(), None, None)
        } else {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported subject in negated restriction clause (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                )));
            };
            (filter, None, None)
        };
    // Several restriction-subject paths recognize controller-relative
    // phrases before the ordinary object-filter parser runs. Preserve the
    // authored head noun's number after those paths converge so
    // "creatures that player controls" cannot render as "a creature".
    if crate::grammar::filters::reference_tag_stage::has_plural_object_head_surface(&subject_tokens)
    {
        filter.set_plural_object_noun_surface(true);
    }
    if restriction_grammar::parse_dealt_damage_this_way_words(&words).is_some()
        && !filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == DAMAGED_THIS_WAY_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(DAMAGED_THIS_WAY_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing restriction tail in negated restriction clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let player_subject = player_negated_restriction_subject(&subject_words);
    if let Some(player) = player_subject {
        if let Some(color) = parse_unspent_mana_retention_tail(&remainder_words) {
            return Ok(Some(ParsedCantRestriction {
                restriction: Restriction::lose_unspent_mana(player, color),
                target: None,
            }));
        }
        if is_mana_retention_tail(&remainder_words) {
            return Ok(None);
        }
        let Some(restriction) = player_negated_restriction_from_tail(&remainder_words, player)
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player negated restriction tail (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            )));
        };
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target: None,
        }));
    }

    if subject_tokens.is_empty() && is_supported_untap_restriction_tail(&remainder_words) {
        filter = ObjectFilter::source();
        target = None;
    }

    let damage_subject = matches!(
        restriction_grammar::parse_restriction_subject_surface_words(&subject_words),
        Some(restriction_grammar::RestrictionSubjectSurface::Damage)
    );
    if damage_subject
        && restriction_grammar::parse_be_prevented_tail_words(&remainder_words).is_some()
    {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::prevent_damage(),
            target: None,
        }));
    }
    if damage_subject
        && let Some(restriction) = damage_cause_life_loss_restriction_from_tail(&remainder_words)
    {
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target: None,
        }));
    }
    if let Some(restriction) = simple_negated_object_restriction(&remainder_words, &filter) {
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target,
        }));
    }
    if let Some(restriction) = source_filtered_target_restriction(&remainder_tokens, &filter)? {
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target,
        }));
    }

    use restriction_grammar::NegatedObjectTailShape;
    let tail_shape = restriction_grammar::parse_negated_object_tail_words(&remainder_words);
    let restriction = match tail_shape {
        Some(NegatedObjectTailShape::AttackYou) => {
            Restriction::attack_player(filter, PlayerFilter::You)
        }
        Some(NegatedObjectTailShape::AttackYouOrPlaneswalkers) => {
            Restriction::attack_player_or_planeswalkers_controlled_by(filter, PlayerFilter::You)
        }
        Some(NegatedObjectTailShape::BeBlockedExceptBy { payload_words })
            if remainder_words.len() > payload_words =>
        {
            let blocker_tokens = trim_commas(&remainder_tokens[payload_words..]);
            let allowed_blocker_filter = parse_and_or_disjunction_filter(&blocker_tokens)?
                .or(parse_subject_object_filter(&blocker_tokens)?)
                .or_else(|| parse_object_filter(&blocker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    ))
                })?;
            let blocker_filter = invert_except_by_blocker_filter(&allowed_blocker_filter)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported except-by blocker filter (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(blocker_filter, filter)
        }
        Some(NegatedObjectTailShape::BeBlockedBy { payload_words })
            if remainder_words.len() > payload_words =>
        {
            let blocker_tokens = trim_commas(&remainder_tokens[payload_words..]);
            let blocker_filter = parse_and_or_disjunction_filter(&blocker_tokens)?
                .or(parse_subject_object_filter(&blocker_tokens)?)
                .or_else(|| parse_object_filter(&blocker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(blocker_filter, filter)
        }
        Some(NegatedObjectTailShape::BeActivated) => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) => {
                Restriction::activate_tap_abilities_of(filter)
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                )));
            }
        },
        Some(NegatedObjectTailShape::BeActivatedUnlessManaAbilities) => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_non_mana_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) | None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                )));
            }
        },
        Some(NegatedObjectTailShape::Block { payload_words })
            if remainder_words.len() > payload_words =>
        {
            let attacker_tokens = trim_commas(&remainder_tokens[payload_words..]);
            let attacker_filter = parse_and_or_disjunction_filter(&attacker_tokens)?
                .or(parse_subject_object_filter(&attacker_tokens)?)
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(filter, attacker_filter)
        }
        None if is_supported_untap_restriction_tail(&remainder_words) => Restriction::untap(filter),
        None if restriction_grammar::parse_effect_action_restriction_tail_words(
            &remainder_words,
        )
        .is_some() =>
        {
            return Ok(None);
        }
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported negated restriction tail (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            )));
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedAbilityScope {
    All,
    TapCostOnly,
}

#[derive(Debug, Clone)]
pub struct ParsedActivatedAbilitySubject {
    filter: ObjectFilter,
    target: Option<TargetAst>,
    scope: ActivatedAbilityScope,
}

pub fn strip_trailing_possessive_token(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    crate::grammar::activation_restrictions::parse_activation_possessive_owner_tokens(tokens)
}

pub fn parse_activated_ability_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedActivatedAbilitySubject>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let Some(owner_shape) = restriction_grammar::parse_activated_ability_owner_shape_tokens(tokens)
    else {
        return Ok(None);
    };
    let owner_tokens = trim_commas(&tokens[owner_shape.owner_tokens]);
    let scope = match owner_shape.scope {
        restriction_grammar::ActivatedAbilityOwnerScope::All => ActivatedAbilityScope::All,
        restriction_grammar::ActivatedAbilityOwnerScope::TapCostOnly => {
            ActivatedAbilityScope::TapCostOnly
        }
    };

    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let normalized_owner_tokens = strip_trailing_possessive_token(&owner_tokens);

    let owner_words = crate::lexer::token_word_refs(&normalized_owner_tokens);
    if restriction_grammar::parse_it_owner_reference_words(&owner_words).is_some() {
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            target: Some(TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(tokens),
            )),
            scope,
        }));
    }

    if starts_with_target_indicator(&normalized_owner_tokens) {
        let target = parse_target_phrase(&normalized_owner_tokens)?;
        let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported target restriction subject (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            ))
        })?;
        ensure_it_tagged_constraint(&mut filter);
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter,
            target: Some(target),
            scope,
        }));
    }

    let Some(filter) = parse_subject_object_filter(&normalized_owner_tokens)?
        .or_else(|| parse_object_filter(&normalized_owner_tokens, false).ok())
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject in negated restriction clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    };

    Ok(Some(ParsedActivatedAbilitySubject {
        filter,
        target: None,
        scope,
    }))
}

pub fn ensure_it_tagged_constraint(filter: &mut ObjectFilter) {
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

pub fn starts_with_possessive_activated_ability_subject(tokens: &[OwnedLexToken]) -> bool {
    restriction_grammar::parse_possessive_activated_ability_subject_tokens(tokens).is_some()
}

#[derive(Debug, Clone)]
pub struct ParsedCantRestriction {
    pub restriction: crate::effect::Restriction,
    pub target: Option<TargetAst>,
}

pub fn starts_with_target_indicator(tokens: &[OwnedLexToken]) -> bool {
    restriction_grammar::parse_target_indicator_tokens(tokens).is_some()
}

pub fn find_negation_span(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    crate::grammar::activation_restrictions::parse_activation_negation_span_tokens(tokens)
        .map(|span| (span.first, span.end))
}

pub fn parse_subject_object_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    match restriction_grammar::parse_restriction_subject_surface_words(&normalized_words) {
        Some(restriction_grammar::RestrictionSubjectSurface::Damage) => {
            return Ok(Some(ObjectFilter::default()));
        }
        Some(restriction_grammar::RestrictionSubjectSurface::Source) => {
            return Ok(Some(ObjectFilter::source()));
        }
        Some(restriction_grammar::RestrictionSubjectSurface::TaggedObjectPronoun) => {
            return Ok(Some(ObjectFilter::tagged(TagKey::from(IT_TAG))));
        }
        Some(restriction_grammar::RestrictionSubjectSurface::Player) | None => {}
    }

    let words_all = crate::lexer::token_word_refs(tokens);
    if let Some(shape) = restriction_grammar::parse_dealt_damage_by_source_subject_words(&words_all)
    {
        let word_view = crate::grammar::primitives::TokenWordView::new(tokens);
        let Some(base_end) = word_view.map_word_or_end_to_token_boundary(shape.base_word_count)
        else {
            return Ok(None);
        };
        let base_tokens = trim_commas(&tokens[..base_end]);
        let mut filter = parse_object_filter_lexed(&base_tokens, false)?;
        filter.dealt_damage_by_source_this_turn = Some(shape.damager);
        return Ok(Some(filter));
    }
    if restriction_grammar::parse_power_or_toughness_subject_words(&words_all).is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject object filter (clause: '{}')",
            words_all.join(" ")
        )));
    }

    if let Some(filter) = parse_distributive_compound_subject_filter(tokens)? {
        return Ok(Some(filter));
    }

    if let Some(filter) = parse_type_adjective_conjunction_filter(tokens)? {
        return Ok(Some(filter));
    }

    if let Ok(mut filter) = parse_object_filter(tokens, false)
        && filter != ObjectFilter::default()
    {
        if crate::grammar::filters::reference_tag_stage::has_plural_object_head_surface(tokens) {
            filter.set_plural_object_noun_surface(true);
        }
        return Ok(Some(filter));
    }

    let target = parse_target_phrase(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject target phrase (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))
    })?;

    Ok(target_ast_to_object_filter(target))
}

#[cfg(test)]
mod blocker_union_tests {
    use super::*;
    use crate::filter::Comparison;
    use crate::lexer::lex_line;

    fn parse_blockers(text: &str) -> ObjectFilter {
        let tokens = lex_line(text, 0).expect("blocking restriction should lex");
        let parsed = parse_negated_object_restriction_clause(&tokens)
            .expect("blocking restriction should route")
            .expect("blocking restriction should parse");
        let crate::effect::Restriction::BlockSpecificAttacker { blockers, .. } = parsed.restriction
        else {
            panic!("expected a blocker restriction");
        };
        blockers
    }

    #[test]
    fn named_self_subject_is_preserved_in_negated_restriction_display() {
        let tokens = lex_line("Locke can't be blocked by creatures with greater power.", 0)
            .expect("named restriction should lex");
        let named = format_negated_restriction_display(&tokens);
        assert_eq!(
            named,
            "Locke can't be blocked by creatures with greater power"
        );

        let ordinary = lex_line(
            "This creature can't be blocked by creatures with greater power.",
            0,
        )
        .expect("ordinary restriction should lex");
        assert_eq!(
            format_negated_restriction_display(&ordinary),
            "this creature can't be blocked by creatures with greater power"
        );
    }

    #[test]
    fn explicit_and_or_blocker_arms_keep_independent_qualifiers() {
        let blockers = parse_blockers(
            "Target creature can't be blocked by creatures with power 2 or less and/or Walls.",
        );

        assert_eq!(blockers.any_of.len(), 2, "{blockers:#?}");
        assert_eq!(
            blockers.union_connective(),
            crate::filter::ObjectFilterUnionConnective::AndOr
        );
        assert!(
            blockers
                .any_of
                .iter()
                .any(|branch| branch.power == Some(Comparison::LessThanOrEqual(2))),
            "{blockers:#?}"
        );
        assert!(
            blockers
                .any_of
                .iter()
                .any(|branch| branch.subtypes.contains(&Subtype::Wall)),
            "{blockers:#?}"
        );
    }

    #[test]
    fn except_by_and_or_union_inverts_every_allowed_arm() {
        let blockers = parse_blockers(
            "Target creature can't be blocked except by artifact creatures and/or red creatures.",
        );

        assert!(
            blockers.excluded_card_types.contains(&CardType::Artifact),
            "{blockers:#?}"
        );
        assert!(
            blockers.excluded_colors.contains(crate::Color::Red),
            "{blockers:#?}"
        );
    }

    #[test]
    fn plural_restriction_subject_preserves_its_head_noun_number() {
        let tokens = lex_line("creatures that player controls can't attack this turn.", 0)
            .expect("plural attack restriction should lex");
        let parsed = parse_negated_object_restriction_clause(&tokens)
            .expect("plural attack restriction should parse")
            .expect("expected a typed restriction");
        let crate::effect::Restriction::Attack(filter) = parsed.restriction else {
            panic!("expected an attack restriction");
        };

        assert!(filter.has_plural_object_noun_surface(), "{filter:#?}");
    }

    #[test]
    fn effect_restrictions_inherit_it_and_they_subjects() {
        for (text, expected_subtype) in [
            (
                "Clerics your opponents control can't block, and they can't attack you.",
                Subtype::Cleric,
            ),
            (
                "Rogues your opponents control can't block, and they can't attack you.",
                Subtype::Rogue,
            ),
        ] {
            let tokens = lex_line(text, 0).expect("restriction should lex");
            let restrictions = parse_cant_restrictions(&tokens)
                .expect("restriction conjunction should parse")
                .expect("expected typed restrictions");
            assert_eq!(restrictions.len(), 2, "{text}: {restrictions:#?}");
            let crate::effect::Restriction::Block(blockers) = &restrictions[0].restriction else {
                panic!("expected block restriction for {text}: {restrictions:#?}");
            };
            // Bare "can't attack you" is the player-only restriction; the
            // planeswalker-covering variant requires the authored
            // "or planeswalkers you control" tail.
            let crate::effect::Restriction::AttackPlayer {
                attackers,
                player: PlayerFilter::You,
            } = &restrictions[1].restriction
            else {
                panic!("expected attack restriction for {text}: {restrictions:#?}");
            };
            assert_eq!(blockers, attackers, "{text}");
            assert!(blockers.subtypes.contains(&expected_subtype), "{text}");
        }
    }

    #[test]
    fn source_relative_damage_subject_keeps_the_affected_creature_set() {
        let tokens = lex_line("Creatures dealt damage by this creature this turn", 0)
            .expect("source-relative restriction subject should lex");
        let filter = parse_subject_object_filter(&tokens)
            .expect("source-relative restriction subject should parse")
            .expect("source-relative restriction subject should produce a filter");

        assert!(!filter.source, "{filter:#?}");
        assert!(
            filter.card_types.contains(&CardType::Creature),
            "{filter:#?}"
        );
        assert_eq!(
            filter.dealt_damage_by_source_this_turn,
            Some(ironsmith_core::DamagedBySource::ThisCreature),
            "{filter:#?}"
        );
    }
}
