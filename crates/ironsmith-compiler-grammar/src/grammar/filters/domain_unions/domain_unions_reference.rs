use super::*;

pub(super) fn trailing_player_scope_is_shared(branches: &[ObjectFilter]) -> bool {
    let Some((last, preceding)) = branches.split_last() else {
        return false;
    };

    (last.controller.is_some() && preceding.iter().all(|branch| branch.controller.is_none()))
        || (last.owner.is_some()
            && preceding
                .iter()
                .all(|branch| branch.owner.is_none() && branch.controller.is_none()))
}

pub(super) fn propagate_trailing_shared_player_scope(branches: &mut [ObjectFilter]) {
    let Some((last, preceding)) = branches.split_last_mut() else {
        return;
    };

    if let Some(controller) = last.controller.clone()
        && preceding.iter().all(|branch| branch.controller.is_none())
    {
        for branch in preceding.iter_mut() {
            branch.controller = Some(controller.clone());
        }
    }
    // A trailing owner is usually a possessive on the last arm's own zone
    // ("... and each creature card in your graveyard"), not a scope stated once
    // for the whole list. An arm that already names its own controller
    // ("each creature you control") therefore keeps that scope alone: adding the
    // owner too would both narrow the match to permanents you own and render as
    // "creature you both own and control".
    if let Some(owner) = last.owner.clone()
        && preceding
            .iter()
            .all(|branch| branch.owner.is_none() && branch.controller.is_none())
    {
        for branch in preceding.iter_mut() {
            branch.owner = Some(owner.clone());
        }
    }
}

pub(super) fn contains_target_player_or_planeswalker_controller_relation(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    (0..words.len()).any(|start| {
        let mut relation = ObjectFilter::default();
        try_apply_player_relation_clause(
            &mut relation,
            &words[start..],
            &PlayerFilter::IteratedPlayer,
        )
        .is_some()
            && relation.controller == Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
    })
}

// A color adjective before a coordinated list of object domains scopes the
// entire list: "blue permanent, spell, or card in a graveyard". Independently
// qualified arms ("blue permanent or a red spell") do not share the adjective.
fn leading_shared_domain_colors(segments: &[&[OwnedLexToken]]) -> Option<crate::color::ColorSet> {
    if segments.len() < 2 {
        return None;
    }
    let domain = |word: &str| {
        matches!(
            word,
            "permanent" | "permanents" | "spell" | "spells" | "card" | "cards"
        )
    };
    if !segments.iter().skip(1).all(|segment| {
        TokenWordView::new(segment)
            .word_refs()
            .first()
            .is_some_and(|word| domain(word))
    }) {
        return None;
    }
    let view = TokenWordView::new(segments[0]);
    let words = view.word_refs();
    // "a red creature or spell": the first arm's noun may be a card type.
    let noun = words.iter().position(|word| {
        domain(word)
            || crate::util::parse_card_type(word).is_some()
            || word
                .strip_suffix('s')
                .is_some_and(|stem| crate::util::parse_card_type(stem).is_some())
    })?;
    let mut prefix = &words[..noun];
    if prefix
        .first()
        .is_some_and(|word| matches!(*word, "a" | "an"))
    {
        prefix = &prefix[1..];
    }
    let mut colors = crate::color::ColorSet::COLORLESS;
    let mut next_color = true;
    for word in prefix {
        if next_color {
            colors = colors.union(crate::util::parse_color(word)?);
        } else if *word != "or" {
            return None;
        }
        next_color = !next_color;
    }
    (!next_color && !colors.is_empty()).then_some(colors)
}

/// A bare singular card-type adjective arm such as `artifact` in `artifact
/// and enchantment cards from all graveyards`: its noun (and so its domain) is
/// elided and shared with the following arm.
///
/// Only a list whose final arm itself reads `<type> card(s) ...` shares the
/// noun that way; `target creature or card in a graveyard` keeps `creature`
/// as its own noun.
fn elided_card_noun_type_arms(segments: &[&[OwnedLexToken]]) -> Vec<bool> {
    let Some((last, preceding)) = segments.split_last() else {
        return Vec::new();
    };
    let last_words = TokenWordView::new(last).word_refs();
    let last_is_typed_card_noun = last_words
        .iter()
        .position(|word| matches!(*word, "card" | "cards"))
        .is_some_and(|noun| {
            noun > 0
                && last_words[..noun].iter().all(|word| {
                    crate::util::parse_card_type(word).is_some()
                        || crate::util::parse_non_type(word).is_some()
                })
        });
    let mut flags = preceding
        .iter()
        .map(|segment| last_is_typed_card_noun && is_elided_card_noun_type_arm(segment))
        .collect::<Vec<_>>();
    flags.push(false);
    flags
}

