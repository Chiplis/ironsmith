use super::*;

struct ImmediateObservation<'a> {
    effect: &'a Effect,
    tag: &'a TagKey,
    may_decider: Option<&'a PlayerFilter>,
}

impl<'a> ImmediateObservation<'a> {
    fn from_effect(effect: &'a Effect) -> Option<Self> {
        let unwrapped = structural_unwrap_render_wrappers(effect);
        if let Some(may) = unwrapped.downcast_ref::<crate::effects::MayEffect>() {
            let [inner] = may.effects.as_slice() else {
                return None;
            };
            let observation = Self::from_direct_effect(inner)?;
            let decider_is_you = may
                .decider
                .as_ref()
                .is_none_or(|decider| *decider == PlayerFilter::You);
            if !decider_is_you || !observation.is_you_observation() {
                return None;
            }
            return Some(Self {
                may_decider: may.decider.as_ref(),
                ..observation
            });
        }
        Self::from_direct_effect(unwrapped)
    }

    fn from_direct_effect(effect: &'a Effect) -> Option<Self> {
        if let Some(reveal) = effect.downcast_ref::<crate::effects::RevealTopEffect>() {
            return Some(Self {
                effect,
                tag: reveal.tag.as_ref()?,
                may_decider: None,
            });
        }
        let look = effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
        if look.count != Value::Fixed(1) {
            return None;
        }
        Some(Self {
            effect,
            tag: &look.tag,
            may_decider: None,
        })
    }

    fn is_you_observation(&self) -> bool {
        if let Some(reveal) = self
            .effect
            .downcast_ref::<crate::effects::RevealTopEffect>()
        {
            return reveal.player == PlayerFilter::You;
        }
        self.effect
            .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            .is_some_and(|look| look.player == PlayerFilter::You)
    }

    fn player(&self) -> &PlayerFilter {
        if let Some(reveal) = self
            .effect
            .downcast_ref::<crate::effects::RevealTopEffect>()
        {
            return &reveal.player;
        }
        &self
            .effect
            .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            .expect("immediate observation is reveal-top or look-at-top")
            .player
    }

    fn is_optional_reveal(&self) -> bool {
        self.may_decider.is_some()
            && self
                .effect
                .downcast_ref::<crate::effects::RevealTopEffect>()
                .is_some()
    }

    fn card_reference(&self) -> &'static str {
        if self
            .effect
            .downcast_ref::<crate::effects::RevealTopEffect>()
            .is_some()
        {
            "the revealed card"
        } else {
            "that card"
        }
    }

    fn text(&self) -> Option<String> {
        let rendered = describe_effect(self.effect);
        let rendered = rendered.trim().trim_end_matches('.');
        if rendered.is_empty() {
            return None;
        }
        if self.may_decider.is_some() {
            return Some(format!("You may {}", lowercase_first(rendered)));
        }
        Some(rendered.to_string())
    }
}

fn collect_observed_filters(
    condition: &Condition,
    observed_tag: &TagKey,
    filters: &mut Vec<ObjectFilter>,
) -> bool {
    match condition {
        Condition::TaggedObjectMatches(tag, filter) if tag == observed_tag => {
            filters.push(filter.clone());
            true
        }
        Condition::Or(left, right) => {
            collect_observed_filters(left, observed_tag, filters)
                && collect_observed_filters(right, observed_tag, filters)
        }
        _ => false,
    }
}

fn observed_filters(condition: &Condition, observed_tag: &TagKey) -> Option<Vec<ObjectFilter>> {
    let mut filters = Vec::new();
    collect_observed_filters(condition, observed_tag, &mut filters)
        .then_some(filters)
        .filter(|filters| !filters.is_empty())
}

fn filter_has_permanent_card_types(filter: &ObjectFilter) -> bool {
    const PERMANENT_TYPES: [CardType; 6] = [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.card_types.len() == PERMANENT_TYPES.len()
        && PERMANENT_TYPES
            .iter()
            .all(|card_type| filter.card_types.contains(card_type))
}

fn describe_permanent_card_filter(filter: &ObjectFilter) -> Option<String> {
    if !filter_has_permanent_card_types(filter) {
        return None;
    }
    let mut remainder = filter.clone();
    remainder.card_types.clear();
    let subtypes = std::mem::take(&mut remainder.subtypes);
    if remainder != ObjectFilter::default() {
        return None;
    }
    if subtypes.is_empty() {
        return Some("a permanent card".to_string());
    }
    let subtype_text = join_with_and(&subtypes.iter().map(ToString::to_string).collect::<Vec<_>>());
    Some(with_indefinite_article(&format!(
        "{subtype_text} permanent card"
    )))
}

fn describe_observed_filter(observed_tag: &TagKey, filter: &ObjectFilter) -> String {
    if let Some(permanent) = describe_permanent_card_filter(filter) {
        return permanent;
    }
    let described = describe_player_tagged_object_text(observed_tag, filter);
    if filter.card_types.is_empty()
        && !filter.subtypes.is_empty()
        && filter.excluded_card_types.is_empty()
        && !described.to_ascii_lowercase().contains(" card")
    {
        return format!("{described} card");
    }
    described
}

fn branch_casts_observed_card(effects: &[Effect], observed_tag: &TagKey) -> bool {
    effects.iter().any(|effect| {
        let effect = structural_unwrap_render_wrappers(effect);
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
            return branch_casts_observed_card(&may.effects, observed_tag);
        }
        effect
            .downcast_ref::<crate::effects::CastTaggedEffect>()
            .is_some_and(|cast| &cast.tag == observed_tag)
    })
}