/// "an instant or sorcery spell that targets ..." / "a Faerie or Wizard
/// permanent spell": every earlier arm is a bare type adjective whose noun
/// (and everything after it) is the final arm's terminal `spell`/`permanent`
/// noun phrase. That is one shared selector, not independently scoped arms.
fn elides_shared_terminal_spell_or_permanent_noun(segments: &[&[OwnedLexToken]]) -> bool {
    let Some((last, preceding)) = segments.split_last() else {
        return false;
    };
    let strip_article = |words: Vec<&str>| -> Vec<String> {
        let mut words = words.into_iter().map(str::to_string).collect::<Vec<_>>();
        if words
            .first()
            .is_some_and(|word| matches!(word.as_str(), "a" | "an"))
        {
            words.remove(0);
        }
        words
    };
    let is_type_word = |word: &str| {
        crate::util::parse_card_type(word).is_some()
            || crate::util::parse_non_type(word).is_some()
            || crate::util::parse_subtype_word(word).is_some()
    };
    let last_words = strip_article(TokenWordView::new(last).word_refs());
    let Some(noun) = last_words
        .iter()
        .position(|word| matches!(word.as_str(), "spell" | "spells"))
    else {
        return false;
    };
    let head = &last_words[..noun];
    if head.is_empty()
        || !head
            .iter()
            .all(|word| word == "permanent" || is_type_word(word.as_str()))
    {
        return false;
    }
    preceding.iter().all(|segment| {
        let words = strip_article(TokenWordView::new(segment).word_refs());
        !words.is_empty() && words.iter().all(|word| is_type_word(word.as_str()))
    })
}

fn is_elided_card_noun_type_arm(segment: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(segment).word_refs();
    !words.is_empty()
        && words.iter().all(|word| {
            !word.ends_with('s')
                && (crate::util::parse_card_type(word).is_some()
                    || crate::util::parse_non_type(word).is_some())
        })
}

/// `Bird and/or Cleric permanent cards ...`: every preceding arm made only of
/// subtype words borrows the final arm's tail after its own subtype words, so
/// `Bird` reads `Bird permanent cards ...`. Returns `None` when no arm needs it.
fn elided_subtype_noun_arms(segments: &[&[OwnedLexToken]]) -> Option<Vec<Vec<OwnedLexToken>>> {
    let (last, preceding) = segments.split_last()?;
    let is_subtype_word = |token: &OwnedLexToken| {
        token
            .as_word()
            .is_some_and(|word| parse_subtype_flexible(word).is_some())
    };
    let tail_start = last.iter().position(|token| !is_subtype_word(token))?;
    if tail_start == 0 {
        return None;
    }
    let tail = &last[tail_start..];
    let shares_noun = tail.first().and_then(OwnedLexToken::as_word).is_some_and(|word| {
        matches!(
            word,
            "permanent" | "permanents" | "card" | "cards" | "creature" | "creatures"
                | "spell" | "spells"
        )
    });
    if !shares_noun {
        return None;
    }
    let mut changed = false;
    let mut expanded = Vec::with_capacity(segments.len());
    for segment in preceding {
        if !segment.is_empty() && segment.iter().all(is_subtype_word) {
            let mut arm = segment.to_vec();
            arm.extend_from_slice(tail);
            expanded.push(arm);
            changed = true;
        } else {
            expanded.push(segment.to_vec());
        }
    }
    expanded.push(last.to_vec());
    changed.then_some(expanded)
}

/// Parse a union whose branches each name their own object class and may
/// carry independent qualifiers.
///
/// Flattening `Creatures and Vehicles you control` produces the impossible
/// intersection "Vehicle creatures"; flattening `an attacking creature you
/// control or a blocking creature an opponent controls` loses both
/// branch-scoped controllers and combat states. Keep independently nouned
/// branches as `any_of` selectors, while factoring a genuinely shared
/// trailing controller/owner scope onto the outer filter.
pub fn parse_branch_scoped_object_filter_union_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    // A comma before an authored value-definition clause is not a list
    // separator. For example, in
    //
    // `creature with toughness X or less, where X is the number of Shrines
    // you control`
    //
    // the comparison grammar owns the entire suffix. Treating the comma as a
    // union boundary invents a second "Shrine you control" object-filter arm
    // and can even leak its controller onto the target creature.
    if tokens.iter().enumerate().any(|(index, token)| {
        token.is_comma()
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_word("where"))
    }) {
        return None;
    }
    if contains_relative_characteristic_union(tokens) {
        return None;
    }
    if contains_shared_characteristic_comparison_union(tokens) {
        return None;
    }
    if contains_other_than_exclusion(tokens) {
        return None;
    }
    if contains_attacking_player_or_planeswalker_relation(tokens) {
        return None;
    }
    if contains_historical_block_partner_relation(tokens) {
        return None;
    }
    if contains_current_block_partner_relation(tokens) {
        return None;
    }
    if contains_target_player_or_planeswalker_controller_relation(tokens) {
        return None;
    }
    if parse_elided_shared_domain_union(tokens, other).is_some() {
        return None;
    }

    let has_plain_or = tokens.iter().any(|token| token.is_word("or"));
    let has_and_or = tokens.iter().any(|token| token.is_word("and/or"));
    let has_plain_and = tokens.iter().any(|token| token.is_word("and"));
    let comma_segments = split_lexed_slices_on_comma(tokens);
    let segments = if comma_segments.len() >= 2 {
        comma_segments
            .into_iter()
            .map(|segment| {
                if segment.first().is_some_and(|token| {
                    token.is_word("and") || token.is_word("or") || token.is_word("and/or")
                }) {
                    segment.get(1..).unwrap_or_default()
                } else {
                    segment
                }
            })
            .collect()
    } else if has_plain_or {
        split_lexed_slices_on_or(tokens)
    } else {
        split_lexed_slices_on_list_conjunction(tokens)
    };
    if segments.len() < 2 {
        return None;
    }
    // "Bird and/or Cleric permanent cards from your graveyard": a bare
    // subtype arm elides the noun phrase that the final arm states once.
    // Give each such arm the final arm's shared tail so it parses with the
    // same noun and zone.
    let shared_tail_arms = elided_subtype_noun_arms(&segments);
    let segments: Vec<&[OwnedLexToken]> = match shared_tail_arms.as_ref() {
        Some(expanded) => expanded.iter().map(Vec::as_slice).collect(),
        None => segments,
    };
    if elides_shared_terminal_spell_or_permanent_noun(&segments) {
        return None;
    }
    let repeats_indefinite_article = segments.iter().skip(1).any(|segment| {
        segment
            .iter()
            .find_map(OwnedLexToken::as_word)
            .is_some_and(|word| matches!(word, "a" | "an"))
    });

    let shared_colors = leading_shared_domain_colors(&segments);
    // A bare singular card-type adjective arm (`artifact` in `artifact and
    // enchantment cards from all graveyards`) elides the card noun it shares
    // with the final arm, so it also shares that arm's trailing card zone.
    let elided_card_noun_arms = elided_card_noun_type_arms(&segments);
    // "instant or sorcery card from your graveyard or exiled card ...": a
    // bare type arm elides the card noun of the arm right after it and
    // shares that arm's domain (zone and owner), not the final arm's.
    let elided_into_next_arm = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            // Arms already sharing the final arm's domain ("enchantment,
            // instant, or sorcery card from an opponent's graveyard") keep it.
            !elided_card_noun_arms.get(index).copied().unwrap_or(false)
                && segments.get(index + 1).is_some_and(|next| {
                is_elided_card_noun_type_arm(segment)
                    && TokenWordView::new(next)
                        .word_refs()
                        .iter()
                        .any(|word| matches!(*word, "card" | "cards"))
            })
        })
        .collect::<Vec<_>>();

    let branches = segments
        .into_iter()
        .enumerate()
        .map(|(index, segment)| {
            let segment = if segment
                .first()
                .is_some_and(|token| token.is_word("all") || token.is_word("each"))
            {
                segment.get(1..).unwrap_or_default()
            } else {
                segment
            };
            let authored_other = segment
                .first()
                .is_some_and(|token| token.is_word("another") || token.is_word("other"));
            let segment = if authored_other {
                segment.get(1..).unwrap_or_default()
            } else {
                segment
            };
            // Some trigger families consume a leading `another` before they
            // delegate to the object-filter grammar. Start with it on the
            // first arm; once all arms are parsed, the shared-suffix analysis
            // below decides whether it scopes the coordinated set or remains
            // local to this independently nouned arm.
            crate::grammar::primitives::probe_shape(parse_object_filter(
                segment,
                authored_other || (other && index == 0),
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    let mut branches = branches
        .into_iter()
        .flat_map(|mut branch| {
            let nested = std::mem::take(&mut branch.any_of);
            if nested.is_empty() {
                vec![branch]
            } else if branch == ObjectFilter::default()
                && branch.union_connective() == ObjectFilterUnionConnective::Or
            {
                nested
            } else {
                // Preserve an opaque nested selector so the explicit-selector
                // guard below rejects an unsafe flattening.
                branch.any_of = nested;
                vec![branch]
            }
        })
        .collect::<Vec<_>>();
    if elided_into_next_arm.len() == branches.len() {
        for index in (0..branches.len()).rev() {
            if !elided_into_next_arm[index] {
                continue;
            }
            let (zone, owner, controller) = {
                let next = &branches[index + 1];
                (next.zone, next.owner.clone(), next.controller.clone())
            };
            let branch = &mut branches[index];
            if matches!(branch.zone, None | Some(Zone::Battlefield))
                && branch.owner.is_none()
                && branch.controller.is_none()
                && zone.is_some_and(|zone| zone != Zone::Battlefield)
            {
                branch.zone = zone;
                branch.owner = owner;
                branch.controller = controller;
            }
        }
    }
    for branch in &mut branches {
        if branch.zone == Some(Zone::Stack) && branch.has_mana_cost && branch.stack_kind.is_none() {
            branch.stack_kind = Some(crate::filter::StackObjectKind::Spell);
        }
    }
    if branches
        .iter()
        .any(|branch| !branch_has_explicit_object_selector(branch))
    {
        return None;
    }

    let shared_player_scope = trailing_player_scope_is_shared(&branches);
    let repeated_card_noun_surface = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .filter(|word| matches!(*word, "card" | "cards"))
        .count()
        >= branches.len();
    let repeated_other_surface = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .filter(|word| matches!(*word, "another" | "other"))
        .count()
        >= branches.len();
    // A leading set modifier ("other target artifact or creature") scopes the
    // whole coordinated list unless a later arm is requantified with its own
    // article ("another creature or an artifact").
    propagate_leading_shared_set_modifiers(
        tokens,
        other,
        shared_player_scope || !repeats_indefinite_article,
        &mut branches,
    );
    propagate_trailing_shared_player_scope(&mut branches);
    let elided_card_noun_arms = if elided_card_noun_arms.len() == branches.len() {
        elided_card_noun_arms
    } else {
        vec![false; branches.len()]
    };
    propagate_trailing_shared_card_zone_scope(
        &mut branches,
        repeated_card_noun_surface,
        &elided_card_noun_arms,
    );
    propagate_trailing_shared_attachment_scope(&mut branches);
    propagate_leading_shared_state(tokens, &mut branches);
    if !repeats_indefinite_article {
        distribute_trailing_head_noun_over_bare_subtype_arms(&mut branches);
    }
    let mut union = ObjectFilter::default();
    union.colors = shared_colors;
    factor_common_domain_scope(&mut branches, &mut union);
    if repeated_other_surface && union.other {
        // Every arm authored its own `other` determiner. Keep that identity
        // constraint branch-local so the union remains faithful when arms
        // are rendered or consumed independently.
        union.other = false;
        for branch in &mut branches {
            branch.other = true;
        }
    }
    let mixes_card_type_and_subtype = branches
        .iter()
        .any(|branch| !branch.card_types.is_empty() || !branch.all_card_types.is_empty())
        && branches.iter().any(|branch| !branch.subtypes.is_empty());
    let has_ability_selector = branches.iter().any(|branch| {
        !branch.ability_markers.is_empty()
            || !branch.excluded_ability_markers.is_empty()
            || !branch.static_abilities.is_empty()
            || !branch.excluded_static_abilities.is_empty()
    });
    if !mixes_card_type_and_subtype
        && !has_ability_selector
        && !branches.iter().any(branch_has_scoped_state)
    {
        // Common zone/controller/owner scope does not make otherwise bare
        // card-type arms branch-local. Let the ordinary filter grammar flatten
        // lists such as "artifact, creature, land, or planeswalker you
        // control" into one typed selector so downstream cost renderers retain
        // the authored list instead of treating it as a generic permanent.
        return None;
    }

    union.any_of = branches;
    // Early union recognition bypasses the ordinary filter tail pass. Shared
    // trailing restrictions still qualify every type arm of this selector.
    super::super::reference_tag_stage::lift_shared_trailing_mana_value_from_type_union(
        &mut union, tokens,
    );
    union.set_explicit_union_branch_articles(repeats_indefinite_article);
    if has_and_or {
        union.set_union_connective(ObjectFilterUnionConnective::AndOr);
    } else if has_plain_and && !has_plain_or {
        union.set_conjunctive_set_surface(true);
    }
    Some(union)
}

/// Parses a conjunction of independently scoped instances of the same object
/// selector. This keeps battlefield/controller and card-zone/owner facts on
/// separate `any_of` arms instead of collapsing them onto one filter.
pub fn parse_domain_union_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    if contains_shared_characteristic_comparison_union(tokens)
        || contains_other_than_exclusion(tokens)
        || contains_attacking_player_or_planeswalker_relation(tokens)
    {
        return None;
    }
    if let Some(filter) = parse_elided_shared_domain_union(tokens, other) {
        return Some(filter);
    }
    let segments = split_lexed_slices_on_list_conjunction(tokens);
    if segments.len() < 2 {
        return None;
    }
    // `artifact and enchantment cards from all graveyards`: a leading bare
    // type adjective shares the final arm's noun and domain, so the list is
    // one selector, not a union of independently scoped domains.
    if elided_card_noun_type_arms(&segments).into_iter().any(|elided| elided) {
        return None;
    }

    let branches = segments
        .into_iter()
        .map(|segment| {
            let segment = if segment.first().is_some_and(|token| token.is_word("each")) {
                segment.get(1..).unwrap_or_default()
            } else {
                segment
            };
            parse_simple_object_filter_lexed(segment, other)
        })
        .collect::<Option<Vec<_>>>()?;

    // Heterogeneous multi-zone target lists ("target spell, nonland
    // permanent, or card in a graveyard") union arms whose domains are all
    // explicit and pairwise distinct; picking one arm silently drops the
    // others.
    // Every arm must carry its own explicit zone: a battlefield-default arm
    // usually shares a trailing zone with its siblings ("Aura and/or
    // Equipment cards from your graveyard") and must not union.
    let distinct_zones: std::collections::HashSet<_> =
        branches.iter().map(|branch| branch.zone).collect();
    let heterogeneous_multi_zone = branches.iter().all(|branch| branch.zone.is_some())
        && distinct_zones.len() == branches.len();

    if !heterogeneous_multi_zone {
        let first_signature = domain_selector_signature(branches.first()?)?;
        if branches.iter().skip(1).any(|branch| {
            domain_selector_signature(branch)
                .as_ref()
                .is_none_or(|signature| signature != &first_signature)
        }) {
            return None;
        }

        let first_scope = ObjectDomainScope::from_filter(branches.first()?);
        if branches
            .iter()
            .skip(1)
            .all(|branch| ObjectDomainScope::from_filter(branch) == first_scope)
        {
            return None;
        }
    }

    let mut union = ObjectFilter {
        any_of: branches,
        ..ObjectFilter::default()
    };
    if tokens.iter().any(|token| token.is_word("and/or")) {
        union.set_union_connective(ObjectFilterUnionConnective::AndOr);
    }
    Some(union)
}

/// "target Wolf or Werewolf creature": the head noun after the last subtype
/// arm is shared by every bare subtype adjective before it, so a noncreature
/// Wolf is not a legal choice.
fn distribute_trailing_head_noun_over_bare_subtype_arms(branches: &mut [ObjectFilter]) {
    let Some((last, preceding)) = branches.split_last_mut() else {
        return;
    };
    if preceding.is_empty() || last.card_types.is_empty() || last.subtypes.is_empty() {
        return;
    }
    let is_bare_subtype_arm = |branch: &ObjectFilter| {
        branch.card_types.is_empty()
            && branch.all_card_types.is_empty()
            && branch.excluded_card_types.is_empty()
            && !branch.subtypes.is_empty()
            && branch.any_of.is_empty()
            && !branch.has_explicit_card_noun()
    };
    if !preceding.iter().all(is_bare_subtype_arm) {
        return;
    }
    for branch in preceding.iter_mut() {
        branch.card_types = last.card_types.clone();
    }
}