fn filters_are_instant_or_sorcery(filters: &[ObjectFilter]) -> bool {
    let types = filters
        .iter()
        .flat_map(|filter| filter.card_types.iter().copied())
        .collect::<Vec<_>>();
    !types.is_empty()
        && types
            .iter()
            .all(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery))
        && types.contains(&CardType::Instant)
        && types.contains(&CardType::Sorcery)
}

fn combine_observed_descriptions(descriptions: &[String], final_noun: &str) -> Option<String> {
    let suffix = format!(" {final_noun}");
    let stems = descriptions
        .iter()
        .map(|description| {
            strip_leading_article(description)
                .trim()
                .strip_suffix(&suffix)
                .map(str::to_string)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(with_indefinite_article(&format!(
        "{} {final_noun}",
        join_with_or(&stems)
    )))
}

fn describe_observed_condition(
    filters: &[ObjectFilter],
    observed_tag: &TagKey,
    true_branch: &[Effect],
) -> String {
    if filters_are_instant_or_sorcery(filters)
        && branch_casts_observed_card(true_branch, observed_tag)
    {
        return "an instant or sorcery spell".to_string();
    }
    let descriptions = filters
        .iter()
        .map(|filter| describe_observed_filter(observed_tag, filter))
        .collect::<Vec<_>>();
    if descriptions.len() == 1 {
        return descriptions[0].clone();
    }
    let spell_noun = branch_casts_observed_card(true_branch, observed_tag)
        && filters_are_instant_or_sorcery(filters);
    let final_noun = if spell_noun { "spell" } else { "card" };
    combine_observed_descriptions(&descriptions, final_noun)
        .unwrap_or_else(|| join_with_or(&descriptions))
}

fn choose_spec_preserves_that_card_surface(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => {
            hints.iter().any(|hint| {
                matches!(
                    hint,
                    crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType(text)
                    ) if text.eq_ignore_ascii_case("that card")
                )
            }) || choose_spec_preserves_that_card_surface(spec)
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_preserves_that_card_surface(inner),
        _ => false,
    }
}

fn effect_preserves_that_card_surface(effect: &Effect) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);
    effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .is_some_and(|apply| {
            apply.runtime_modifications.iter().any(|modification| {
                matches!(
                    modification,
                    crate::effects::continuous::RuntimeModification::CopyOf { source, .. }
                        if choose_spec_preserves_that_card_surface(source)
                )
            })
        })
}

fn normalize_observed_action(effect: &Effect, text: &str) -> Option<String> {
    let text = text.trim().trim_end_matches('.');
    if text.is_empty() || text.contains(". ") || text.starts_with("If effect #") {
        return None;
    }
    let text = lowercase_first(text);
    if effect_preserves_that_card_surface(effect) {
        return Some(text);
    }
    Some(text.replace("That card", "It").replace("that card", "it"))
}

fn describe_may_actions(may: &crate::effects::MayEffect) -> Option<String> {
    let who = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let mut actions = may
        .effects
        .iter()
        .map(describe_branch_member)
        .collect::<Option<Vec<_>>>()?;
    if actions.is_empty() {
        return None;
    }
    if who == "you" {
        for action in &mut actions {
            if let Some(rest) = action.strip_prefix("you ") {
                *action = rest.to_string();
            }
        }
    }
    Some(format!("{who} may {}", join_with_and(&actions)))
}

fn describe_branch_member(effect: &Effect) -> Option<String> {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return describe_may_actions(may);
    }
    if effect.downcast_ref::<crate::effects::IfEffect>().is_some() {
        return None;
    }
    normalize_observed_action(effect, &describe_effect(effect))
}

fn describe_branch(effects: &[Effect]) -> Option<String> {
    if effects.is_empty() {
        return None;
    }
    let actions = effects
        .iter()
        .map(describe_branch_member)
        .collect::<Option<Vec<_>>>()?;
    Some(join_with_and(&actions))
}

fn observed_stat_axis(value: &Value, observed_tag: &TagKey) -> Option<&'static str> {
    match value.unhinted() {
        Value::PowerOf(spec) if choose_spec_references_tagged_object(spec, observed_tag) => {
            Some("power")
        }
        Value::ToughnessOf(spec) if choose_spec_references_tagged_object(spec, observed_tag) => {
            Some("toughness")
        }
        _ => None,
    }
}

fn describe_observed_life_stat_move_sequence(
    effects: &[Effect],
    observed_tag: &TagKey,
) -> Option<String> {
    let [gain_effect, lose_effect, move_effect] = effects else {
        return None;
    };
    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::GainLifeEffect>()?;
    let lose = structural_unwrap_render_wrappers(lose_effect)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if gain.player != lose.player {
        return None;
    }
    let gain_axis = observed_stat_axis(&gain.amount, observed_tag)?;
    let lose_axis = observed_stat_axis(&lose.amount, observed_tag)?;
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !choose_spec_references_tagged_object(&move_to_zone.target, observed_tag) {
        return None;
    }

    let actor = describe_choose_spec(&gain.player);
    let gain_verb = player_verb(&actor, "gain", "gains");
    let lose_verb = player_verb(&actor, "lose", "loses");
    let move_text = normalize_observed_action(move_effect, &describe_effect(move_effect))?;
    Some(format!(
        "{actor} {gain_verb} life equal to that card's {gain_axis}, {lose_verb} life equal to its {lose_axis}, then {move_text}"
    ))
}

fn describe_observed_move_with_actor(
    effect: &Effect,
    observed_tag: &TagKey,
    observing_player: &PlayerFilter,
) -> Option<String> {
    if *observing_player == PlayerFilter::You {
        return None;
    }
    let move_to_zone = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !choose_spec_references_tagged_object(&move_to_zone.target, observed_tag)
        || !move_to_zone
            .destination_player_surface
            .as_ref()
            .is_some_and(|player| player_filters_refer_to_same_player(player, observing_player))
    {
        return None;
    }
    let action = normalize_observed_action(effect, &describe_effect(effect))?;
    let rest = action.strip_prefix("put ")?;
    let actor = if *observing_player == PlayerFilter::IteratedPlayer {
        "the player".to_string()
    } else {
        describe_player_filter(observing_player)
    };
    Some(format!("{actor} puts {rest}"))
}

fn describe_observed_branch(
    effects: &[Effect],
    observed_tag: &TagKey,
    observing_player: &PlayerFilter,
) -> Option<String> {
    if let Some(sequence) = describe_observed_life_stat_move_sequence(effects, observed_tag) {
        return Some(sequence);
    }
    if effects.is_empty() {
        return None;
    }
    let actions = effects
        .iter()
        .map(|effect| {
            describe_observed_move_with_actor(effect, observed_tag, observing_player)
                .or_else(|| describe_branch_member(effect))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(join_with_and(&actions))
}

fn choose_spec_identity_tag(spec: &ChooseSpec) -> Option<&TagKey> {
    match spec.unhinted() {
        ChooseSpec::Tagged(tag) => Some(tag),
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_identity_tag(inner),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            let mut tags = filter.tagged_constraints.iter().filter_map(|constraint| {
                (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
                    .then_some(&constraint.tag)
            });
            let tag = tags.next()?;
            tags.next().is_none().then_some(tag)
        }
        _ => None,
    }
}

fn choose_spec_has_that_demonstrative(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => {
            hints.iter().any(|hint| {
                matches!(
                    hint,
                    crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType(text)
                    ) if text.to_ascii_lowercase().starts_with("that ")
                )
            }) || choose_spec_has_that_demonstrative(spec)
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_has_that_demonstrative(inner),
        _ => false,
    }
}

fn replace_shared_else_subject_with_it(
    true_effects: &[Effect],
    false_effects: &[Effect],
    false_text: String,
) -> String {
    let Some(true_target) = true_effects.iter().find_map(|effect| {
        rendered_action_target(effect).filter(|target| choose_spec_has_that_demonstrative(target))
    }) else {
        return false_text;
    };
    let Some(true_tag) = choose_spec_identity_tag(true_target) else {
        return false_text;
    };
    let Some(false_target) = false_effects
        .iter()
        .find_map(|effect| rendered_action_target(effect))
    else {
        return false_text;
    };
    if choose_spec_identity_tag(false_target) != Some(true_tag) {
        return false_text;
    }

    const VERB_MARKERS: [&str; 9] = [
        " gets ",
        " gains ",
        " becomes ",
        " has ",
        " loses ",
        " is ",
        " can't ",
        " doesn't ",
        " must ",
    ];
    let Some(subject_end) = VERB_MARKERS
        .iter()
        .filter_map(|marker| false_text.find(marker))
        .min()
    else {
        return false_text;
    };
    format!("it{}", &false_text[subject_end..])
}

struct SharedDeclineFallback {
    primary: String,
    fallback: String,
    battlefield_tag: Option<TagKey>,
    predicate: EffectPredicate,
}

fn tagged_battlefield_move(may: &crate::effects::MayEffect) -> Option<&TagKey> {
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield {
        return None;
    }
    match move_to_zone.target.base() {
        ChooseSpec::Tagged(tag) => Some(tag),
        _ => None,
    }
}

fn normalize_optional_battlefield_reference(text: String, reference: &str) -> String {
    ["it", "them", "that card", "the card"]
        .into_iter()
        .fold(text, |text, current| {
            text.replace(
                &format!("put {current} onto the battlefield"),
                &format!("put {reference} onto the battlefield"),
            )
        })
}

fn describe_shared_decline_fallback(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<SharedDeclineFallback> {
    let [primary, on_decline] = conditional.if_true.as_slice() else {
        return None;
    };
    let with_id = primary.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    let battlefield_tag = tagged_battlefield_move(may).cloned();
    let if_effect = on_decline.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || !matches!(
            if_effect.predicate,
            EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined
        )
        || !if_effect.else_.is_empty()
        || conditional.if_false.is_empty()
    {
        return None;
    }
    let decline = describe_branch(&if_effect.then)?;
    let false_branch = describe_branch(&conditional.if_false)?;
    if decline != false_branch {
        return None;
    }
    Some(SharedDeclineFallback {
        primary: describe_branch(std::slice::from_ref(primary))?,
        fallback: false_branch,
        battlefield_tag,
        predicate: if_effect.predicate.clone(),
    })
}

fn describe_was_declined_battlefield_fallback(
    shared: SharedDeclineFallback,
    condition_prefix: &str,
    primary_reference: &str,
    fallback_reference: &str,
) -> Option<String> {
    if shared.predicate != EffectPredicate::WasDeclined {
        return None;
    }
    let primary = normalize_optional_battlefield_reference(shared.primary, primary_reference);
    Some(format!(
        "{condition_prefix}, {primary}. If you don't put {fallback_reference} onto the battlefield, {}",
        shared.fallback
    ))
}

pub(super) fn describe_was_declined_optional_battlefield_fallback_conditional(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    let shared = describe_shared_decline_fallback(conditional)?;
    if shared.predicate != EffectPredicate::WasDeclined {
        return None;
    }
    let battlefield_tag = shared.battlefield_tag.as_ref()?;
    let searched_card = battlefield_tag.as_str() == "searched";
    let primary_reference = if searched_card { "that card" } else { "it" };
    let fallback_reference = if searched_card { "the card" } else { "it" };
    let mut condition = describe_condition(&conditional.condition);
    if let Some(rest) = condition.strip_prefix("its mana value is ") {
        condition = format!("it has mana value {rest}");
    }
    if matches!(&conditional.condition, Condition::YourTurn) {
        let primary = normalize_optional_battlefield_reference(shared.primary, primary_reference);
        return Some(format!(
            "{} if it's your turn. If you don't put {fallback_reference} onto the battlefield, {}",
            capitalize_first(&primary),
            shared.fallback
        ));
    }
    describe_was_declined_battlefield_fallback(
        shared,
        &format!("If {condition}"),
        primary_reference,
        fallback_reference,
    )
}

fn describe_observed_conditional(
    conditional: &crate::effects::ConditionalEffect,
    observed_tag: &TagKey,
    observing_player: &PlayerFilter,
    optional_reveal: bool,
) -> Option<String> {
    let filters = observed_filters(&conditional.condition, observed_tag)?;
    let condition = describe_observed_condition(&filters, observed_tag, &conditional.if_true);
    let (true_branch, false_branch) = if let Some(shared) =
        describe_shared_decline_fallback(conditional)
    {
        if shared.predicate == EffectPredicate::WasDeclined
            && shared.battlefield_tag.as_ref() == Some(observed_tag)
        {
            let condition_prefix = if optional_reveal {
                format!("If {condition} is revealed this way")
            } else {
                format!("If it's {condition}")
            };
            let land_card = filters
                .iter()
                .all(|filter| filter.card_types.as_slice() == [CardType::Land]);
            let fallback_reference = if land_card { "the card" } else { "it" };
            return describe_was_declined_battlefield_fallback(
                shared,
                &condition_prefix,
                "it",
                fallback_reference,
            );
        }
        (shared.primary, Some(shared.fallback))
    } else {
        let false_branch = if conditional.if_false.is_empty() {
            None
        } else {
            let rendered =
                describe_observed_branch(&conditional.if_false, observed_tag, observing_player)?;
            Some(replace_shared_else_subject_with_it(
                &conditional.if_true,
                &conditional.if_false,
                rendered,
            ))
        };
        (
            describe_observed_branch(&conditional.if_true, observed_tag, observing_player)?,
            false_branch,
        )
    };
    let mut rendered = if optional_reveal {
        format!("If {condition} is revealed this way, {true_branch}")
    } else {
        format!("If it's {condition}, {true_branch}")
    };
    if let Some(false_branch) = false_branch {
        rendered.push_str(&format!(". Otherwise, {false_branch}"));
    }
    Some(rendered)
}

fn describe_observation_continuation(
    effect: &Effect,
    observation: &ImmediateObservation<'_>,
) -> Option<String> {
    let unwrapped = structural_unwrap_render_wrappers(effect);
    if let Some(move_to_zone) = unwrapped.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        if !choose_spec_references_tagged_object(&move_to_zone.target, observation.tag) {
            return None;
        }
        let mut action = normalize_observed_action(effect, &describe_effect(effect))?;
        for antecedent in ["it", "that card"] {
            if let Some(rest) = action.strip_prefix(&format!("put {antecedent} ")) {
                action = format!("put {} {rest}", observation.card_reference());
                break;
            }
        }
        return Some(format!("Then {action}"));
    }

    let shuffle = unwrapped.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !player_filters_refer_to_same_player(&shuffle.player, observation.player()) {
        return None;
    }
    if *observation.player() == PlayerFilter::You {
        Some("Then you shuffle".to_string())
    } else {
        Some("Then that player shuffles".to_string())
    }
}

fn branch_moves_observed_to_battlefield(effects: &[Effect], observed_tag: &TagKey) -> bool {
    effects.iter().any(|effect| {
        let effect = structural_unwrap_render_wrappers(effect);
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
            return branch_moves_observed_to_battlefield(&may.effects, observed_tag);
        }
        effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_to_zone| {
                move_to_zone.zone == Zone::Battlefield
                    && matches!(
                        move_to_zone.target.base(),
                        ChooseSpec::Tagged(tag) if tag == observed_tag
                    )
            })
    })
}

fn is_observed_not_on_battlefield(condition: &Condition, observed_tag: &TagKey) -> bool {
    let Condition::Not(inner) = condition else {
        return false;
    };
    matches!(
        inner.as_ref(),
        Condition::PlayerTaggedObjectMatches { player, tag, filter }
            if *player == PlayerFilter::You
                && tag == observed_tag
                && filter.zone == Some(Zone::Battlefield)
    )
}

fn describe_if_not_moved_fallback(
    previous: &crate::effects::ConditionalEffect,
    fallback: &crate::effects::ConditionalEffect,
    observed_tag: &TagKey,
) -> Option<String> {
    if !fallback.if_false.is_empty()
        || !is_observed_not_on_battlefield(&fallback.condition, observed_tag)
        || !branch_moves_observed_to_battlefield(&previous.if_true, observed_tag)
    {
        return None;
    }
    Some(format!(
        "If you don't put the card onto the battlefield, {}",
        describe_branch(&fallback.if_true)?
    ))
}

fn describe_observation_after_if_result(effects: &[&Effect]) -> Option<(String, usize)> {
    let with_id = effects
        .first()?
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let if_effect = effects.get(1)?.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [observation_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let observation = ImmediateObservation::from_effect(observation_effect)?;
    let conditional = effects
        .get(2)?
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let observed_conditional = describe_observed_conditional(
        conditional,
        observation.tag,
        observation.player(),
        observation.is_optional_reveal(),
    )?;
    let setup = describe_optional_setup_effect_for_if_happened(with_id)
        .unwrap_or_else(|| describe_effect(&with_id.effect));
    let observation_branch = describe_with_id_if_clause(with_id, if_effect)?;
    Some((
        format!("{setup}. {observation_branch}. {observed_conditional}"),
        3,
    ))
}

fn describe_observed_life_payment(effects: &[&Effect]) -> Option<(String, usize)> {
    let observation = ImmediateObservation::from_effect(*effects.first()?)?;
    let with_id = effects
        .get(1)?
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let conditional = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let filters = observed_filters(&conditional.condition, observation.tag)?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You)) {
        return None;
    }
    let [life_payment] = may.effects.as_slice() else {
        return None;
    };
    let lose_life = structural_unwrap_render_wrappers(life_payment)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose_life.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let if_effect = effects.get(2)?.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let condition = describe_observed_condition(&filters, observation.tag, &conditional.if_true);
    let followup = describe_branch(&if_effect.then)?;
    Some((
        format!(
            "{}. If it's {condition}, you may pay {} life. If you do, {followup}",
            observation.text()?,
            describe_value(&lose_life.amount)
        ),
        3,
    ))
}

pub(super) fn describe_immediate_observation_conditionals(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    if let Some(rendered) = describe_observation_after_if_result(effects) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_observed_life_payment(effects) {
        return Some(rendered);
    }
    let observation = ImmediateObservation::from_effect(*effects.first()?)?;
    let first_conditional = effects
        .get(1)?
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let mut sentences = vec![
        observation.text()?,
        describe_observed_conditional(
            first_conditional,
            observation.tag,
            observation.player(),
            observation.is_optional_reveal(),
        )?,
    ];
    let mut consumed = 2;
    let mut previous = first_conditional;
    while let Some(conditional) = effects
        .get(consumed)
        .and_then(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
    {
        let rendered = describe_if_not_moved_fallback(previous, conditional, observation.tag)
            .or_else(|| {
                describe_observed_conditional(
                    conditional,
                    observation.tag,
                    observation.player(),
                    observation.is_optional_reveal(),
                )
            });
        let Some(rendered) = rendered else {
            break;
        };
        sentences.push(rendered);
        previous = conditional;
        consumed += 1;
    }
    while let Some(effect) = effects.get(consumed) {
        let Some(rendered) = describe_observation_continuation(effect, &observation) else {
            break;
        };
        sentences.push(rendered);
        consumed += 1;
    }
    Some((sentences.join(". "), consumed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith_core::EffectId;

    fn tagged_move(tag: &TagKey, zone: Zone) -> Effect {
        Effect::new(
            crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag.clone()), zone, false)
                .with_destination_player_surface(PlayerFilter::You),
        )
    }

    #[test]
    fn reveal_conditional_preserves_filter_qualifiers() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let mut creature = ObjectFilter::creature();
        creature.chosen_creature_type = true;
        let effects = vec![
            Effect::reveal_top(PlayerFilter::You, tag.clone()),
            Effect::conditional(
                Condition::TaggedObjectMatches(tag.clone(), creature),
                vec![tagged_move(&tag, Zone::Hand)],
                vec![tagged_move(&tag, Zone::Graveyard)],
            ),
        ];
        let rendered = describe_effect_list(&effects);
        assert!(
            rendered.contains("If it's a creature card of the chosen type"),
            "{rendered}"
        );
        assert!(!rendered.contains("was revealed this way"), "{rendered}");
        assert!(!rendered.contains("Then if"), "{rendered}");
    }

    #[test]
    fn reveal_conditional_preserves_mana_value_qualifier() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let mut creature = ObjectFilter::creature();
        creature.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(3));
        let effects = vec![
            Effect::reveal_top(PlayerFilter::You, tag.clone()),
            Effect::conditional_only(
                Condition::TaggedObjectMatches(tag.clone(), creature),
                vec![tagged_move(&tag, Zone::Battlefield)],
            ),
        ];

        assert_eq!(
            describe_effect_list(&effects),
            "Reveal the top card of your library. If it's a creature card with mana value 3 or less, put it onto the battlefield"
        );
    }

    #[test]
    fn payment_gated_reveal_keeps_observation_conditional_surface() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let payment = Effect::new(crate::effects::PayManaEffect::new(
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(2), ManaSymbol::Red]),
            ChooseSpec::Player(PlayerFilter::You),
        ));
        let setup = Effect::with_id(0, Effect::may_single(payment));
        let reveal = Effect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::reveal_top(PlayerFilter::You, tag.clone())],
        );
        let mut goblin_permanent = ObjectFilter::permanent_card();
        goblin_permanent.subtypes.push(Subtype::Goblin);
        let conditional = Effect::conditional(
            Condition::TaggedObjectMatches(tag.clone(), goblin_permanent),
            vec![tagged_move(&tag, Zone::Battlefield)],
            vec![tagged_move(&tag, Zone::Graveyard)],
        );

        assert_eq!(
            describe_effect_list(&[setup, reveal, conditional]),
            "You may pay {2}{R}. If you do, reveal the top card of your library. If it's a Goblin permanent card, put it onto the battlefield. Otherwise, put it into your graveyard"
        );
    }

    #[test]
    fn observed_optional_life_payment_renders_as_payment() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let mut nonland = ObjectFilter::default();
        nonland.excluded_card_types.push(CardType::Land);
        let observed_choice = Effect::with_id(
            0,
            Effect::conditional_only(
                Condition::TaggedObjectMatches(tag.clone(), nonland),
                vec![Effect::may_single(Effect::lose_life(2))],
            ),
        );
        let if_paid = Effect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![tagged_move(&tag, Zone::Graveyard)],
        );
        let effects = vec![
            Effect::look_at_top_cards(PlayerFilter::You, 1, tag),
            observed_choice,
            if_paid,
        ];

        assert_eq!(
            describe_effect_list(&effects),
            "Look at the top card of your library. If it's a nonland card, you may pay 2 life. If you do, put it into your graveyard"
        );
    }

    #[test]
    fn private_look_renders_if_you_do_not_move_fallback() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let battlefield_move = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(tag.clone()),
                Zone::Battlefield,
                false,
            )
            .tapped(),
        );
        let effects = vec![
            Effect::look_at_top_cards(PlayerFilter::You, 1, tag.clone()),
            Effect::conditional_only(
                Condition::TaggedObjectMatches(tag.clone(), ObjectFilter::land()),
                vec![Effect::may_single(battlefield_move)],
            ),
            Effect::conditional_only(
                Condition::Not(Box::new(Condition::PlayerTaggedObjectMatches {
                    player: PlayerFilter::You,
                    tag: tag.clone(),
                    filter: ObjectFilter::default().in_zone(Zone::Battlefield),
                })),
                vec![tagged_move(&tag, Zone::Hand)],
            ),
        ];
        let rendered = describe_effect_list(&effects);
        assert!(
            rendered.starts_with("Look at the top card of your library"),
            "{rendered}"
        );
        assert!(
            rendered.contains("If you don't put the card onto the battlefield"),
            "{rendered}"
        );
        assert!(!rendered.contains("Then if not"), "{rendered}");
        assert!(!rendered.contains("was revealed this way"), "{rendered}");
    }

    #[test]
    fn optional_look_and_cast_uses_spell_surface() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let mut instant = ObjectFilter::default();
        instant.card_types.push(CardType::Instant);
        let mut sorcery = ObjectFilter::default();
        sorcery.card_types.push(CardType::Sorcery);
        let effects = vec![
            Effect::may_single(Effect::look_at_top_cards(PlayerFilter::You, 1, tag.clone())),
            Effect::conditional_only(
                Condition::Or(
                    Box::new(Condition::TaggedObjectMatches(tag.clone(), instant)),
                    Box::new(Condition::TaggedObjectMatches(tag.clone(), sorcery)),
                ),
                vec![Effect::may_single(Effect::cast_tagged(
                    tag.clone(),
                    PlayerFilter::You,
                    false,
                    false,
                    true,
                    None,
                ))],
            ),
        ];
        let rendered = describe_effect_list(&effects);
        assert!(
            rendered.starts_with("You may look at the top card of your library"),
            "{rendered}"
        );
        assert!(
            rendered.contains("an instant or sorcery spell"),
            "{rendered}"
        );
        assert!(
            rendered.contains("you may cast it without paying its mana cost"),
            "{rendered}"
        );
    }

    #[test]
    fn may_scope_keeps_multiple_actions_conjoined() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let mut nonland = ObjectFilter::default();
        nonland.excluded_card_types.push(CardType::Land);
        let effects = vec![
            Effect::reveal_top(PlayerFilter::You, tag.clone()),
            Effect::conditional_only(
                Condition::TaggedObjectMatches(tag.clone(), nonland),
                vec![Effect::may(vec![
                    tagged_move(&tag, Zone::Battlefield),
                    Effect::draw(1),
                ])],
            ),
        ];
        let rendered = describe_effect_list(&effects);
        assert!(
            rendered.contains("you may put it onto the battlefield and draw a card"),
            "{rendered}"
        );
        assert!(!rendered.contains(". Draw a card"), "{rendered}");
    }

    #[test]
    fn declined_matching_move_shares_otherwise_fallback() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let primary = Effect::with_id(7, Effect::may_single(tagged_move(&tag, Zone::Battlefield)));
        let fallback = Effect::may_single(tagged_move(&tag, Zone::Library));
        let effects = vec![
            Effect::reveal_top(PlayerFilter::You, tag.clone()),
            Effect::conditional(
                Condition::TaggedObjectMatches(tag, ObjectFilter::land()),
                vec![
                    primary,
                    Effect::if_then(
                        EffectId(7),
                        EffectPredicate::DidNotHappen,
                        vec![fallback.clone()],
                    ),
                ],
                vec![fallback],
            ),
        ];
        let rendered = describe_effect_list(&effects);
        assert!(rendered.contains(". Otherwise, you may"), "{rendered}");
        assert!(!rendered.contains("If effect #7"), "{rendered}");
    }

    #[test]
    fn was_declined_matching_move_preserves_explicit_destination_fallback() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let primary = Effect::with_id(7, Effect::may_single(tagged_move(&tag, Zone::Battlefield)));
        let fallback = Effect::may_single(tagged_move(&tag, Zone::Library));
        let effects = vec![
            Effect::reveal_top(PlayerFilter::You, tag.clone()),
            Effect::conditional(
                Condition::TaggedObjectMatches(tag, ObjectFilter::land()),
                vec![
                    primary,
                    Effect::if_then(
                        EffectId(7),
                        EffectPredicate::WasDeclined,
                        vec![fallback.clone()],
                    ),
                ],
                vec![fallback],
            ),
        ];
        let rendered = describe_effect_list(&effects);
        assert!(
            rendered.contains("you may put it onto the battlefield. If you don't put the card onto the battlefield, you may put it on the bottom of your library"),
            "{rendered}"
        );
        assert!(!rendered.contains("If you do"), "{rendered}");
        assert!(!rendered.contains("Otherwise"), "{rendered}");
        assert!(!rendered.contains("you may You"), "{rendered}");
    }

    #[test]
    fn generic_was_declined_battlefield_move_uses_negative_wording() {
        let tag = TagKey::from("targeted_test");
        let primary = Effect::with_id(9, Effect::may_single(tagged_move(&tag, Zone::Battlefield)));
        let fallback = tagged_move(&tag, Zone::Hand);
        let mut filter = ObjectFilter::default();
        filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(3));
        let effects = vec![Effect::conditional(
            Condition::TaggedObjectMatches(tag, filter),
            vec![
                primary,
                Effect::if_then(
                    EffectId(9),
                    EffectPredicate::WasDeclined,
                    vec![fallback.clone()],
                ),
            ],
            vec![fallback],
        )];
        let rendered = describe_effect_list(&effects);
        assert!(
            rendered
                .contains("If it has mana value 3 or less, you may put it onto the battlefield"),
            "{rendered}"
        );
        assert!(
            rendered.contains("If you don't put it onto the battlefield, put it into your hand"),
            "{rendered}"
        );
        assert!(!rendered.contains("If you do"), "{rendered}");
        assert!(!rendered.contains("Otherwise"), "{rendered}");
    }

    #[test]
    fn reveal_conditional_consumes_trailing_move_with_revealed_card_antecedent() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let effects = vec![
            Effect::reveal_top(PlayerFilter::You, tag.clone()),
            Effect::conditional_only(
                Condition::TaggedObjectMatches(tag.clone(), ObjectFilter::land()),
                vec![Effect::gain_life(1)],
            ),
            tagged_move(&tag, Zone::Library),
        ];

        let rendered = describe_effect_list(&effects);
        assert!(
            rendered.contains("Then put the revealed card on the bottom"),
            "{rendered}"
        );
        assert!(!rendered.contains(". Put it on the bottom"), "{rendered}");
    }

    #[test]
    fn target_player_reveal_conditional_consumes_same_player_shuffle() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let player = PlayerFilter::target_opponent();
        let effects = vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_opponent(),
            )),
            Effect::reveal_top(player.clone(), tag.clone()),
            Effect::conditional_only(
                Condition::TaggedObjectMatches(tag, ObjectFilter::land()),
                vec![Effect::gain_life(1)],
            ),
            Effect::shuffle_library_player(player),
        ];

        let rendered = describe_effect_clause_list(&effects)
            .expect("target-player observation should render as one clause list");
        assert!(
            rendered.starts_with("reveal the top card of target opponent's library"),
            "{rendered}"
        );
        assert!(rendered.contains("If it's a land card"), "{rendered}");
        assert!(rendered.contains("Then that player shuffles"), "{rendered}");
        assert!(!rendered.contains("Shuffle their library"), "{rendered}");
    }

    #[test]
    fn observed_life_stat_chain_keeps_one_card_antecedent() {
        let tag = TagKey::from("__sentence_helper_revealed_test");
        let tagged = ChooseSpec::Tagged(tag.clone());
        let effects = vec![
            Effect::gain_life(Value::ToughnessOf(Box::new(tagged.clone()))),
            Effect::lose_life(Value::PowerOf(Box::new(tagged))),
            tagged_move(&tag, Zone::Hand),
        ];

        assert_eq!(
            describe_observed_branch(&effects, &tag, &PlayerFilter::You).as_deref(),
            Some(
                "you gain life equal to that card's toughness, lose life equal to its power, then put it into your hand"
            )
        );
    }

    #[test]
    fn shared_conditional_object_uses_it_in_otherwise_branch() {
        let tag = TagKey::from("enchanted");
        let that_creature = ChooseSpec::Tagged(tag.clone()).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "that creature".to_string(),
                ),
            ),
        );
        let destroy = Effect::destroy(that_creature);
        let pump = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
            ChooseSpec::Tagged(tag),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: Value::Fixed(3),
                toughness: Value::Fixed(3),
            },
            Until::EndOfTurn,
        ));

        assert_eq!(
            replace_shared_else_subject_with_it(
                &[destroy],
                &[pump],
                "enchanted creature gets +3/+3 until end of turn".to_string(),
            ),
            "it gets +3/+3 until end of turn"
        );
    }

    #[test]
    fn other_player_move_gets_actor_and_copy_keeps_that_card_surface() {
        let tag = TagKey::from("revealed");
        let card = ChooseSpec::Tagged(tag.clone()).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType("the card".to_string()),
            ),
        );
        let move_effect = Effect::new(
            crate::effects::MoveToZoneEffect::new(card, Zone::Graveyard, false)
                .with_destination_player_surface(PlayerFilter::IteratedPlayer),
        );
        assert_eq!(
            describe_observed_move_with_actor(&move_effect, &tag, &PlayerFilter::IteratedPlayer,)
                .as_deref(),
            Some("the player puts the card into their graveyard")
        );

        let that_card = ChooseSpec::Tagged(tag).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType("that card".to_string()),
            ),
        );
        let copy = Effect::new(crate::effects::ApplyContinuousEffect::new_runtime(
            crate::continuous::EffectTarget::Source,
            crate::effects::continuous::RuntimeModification::CopyOf {
                source: that_card,
                preserve_source_abilities: false,
                name_override: None,
                name_override_surface: None,
                add_supertypes: Vec::new(),
                copy_exception_surface: None,
            },
            Until::Forever,
        ));
        assert_eq!(
            normalize_observed_action(&copy, "this enchantment becomes a copy of that card",)
                .as_deref(),
            Some("this enchantment becomes a copy of that card")
        );
    }
}
