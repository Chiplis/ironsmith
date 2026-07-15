use super::*;

/// Render the common ordered library-selection prefix as its own Oracle
/// sentence: "Scry/Surveil N, then draw ...".  The effect tree intentionally
/// stores only execution order, so without this structural boundary a later
/// action (life loss, energy, damage, and so on) can absorb the draw into the
/// wrong conjunction.
pub(super) fn describe_leading_selection_then_draw_sequence(effects: &[Effect]) -> Option<String> {
    let [selection_effect, draw_effect, rest @ ..] = effects else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;

    let selection_then_draw = if let Some(scry) =
        selection_effect.downcast_ref::<crate::effects::ScryEffect>()
    {
        describe_scry_then_draw(scry, draw)?
    } else if let Some(surveil) = selection_effect.downcast_ref::<crate::effects::SurveilEffect>() {
        if surveil.player != draw.player {
            return None;
        }
        let selection = describe_effect(selection_effect);
        let draw = describe_effect(draw_effect);
        format!(
            "{}, then {}",
            selection.trim_end_matches('.'),
            lowercase_first(draw.trim_end_matches('.'))
        )
    } else {
        return None;
    };

    let mut rendered = lowercase_first(selection_then_draw.trim_end_matches('.'));
    if !rest.is_empty() {
        let rest_text =
            describe_effect_clause_list(rest).unwrap_or_else(|| describe_effect_list(rest));
        if !rest_text.trim().is_empty() {
            rendered.push_str(". ");
            rendered.push_str(&capitalize_first(rest_text.trim().trim_end_matches('.')));
        }
    }
    Some(rendered)
}

pub(super) fn normalize_imperative_you_clause(text: &str) -> String {
    for prefix in ["You ", "you "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            const IMPERATIVE_VERBS: &[&str] = &[
                "pay ",
                "lose ",
                "gain ",
                "draw ",
                "put ",
                "discard ",
                "sacrifice ",
                "choose ",
                "mill ",
                "reveal ",
                "scry ",
                "search ",
                "shuffle ",
                "surveil ",
            ];
            if IMPERATIVE_VERBS.iter().any(|verb| rest.starts_with(verb)) {
                return rest.to_string();
            }
            let normalized = normalize_you_verb_phrase(rest);
            if normalized != rest {
                return normalized;
            }
        }
    }
    text.to_string()
}

/// Compact two ordinary resolution actions when the trailing action is the
/// common counter-or-card-draw follow-up. These are simultaneous sequential
/// instructions in Oracle text ("exile ... and put a counter", "put a
/// counter ... and draw"), so rendering them as separate sentences or with
/// an invented "then" loses the source clause's composition.
pub(super) fn describe_conjoined_counter_or_draw_sequence(effects: &[&Effect]) -> Option<String> {
    if !(2..=4).contains(&effects.len()) {
        return None;
    }
    let second_action = unwrap_basic_tag_wrappers(effects.last()?);
    let is_counter_followup = second_action
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .is_some();
    let is_your_draw_followup = second_action
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some_and(|draw| draw.player == PlayerFilter::You);
    let is_counter_then_your_life_gain = second_action
        .downcast_ref::<crate::effects::GainLifeEffect>()
        .is_some_and(|gain| gain.player == ChooseSpec::Player(PlayerFilter::You))
        && effects[..effects.len() - 1].iter().any(|effect| {
            unwrap_basic_tag_wrappers(effect)
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .is_some()
        });
    if !is_counter_followup && !is_your_draw_followup && !is_counter_then_your_life_gain {
        return None;
    }

    fn simple_imperative_clause(effect: &Effect) -> Option<String> {
        let rendered = describe_effect(effect);
        let trimmed = rendered.trim().trim_end_matches('.');
        if trimmed.is_empty()
            || trimmed.contains(". ")
            || trimmed.contains(": ")
            || trimmed.starts_with("If ")
            || trimmed.starts_with("When ")
            || trimmed.starts_with("Whenever ")
            || trimmed.starts_with("At ")
        {
            return None;
        }

        let normalized = lowercase_first(&normalize_imperative_you_clause(trimmed));
        const IMPERATIVE_ACTIONS: &[&str] = &[
            "add ",
            "choose ",
            "counter ",
            "create ",
            "destroy ",
            "discard ",
            "draw ",
            "exile ",
            "gain ",
            "lose ",
            "mill ",
            "put ",
            "remove ",
            "return ",
            "sacrifice ",
            "scry ",
            "search ",
            "surveil ",
            "tap ",
            "untap ",
            "reveal ",
        ];
        IMPERATIVE_ACTIONS
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
            .then_some(normalized)
    }

    // Scry followed by a draw is conventionally ordered with "then" and has
    // an existing dedicated renderer. Keep that temporal surface distinct
    // while allowing the common "scry ... and put a counter" sequence.
    if is_your_draw_followup
        && effects[..effects.len() - 1].iter().any(|effect| {
            unwrap_basic_tag_wrappers(effect)
                .downcast_ref::<crate::effects::ScryEffect>()
                .is_some()
        })
    {
        return None;
    }

    let mut clauses = effects
        .iter()
        .map(|effect| simple_imperative_clause(effect))
        .collect::<Option<Vec<_>>>()?;
    let mut last = clauses.pop()?;
    if is_counter_then_your_life_gain {
        last = format!("you {last}");
    }
    let body = if clauses.len() == 1 {
        format!("{} and {last}", clauses[0])
    } else {
        format!("{}, and {last}", clauses.join(", "))
    };
    Some(capitalize_first(&body))
}

pub(super) fn describe_longest_conjoined_counter_or_draw_sequence(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    for sequence_len in (2..=4.min(effects.len())).rev() {
        if let Some(rendered) =
            describe_conjoined_counter_or_draw_sequence(&effects[..sequence_len])
        {
            return Some((rendered, sequence_len));
        }
    }
    None
}

pub(super) fn describe_linked_same_source_damage_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    // A targeted damage clause followed by damage to that same permanent's
    // controller is the lowered form of a single conjoined Oracle sentence
    // (for example, "... to target creature or planeswalker and 1 damage to
    // that permanent's controller"). Preserve the shared source and omit the
    // synthetic temporal "then" between simultaneous damage instructions.
    if let Some(tagged) = first.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(first_damage) = unwrap_basic_tag_wrappers(&tagged.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && let Some(second_damage) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::DealDamageEffect>()
        && first_damage.source_is_combat == second_damage.source_is_combat
        && first_damage.unpreventable == second_damage.unpreventable
        && matches!(
            second_damage.target.base(),
            ChooseSpec::Player(PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag)))
                | ChooseSpec::Player(PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag)))
                if *tag == tagged.tag
        )
    {
        let first_clause = describe_effect(first)
            .trim()
            .trim_end_matches('.')
            .to_string();
        let (amount, where_x) = describe_damage_amount_clause(&second_damage.amount);
        let mut rendered = format!(
            "{first_clause} and {amount} to {}",
            describe_damage_target(&second_damage.target)
        );
        if let Some(where_x) = where_x {
            rendered.push_str(&format!(", where X is {where_x}"));
        }
        return Some(rendered);
    }

    // Adjacent same-source damage effects whose second amount is explicitly
    // derived from the first are the structural form of an elided conjoined
    // clause: "... to target player and that much damage to target creature."
    // Keep the first clause's source surface, but omit the repeated verb and
    // source from the linked follow-up.
    let (first_inner, first_id) = unwrap_with_id(first);
    if let Some(first_id) = first_id
        && let Some(first_exec) = unwrap_basic_tag_wrappers(first_inner)
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        && let Some(first_damage) = unwrap_basic_tag_wrappers(&first_exec.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && let Some(second_exec) = unwrap_basic_tag_wrappers(second)
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        && let Some(second_damage) = unwrap_basic_tag_wrappers(&second_exec.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && first_exec.source.unhinted() == second_exec.source.unhinted()
        && first_damage.source_is_combat == second_damage.source_is_combat
        && first_damage.unpreventable == second_damage.unpreventable
        && matches!(second_damage.amount.unhinted(), Value::EffectValue(id) if *id == first_id)
    {
        let first_clause = describe_effect(first)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return Some(format!(
            "{first_clause} and that much damage to {}",
            describe_choose_spec(&second_damage.target)
        ));
    }

    None
}

/// Render the common enter-trigger shape where the triggering object is the
/// source of two or more simultaneous damage instructions. Lowering keeps an
/// invisible triggering-object tag followed by one `ExecuteWithSourceEffect`
/// per recipient; without this structural view the generic list renderer
/// repeats "that creature" and invents sentence boundaries between the
/// independently targeted damage effects.
pub(super) fn describe_triggering_object_coordinated_damage(effects: &[Effect]) -> Option<String> {
    let [tag_effect, damage_effects @ ..] = effects else {
        return None;
    };
    if damage_effects.len() < 2 {
        return None;
    }

    let triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let mut damages = Vec::with_capacity(damage_effects.len());
    for effect in damage_effects {
        let execute = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
        if !matches!(
            execute.source.unhinted(),
            ChooseSpec::Tagged(tag) if *tag == triggering.tag
        ) {
            return None;
        }
        let damage = structural_unwrap_render_wrappers(&execute.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()?;
        damages.push(damage);
    }

    let first = *damages.first()?;
    if damages.iter().skip(1).any(|damage| {
        damage.source_is_combat != first.source_is_combat
            || damage.unpreventable != first.unpreventable
    }) {
        return None;
    }

    let mut parts = Vec::with_capacity(damages.len());
    for (index, damage) in damages.into_iter().enumerate() {
        let (amount, where_x) = describe_damage_amount_clause(&damage.amount);
        let subject = if index == 0 { "it deals " } else { "" };
        let mut part = format!(
            "{subject}{amount} to {}",
            describe_choose_spec(&damage.target)
        );
        if let Some(where_x) = where_x {
            part.push_str(&format!(", where X is {where_x}"));
        }
        parts.push(part);
    }
    join_coordinated_parts(&parts)
}

#[cfg(test)]
#[test]
fn triggering_object_coordinated_damage_preserves_targets_and_amounts() {
    let triggering = TagKey::from("triggering");
    let source = ChooseSpec::Tagged(triggering.clone());
    let effects = vec![
        Effect::tag_triggering_object(triggering),
        Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            source.clone(),
            Effect::deal_damage(
                Value::Fixed(4),
                ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Opponent),
            ),
        )),
        Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            source,
            Effect::deal_damage(
                Value::Fixed(1),
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
                    .with_count(ChoiceCount::up_to(1)),
            ),
        )),
    ];

    assert_eq!(
        describe_triggering_object_coordinated_damage(&effects).as_deref(),
        Some(
            "it deals 4 damage to target opponent or planeswalker and 1 damage to up to one target creature"
        )
    );
    assert_eq!(
        describe_effect_list(&effects),
        "it deals 4 damage to target opponent or planeswalker and 1 damage to up to one target creature"
    );
}

pub(super) fn join_coordinated_parts(parts: &[String]) -> Option<String> {
    match parts {
        [] => None,
        [only] => Some(only.clone()),
        [first, second] => Some(format!("{first} and {second}")),
        _ => {
            let (last, leading) = parts.split_last()?;
            Some(format!("{}, and {last}", leading.join(", ")))
        }
    }
}

fn coordinated_damage_view(
    effect: &Effect,
) -> Option<(Option<&ChooseSpec>, &crate::effects::DealDamageEffect)> {
    let effect = unwrap_basic_tag_wrappers(effect);
    if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        && let Some(damage) = with_source
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
    {
        return Some((Some(&with_source.source), damage));
    }
    effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .map(|damage| (None, damage))
}

fn describe_coordinated_damage(effects: &[Effect]) -> Option<String> {
    let mut views = effects
        .iter()
        .map(coordinated_damage_view)
        .collect::<Option<Vec<_>>>()?;
    if views.len() < 2 {
        return None;
    }
    let (first_source, first_damage) = views.remove(0);
    if views.iter().any(|(source, damage)| {
        source.map(ChooseSpec::unhinted) != first_source.map(ChooseSpec::unhinted)
            || damage.source_is_combat != first_damage.source_is_combat
            || damage.unpreventable != first_damage.unpreventable
    }) {
        return None;
    }

    let mut parts = vec![
        describe_effect(&effects[0])
            .trim()
            .trim_end_matches('.')
            .to_string(),
    ];
    for (_, damage) in views {
        let (amount, where_x) = describe_damage_amount_clause(&damage.amount);
        let mut part = format!("{amount} to {}", describe_choose_spec(&damage.target));
        if let Some(where_x) = where_x {
            part.push_str(&format!(", where X is {where_x}"));
        }
        parts.push(part);
    }
    join_coordinated_parts(&parts)
}

fn coordinated_target_spec<'a>(effect: &'a Effect, family: &str) -> Option<&'a ChooseSpec> {
    let effect = unwrap_basic_tag_wrappers(effect);
    match family {
        "Destroy" => effect
            .downcast_ref::<crate::effects::DestroyEffect>()
            .map(|destroy| &destroy.spec)
            .or_else(|| {
                effect
                    .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
                    .map(|destroy| &destroy.spec)
            }),
        "Exile" => effect
            .downcast_ref::<crate::effects::ExileEffect>()
            .filter(|exile| !exile.face_down)
            .map(|exile| &exile.spec),
        "ReturnFromGraveyardToHand" => effect
            .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
            .filter(|returned| !returned.random)
            .map(|returned| &returned.target),
        "ReturnFromGraveyardToBattlefield" => effect
            .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
            .filter(|returned| returned.as_aura.is_none())
            .map(|returned| &returned.target),
        "ReturnToHand" => effect
            .downcast_ref::<crate::effects::ReturnToHandEffect>()
            .map(|returned| &returned.spec),
        "Tap" => effect
            .downcast_ref::<crate::effects::TapEffect>()
            .map(|tap| &tap.target),
        "Untap" => effect
            .downcast_ref::<crate::effects::UntapEffect>()
            .map(|untap| &untap.target),
        _ => None,
    }
}

fn split_rendered_coordinated_target(
    effect: &Effect,
    verb: &str,
    spec: &ChooseSpec,
) -> Option<(String, String)> {
    let rendered = describe_effect(effect);
    let body = rendered.trim().trim_end_matches('.').strip_prefix(verb)?;
    let body = body.strip_prefix(' ')?;
    let candidates = [
        describe_choose_spec_without_graveyard_zone(spec),
        describe_choose_spec(spec),
    ];
    candidates.into_iter().find_map(|target| {
        body.strip_prefix(&target)
            .map(|suffix| (target, suffix.to_string()))
    })
}

fn describe_coordinated_same_action(
    effects: &[Effect],
    family: &str,
    verb: &str,
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let mut targets = Vec::with_capacity(effects.len());
    let mut shared_suffix = None;
    for effect in effects {
        let spec = coordinated_target_spec(effect, family)?;
        let (target, suffix) = split_rendered_coordinated_target(effect, verb, spec)?;
        if let Some(expected) = &shared_suffix {
            if expected != &suffix {
                return None;
            }
        } else {
            shared_suffix = Some(suffix);
        }
        targets.push(target);
    }
    let mut suffix = shared_suffix.unwrap_or_default();
    if targets.len() > 1 && suffix.starts_with(". It can't be regenerated") {
        suffix = suffix.replacen(
            ". It can't be regenerated",
            ". They can't be regenerated",
            1,
        );
    }
    Some(format!(
        "{verb} {}{}",
        join_coordinated_parts(&targets)?,
        suffix
    ))
}

fn describe_coordinated_joint_player_sacrifices(effects: &[Effect]) -> Option<String> {
    let [choose_you, sacrifice_you, choose_other, sacrifice_other] = effects else {
        return None;
    };
    let choose_you = choose_you.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let choose_other = choose_other.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let you = describe_choose_then_sacrifice(choose_you, sacrifice_view(sacrifice_you)?)?;
    let other = describe_choose_then_sacrifice(choose_other, sacrifice_view(sacrifice_other)?)?;
    let object = you.strip_prefix("you sacrifice ")?;
    let other_object = other
        .strip_prefix("that player sacrifices ")
        .or_else(|| other.strip_prefix("target opponent sacrifices "))?;
    let other_object = other_object
        .strip_suffix(" of their choice")
        .unwrap_or(other_object);
    if object != other_object {
        return None;
    }
    let other_player = if other.starts_with("target opponent ") {
        "target opponent"
    } else {
        "that player"
    };
    Some(format!("You and {other_player} each sacrifice {object}"))
}

pub(super) fn coordinated_graveyard_to_hand_view(
    effect: &Effect,
) -> Option<(String, String, String)> {
    let returned = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if returned.random || !returned.target.is_target() {
        return None;
    }
    let owner = graveyard_owner_from_spec(&returned.target)?;
    let target = describe_choose_spec_without_graveyard_zone(&returned.target);
    let from = returned
        .graveyard_player_surface
        .as_ref()
        .map(|player| format!("{} graveyard", describe_possessive_player_filter(player)))
        .unwrap_or_else(|| match &owner {
            Some(owner) => format!(
                "{} graveyard",
                describe_possessive_graveyard_owner_filter(owner)
            ),
            None => "a graveyard".to_string(),
        });
    let to = returned
        .destination_player_surface
        .as_ref()
        .map(|player| format!("{} hand", describe_possessive_player_filter(player)))
        .unwrap_or_else(|| match &owner {
            Some(owner) => format!("{} hand", describe_possessive_player_filter(owner)),
            None => owner_hand_phrase_for_spec(&returned.target).to_string(),
        });
    Some((target, from, to))
}

/// Several independently targeted cards returned from the same graveyard to
/// the same hand are one coordinated action, not an ordered sentence list.
fn describe_coordinated_graveyard_to_hand(effects: &[Effect]) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let mut targets = Vec::with_capacity(effects.len());
    let mut shared_route: Option<(String, String)> = None;
    for effect in effects {
        let (target, from, to) = coordinated_graveyard_to_hand_view(effect)?;
        if let Some((expected_from, expected_to)) = &shared_route {
            if expected_from != &from || expected_to != &to {
                return None;
            }
        } else {
            shared_route = Some((from, to));
        }
        targets.push(target);
    }
    let (from, to) = shared_route?;
    Some(format!(
        "Return {} from {from} to {to}",
        join_coordinated_parts(&targets)?
    ))
}

fn coordinated_pt_component(value: &Value) -> (String, Option<String>, bool) {
    match value.unhinted() {
        Value::Fixed(value) => (
            describe_signed_value(&Value::Fixed(*value)),
            None,
            *value < 0,
        ),
        Value::X => ("+X".to_string(), None, false),
        Value::XTimes(factor) if *factor < 0 => ("-X".to_string(), None, true),
        Value::XTimes(_) => ("+X".to_string(), None, false),
        Value::Scaled(inner, factor) if *factor < 0 => (
            "-X".to_string(),
            Some(describe_where_x_basis(inner).unwrap_or_else(|| describe_value(inner))),
            true,
        ),
        dynamic => (
            "+X".to_string(),
            Some(describe_where_x_basis(dynamic).unwrap_or_else(|| describe_value(dynamic))),
            false,
        ),
    }
}

fn describe_coordinated_pt_modifiers(effects: &[Effect], leading_duration: bool) -> Option<String> {
    let modifiers = effects
        .iter()
        .map(|effect| {
            unwrap_basic_tag_wrappers(effect)
                .downcast_ref::<crate::effects::ModifyPowerToughnessEffect>()
        })
        .collect::<Option<Vec<_>>>()?;
    let first = modifiers.first()?;
    if modifiers.len() < 2
        || modifiers
            .iter()
            .any(|modifier| modifier.duration != first.duration)
    {
        return None;
    }
    let duration = describe_until(&first.duration);
    if duration.is_empty() {
        return None;
    }

    let mut where_x = None;
    let mut parts = Vec::with_capacity(modifiers.len());
    for modifier in modifiers {
        let (mut power, power_basis, power_negative) = coordinated_pt_component(&modifier.power);
        let (mut toughness, toughness_basis, toughness_negative) =
            coordinated_pt_component(&modifier.toughness);
        if matches!(modifier.power.unhinted(), Value::Fixed(0)) && toughness_negative {
            power = "-0".to_string();
        }
        if matches!(modifier.toughness.unhinted(), Value::Fixed(0)) && power_negative {
            toughness = "-0".to_string();
        }
        for basis in [power_basis, toughness_basis].into_iter().flatten() {
            if let Some(expected) = &where_x {
                if expected != &basis {
                    return None;
                }
            } else {
                where_x = Some(basis);
            }
        }
        let target = describe_choose_spec(&modifier.target);
        let verb = if choose_spec_is_plural(&modifier.target) {
            "get"
        } else {
            "gets"
        };
        let suffix = if leading_duration {
            String::new()
        } else {
            format!(" {duration}")
        };
        parts.push(format!("{target} {verb} {power}/{toughness}{suffix}"));
    }
    let mut rendered = join_coordinated_parts(&parts)?;
    if leading_duration {
        rendered = format!("{}, {}", capitalize_first(&duration), rendered);
    }
    if let Some(where_x) = where_x {
        rendered.push_str(&format!(", where X is {where_x}"));
    }
    Some(rendered)
}

fn apply_continuous_actions_match(
    first: &crate::effects::ApplyContinuousEffect,
    other: &crate::effects::ApplyContinuousEffect,
) -> bool {
    first.modification == other.modification
        && first.additional_modifications == other.additional_modifications
        && first.runtime_modifications == other.runtime_modifications
        && first.until == other.until
        && first.condition == other.condition
        && first.source_type == other.source_type
        && first.type_retention_surface == other.type_retention_surface
        && first.lock_filter_at_resolution == other.lock_filter_at_resolution
        && first.resolve_set_pt_values_at_resolution == other.resolve_set_pt_values_at_resolution
        && first.require_creature_target == other.require_creature_target
}

/// Render one typed coordinated predicate shared by independently selected
/// subjects. This is intentionally gated on identical continuous actions;
/// serial modifiers such as Blue Dragon retain one predicate per target.
fn describe_coordinated_shared_continuous_action(effects: &[Effect]) -> Option<String> {
    let applies = effects
        .iter()
        .map(coordinated_apply_continuous)
        .collect::<Option<Vec<_>>>()?;
    let first = *applies.first()?;
    if applies.len() < 2
        || applies
            .iter()
            .skip(1)
            .any(|other| !apply_continuous_actions_match(first, other))
    {
        return None;
    }

    let subjects = applies
        .iter()
        .map(|apply| describe_apply_continuous_target(apply).0)
        .collect::<Vec<_>>();
    if subjects.iter().any(String::is_empty) || subjects.windows(2).any(|pair| pair[0] == pair[1]) {
        return None;
    }
    let clauses = describe_apply_continuous_clauses(first, true);
    if clauses.is_empty() {
        return None;
    }
    let marker = if clauses.iter().all(|clause| clause.starts_with("gain ")) {
        "both"
    } else {
        "each"
    };
    let mut action = join_with_and(&clauses);
    if let Some(tail) = describe_apply_continuous_tail(first) {
        action.push(' ');
        action.push_str(&tail);
    }
    Some(format!(
        "{} {marker} {action}",
        capitalize_first(&join_coordinated_parts(&subjects)?)
    ))
}

fn describe_coordinated_shared_cant_be_blocked(effects: &[Effect]) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let mut subjects = Vec::with_capacity(effects.len());
    let mut index = 0;
    while index < effects.len() {
        let effect = &effects[index];
        let (rendered, consumed) = if let Some(cant) =
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::CantEffect>()
        {
            if cant.duration != Until::EndOfTurn
                || !matches!(&cant.restriction, crate::effect::Restriction::BeBlocked(_))
            {
                return None;
            }
            (describe_effect(effect), 1)
        } else {
            let pair = effects.get(index..index + 2)?;
            let cant =
                unwrap_basic_tag_wrappers(&pair[1]).downcast_ref::<crate::effects::CantEffect>()?;
            if cant.duration != Until::EndOfTurn
                || !matches!(&cant.restriction, crate::effect::Restriction::BeBlocked(_))
                || unwrap_basic_tag_wrappers(&pair[0])
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_none()
            {
                return None;
            }
            (describe_effect_list(pair), 2)
        };
        let subject = rendered
            .trim()
            .trim_end_matches('.')
            .strip_suffix(" can't be blocked this turn")?;
        subjects.push(subject.to_string());
        index += consumed;
    }
    Some(format!(
        "{} can't be blocked this turn",
        join_coordinated_parts(&subjects)?
    ))
}

fn coordinated_apply_continuous(effect: &Effect) -> Option<&crate::effects::ApplyContinuousEffect> {
    unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::ApplyContinuousEffect>()
}

fn coordinated_effect_tag(effect: &Effect) -> Option<&TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return coordinated_effect_tag(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tagged.tag);
    }
    None
}

fn coordinated_apply_targets_previous(
    previous_effect: &Effect,
    current_effect: &Effect,
    previous: &crate::effects::ApplyContinuousEffect,
    current: &crate::effects::ApplyContinuousEffect,
) -> bool {
    // A later tagged wrapper can still be a continuation of the previous
    // modification: lowering tags each step so subsequent clauses can refer
    // to its result. Honor that explicit back-reference before distinguishing
    // independently introduced target slots by their wrapper tags.
    if let Some(previous_tag) = coordinated_effect_tag(previous_effect)
        && current
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_exact_tag(spec, previous_tag))
    {
        return true;
    }

    let previous_introduces_target = previous
        .target_spec
        .as_ref()
        .is_some_and(ChooseSpec::is_target);
    let current_introduces_target = current
        .target_spec
        .as_ref()
        .is_some_and(ChooseSpec::is_target);
    if previous_introduces_target && current_introduces_target {
        // Repeating an explicit target phrase introduces another target slot,
        // even when the two filters are textually identical. A shared subject
        // is lowered as Source/Tagged (or as the explicit back-reference
        // handled above), so it must not take this branch. Equal wrapper tags
        // are the one structural proof that both explicit specs name the same
        // already-introduced slot.
        return matches!(
            (
                coordinated_effect_tag(previous_effect),
                coordinated_effect_tag(current_effect),
            ),
            (Some(previous_tag), Some(current_tag)) if previous_tag == current_tag
        );
    }
    if let (Some(previous_spec), Some(current_spec)) =
        (previous.target_spec.as_ref(), current.target_spec.as_ref())
        && previous_spec.unhinted() == current_spec.unhinted()
    {
        return true;
    }
    if let (Some(previous_filter), Some(current_filter)) = (
        apply_continuous_filter(previous),
        apply_continuous_filter(current),
    ) && previous_filter == current_filter
    {
        return true;
    }
    previous.target_spec.is_none()
        && current.target_spec.is_none()
        && previous.target == current.target
}

fn split_coordinated_duration(rendered: &str, duration: &Until) -> Option<(String, String)> {
    let duration = describe_until(duration);
    if duration.is_empty() {
        return None;
    }
    let rendered = rendered.trim().trim_end_matches('.');
    let needle = format!(" {duration}");
    let duration_idx = rendered.rfind(&needle)?;
    let trailing = &rendered[duration_idx + needle.len()..];
    if !trailing.is_empty() && !trailing.starts_with(", where X is ") {
        return None;
    }
    Some((rendered[..duration_idx].to_string(), trailing.to_string()))
}

fn coordinated_apply_prefers_where_x(apply: &crate::effects::ApplyContinuousEffect) -> bool {
    let modification_prefers_where_x =
        |modification: &crate::continuous::Modification| match modification {
            crate::continuous::Modification::SetPowerToughness {
                power, toughness, ..
            } => value_prefers_where_x(power) || value_prefers_where_x(toughness),
            crate::continuous::Modification::SetPower { value, .. }
            | crate::continuous::Modification::SetToughness { value, .. } => {
                value_prefers_where_x(value)
            }
            _ => false,
        };
    let runtime_prefers_where_x =
        |modification: &crate::effects::continuous::RuntimeModification| match modification {
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            } => value_prefers_where_x(power) || value_prefers_where_x(toughness),
            crate::effects::continuous::RuntimeModification::ModifyPower { value }
            | crate::effects::continuous::RuntimeModification::ModifyToughness { value } => {
                value_prefers_where_x(value)
            }
            _ => false,
        };

    apply
        .modification
        .as_ref()
        .is_some_and(modification_prefers_where_x)
        || apply
            .additional_modifications
            .iter()
            .any(modification_prefers_where_x)
        || apply
            .runtime_modifications
            .iter()
            .any(runtime_prefers_where_x)
}

fn trailing_where_x_clause(rendered: &str) -> Option<String> {
    let (_, basis) = rendered
        .trim()
        .trim_end_matches('.')
        .rsplit_once(", where X is ")?;
    (!basis.trim().is_empty()).then(|| format!(", where X is {basis}"))
}

/// Preserve independently introduced target slots when a coordinated clause
/// applies a different power/toughness modifier to each one. Each child keeps
/// its own target and count surface; only the common duration is factored out.
fn describe_coordinated_independent_apply_pt_modifiers(
    effects: &[Effect],
    leading_duration: bool,
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let applies = effects
        .iter()
        .map(coordinated_apply_continuous)
        .collect::<Option<Vec<_>>>()?;
    let first = *applies.first()?;
    let duration = &first.until;
    let duration_text = describe_until(duration);
    if duration_text.is_empty()
        || applies.iter().any(|apply| {
            apply.until != *duration
                || apply.condition.is_some()
                || apply.modification.is_some()
                || !apply.additional_modifications.is_empty()
                || !matches!(
                    apply.runtime_modifications.as_slice(),
                    [crate::effects::continuous::RuntimeModification::ModifyPowerToughness { .. }]
                )
                || !apply.target_spec.as_ref().is_some_and(|spec| {
                    spec.is_target() && matches!(spec.base(), ChooseSpec::Object(_))
                })
                || describe_apply_continuous_tail(apply).as_deref() != Some(duration_text.as_str())
        })
    {
        return None;
    }

    // Distinct wrapper tags are lowering's durable identity for the separate
    // target groups. Reject a continuation that refers back to an earlier
    // group, since that belongs in the same-object compactor below.
    let tags = effects
        .iter()
        .map(coordinated_effect_tag)
        .collect::<Option<Vec<_>>>()?;
    for (index, tag) in tags.iter().enumerate() {
        if tags[..index].iter().any(|previous| previous == tag) {
            return None;
        }
    }
    if effects
        .windows(2)
        .zip(applies.windows(2))
        .any(|(effect_pair, apply_pair)| {
            coordinated_apply_targets_previous(
                &effect_pair[0],
                &effect_pair[1],
                apply_pair[0],
                apply_pair[1],
            )
        })
    {
        return None;
    }

    let mut parts = Vec::with_capacity(effects.len());
    let mut shared_trailing: Option<String> = None;
    for effect in effects {
        let rendered = describe_effect(effect);
        let (head, trailing) = split_coordinated_duration(&rendered, duration)?;
        if head.contains(". ") {
            return None;
        }
        if shared_trailing
            .as_ref()
            .is_some_and(|expected| expected != &trailing)
        {
            return None;
        }
        shared_trailing.get_or_insert(trailing);
        parts.push(head);
    }
    let body = join_coordinated_parts(&parts)?;
    let trailing = shared_trailing.unwrap_or_default();
    let rendered = if leading_duration {
        format!(
            "{}, {}{trailing}",
            capitalize_first(&duration_text),
            lowercase_first(&body)
        )
    } else {
        format!("{body} {duration_text}{trailing}")
    };
    Some(capitalize_first(&rendered))
}

fn describe_coordinated_same_object_modifiers(
    effects: &[Effect],
    leading_duration: bool,
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let applies = effects
        .iter()
        .map(coordinated_apply_continuous)
        .collect::<Option<Vec<_>>>()?;
    let first = *applies.first()?;
    let duration = &first.until;
    let duration_text = describe_until(duration);
    if duration_text.is_empty()
        || applies.iter().any(|apply| {
            apply.until != *duration
                || apply.condition.is_some()
                || describe_apply_continuous_tail(apply).as_deref() != Some(duration_text.as_str())
        })
    {
        return None;
    }
    if effects
        .windows(2)
        .zip(applies.windows(2))
        .any(|(effect_pair, apply_pair)| {
            !coordinated_apply_targets_previous(
                &effect_pair[0],
                &effect_pair[1],
                apply_pair[0],
                apply_pair[1],
            )
        })
    {
        return None;
    }

    // Only the first rendered child supplies the common subject/action head.
    // Later children are rebuilt from their typed continuous clauses below.
    // Requiring their fully rendered text to place the shared duration at the
    // end breaks quoted abilities whose text contains the same duration.
    let first_rendered = describe_effect(&effects[0]);
    let (head, first_where_clause) = split_coordinated_duration(&first_rendered, duration)?;
    let mut where_clause: Option<String> = None;
    for (index, (effect, apply)) in effects.iter().zip(applies.iter()).enumerate() {
        if !coordinated_apply_prefers_where_x(apply) {
            continue;
        }
        let candidate = if index == 0 {
            (!first_where_clause.is_empty()).then(|| first_where_clause.clone())
        } else {
            trailing_where_x_clause(&describe_effect(effect))
        }?;
        if where_clause
            .as_ref()
            .is_some_and(|expected| expected != &candidate)
        {
            return None;
        }
        where_clause = Some(candidate);
    }
    let where_clause = where_clause.unwrap_or_default();
    if head.contains(". ") {
        return None;
    }
    let (_, plural_subject) = describe_apply_continuous_target(first);
    let mut parts = vec![head];
    for apply in applies.iter().skip(1) {
        let clauses = describe_apply_continuous_clauses(apply, plural_subject);
        if clauses.is_empty() {
            return None;
        }
        parts.extend(clauses);
    }
    let body = join_coordinated_parts(&parts)?;
    let rendered = if leading_duration {
        format!(
            "{}, {}{where_clause}",
            capitalize_first(&duration_text),
            lowercase_first(&body)
        )
    } else {
        format!("{body} {duration_text}{where_clause}")
    };
    Some(capitalize_first(&rendered))
}

fn describe_coordinated_named_possessive_base_pt_and_grant(
    effects: &[Effect],
    leading_duration: bool,
) -> Option<String> {
    if !leading_duration {
        return None;
    }
    let [first_effect, second_effect] = effects else {
        return None;
    };
    let first = coordinated_apply_continuous(first_effect)?;
    let second = coordinated_apply_continuous(second_effect)?;
    if first.until != second.until
        || first.condition.is_some()
        || second.condition.is_some()
        || !first.additional_modifications.is_empty()
        || !second.additional_modifications.is_empty()
        || !first.runtime_modifications.is_empty()
        || !second.runtime_modifications.is_empty()
        || !coordinated_apply_targets_previous(first_effect, second_effect, first, second)
    {
        return None;
    }
    let crate::target::SourceReferenceSurface::FullName(name) =
        first.source_reference_surface.as_ref()?
    else {
        return None;
    };
    // Compound named sources use a possessive characteristic sentence and a
    // plural pronoun. Keep this deliberately narrower than subtype/filter
    // subjects, whose `and` is a set connective rather than part of a name.
    if !name.to_ascii_lowercase().contains(" and ") {
        return None;
    }
    let Some(crate::continuous::Modification::SetPowerToughness {
        power,
        toughness,
        sublayer: crate::continuous::PtSublayer::Setting,
    }) = first.modification.as_ref()
    else {
        return None;
    };
    let Some(crate::continuous::Modification::AddAbility(ability)) = second.modification.as_ref()
    else {
        return None;
    };
    let duration = describe_until(&first.until);
    if duration.is_empty() {
        return None;
    }
    let possessive = if name.ends_with('s') {
        format!("{name}'")
    } else {
        format!("{name}'s")
    };
    Some(format!(
        "{}, {possessive} base power and toughness become {}/{} and they gain {}",
        capitalize_first(&duration),
        describe_value(power),
        describe_value(toughness),
        lowercase_first(ability.display().trim_end_matches('.')),
    ))
}

pub(super) fn describe_retained_land_noncreature_condition(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<&'static str> {
    if !conditional.if_false.is_empty() {
        return None;
    }
    let crate::effect::Condition::Not(inner) = &conditional.condition else {
        return None;
    };
    if !matches!(inner.as_ref(), crate::effect::Condition::SourceMatches(filter) if *filter == ObjectFilter::creature())
    {
        return None;
    }
    let [effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let apply = unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    matches!(
        apply.type_retention_surface,
        Some(ironsmith_core::TypeRetentionSurface::StillALand)
    )
    .then_some("this land isn't a creature")
}

fn describe_leading_then_coordinated_same_object_modifiers(effects: &[Effect]) -> Option<String> {
    let [leading, _, _] = effects else {
        return None;
    };
    if coordinated_apply_continuous(leading).is_some() {
        return None;
    }
    let suffix = describe_coordinated_same_object_modifiers(&effects[1..], false)?;
    if !suffix.contains(", where X is ") {
        return None;
    }
    let leading = capitalize_first(describe_effect(leading).trim().trim_end_matches('.'));
    Some(format!("{leading}, then {}", lowercase_first(&suffix)))
}

fn describe_coordinated_source_damage_then_grant(effects: &[Effect]) -> Option<String> {
    let [damage_effect, grant_effect] = effects else {
        return None;
    };
    let (source, _) = coordinated_damage_view(damage_effect)?;
    let source = source?;
    let grant = coordinated_apply_continuous(grant_effect)?;
    if grant.condition.is_some()
        || grant.until == Until::Forever
        || !grant.runtime_modifications.is_empty()
    {
        return None;
    }
    let grant_target = grant.target_spec.as_ref()?;
    if grant_target.unhinted() != source.unhinted() {
        return None;
    }
    let duration = describe_until(&grant.until);
    if duration.is_empty()
        || describe_apply_continuous_tail(grant).as_deref() != Some(duration.as_str())
    {
        return None;
    }
    let clauses = describe_apply_continuous_clauses(grant, false);
    if clauses.is_empty() || clauses.iter().any(|clause| !clause.starts_with("gains ")) {
        return None;
    }
    let damage = describe_effect(damage_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let parts = std::iter::once(damage).chain(clauses).collect::<Vec<_>>();
    Some(format!("{} {duration}", join_coordinated_parts(&parts)?))
}

fn describe_coordinated_tap_then_next_untap(effects: &[Effect]) -> Option<String> {
    let [tap_effect, cant_effect] = effects else {
        return None;
    };
    let tap = unwrap_basic_tag_wrappers(tap_effect).downcast_ref::<crate::effects::TapEffect>()?;
    let cant =
        unwrap_basic_tag_wrappers(cant_effect).downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::ControllersNextUntapStep {
        return None;
    }

    let same_target = match tap.target.base() {
        ChooseSpec::Object(tap_filter) => tap_filter == filter,
        ChooseSpec::Tagged(tag) => object_filter_has_tag(filter, tag),
        ChooseSpec::Target(inner) => {
            matches!(inner.base(), ChooseSpec::Object(tap_filter) if tap_filter == filter)
        }
        _ => false,
    } || wrapped_effect_tag(tap_effect)
        .is_some_and(|tag| object_filter_has_tag(filter, tag));
    let singular_target = !tap.target.is_all()
        && !tap.target.count().is_dynamic_x()
        && tap.target.count().max.is_some_and(|max| max <= 1);
    if !same_target || !singular_target {
        return None;
    }

    let target = match tap.target.base() {
        ChooseSpec::Tagged(tag) if tag.as_str() == "damaged" => "that creature".to_string(),
        _ => describe_choose_spec(&tap.target),
    };
    Some(format!(
        "Tap {target} and it doesn't untap during its controller's next untap step"
    ))
}

fn describe_coordinated_continuous_then_must_be_blocked(effects: &[Effect]) -> Option<String> {
    let [continuous_effect, restriction_effect] = effects else {
        return None;
    };
    let continuous_view = unwrap_basic_tag_wrappers(continuous_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let restriction = unwrap_basic_tag_wrappers(restriction_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::MustBeBlocked(restriction_filter) = &restriction.restriction
    else {
        return None;
    };
    if continuous_view.until != Until::EndOfTurn || restriction.duration != Until::EndOfTurn {
        return None;
    }
    let same_subject = wrapped_effect_tag(continuous_effect)
        .is_some_and(|tag| filter_is_exactly_tagged(restriction_filter, tag))
        || apply_continuous_filter(continuous_view)
            .is_some_and(|filter| filter == restriction_filter);
    if !same_subject {
        return None;
    }

    let continuous_rendered = describe_effect(continuous_effect);
    let continuous = continuous_rendered.trim().trim_end_matches('.');
    let restriction_rendered = describe_effect(restriction_effect);
    let restriction = restriction_rendered.trim().trim_end_matches('.');
    let action = if let Some(action) = restriction.strip_prefix("It ") {
        action
    } else {
        let (subject, action) = restriction.split_once(" must be blocked")?;
        if !continuous.starts_with(subject) {
            return None;
        }
        return Some(format!("{continuous} and must be blocked{action}"));
    };
    Some(format!("{continuous} and {action}"))
}

fn describe_coordinated_copy_all_stack_sets(effects: &[Effect]) -> Option<String> {
    let [first_effect, second_effect] = effects else {
        return None;
    };
    let first = copy_spell_from_effect(first_effect)?;
    let second = copy_spell_from_effect(second_effect)?;
    if first.count != Value::Fixed(1)
        || second.count != Value::Fixed(1)
        || first.copier != PlayerFilter::You
        || second.copier != PlayerFilter::You
        || !first.removed_supertypes.is_empty()
        || !second.removed_supertypes.is_empty()
        || !matches!(first.target, ChooseSpec::All(_))
        || !matches!(second.target, ChooseSpec::All(_))
    {
        return None;
    }
    let first = describe_stack_object_copy_target(&first.target);
    let second = describe_stack_object_copy_target(&second.target);
    Some(format!("Copy {first}, then copy {second}"))
}

/// Preserve the coordinated family whose lowering needs a redundant
/// target-only prelude for target collection. The typed fallback below still
/// starts from the established effect-list renderer, so its structural bundle
/// compactors remain visible before sentence boundaries are rejoined.
fn describe_coordinated_gain_life_and_suspect(effects: &[Effect]) -> Option<String> {
    let [target_effect, gain_effect, suspect_effect] = effects else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    let suspect = structural_unwrap_render_wrappers(suspect_effect)
        .downcast_ref::<crate::effects::SuspectEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You)
        || !target_specs_select_same_objects(&target_only.target, &suspect.target)
    {
        return None;
    }

    let gain = describe_effect(gain_effect);
    let suspect = describe_effect(suspect_effect);
    Some(format!(
        "{} and {}",
        gain.trim().trim_end_matches('.'),
        lowercase_first(suspect.trim().trim_end_matches('.'))
    ))
}

/// Restore an explicit source conjunction after the ordinary effect-list
/// renderer has preserved the child clauses' established surfaces.
fn describe_typed_coordinated_clause_fallback(effects: &[Effect]) -> Option<String> {
    let rendered = describe_effect_list(effects);
    let mut parts = rendered
        .split(". ")
        .map(|part| part.trim().trim_end_matches('.').to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        // Some established structural compactors infer a top-level `then`
        // from runtime dependency (notably producer/attachment pairs) and
        // therefore collapse two authored clauses into one rendered string.
        // When the typed sequence still has exactly two runtime children,
        // render those clauses independently so the explicit source `and`
        // remains authoritative. Reject multi-sentence children: any genuine
        // nested sequencing they contain must stay within its own surface.
        let [first, second] = effects else {
            return None;
        };
        parts = [first, second]
            .into_iter()
            .map(describe_effect)
            .map(|part| part.trim().trim_end_matches('.').to_string())
            .collect();
        if parts
            .iter()
            .any(|part| part.is_empty() || part.contains(". "))
        {
            return None;
        }
    }
    for part in parts.iter_mut().skip(1) {
        *part = lowercase_first(part);
    }
    join_coordinated_parts(&parts)
}

/// Render only the typed coordination introduced inside a result branch.
///
/// Existing top-level `SequenceEffect::Coordinated` values cover several
/// older lowering shapes whose established sentence surfaces are more exact
/// than the generic clause joiner. The parser's result-prefix preservation,
/// by contrast, produces one direct coordinated sequence as the complete
/// `IfEffect`/`ReflexiveTriggerEffect` body. Keeping the fallback behind this
/// exact structural boundary prevents unrelated coordinated sequences from
/// changing surface text.
pub(super) fn describe_typed_coordinated_result_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if !matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::ResultConjunction { .. }
    ) {
        return None;
    }
    describe_typed_coordinated_clause_fallback(&sequence.effects)
}

pub(super) fn describe_result_branch_effect_list(effects: &[Effect]) -> String {
    describe_typed_coordinated_result_branch(effects)
        .unwrap_or_else(|| describe_effect_list(effects))
}

pub(super) fn describe_coordinated_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    let leading_duration = matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::CoordinatedLeadingDuration
            | ironsmith_core::SequenceSurface::ResultConjunction {
                leading_duration: true
            }
    );
    if let Some((compact, consumed)) =
        describe_target_same_name_action_fanout_prefix(&sequence.effects)
    {
        if consumed == sequence.effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_list(&sequence.effects[consumed..]);
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Sequential
    ) {
        return None;
    }
    if let Some(compact) =
        describe_coordinated_put_counters_then_grant_same_filter(&sequence.effects)
    {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_joint_subject_pair(first, second)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_coordinated_copy_all_stack_sets(&sequence.effects) {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_target_continuous_fanout_pair(first, second)
            .or_else(|| describe_target_prevention_fanout_pair(first, second))
    {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_target_creature_damage_fanout_pair(first, second)
    {
        return Some(compact);
    }
    describe_leading_then_coordinated_same_object_modifiers(&sequence.effects)
        .or_else(|| describe_coordinated_source_damage_then_grant(&sequence.effects))
        .or_else(|| describe_coordinated_tap_then_next_untap(&sequence.effects))
        .or_else(|| describe_coordinated_shared_continuous_action(&sequence.effects))
        .or_else(|| describe_coordinated_shared_cant_be_blocked(&sequence.effects))
        .or_else(|| describe_coordinated_continuous_then_must_be_blocked(&sequence.effects))
        .or_else(|| {
            describe_coordinated_named_possessive_base_pt_and_grant(
                &sequence.effects,
                leading_duration,
            )
        })
        .or_else(|| {
            describe_coordinated_independent_apply_pt_modifiers(&sequence.effects, leading_duration)
        })
        .or_else(|| describe_coordinated_same_object_modifiers(&sequence.effects, leading_duration))
        .or_else(|| describe_coordinated_damage(&sequence.effects))
        .or_else(|| describe_coordinated_graveyard_to_hand(&sequence.effects))
        .or_else(|| describe_coordinated_same_action(&sequence.effects, "Destroy", "Destroy"))
        .or_else(|| describe_coordinated_same_action(&sequence.effects, "Exile", "Exile"))
        .or_else(|| {
            describe_coordinated_same_action(
                &sequence.effects,
                "ReturnFromGraveyardToHand",
                "Return",
            )
        })
        .or_else(|| {
            describe_coordinated_same_action(
                &sequence.effects,
                "ReturnFromGraveyardToBattlefield",
                "Return",
            )
        })
        .or_else(|| describe_coordinated_same_action(&sequence.effects, "ReturnToHand", "Return"))
        .or_else(|| describe_coordinated_joint_player_sacrifices(&sequence.effects))
        .or_else(|| describe_coordinated_same_action(&sequence.effects, "Tap", "Tap"))
        .or_else(|| describe_coordinated_same_action(&sequence.effects, "Untap", "Untap"))
        .or_else(|| describe_coordinated_pt_modifiers(&sequence.effects, leading_duration))
        .or_else(|| {
            matches!(
                sequence.surface,
                ironsmith_core::SequenceSurface::Coordinated
            )
            .then(|| describe_coordinated_gain_life_and_suspect(&sequence.effects))
            .flatten()
        })
        .or_else(|| describe_typed_coordinated_clause_fallback(&sequence.effects))
}

#[cfg(test)]
mod coordinated_sequence_tests {
    use super::*;

    #[test]
    fn coordinated_damage_elides_only_the_typed_shared_action() {
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::deal_damage(Value::Fixed(2), ChooseSpec::target_creature()),
            Effect::deal_damage(
                Value::Fixed(2),
                ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any),
            ),
        ]));
        assert_eq!(
            describe_effect(&sequence),
            "Deal 2 damage to target creature and 2 damage to player or planeswalker"
        );
    }

    #[test]
    fn coordinated_copy_all_stack_sets_preserves_both_fanouts() {
        let spells = ObjectFilter::spell().controlled_by(PlayerFilter::You);
        let mut triggered = ObjectFilter::ability();
        triggered.stack_kind = Some(StackObjectKind::TriggeredAbility);
        let mut abilities = ObjectFilter::default().in_zone(Zone::Stack);
        abilities.controller = Some(PlayerFilter::You);
        abilities.other = true;
        abilities.any_of = vec![ObjectFilter::activated_ability(), triggered];
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::CopySpellEffect::single(ChooseSpec::All(
                spells,
            ))),
            Effect::new(crate::effects::CopySpellEffect::single(ChooseSpec::All(
                abilities,
            ))),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Copy all spells you control, then copy all other activated and triggered abilities you control"
        );
        assert_eq!(
            describe_result_branch_effect_list(std::slice::from_ref(&sequence)),
            "Copy all spells you control, then copy all other activated and triggered abilities you control",
            "an ordinary coordinated specialist nested in a result branch must not use the generic result joiner"
        );
    }

    #[test]
    fn coordinated_gain_life_and_suspect_ignore_redundant_target_declaration() {
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        ))
        .with_count(ChoiceCount::up_to(1));
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())),
            Effect::gain_life(Value::Fixed(3)),
            Effect::suspect(target).tag("suspected_0"),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "You gain 3 life and suspect up to one target creature an opponent controls"
        );
    }

    #[test]
    fn typed_coordinated_fallback_preserves_ordinary_source_conjunctions() {
        let effects = vec![
            Effect::draw(Value::Fixed(1)),
            Effect::gain_life(Value::Fixed(2)),
        ];
        let established_surface = describe_effect_list(&effects);
        let ordinary = Effect::new(crate::effects::SequenceEffect::coordinated(effects.clone()));
        let result = Effect::new(crate::effects::SequenceEffect::result_conjunction(
            effects, false,
        ));

        assert_ne!(describe_effect(&ordinary), established_surface);
        assert_eq!(
            describe_effect(&ordinary),
            "Draw a card and you gain 2 life"
        );
        assert_eq!(
            describe_typed_coordinated_result_branch(std::slice::from_ref(&ordinary)),
            None
        );
        assert_eq!(
            describe_typed_coordinated_result_branch(std::slice::from_ref(&result)).as_deref(),
            Some("Draw a card and you gain 2 life")
        );
    }

    #[test]
    fn unrelated_coordinated_bundle_keeps_effect_list_fallback() {
        let battlefield = Effect::new(crate::effects::ReturnToHandEffect::all(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
        ));
        let graveyards = Effect::new(crate::effects::ReturnToHandEffect::all(
            ObjectFilter::creature().in_zone(Zone::Graveyard),
        ));
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            battlefield,
            graveyards,
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Return all creatures on the battlefield and all creature cards in graveyards to their owners' hands"
        );
    }

    #[test]
    fn sequential_damage_is_not_conjoined_by_adjacency() {
        let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
            Effect::deal_damage(Value::Fixed(2), ChooseSpec::target_creature()),
            Effect::deal_damage(
                Value::Fixed(2),
                ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any),
            ),
        ]));
        assert_ne!(
            describe_effect(&sequence),
            "Deal 2 damage to target creature and 2 damage to player or planeswalker"
        );
    }

    #[test]
    fn coordinated_tap_then_next_untap_keeps_and_surface() {
        let damaged = TagKey::from("damaged");
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::tap(ChooseSpec::Tagged(damaged.clone())),
            Effect::cant_until(
                crate::effect::Restriction::Untap(ObjectFilter::tagged(damaged)),
                Until::ControllersNextUntapStep,
            ),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Tap that creature and it doesn't untap during its controller's next untap step"
        );
    }

    #[test]
    fn coordinated_target_pump_and_keyword_share_target_and_duration() {
        let pumped = TagKey::from("pumped_0");
        let pump = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
            ChooseSpec::target_creature(),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: Value::Fixed(2),
                toughness: Value::Fixed(2),
            },
            Until::EndOfTurn,
        ))
        .tag(pumped.clone());
        let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(pumped),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::trample(),
            ),
            Until::EndOfTurn,
        ));
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            pump, grant,
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Target creature gets +2/+2 and gains trample until end of turn"
        );
    }

    #[test]
    fn coordinated_named_possessive_base_pt_and_grant_stays_one_source_clause() {
        let source_surface = crate::target::SourceReferenceSurface::FullName(
            "Moon Girl and Devil Dinosaur".to_string(),
        );
        let base_pt = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::Source,
                crate::continuous::Modification::SetPowerToughness {
                    power: Value::Fixed(6),
                    toughness: Value::Fixed(6),
                    sublayer: crate::continuous::PtSublayer::Setting,
                },
                Until::EndOfTurn,
            )
            .with_source_reference_surface(source_surface.clone()),
        );
        let trample = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::Source,
                crate::continuous::Modification::AddAbility(
                    crate::static_abilities::StaticAbility::trample(),
                ),
                Until::EndOfTurn,
            )
            .with_source_reference_surface(source_surface),
        );
        let sequence = Effect::new(
            crate::effects::SequenceEffect::coordinated_with_leading_duration(vec![
                base_pt, trample,
            ]),
        );

        assert_eq!(
            describe_effect(&sequence),
            "Until end of turn, Moon Girl and Devil Dinosaur's base power and toughness become 6/6 and they gain trample"
        );
    }

    #[test]
    fn coordinated_target_identity_distinguishes_introductions_from_shared_chains() {
        let apply = |target| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                target,
                crate::continuous::Modification::AddAbility(
                    crate::static_abilities::StaticAbility::flying(),
                ),
                Until::EndOfTurn,
            ))
        };

        let first_target = apply(ChooseSpec::target_creature()).tag("first_target");
        let second_target = apply(ChooseSpec::target_creature()).tag("second_target");
        assert!(!coordinated_apply_targets_previous(
            &first_target,
            &second_target,
            coordinated_apply_continuous(&first_target).expect("first target apply"),
            coordinated_apply_continuous(&second_target).expect("second target apply"),
        ));

        let first_source = Effect::with_id(1, apply(ChooseSpec::Source).tag("first_source_result"));
        let second_source =
            Effect::with_id(2, apply(ChooseSpec::Source).tag("second_source_result"));
        assert!(coordinated_apply_targets_previous(
            &first_source,
            &second_source,
            coordinated_apply_continuous(&first_source).expect("first source apply"),
            coordinated_apply_continuous(&second_source).expect("second source apply"),
        ));

        let shared = TagKey::from("shared_target");
        let first_shared = apply(ChooseSpec::Tagged(shared.clone())).tag("first_shared_result");
        let second_shared = apply(ChooseSpec::Tagged(shared)).tag("second_shared_result");
        assert!(coordinated_apply_targets_previous(
            &first_shared,
            &second_shared,
            coordinated_apply_continuous(&first_shared).expect("first shared apply"),
            coordinated_apply_continuous(&second_shared).expect("second shared apply"),
        ));
    }

    #[test]
    fn coordinated_independent_pt_modifiers_keep_each_target_group() {
        let first_target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        ));
        let later_target = ChooseSpec::target_creature().with_count(ChoiceCount::up_to(1));
        let modifier = |target, amount, tag: &'static str| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                target,
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: Value::Fixed(amount),
                    toughness: Value::Fixed(0),
                },
                Until::YourNextTurn,
            ))
            .tag(tag)
        };
        let sequence = Effect::new(
            crate::effects::SequenceEffect::coordinated_with_leading_duration(vec![
                modifier(first_target, -3, "first_target"),
                modifier(later_target.clone(), -2, "second_target"),
                modifier(later_target, -1, "third_target"),
            ]),
        );

        let rendered = describe_effect(&sequence);
        assert!(
            rendered.starts_with("Until your next turn, target creature an opponent controls"),
            "{rendered}"
        );
        assert!(rendered.contains("gets -3/-0"), "{rendered}");
        assert!(rendered.contains("gets -2/-0"), "{rendered}");
        assert!(rendered.contains("gets -1/-0"), "{rendered}");
        assert_eq!(rendered.matches("gets ").count(), 3, "{rendered}");
        assert_eq!(
            rendered
                .to_ascii_lowercase()
                .matches("until your next turn")
                .count(),
            1,
            "{rendered}"
        );
    }

    #[test]
    fn coordinated_animation_preserves_remove_and_grant_siblings() {
        let animated = TagKey::from("animated_0");
        let removed = TagKey::from("removed_1");
        let animation = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::target_creature(),
                crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
                Until::EndOfTurn,
            )
            .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(4),
                toughness: Value::Fixed(4),
                sublayer: crate::continuous::PtSublayer::Setting,
            })
            .with_additional_modification(crate::continuous::Modification::SetColors(
                crate::color::ColorSet::BLUE,
            ))
            .with_additional_modification(
                crate::continuous::Modification::AddSubtypes(vec![
                    Subtype::Dragon,
                    Subtype::Illusion,
                ]),
            ),
        )
        .tag(animated.clone());
        let remove = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
            ChooseSpec::Tagged(animated),
            crate::effects::continuous::RuntimeModification::RemoveAllAbilities,
            Until::EndOfTurn,
        ))
        .tag(removed.clone());
        let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(removed),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::flying(),
            ),
            Until::EndOfTurn,
        ));
        let sequence = Effect::new(
            crate::effects::SequenceEffect::coordinated_with_leading_duration(vec![
                animation, remove, grant,
            ]),
        );

        let rendered = describe_effect(&sequence);
        assert!(
            rendered.starts_with("Until end of turn, target creature"),
            "{rendered}"
        );
        assert!(
            rendered.contains("loses all abilities, and gains flying"),
            "{rendered}"
        );
    }

    #[test]
    fn coordinated_graveyard_returns_share_one_typed_route() {
        let returned = |subtype| {
            let target = ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You)
                    .with_subtype(subtype),
            ))
            .with_count(ChoiceCount::up_to(1));
            Effect::new(crate::effects::ReturnFromGraveyardToHandEffect::new(
                target, false,
            ))
        };
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            returned(Subtype::Pirate),
            returned(Subtype::Vampire),
            returned(Subtype::Dinosaur),
        ]));
        let rendered = describe_effect(&sequence);
        assert!(rendered.starts_with("Return "), "{rendered}");
        assert!(rendered.contains(", and "), "{rendered}");
        assert_eq!(
            rendered
                .matches(" from your graveyard to your hand")
                .count(),
            1
        );
        assert!(!rendered.contains(". Return"), "{rendered}");
    }
}

/// "You and that player each <verb> ..." for adjacent same-payload effects
/// whose only difference is the affected player (you + a back-reference).
pub(in crate::compiled_text) fn describe_joint_subject_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    fn joint_other_surface(player: &PlayerFilter) -> Option<&'static str> {
        match player {
            PlayerFilter::DamagedPlayer
            | PlayerFilter::TaggedPlayer(_)
            | PlayerFilter::ChosenPlayer => Some("that player"),
            PlayerFilter::Target(inner) if **inner == PlayerFilter::Opponent => {
                Some("target opponent")
            }
            PlayerFilter::Target(inner) if **inner == PlayerFilter::Any => Some("target player"),
            _ => None,
        }
    }

    fn joined_damage_text(amount: &Value, recipients: &str) -> String {
        let (amount_text, where_x) = describe_damage_amount_clause(amount);
        let mut text = format!("this deals {amount_text} to {recipients}");
        if let Some(where_x) = where_x {
            text.push_str(&format!(", where X is {where_x}"));
        }
        text
    }

    if let Some(compact) = describe_linked_same_source_damage_pair(first, second) {
        return Some(compact);
    }

    if let Some(first_gain) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::GainLifeEffect>()
        && let Some(second_gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && first_gain.amount == second_gain.amount
        && matches!(&first_gain.player, ChooseSpec::Player(PlayerFilter::You))
        && let ChooseSpec::Player(second_player) = &second_gain.player
        && let Some(other) = joint_other_surface(second_player)
    {
        return Some(format!(
            "You and {other} each gain {}",
            describe_life_amount_phrase(&first_gain.amount)
        ));
    }

    if let Some(first_draw) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(second_draw) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::DrawCardsEffect>()
        && first_draw.count == second_draw.count
        && first_draw.player == PlayerFilter::You
        && let Some(other) = joint_other_surface(&second_draw.player)
    {
        return Some(format!(
            "You and {other} each draw {}",
            describe_card_count(&first_draw.count)
        ));
    }

    // "Target creature you control deals damage equal to its power to each
    // of two other target creatures" — tagged target prelude + execute-with-
    // source power damage compact to the oracle's single sentence.
    if let Some(first_tagged) = first.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(target_only) = first_tagged
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(second_exec) = unwrap_basic_tag_wrappers(second)
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        && matches!(&second_exec.source, ChooseSpec::Tagged(tag) if *tag == first_tagged.tag)
        && let Some(damage) = second_exec
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(
            &damage.amount,
            Value::PowerOf(spec)
                if matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if *tag == first_tagged.tag)
        )
    {
        let source_text = describe_choose_spec(&target_only.target);
        let raw_target = describe_choose_spec(&damage.target);
        let target_text = if let Some((count_word, rest)) = raw_target.split_once(" target other ")
        {
            format!("each of {count_word} other target {rest}")
        } else {
            raw_target
        };
        return Some(format!(
            "{} deals damage equal to its power to {target_text}",
            capitalize_first(&source_text)
        ));
    }

    // "deals X damage to each creature and each player" — for-each-creature
    // damage followed by for-each-player damage with the same amount.
    if let Some(for_each) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::ForEachObject>()
        && let [inner] = for_each.effects.as_slice()
        && let Some(first_damage) =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(first_damage.target, ChooseSpec::Iterated)
        && for_each.filter.card_types == [CardType::Creature]
        && for_each.filter.controller.is_none()
        && for_each.filter.subtypes.is_empty()
        && for_each.filter.tagged_constraints.is_empty()
        && let Some(for_players) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::ForPlayersEffect>()
        && for_players.filter == PlayerFilter::Any
        && let [player_inner] = for_players.effects.as_slice()
        && let Some(second_damage) = unwrap_basic_tag_wrappers(player_inner)
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && second_damage.amount == first_damage.amount
        && matches!(
            &second_damage.target,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    {
        let creature_filter = describe_damage_fanout_filter(&for_each.filter)?;
        let recipients = format!("each {creature_filter} and each player");
        return Some(joined_damage_text(&first_damage.amount, &recipients));
    }

    // "deals X damage to each creature and each planeswalker" — same as
    // the creature/player compaction above, but both halves are object
    // fanouts.
    if let Some(for_each) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::ForEachObject>()
        && let [inner] = for_each.effects.as_slice()
        && let Some(first_damage) =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(first_damage.target, ChooseSpec::Iterated)
        && for_each.filter.card_types == [CardType::Creature]
        && for_each.filter.controller.is_none()
        && for_each.filter.subtypes.is_empty()
        && for_each.filter.tagged_constraints.is_empty()
        && let Some(for_each_planeswalker) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::ForEachObject>()
        && let [planeswalker_inner] = for_each_planeswalker.effects.as_slice()
        && let Some(second_damage) = unwrap_basic_tag_wrappers(planeswalker_inner)
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(second_damage.target, ChooseSpec::Iterated)
        && second_damage.amount == first_damage.amount
        && for_each_planeswalker.filter.card_types == [CardType::Planeswalker]
        && for_each_planeswalker.filter.controller.is_none()
        && for_each_planeswalker.filter.subtypes.is_empty()
        && for_each_planeswalker.filter.tagged_constraints.is_empty()
    {
        let creature_filter = describe_damage_fanout_filter(&for_each.filter)?;
        let planeswalker_filter = describe_damage_fanout_filter(&for_each_planeswalker.filter)?;
        let recipients = format!("each {creature_filter} and each {planeswalker_filter}");
        return Some(joined_damage_text(&first_damage.amount, &recipients));
    }

    if let Some(first_create) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::CreateTokenEffect>()
        && let Some(second_create) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::CreateTokenEffect>()
        && first_create.count == second_create.count
        && first_create.controller == PlayerFilter::You
        && let Some(other) = joint_other_surface(&second_create.controller)
    {
        // Compare the rendered token descriptions (after the count word) so
        // instance-specific token ids don't break the pairing.
        let first_rendered = describe_effect(first);
        let first_body = first_rendered
            .trim()
            .trim_end_matches('.')
            .strip_prefix("Create ")?
            .to_string();
        let second_rendered = describe_effect(second);
        let second_lower = second_rendered.to_ascii_lowercase();
        let creates_idx = second_lower.find("creates ")?;
        let second_body = second_rendered[creates_idx + "creates ".len()..]
            .trim_end_matches('.')
            .to_string();
        let description_after_count = |body: &str| {
            body.split_once(' ')
                .map(|(_, rest)| rest.to_ascii_lowercase())
        };
        if description_after_count(&first_body)? == description_after_count(&second_body)? {
            return Some(format!("You and {other} each create {first_body}"));
        }
    }

    None
}

pub(super) fn describe_player_or_planeswalker_damage_then_controlled_creature_damage(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let first_damage = deal_damage_effect_view(first)?;
    if describe_choose_spec(&first_damage.target) != "target player or planeswalker" {
        return None;
    }

    let for_each =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::ForEachObject>()?;
    let mut expected_filter = ObjectFilter::creature();
    expected_filter.controller = Some(PlayerFilter::TargetPlayerOrControllerOfTarget);
    if for_each.filter != expected_filter {
        return None;
    }

    let [inner] = for_each.effects.as_slice() else {
        return None;
    };
    let second_damage = deal_damage_effect_view(inner)?;
    if second_damage.amount != first_damage.amount
        || !matches!(second_damage.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let (amount_text, where_x) = describe_damage_amount_clause(&first_damage.amount);
    let mut text = format!(
        "Deal {amount_text} to target player or planeswalker and each creature that player or that planeswalker's controller controls"
    );
    if let Some(where_x) = where_x {
        text.push_str(&format!(", where X is {where_x}"));
    }
    Some(text)
}

pub(crate) fn describe_false_only_conditional(
    condition: &crate::effect::Condition,
    false_branch: &str,
) -> String {
    if let crate::effect::Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
    } = condition
    {
        let verb = if tag.as_str().starts_with("discarded_") {
            Some("discard")
        } else if tag.as_str().starts_with("sacrificed_") {
            Some("sacrifice")
        } else if tag.as_str().starts_with("exiled_") {
            Some("exile")
        } else if tag.as_str().starts_with("destroyed_") {
            Some("destroy")
        } else {
            None
        };
        if let Some(verb) = verb {
            let object_text = if (tag.as_str().starts_with("discarded_")
                || tag.as_str().starts_with("exiled_")
                || tag.as_str().starts_with("revealed_"))
                && !filter.card_types.is_empty()
                && filter.zone.is_none()
                && filter.controller.is_none()
                && filter.owner.is_none()
                && filter.subtypes.is_empty()
                && filter.any_of.is_empty()
                && filter.tagged_constraints.is_empty()
            {
                let words = filter
                    .card_types
                    .iter()
                    .map(|card_type| describe_card_type_word_local(*card_type).to_string())
                    .collect::<Vec<_>>();
                with_indefinite_article(&format!("{} card", join_with_or(&words)))
            } else {
                let desc = filter.description();
                let stripped = strip_leading_article(&desc).to_ascii_lowercase();
                if (tag.as_str().starts_with("discarded_")
                    || tag.as_str().starts_with("exiled_")
                    || tag.as_str().starts_with("revealed_"))
                    && stripped == "land"
                {
                    "a land card".to_string()
                } else if (tag.as_str().starts_with("discarded_")
                    || tag.as_str().starts_with("exiled_")
                    || tag.as_str().starts_with("revealed_"))
                    && stripped == "creature"
                {
                    "a creature card".to_string()
                } else {
                    with_indefinite_article(&desc)
                }
            };
            return format!(
                "If {} didn't {} {} this way, {}",
                describe_player_filter(player),
                verb,
                object_text,
                false_branch
            );
        }
    }

    if matches!(condition, crate::effect::Condition::ThisSpellEscaped) {
        let branch = false_branch.trim().trim_end_matches('.');
        if branch.is_empty() {
            return "Unless it escaped".to_string();
        }
        return format!("{branch} unless it escaped");
    }

    format!(
        "Unless {}, {}",
        lowercase_first(&describe_condition(condition)),
        false_branch
    )
}

pub(crate) fn describe_exile_then_return(
    tagged: &crate::effects::TaggedEffect,
    move_back: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if move_back.zone != Zone::Battlefield {
        return None;
    }
    let crate::target::ChooseSpec::Tagged(return_tag) = &move_back.target else {
        return None;
    };
    if return_tag != &tagged.tag {
        return None;
    }
    let exile_move = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_move.zone != Zone::Exile {
        return None;
    }
    let target = describe_choose_spec(&exile_move.target);
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "those cards"
    } else {
        "that card"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix}{controller_suffix}"
    ))
}

pub(super) fn describe_source_exile_then_return(first: &Effect, second: &Effect) -> Option<String> {
    let exile_move =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_back =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_move.zone != Zone::Exile || move_back.zone != Zone::Battlefield {
        return None;
    }
    if !matches!(exile_move.target.unhinted(), ChooseSpec::Source) {
        return None;
    }
    if !matches!(move_back.target.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    {
        return None;
    }

    let target = describe_source_motion_reference(&exile_move.target, "this");
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix}{controller_suffix}"
    ))
}

pub(super) fn describe_source_motion_reference(spec: &ChooseSpec, named_fallback: &str) -> String {
    let Some(surface) = spec.source_reference_surface() else {
        return named_fallback.to_string();
    };
    match surface {
        crate::target::SourceReferenceSurface::ThisPermanentType(text)
            if matches!(
                text.to_ascii_lowercase().as_str(),
                "this"
                    | "it"
                    | "itself"
                    | "him"
                    | "himself"
                    | "her"
                    | "herself"
                    | "them"
                    | "themselves"
            ) =>
        {
            named_fallback.to_string()
        }
        crate::target::SourceReferenceSurface::ThisPermanentType(text) => text.clone(),
        crate::target::SourceReferenceSurface::FullName(_)
        | crate::target::SourceReferenceSurface::ShortName(_) => named_fallback.to_string(),
    }
}

pub(super) fn describe_exile_then_return_transformed_with_counter(
    exile_effect: &Effect,
    return_effect: &Effect,
    transform_effect: &Effect,
    put_counter_effect: &Effect,
) -> Option<String> {
    let exile_tag = wrapped_effect_tag(exile_effect);
    let exile_move = unwrap_basic_tag_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_back = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let transform = unwrap_basic_tag_wrappers(transform_effect)
        .downcast_ref::<crate::effects::TransformEffect>()?;
    let put_counter = unwrap_basic_tag_wrappers(put_counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if move_back.zone != Zone::Battlefield
        || exile_move.zone != Zone::Exile
        || put_counter.distributed
        || put_counter.target_count.is_some()
    {
        return None;
    }
    let crate::target::ChooseSpec::Tagged(return_tag) = &move_back.target else {
        return None;
    };
    let exile_target_is_source = matches!(exile_move.target.unhinted(), ChooseSpec::Source);
    let returns_exiled_source =
        return_tag.as_str() == "__source_exiled__" && exile_target_is_source;
    let returns_wrapped_exile = return_tag.as_str().starts_with("exiled_")
        && exile_tag.is_some_and(|exile_tag| exile_tag == return_tag);
    if !returns_exiled_source && !returns_wrapped_exile {
        return None;
    }
    if !matches!(&transform.target, ChooseSpec::Tagged(tag) if tag == return_tag)
        || !matches!(&put_counter.target, ChooseSpec::Tagged(tag) if tag == return_tag)
    {
        return None;
    }

    let target = if exile_target_is_source {
        describe_source_motion_reference(&exile_move.target, "it")
    } else {
        describe_choose_spec(&exile_move.target)
    };
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    let counter_text = describe_put_counter_phrase(&put_counter.amount, put_counter.counter_type);
    Some(format!(
        "Exile {target}, then put {return_object} onto the battlefield{tapped_suffix} transformed{controller_suffix} with {counter_text} on it"
    ))
}

pub(crate) fn describe_exile_return_then_transform(
    exile_effect: &Effect,
    return_effect: &Effect,
    transform_effect: &Effect,
) -> Option<String> {
    let exile_move = unwrap_basic_tag_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_back = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let transform = unwrap_basic_tag_wrappers(transform_effect)
        .downcast_ref::<crate::effects::TransformEffect>()?;
    if exile_move.zone != Zone::Exile || move_back.zone != Zone::Battlefield {
        return None;
    }
    let exile_target_is_source = matches!(exile_move.target.unhinted(), ChooseSpec::Source);
    let returns_exiled_source = exile_target_is_source
        && matches!(move_back.target.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == "__source_exiled__")
        && move_back.target.unhinted() == transform.target.unhinted();
    let return_and_transform_source = matches!(move_back.target.unhinted(), ChooseSpec::Source)
        && matches!(transform.target.unhinted(), ChooseSpec::Source);
    let direct_same_target = exile_move.target.unhinted() == move_back.target.unhinted()
        && move_back.target.unhinted() == transform.target.unhinted();
    let source_name_exile = return_and_transform_source
        && is_plain_source_name_exile_filter(exile_move.target.unhinted());
    if !returns_exiled_source && !direct_same_target && !source_name_exile {
        return None;
    }

    let target = if return_and_transform_source || exile_target_is_source {
        let fallback = describe_choose_spec(&exile_move.target);
        describe_source_motion_reference(&exile_move.target, &fallback)
    } else {
        describe_choose_spec(&exile_move.target)
    };
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix} transformed{controller_suffix}"
    ))
}

pub(super) fn is_plain_source_name_exile_filter(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Object(filter) = spec else {
        return false;
    };
    filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.len() == 1
        && filter.name.is_none()
        && !filter.source
        && filter.any_of.is_empty()
}

pub(super) fn move_to_zone_for_transform_compaction(
    effect: &Effect,
) -> Option<(
    &crate::effects::MoveToZoneEffect,
    Option<&crate::tag::TagKey>,
)> {
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return Some((move_to_zone, None));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let move_to_zone = tagged
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        return Some((move_to_zone, Some(&tagged.tag)));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return move_to_zone_for_transform_compaction(&with_id.effect);
    }
    None
}

pub(super) fn transform_targets_returned_object(
    move_back: &crate::effects::MoveToZoneEffect,
    returned_tag: Option<&crate::tag::TagKey>,
    transform: &crate::effects::TransformEffect,
) -> bool {
    if let Some(returned_tag) = returned_tag
        && matches!(&transform.target, ChooseSpec::Tagged(transform_tag) if transform_tag == returned_tag)
    {
        return true;
    }
    move_back.target.unhinted() == transform.target.unhinted()
}

pub(crate) fn describe_return_then_transform(
    return_effect: &Effect,
    transform_effect: &Effect,
) -> Option<String> {
    let (move_back, returned_tag) = move_to_zone_for_transform_compaction(return_effect)?;
    let transform = transform_effect.downcast_ref::<crate::effects::TransformEffect>()?;
    if move_back.zone != Zone::Battlefield {
        return None;
    }
    if !transform_targets_returned_object(move_back, returned_tag, transform) {
        return None;
    }

    let return_object = if choose_spec_allows_multiple(&move_back.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&move_back.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Return {return_object} to the battlefield{tapped_suffix} transformed{controller_suffix}"
    ))
}

pub(crate) fn describe_reveal_top_then_if_put_into_hand(
    reveal_top: &crate::effects::RevealTopEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let reveal_tag = reveal_top.tag.as_ref()?;
    let Condition::TaggedObjectMatches(cond_tag, filter) = &conditional.condition else {
        return None;
    };
    if cond_tag != reveal_tag {
        return None;
    }
    let move_to_zone = conditional.if_true[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand {
        return None;
    }
    if !matches!(
        move_to_zone.target.base(),
        ChooseSpec::Tagged(tag) if tag == reveal_tag
    ) {
        return None;
    }

    let subject = describe_player_filter(&reveal_top.player);
    let is_you = subject == "you";
    let reveal_sentence = if is_you {
        "Reveal the top card of your library".to_string()
    } else {
        let mut reveal_subject = subject;
        if matches!(
            reveal_top.player,
            PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::DamagedPlayer
        ) {
            if let Some(rest) = reveal_subject.strip_prefix("the ") {
                reveal_subject = rest.to_string();
            }
        }
        let verb = player_verb(&reveal_subject, "reveal", "reveals");
        format!("{reveal_subject} {verb} the top card of their library")
    };

    // Match the common oracle pattern for "if it's a <type> card".
    let desc = filter.description();
    let stripped = strip_leading_article(&desc).trim().to_ascii_lowercase();
    let noun_phrase = if stripped.ends_with(" card") {
        stripped.clone()
    } else if matches!(
        stripped.as_str(),
        "land"
            | "creature"
            | "artifact"
            | "enchantment"
            | "planeswalker"
            | "battle"
            | "permanent"
            | "instant"
            | "sorcery"
    ) {
        format!("{stripped} card")
    } else {
        return None;
    };
    let condition_text = format!("it's {}", with_indefinite_article(&noun_phrase));

    let move_sentence = if is_you {
        "put it into your hand".to_string()
    } else {
        "that player puts it into their hand".to_string()
    };

    Some(format!(
        "{reveal_sentence}. If {condition_text}, {move_sentence}"
    ))
}

pub(super) fn move_to_zone_for_tag<'a>(
    effect: &'a Effect,
    tag: &crate::TagKey,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return move_to_zone_for_tag(&tagged.effect, tag);
    }
    let move_to_zone = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    matches!(move_to_zone.target.base(), ChooseSpec::Tagged(target_tag) if target_tag == tag)
        .then_some(move_to_zone)
}

pub(super) fn describe_reveal_top_may_put_otherwise_hand(
    reveal_top: &crate::effects::RevealTopEffect,
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if if_effect.condition != with_id.id
        || !matches!(if_effect.predicate, EffectPredicate::DidNotHappen)
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return None;
    }

    let reveal_tag = reveal_top.tag.as_ref()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [may_effect] = may.effects.as_slice() else {
        return None;
    };
    let conditional = may_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let Condition::TaggedObjectMatches(cond_tag, _) = &conditional.condition else {
        return None;
    };
    if cond_tag != reveal_tag {
        return None;
    }

    let battlefield_move = move_to_zone_for_tag(&conditional.if_true[0], reveal_tag)?;
    if battlefield_move.zone != Zone::Battlefield || battlefield_move.to_top {
        return None;
    }
    let hand_move = move_to_zone_for_tag(&if_effect.then[0], reveal_tag)?;
    if hand_move.zone != Zone::Hand || hand_move.to_top {
        return None;
    }

    let revealer = describe_player_filter(&reveal_top.player);
    let revealer_is_you = revealer == "you";
    let reveal_sentence = if revealer_is_you {
        "Reveal the top card of your library".to_string()
    } else {
        let verb = player_verb(&revealer, "reveal", "reveals");
        format!("{revealer} {verb} the top card of their library")
    };

    let decider = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let may_prefix = if decider == "you" {
        "You may".to_string()
    } else {
        format!("{decider} may")
    };

    let tapped_suffix = if battlefield_move.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&battlefield_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let controller_suffix = match battlefield_move.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    let condition = lowercase_first(&describe_condition(&conditional.condition));

    let hand_clause = if revealer_is_you {
        "put that card into your hand".to_string()
    } else {
        format!("{revealer} puts that card into their hand")
    };

    Some(format!(
        "{reveal_sentence}. {may_prefix} put that card onto the battlefield{tapped_suffix}{controller_suffix} if {condition}. Otherwise, {hand_clause}"
    ))
}

pub(crate) fn describe_tagged_target_then_power_damage(
    tagged: &crate::effects::TaggedEffect,
    deal: &crate::effects::DealDamageEffect,
) -> Option<String> {
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let Value::PowerOf(source_spec) = &deal.amount else {
        return None;
    };
    let source_tag = match source_spec.as_ref() {
        ChooseSpec::Tagged(tag) => tag,
        _ => return None,
    };
    if source_tag.as_str() != tagged.tag.as_str() {
        return None;
    }

    if let ChooseSpec::Player(
        PlayerFilter::ControllerOf(controller_ref) | PlayerFilter::OwnerOf(controller_ref),
    ) = deal.target.base()
        && matches!(controller_ref, crate::filter::ObjectRef::Tagged(tag) if tag.as_str() == source_tag.as_str())
    {
        return None;
    }

    let source_text = describe_choose_spec(&target_only.target);
    if matches!(
        deal.target,
        ChooseSpec::Tagged(ref target_tag) if target_tag == source_tag
    ) {
        return Some(format!(
            "{source_text} deals damage to itself equal to its power"
        ));
    }

    let raw_target = describe_choose_spec(&deal.target);
    // Multi-target full damage reads "each of two other target creatures".
    let target_text = if let Some((count_word, rest)) = raw_target.split_once(" target other ") {
        format!("each of {count_word} other target {rest}")
    } else {
        raw_target
    };
    Some(format!(
        "{source_text} deals damage equal to its power to {target_text}"
    ))
}

pub(super) fn describe_execute_power_damage_from_tag<'a>(
    effect: &'a Effect,
    source_tag: &crate::TagKey,
) -> Option<&'a crate::effects::DealDamageEffect> {
    let mut effect = effect;
    loop {
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            effect = &tag_all.effect;
            continue;
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            effect = &tagged.effect;
            continue;
        }
        break;
    }
    let with_source = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let ChooseSpec::Tagged(tag) = &with_source.source else {
        return None;
    };
    if tag.as_str() != source_tag.as_str() {
        return None;
    }
    let deal = with_source
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let Value::PowerOf(power_source) = &deal.amount else {
        return None;
    };
    let ChooseSpec::Tagged(power_tag) = power_source.as_ref() else {
        return None;
    };
    if power_tag.as_str() != source_tag.as_str() {
        return None;
    }
    Some(deal)
}

pub(super) fn describe_damage_amount_clause(amount: &Value) -> (String, Option<String>) {
    if matches!(amount.unhinted(), Value::XTimes(2)) {
        return ("twice X damage".to_string(), None);
    }
    if let Some((amount_text, where_x)) = describe_damage_amount_with_revealed_count_where_x(amount)
    {
        return (amount_text, Some(where_x));
    }
    if value_prefers_equal_to(amount)
        || power_damage_prefers_equal_to(amount)
        || (!value_prefers_where_x(amount) && count_damage_prefers_equal_to(amount))
    {
        return (format!("damage equal to {}", describe_value(amount)), None);
    }
    if let Some(where_x) = describe_where_x_basis(amount) {
        return ("X damage".to_string(), Some(where_x));
    }
    if is_effect_count_reference(amount, None) {
        return ("that much".to_string(), None);
    }
    (format!("{} damage", describe_value(amount)), None)
}

pub(super) fn describe_where_x_offset_value(value: &Value) -> Option<(String, String)> {
    let Value::Add(left, right) = value.unhinted() else {
        return None;
    };
    let (basis_value, offset) = match (left.as_ref(), right.as_ref()) {
        (Value::Fixed(offset), basis) | (basis, Value::Fixed(offset))
            if value_prefers_where_x(basis) =>
        {
            (basis, *offset)
        }
        _ => return None,
    };
    let basis = describe_where_x_basis(basis_value)?;
    let amount = if offset > 0 {
        format!("X plus {offset}")
    } else if offset < 0 {
        format!("X minus {}", -offset)
    } else {
        "X".to_string()
    };
    Some((amount, basis))
}

pub(super) fn choose_spec_filter_where_x_clause(spec: &ChooseSpec) -> Option<String> {
    fn comparison_value(comparison: &Option<crate::filter::Comparison>) -> Option<&Value> {
        let value = match comparison.as_ref()? {
            crate::filter::Comparison::EqualExpr(value)
            | crate::filter::Comparison::NotEqualExpr(value)
            | crate::filter::Comparison::LessThanExpr(value)
            | crate::filter::Comparison::LessThanOrEqualExpr(value)
            | crate::filter::Comparison::GreaterThanExpr(value)
            | crate::filter::Comparison::GreaterThanOrEqualExpr(value) => value,
            _ => return None,
        };
        (!value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ExplicitComparison))
            .then_some(value)
    }

    fn filter_basis(filter: &ObjectFilter) -> Option<String> {
        [&filter.power, &filter.toughness, &filter.mana_value]
            .into_iter()
            .filter_map(comparison_value)
            .find_map(describe_where_x_basis)
            .or_else(|| filter.any_of.iter().find_map(filter_basis))
    }

    let filter = match spec.unhinted() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => {
            return choose_spec_filter_where_x_clause(inner);
        }
        _ => return None,
    };
    filter_basis(filter).map(|basis| format!(", where X is {basis}"))
}

pub(super) fn describe_damage_target(target: &ChooseSpec) -> String {
    if let Some(text) = describe_counted_any_damage_target(target) {
        return text;
    }
    if let ChooseSpec::Player(PlayerFilter::ControllerOf(reference)) = target.base() {
        return match reference {
            crate::target::ObjectRef::Tagged(tag) if tag.as_str().starts_with("damaged_") => {
                "that permanent's controller".to_string()
            }
            crate::target::ObjectRef::Tagged(tag)
                if tag.as_str() == "triggering"
                    || tag.as_str().starts_with("targeted_")
                    || tag.as_str() == "__it__" =>
            {
                "that creature's controller".to_string()
            }
            crate::target::ObjectRef::Target => "that permanent's controller".to_string(),
            _ => "that object's controller".to_string(),
        };
    }
    if let ChooseSpec::Player(PlayerFilter::OwnerOf(reference)) = target.base() {
        return match reference {
            crate::target::ObjectRef::Tagged(tag) if tag.as_str().starts_with("damaged_") => {
                "that permanent's owner".to_string()
            }
            crate::target::ObjectRef::Tagged(tag)
                if tag.as_str() == "triggering"
                    || tag.as_str().starts_with("targeted_")
                    || tag.as_str() == "__it__" =>
            {
                "that creature's owner".to_string()
            }
            crate::target::ObjectRef::Target => "that permanent's owner".to_string(),
            _ => "that object's owner".to_string(),
        };
    }
    describe_choose_spec(target)
}

pub(super) fn describe_counted_any_damage_target(target: &ChooseSpec) -> Option<String> {
    let ChooseSpec::WithCount(inner, count) = target.unhinted() else {
        return None;
    };
    if !matches!(inner.unhinted(), ChooseSpec::AnyTarget) || count.random {
        return None;
    }
    if count.is_up_to_dynamic_x() {
        return Some("each of up to X targets".to_string());
    }
    if count.is_dynamic_x() {
        return Some("each of X targets".to_string());
    }

    match (count.min, count.max) {
        (0, Some(1)) => Some("up to one target".to_string()),
        (0, Some(max)) if max > 1 => {
            let count_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("each of up to {count_text} targets"))
        }
        (min, Some(max)) if min == max && max > 1 => {
            let count_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("each of {count_text} targets"))
        }
        (1, Some(2)) => Some("each of one or two targets".to_string()),
        _ => None,
    }
}

pub(super) fn count_damage_prefers_equal_to(amount: &Value) -> bool {
    match amount.unhinted() {
        Value::Count(_) | Value::CountScaled(_, _) | Value::ManaSymbolsInManaCostOf { .. } => true,
        Value::CountersOnSource(_) | Value::CountersOn(_, _) => true,
        Value::Scaled(inner, _) => count_damage_prefers_equal_to(inner),
        _ => false,
    }
}

pub(super) fn power_damage_prefers_equal_to(amount: &Value) -> bool {
    match amount.unhinted() {
        Value::SourcePower | Value::SourceToughness | Value::PowerOf(_) | Value::ToughnessOf(_) => {
            true
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => power_damage_prefers_equal_to(inner),
        Value::Add(left, right) => {
            power_damage_prefers_equal_to(left) || power_damage_prefers_equal_to(right)
        }
        _ => false,
    }
}

pub(super) fn describe_counter_count_with_where_x(
    value: &Value,
    counter_type: CounterType,
) -> Option<(String, String)> {
    if !value_prefers_where_x(value) {
        return None;
    }
    let where_x = describe_where_x_basis(value)?;
    Some((
        format!("X {} counters", describe_counter_type(counter_type)),
        where_x,
    ))
}

pub(crate) fn describe_target_power_damage_to_other_and_self(
    target_effect: &Effect,
    other_damage_effect: &Effect,
    self_damage_effect: &Effect,
) -> Option<String> {
    let tagged_target = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged_target
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let other_damage =
        describe_execute_power_damage_from_tag(other_damage_effect, &tagged_target.tag)?;
    let self_damage =
        describe_execute_power_damage_from_tag(self_damage_effect, &tagged_target.tag)?;
    if !matches!(other_damage.target, ChooseSpec::AnyOtherTarget) {
        return None;
    }
    if !matches!(
        self_damage.target,
        ChooseSpec::Tagged(ref target_tag) if target_tag.as_str() == tagged_target.tag.as_str()
    ) {
        return None;
    }

    let subject = capitalize_first(&describe_choose_spec(&target_only.target));
    let verb = if choose_spec_is_plural(&target_only.target) {
        "deal"
    } else {
        "deals"
    };
    Some(format!(
        "{subject} {verb} X damage to {} and X damage to itself, where X is its power",
        describe_choose_spec(&other_damage.target)
    ))
}

pub(crate) fn cleanup_decompiled_text(text: &str) -> String {
    let mut out = text.to_string();
    fn combine_until_end_choice_grant(text: &str, marker: &str) -> Option<String> {
        let (head, tail) = text.split_once(marker)?;
        if !head.contains(" gets ") {
            return None;
        }
        let choice = tail.strip_suffix(" until end of turn")?;
        Some(format!(
            "{head} and gains your choice of {choice} until end of turn"
        ))
    }

    for marker in [
        " until end of turn. it gains your choice of ",
        " until end of turn. this creature gains your choice of ",
        " until end of turn. This creature gains your choice of ",
    ] {
        if let Some(combined) = combine_until_end_choice_grant(&out, marker) {
            out = combined;
        }
    }

    for (from, to) in [
        (
            "votes are revealed, for each vote",
            "votes are revealed. For each vote",
        ),
        (", then for each vote", ". For each vote"),
        ("you gets", "you get"),
        ("you puts", "you put"),
        ("a artifact", "an artifact"),
        ("a Assassin", "an Assassin"),
        ("a another", "another"),
        ("a enchantment", "an enchantment"),
        ("a untapped", "an untapped"),
        ("a opponent", "an opponent"),
        (" ors ", " or "),
        ("creature are", "creatures are"),
        ("creatures that shares ", "creatures that share "),
        ("Creatures that shares ", "Creatures that share "),
        ("Creatures token", "Creature tokens"),
        ("creatures token", "creature tokens"),
        ("that many color plus one", "that many colors plus one"),
        ("target any target", "any target"),
        (" to any while ", " to any target while "),
        (" to any until ", " to any target until "),
        (" to any.", " to any target."),
        (" to any,", " to any target,"),
    ] {
        while out.contains(from) {
            out = out.replace(from, to);
        }
    }
    while out.contains("target target") {
        out = out.replace("target target", "target");
    }
    while out.contains("Target target") {
        out = out.replace("Target target", "Target");
    }
    out
}

pub(crate) fn describe_inline_ability(ability: &Ability) -> String {
    describe_inline_ability_with_self_subject(ability, "this creature")
}

pub(super) fn rewrite_cost_bound_x_phrases(
    mut effects: String,
    costs: &[crate::costs::Cost],
) -> String {
    if let Some(x_phrase) = removed_counters_this_way_x_phrase(costs) {
        effects = effects.replace("where X is X", &format!("where X is {x_phrase}"));
        effects = effects.replace(
            "deals X damage to",
            &format!("deals damage equal to {x_phrase} to"),
        );
        effects = effects.replace(
            "Deal X damage to",
            &format!("Deal damage equal to {x_phrase} to"),
        );
        effects = effects.replace(
            &format!("deals damage equal to {x_phrase} to"),
            "deals that much damage to",
        );
        effects = effects.replace(
            &format!("Deals damage equal to {x_phrase} to"),
            "Deals that much damage to",
        );
    }
    effects
}

pub(super) fn removed_counters_this_way_x_phrase(costs: &[crate::costs::Cost]) -> Option<String> {
    fn counter_phrase(counter_type: Option<CounterType>) -> String {
        match counter_type {
            Some(counter_type) => format!("{} counters", counter_type.description()),
            None => "counters".to_string(),
        }
    }

    for cost in costs {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        if let Some(remove_among) =
            effect.downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
            && remove_among.dynamic_count
            && !remove_among.display_x
        {
            return Some(format!(
                "the number of {} removed this way",
                counter_phrase(remove_among.counter_type)
            ));
        }
        if let Some(remove_from_source) =
            effect.downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
        {
            if remove_from_source.display_x {
                continue;
            }
            return Some(format!(
                "the number of {} removed this way",
                counter_phrase(remove_from_source.counter_type)
            ));
        }
    }
    None
}

pub(crate) fn describe_inline_ability_with_self_subject(
    ability: &Ability,
    self_subject: &str,
) -> String {
    if let Some(keyword) = describe_keyword_ability(ability) {
        return keyword;
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            describe_static_ability_with_subject(static_ability, self_subject)
        }
        AbilityKind::Triggered(triggered) => {
            describe_triggered_inline_ability(triggered, self_subject)
        }
        AbilityKind::Activated(activated) if activated.is_mana_ability() => {
            let mut line = String::new();
            if !activated.mana_cost.costs().is_empty() {
                line.push_str(&describe_cost_list(activated.mana_cost.costs()));
            }
            let mut payload = String::new();
            let mana_symbols = activated.mana_symbols();
            if !mana_symbols.is_empty() {
                payload.push_str("Add ");
                payload.push_str(
                    &mana_symbols
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join(""),
                );
            } else if !activated.effects.is_empty() {
                let rendered =
                    super::ast_render::describe_mana_ability_resolution_program(&activated.effects)
                        .unwrap_or_else(|| {
                            super::ast_render::describe_resolution_program(&activated.effects)
                        });
                payload.push_str(&rendered);
            }
            if !payload.is_empty() {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                line.push_str(&payload);
            }
            if let Some(prefix) = station_threshold_prefix(activated) {
                line = prefix_rendered_ability_body(line, &format!("{prefix} | "));
            } else if let Some(condition) =
                activation_condition_without_presentation_label(activated)
            {
                let clause = describe_mana_activation_condition(&condition);
                if !clause.is_empty() {
                    if !line.is_empty() {
                        line.push_str(". ");
                    }
                    line.push_str(&clause);
                }
            }
            for clause in describe_mana_usage_restriction_clauses_for_activated(activated) {
                if !line.is_empty() {
                    line.push_str(". ");
                }
                line.push_str(&clause);
            }
            if let Some(label) = activated_presentation_label(activated)
                && !line.starts_with(label)
            {
                line = format!("{label} — {line}");
            }
            if line.is_empty() {
                "a mana ability".to_string()
            } else {
                normalize_ability_self_reference_surface(&line, self_subject)
            }
        }
        AbilityKind::Activated(activated) => {
            if let Some(level_up) = describe_level_up_activation(activated) {
                return level_up;
            }
            if let Some(level) = activated
                .additional_restrictions
                .iter()
                .find_map(|restriction| restriction.strip_prefix("__ironsmith_class_level:"))
            {
                let effects = activated.effects.flattened_default_effects();
                if let [effect] = effects
                    && let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>()
                    && put.counter_type == crate::CounterType::Level
                    && matches!(put.target, ChooseSpec::Source)
                {
                    return format!(
                        "{}: Level {level}",
                        describe_cost_list(activated.mana_cost.costs())
                    );
                }
            }
            let mut line = String::new();
            let mut pre = Vec::new();
            let mut trailing_x_definition = None;
            if !activated.mana_cost.costs().is_empty() {
                let (cost_text, x_definition) =
                    describe_cost_list_with_trailing_x_definition(activated.mana_cost.costs());
                pre.push(cost_text);
                trailing_x_definition = x_definition;
            }
            if !activated.choices.is_empty()
                && !(!activated.effects.is_empty()
                    && choices_are_simple_targets(&activated.choices))
            {
                pre.push(format!(
                    "choose {}",
                    activated
                        .choices
                        .iter()
                        .map(describe_choose_spec)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !pre.is_empty() {
                line.push_str(&pre.join(", "));
            }
            if !activated.effects.is_empty() {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                let effects = super::ast_render::describe_resolution_program(&activated.effects);
                let mut effects = rewrite_damage_phrases_for_permanent_abilities(
                    &effects,
                    &capitalize_first(self_subject),
                    true,
                );
                effects = rewrite_cost_bound_x_phrases(effects, activated.mana_cost.costs());
                let self_subject = if self_subject == "this spell" {
                    "this creature"
                } else {
                    self_subject
                };
                effects = replace_this_spell_self_reference(effects, self_subject);
                line.push_str(&normalize_ability_self_reference_surface(
                    &effects,
                    self_subject,
                ));
            }
            if let Some(x_definition) = trailing_x_definition {
                append_sentence_clause(&mut line, &x_definition);
            }
            let restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            if !restriction_clauses.is_empty() {
                append_activation_clause(
                    &mut line,
                    &join_activation_restriction_clauses(&restriction_clauses),
                );
            }
            for clause in describe_mana_usage_restriction_clauses_for_activated(activated) {
                append_activation_clause(&mut line, &clause);
            }
            let line = if line.is_empty() {
                "an activated ability".to_string()
            } else if activated.is_exhaust_ability() {
                format!(
                    "Exhaust — {}",
                    normalize_ability_self_reference_surface(&line, self_subject)
                )
            } else {
                normalize_ability_self_reference_surface(&line, self_subject)
            };
            let line = if let Some(prefix) = level_range_activation_prefix(activated) {
                format!("{prefix}. {line}")
            } else {
                line
            };
            if let Some(label) = activated_presentation_label(activated)
                && !line.starts_with(label)
            {
                format!("{label} — {line}")
            } else {
                line
            }
        }
    }
}

pub(super) fn activated_presentation_label(
    activated: &crate::ability::ActivatedAbility,
) -> Option<&str> {
    activated
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix("__ironsmith_activation_label:"))
        .or_else(|| inferred_throw_activation_label(activated))
}

pub(super) fn activation_condition_without_presentation_label(
    activated: &crate::ability::ActivatedAbility,
) -> Option<crate::ConditionExpr> {
    let condition = activated.activation_condition.as_ref()?;
    let Some(label) = activated_presentation_label(activated) else {
        return Some(condition.clone());
    };
    if !label.trim().eq_ignore_ascii_case("Max speed") {
        return Some(condition.clone());
    }

    fn remove_max_speed(condition: &crate::ConditionExpr) -> Option<crate::ConditionExpr> {
        match condition {
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::Speed(crate::target::PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(4),
            } => None,
            crate::ConditionExpr::And(left, right) => {
                match (remove_max_speed(left), remove_max_speed(right)) {
                    (Some(left), Some(right)) => {
                        Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
                    }
                    (Some(only), None) | (None, Some(only)) => Some(only),
                    (None, None) => None,
                }
            }
            other => Some(other.clone()),
        }
    }

    remove_max_speed(condition)
}

pub(super) fn describe_ninjutsu_activation(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if ability.functional_zones != [Zone::Hand]
        || !activated.choices.is_empty()
        || !matches!(activated.timing, ActivationTiming::DuringCombat)
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.activation_condition.is_some()
        || !activated.mana_usage_restrictions.is_empty()
    {
        return None;
    }

    let mut mana_cost = None;
    let mut saw_ninjutsu_cost = false;
    for cost in activated.mana_cost.costs() {
        if let Some(cost) = cost.mana_cost_ref() {
            if mana_cost.is_some() {
                return None;
            }
            mana_cost = Some(cost);
            continue;
        }
        if cost.effect_ref().is_some_and(|effect| {
            effect
                .downcast_ref::<crate::effects::NinjutsuCostEffect>()
                .is_some()
        }) {
            saw_ninjutsu_cost = true;
            continue;
        }
        return None;
    }

    let effects = activated.effects.flattened_default_effects();
    if !saw_ninjutsu_cost
        || effects.len() != 1
        || effects[0]
            .downcast_ref::<crate::effects::NinjutsuEffect>()
            .is_none()
    {
        return None;
    }

    Some(format!("Ninjutsu {}", mana_cost?.to_oracle()))
}

pub(crate) fn level_range_activation_prefix(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    let range = activated
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix("__ironsmith_level_range:"))?;
    let (min, max) = range.split_once(':')?;
    if max == "+" {
        Some(format!("Level {min}+"))
    } else if min == max {
        Some(format!("Level {min}"))
    } else {
        Some(format!("Level {min}-{max}"))
    }
}

pub(super) fn inferred_throw_activation_label(
    activated: &crate::ability::ActivatedAbility,
) -> Option<&'static str> {
    let unattach_tag = activated
        .mana_cost
        .costs()
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::UnattachObjectsEffect>()
                .and_then(|unattach| match unattach.objects.base() {
                    ChooseSpec::Tagged(tag) => Some(tag.as_str()),
                    _ => None,
                })
        })?;

    activated
        .effects
        .flattened_default_effects()
        .iter()
        .any(|effect| effect_is_distributed_damage_from_tag(effect, unattach_tag))
        .then_some("Throw ...")
}

pub(super) fn effect_is_distributed_damage_from_tag(effect: &Effect, tag: &str) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_is_distributed_damage_from_tag(tagged.effect.as_ref(), tag);
    }

    effect
        .downcast_ref::<crate::effects::DealDistributedDamageEffect>()
        .is_some_and(|damage| {
            matches!(
                &damage.amount,
                crate::effect::Value::ManaValueOf(spec)
                    if matches!(spec.as_ref(), ChooseSpec::Tagged(amount_tag) if amount_tag.as_str() == tag)
            )
        })
}

pub(super) fn describe_granted_ability_phrase(ability: &Ability, self_subject: &str) -> String {
    let text = normalize_granted_triggered_ability_surface(
        describe_inline_ability_with_self_subject(ability, self_subject),
    );
    strip_redundant_granted_subject(text, self_subject)
}

pub(super) fn replace_this_spell_self_reference(text: String, subject: &str) -> String {
    const CAST_THIS_SPELL: &str = "__ironsmith_cast_this_spell__";
    let protected = text.replace("cast this spell", CAST_THIS_SPELL);
    let replaced = protected
        .replace("This spell", &capitalize_first(subject))
        .replace("this spell", &lowercase_first(subject));
    replaced.replace(CAST_THIS_SPELL, "cast this spell")
}

pub(super) fn strip_redundant_granted_subject(text: String, self_subject: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let subject_lower = self_subject.to_ascii_lowercase();
    let subject_prefix = format!("{subject_lower} ");
    if let Some(rest) = lower.strip_prefix(&subject_prefix)
        && (rest.starts_with("can't ") || rest.starts_with("cant "))
    {
        return text[subject_prefix.len()..].to_string();
    }
    text
}

pub(super) fn describe_add_mana_destination_suffix(player: &PlayerFilter) -> String {
    if matches!(player, PlayerFilter::You) {
        String::new()
    } else {
        format!(" to {}", describe_mana_pool_owner(player))
    }
}

/// When another player adds the mana, the runtime also gives THAT player the
/// color choice (`choose_mana_colors` is invoked for the resolved player), so
/// oracle idiom is subject-form: "That player adds one mana of any color they
/// choose" (Spectral Searchlight, Stadium Vendors).
pub(super) fn describe_other_player_adds_any_color(
    player: &PlayerFilter,
    amount: &Value,
    any_one_color: bool,
) -> Option<String> {
    if matches!(player, PlayerFilter::You) {
        return None;
    }
    let subject = match player {
        PlayerFilter::TaggedPlayer(_) | PlayerFilter::ChosenPlayer => "That player".to_string(),
        PlayerFilter::Target(inner) if matches!(**inner, PlayerFilter::Any) => {
            "Target player".to_string()
        }
        PlayerFilter::Target(inner) if matches!(**inner, PlayerFilter::Opponent) => {
            "Target opponent".to_string()
        }
        _ => return None,
    };
    let color_phrase = if any_one_color {
        "mana of any one color they choose"
    } else {
        "mana of any color they choose"
    };
    Some(format!(
        "{subject} adds {} {color_phrase}",
        describe_mana_amount_for_add_effect(amount)
    ))
}

pub(super) fn describe_mana_amount_for_add_effect(value: &Value) -> String {
    if let Value::Fixed(amount) = value
        && *amount >= 0
        && let Some(word) = small_number_word(*amount as u32)
    {
        return word.to_string();
    }
    describe_value(value)
}

pub(crate) fn describe_static_ability_with_subject(
    static_ability: &crate::static_abilities::StaticAbility,
    subject: &str,
) -> String {
    if let Some(spec) = static_ability.conditional_spell_keyword_spec() {
        return describe_conditional_spell_keyword_spec(spec);
    }
    if static_ability.id() == crate::static_abilities::StaticAbilityId::ChooseColorAsEnters {
        let mut line = format!("As {subject} enters, choose a color");
        if let Some(excluded) = static_ability
            .color_choice_as_enters()
            .and_then(|choice| choice.excluded)
        {
            line.push_str(" other than ");
            line.push_str(excluded.name());
        }
        return line;
    }

    let rendered = static_ability.display();
    let trimmed = rendered.trim();
    if trimmed.is_empty() {
        return rendered;
    }
    let static_display = trimmed.trim_end_matches('.').to_ascii_lowercase();
    if static_ability.id() == crate::static_abilities::StaticAbilityId::GrantAbility
        && matches!(
            static_display.as_str(),
            "creatures have blocks each combat if able"
                | "all creatures have blocks each combat if able"
        )
    {
        return format!("All creatures able to block {subject} do so");
    }
    if trimmed.starts_with("This spell ") || trimmed.starts_with("this spell ") {
        return trimmed.to_string();
    }
    if trimmed.starts_with("This card ") || trimmed.starts_with("this card ") {
        return capitalize_first(trimmed);
    }
    if !subject.starts_with("this ") {
        if let Some(rest) = trimmed.strip_prefix("As this enters") {
            return format!("As {subject} enters{rest}");
        }
        if let Some(rest) = trimmed.strip_prefix("as this enters") {
            return format!("As {subject} enters{rest}");
        }
    }
    if trimmed.starts_with("Cards in ") || trimmed.starts_with("Each ") {
        if let Some(body) = trimmed.strip_suffix(" as long as it's your turn") {
            return format!("During your turn, {}", lowercase_first(body));
        }
        return trimmed.to_string();
    }

    let capitalized_subject = capitalize_first(subject);
    let typed_self_subject = matches!(
        subject,
        "this Aura"
            | "this Class"
            | "this Equipment"
            | "this Fortification"
            | "this Saga"
            | "this Siege"
            | "this Vehicle"
    );
    let mut line = if let Some(rest) = trimmed.strip_prefix("This ") {
        if typed_self_subject
            && let Some(tail) = ["creature", "permanent", "artifact", "enchantment", "land"]
                .into_iter()
                .find_map(|generic| {
                    rest.strip_prefix(generic)
                        .filter(|tail| tail.starts_with(' ') || tail.starts_with("'s"))
                })
        {
            format!("{capitalized_subject}{tail}")
        } else if let Some(subject_kind) = subject.strip_prefix("this ")
            && let Some(tail) = rest.strip_prefix(subject_kind)
            && tail.starts_with(' ')
        {
            format!("{capitalized_subject}{tail}")
        } else {
            format!("{capitalized_subject} {rest}")
        }
    } else if let Some(rest) = trimmed.strip_prefix("this ") {
        format!("{subject} {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Attacks ") {
        format!("{capitalized_subject} attacks {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("attacks ") {
        format!("{subject} attacks {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Enters ") {
        format!("{capitalized_subject} enters {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("enters ") {
        format!("{subject} enters {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Escapes ") {
        format!("{capitalized_subject} escapes {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("escapes ") {
        format!("{subject} escapes {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Can't ") {
        format!("{capitalized_subject} can't {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("can't ") {
        format!("{subject} can't {rest}")
    } else {
        normalize_ability_self_reference_surface(trimmed, subject)
    };

    line = line.replace(
        " as long as PlayerHasCitysBlessing { player: You }",
        " as long as you have the city's blessing",
    );
    line = line.replace("This creature creature ", "This creature ");
    line = line.replace("this creature creature ", "this creature ");
    line = line.replace("This land land ", "This land ");
    line = line.replace("this land land ", "this land ");
    line = line.replace(
        "number of other creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    if let Some((ability_subject, predicate)) = line.rsplit_once(" has ") {
        let normalized = normalize_keyword_predicate_case(predicate.trim_end_matches('.'));
        let verb = if subject_text_uses_have(ability_subject) {
            "have"
        } else {
            "has"
        };
        if let Some(keyword) = normalized.strip_suffix(" as long as it's your turn") {
            return format!(
                "During your turn, {} {verb} {keyword}",
                lowercase_first(ability_subject)
            );
        }
        if normalized != predicate || verb != "has" {
            line = format!("{ability_subject} {verb} {normalized}");
        }
    }

    if let Some(rest) = line.strip_prefix("This creature has ")
        && let Some(keyword) = rest.strip_suffix(" as long as it's your turn")
    {
        return format!(
            "During your turn, this creature has {}",
            lowercase_first(keyword)
        );
    }
    if let Some(rest) = line.strip_prefix("During your turn, this creature has ")
        && rest.to_ascii_lowercase().starts_with("prevent ")
    {
        return format!("During your turn, {}", lowercase_first(rest));
    }
    if let Some(rest) = line.strip_prefix("This creature gets ")
        && let Some(pump) = rest.strip_suffix(" as long as it's your turn")
    {
        return format!("During your turn, this creature gets {pump}");
    }

    line
}

pub(super) fn describe_conditional_spell_keyword_spec(
    spec: crate::static_abilities::ConditionalSpellKeywordSpec,
) -> String {
    let keyword = match spec.keyword {
        crate::static_abilities::ConditionalSpellKeywordKind::Flash => "flash",
        crate::static_abilities::ConditionalSpellKeywordKind::Cascade => "cascade",
    };
    let metric = match spec.metric {
        crate::static_abilities::GraveyardCountMetric::CardTypes => "card types",
        crate::static_abilities::GraveyardCountMetric::ManaValues => "mana values",
    };
    let threshold =
        number_word(spec.threshold as i32).unwrap_or_else(|| spec.threshold.to_string());
    format!(
        "This spell has {keyword} as long as there are {threshold} or more {metric} among cards in your graveyard."
    )
}

pub(crate) fn subject_text_uses_have(subject: &str) -> bool {
    let lower = subject.to_ascii_lowercase();
    lower.contains("creatures")
        || lower.contains("permanents")
        || lower.contains("artifacts")
        || lower.contains("enchantments")
        || lower.contains("lands")
}

pub(super) fn rewrite_capped_trigger_surface(
    triggered: &crate::ability::TriggeredAbility,
    trigger_frequency: Option<TriggerFrequencySurface>,
) -> Option<String> {
    let capped_once = matches!(
        trigger_frequency,
        Some(TriggerFrequencySurface::AbilityMaxTimesEachTurn(1))
            | Some(TriggerFrequencySurface::DoThisMaxTimesEachTurn(1))
    );
    if !capped_once {
        return None;
    }

    if let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        && zone_change.player == crate::triggers::zone_changes::PlayerRelation::Any
        && zone_change.count_mode == crate::triggers::zone_changes::CountMode::Each
        && zone_change.from
            == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        && zone_change.to == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        && zone_change
            .object_filter
            .card_types
            .contains(&CardType::Creature)
    {
        let subject = pluralize_noun_phrase(strip_leading_article(
            &zone_change.object_filter.description(),
        ));
        return Some(format!("Whenever one or more {subject} die"));
    }

    if let Some(sacrifice) = triggered
        .trigger
        .downcast_ref::<crate::triggers::other::PlayerSacrificesTrigger>()
    {
        if !sacrifice.one_or_more_surface {
            return None;
        }
        let subject = pluralize_noun_phrase(strip_leading_article(&sacrifice.filter.description()));
        return match sacrifice.player {
            PlayerFilter::You => Some(format!("Whenever you sacrifice one or more {subject}")),
            PlayerFilter::Opponent => Some(format!(
                "Whenever one or more opponents sacrifice one or more {subject}"
            )),
            PlayerFilter::Any => Some(format!(
                "Whenever one or more players sacrifice one or more {subject}"
            )),
            _ => None,
        };
    }

    None
}

pub(super) fn describe_trigger_surface_with_frequency(
    triggered: &crate::ability::TriggeredAbility,
    trigger_frequency: Option<TriggerFrequencySurface>,
    self_subject: &str,
) -> String {
    if let Some(rewritten) = rewrite_capped_trigger_surface(triggered, trigger_frequency) {
        return rewrite_source_bound_trigger_subject(triggered, rewritten, self_subject);
    }

    if let Some(chapters) = triggered.trigger.saga_chapters() {
        let chapter_text = chapters
            .iter()
            .filter_map(|chapter| chapter_number_to_roman(*chapter))
            .collect::<Vec<_>>();
        if !chapter_text.is_empty() && chapter_text.len() == chapters.len() {
            return chapter_text.join(", ");
        }
    }

    // An object identified by its unique history with the source is definite,
    // even though the ordinary object-filter surface is intentionally
    // indefinite. For example: "the creature put onto the battlefield with
    // this enchantment", not "a creature ...".
    if let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        && zone_change.count_mode == crate::triggers::zone_changes::CountMode::Each
        && !zone_change.this_object
        && zone_change.object_filter.put_onto_battlefield_with_source
    {
        let displayed = triggered.trigger.display();
        for (indefinite, definite) in [
            ("When a ", "When the "),
            ("When an ", "When the "),
            ("Whenever a ", "Whenever the "),
            ("Whenever an ", "Whenever the "),
        ] {
            if let Some(rest) = displayed.strip_prefix(indefinite) {
                return format!("{definite}{rest}");
            }
        }
    }

    if let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        && zone_change.player == crate::triggers::zone_changes::PlayerRelation::Any
        && zone_change.count_mode == crate::triggers::zone_changes::CountMode::Each
        && zone_change.from
            == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        && zone_change.to == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        && zone_change.object_filter.owner == Some(PlayerFilter::You)
    {
        let mut filter = zone_change.object_filter.clone();
        filter.owner = None;
        let subject = with_indefinite_article(&filter.description());
        return format!("Whenever {subject} is put into your graveyard from the battlefield");
    }

    if triggered
        .trigger
        .downcast_ref::<crate::triggers::phase_step::BeginningOfEndStepTrigger>()
        .is_some_and(|trigger| trigger.player == PlayerFilter::Any)
        && triggered.intervening_if.as_ref().is_some_and(|condition| {
            matches!(
                condition,
                Condition::ValueComparison {
                    left: Value::Count(filter),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    ..
                } if is_source_exiled_cards_filter(filter)
            )
        })
    {
        return "At the beginning of the end step".to_string();
    }

    if triggered_unblocked_attacker_untap_remove_from_combat(triggered) {
        if let Some(trigger) = triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::AttacksAndIsntBlockedTrigger>(
        ) {
            return format!(
                "Whenever {} attacks and isn't blocked this combat",
                with_indefinite_article(&trigger.filter.description())
            );
        }
        if triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksAndIsntBlockedTrigger>()
            .is_some()
        {
            return rewrite_source_bound_trigger_subject(
                triggered,
                "Whenever this creature attacks and isn't blocked this combat".to_string(),
                self_subject,
            );
        }
    }

    let mut trigger_surface =
        describe_this_enters_and_your_upkeep_trigger(&triggered.trigger, self_subject)
            .or_else(|| describe_this_attacks_or_dies_trigger(&triggered.trigger))
            .or_else(|| describe_this_blocks_or_becomes_blocked_by_trigger(&triggered.trigger))
            .or_else(|| describe_becomes_blocked_trigger(&triggered.trigger))
            .unwrap_or_else(|| triggered.trigger.display());
    if matches!(
        trigger_frequency,
        Some(TriggerFrequencySurface::FirstTimeThisTurn)
    ) {
        trigger_surface.push_str(" for the first time each turn");
    }
    rewrite_source_bound_trigger_subject(triggered, trigger_surface, self_subject)
}

pub(super) fn describe_this_enters_and_your_upkeep_trigger(
    trigger: &crate::triggers::Trigger,
    self_subject: &str,
) -> Option<String> {
    let or_trigger = trigger.downcast_ref::<crate::triggers::OrTrigger>()?;
    let [first, second] = or_trigger.triggers.as_slice() else {
        return None;
    };
    let enters = first
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>())?;
    let upkeep = first
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>())?;
    if enters.from != crate::triggers::zone_changes::ZonePattern::Any
        || enters.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || !enters.this_object
        || enters.player != crate::triggers::zone_changes::PlayerRelation::Any
        || enters.count_mode != crate::triggers::zone_changes::CountMode::Each
        || enters.cause_filter.is_some()
        || enters.during_turn.is_some()
        || upkeep.player != PlayerFilter::You
    {
        return None;
    }

    Some(format!(
        "When {self_subject} enters and at the beginning of your upkeep"
    ))
}

#[cfg(test)]
mod enters_and_upkeep_surface_tests {
    use super::*;

    #[test]
    fn joins_source_entry_and_your_upkeep_with_oracle_conjunction() {
        let mut enters = crate::triggers::zone_changes::ZoneChangeTrigger::default();
        enters.to = crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield);
        enters.this_object = true;
        let trigger = crate::triggers::Trigger::new(crate::triggers::OrTrigger::two(
            crate::triggers::Trigger::new(enters),
            crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::You),
        ));

        assert_eq!(
            describe_this_enters_and_your_upkeep_trigger(&trigger, "this Aura").as_deref(),
            Some("When this Aura enters and at the beginning of your upkeep")
        );
    }
}

fn rewrite_source_bound_trigger_subject(
    triggered: &crate::ability::TriggeredAbility,
    surface: String,
    self_subject: &str,
) -> String {
    if self_subject.eq_ignore_ascii_case("this creature") {
        return surface;
    }

    let trigger = &triggered.trigger;
    let source_bound_combat = trigger
        .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
        .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksPlayerWhoControlsAtLeastTrigger>()
            .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksAndIsntBlockedTrigger>()
            .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksWhileSaddledTrigger>()
            .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksPlayerWithMostLifeTrigger>()
            .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksWithGreaterPowerTrigger>()
            .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksWithNOthersTrigger>()
            .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisDealsCombatDamageToPlayerTrigger>()
            .is_some();
    let implicit_source_zone_change = trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        .is_some_and(|zone_change| {
            zone_change.this_object && zone_change.this_object_surface.is_none()
        });
    if !source_bound_combat && !implicit_source_zone_change {
        return surface;
    }

    for prefix in ["Whenever this creature", "When this creature"] {
        if surface.starts_with(prefix) {
            return surface.replacen("this creature", self_subject, 1);
        }
    }
    surface
}

pub(super) fn triggered_unblocked_attacker_untap_remove_from_combat(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    segment.self_replacements.is_empty()
        && describe_untap_triggering_then_remove_from_combat(&segment.default_effects).is_some()
}

/// "Whenever this creature blocks or becomes blocked by a creature" — an
/// OrTrigger pairing this-blocks-object with this-becomes-blocked-by-object
/// over the same filter compacts to the oracle's joint surface.
pub(super) fn describe_this_blocks_or_becomes_blocked_by_trigger(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let or_trigger = trigger.downcast_ref::<crate::triggers::OrTrigger>()?;
    let [first, second] = or_trigger.triggers.as_slice() else {
        return None;
    };
    let blocks = first
        .downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>())?;
    let blocked_by = first
        .downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>())?;
    if blocks.blocked_filter != blocked_by.blocker_filter {
        return None;
    }
    let filter_text = if blocks.blocked_filter.union_is_one_or_more() {
        let description = blocks.blocked_filter.description();
        let plural = if blocks.blocked_filter.card_types.len() <= 1 {
            blocks
                .blocked_filter
                .card_types
                .iter()
                .chain(blocks.blocked_filter.all_card_types.iter())
                .find_map(|card_type| {
                    description
                        .strip_suffix(card_type.name())
                        .map(|prefix| format!("{prefix}{}", card_type.plural_name()))
                })
                .unwrap_or_else(|| pluralize_noun_phrase(&description))
        } else {
            pluralize_noun_phrase(&description)
        };
        format!("one or more {plural}")
    } else {
        with_indefinite_article(&blocks.blocked_filter.description())
    };
    Some(format!(
        "Whenever this creature blocks or becomes blocked by {filter_text}"
    ))
}

pub(super) fn describe_becomes_blocked_trigger(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let trigger = trigger.downcast_ref::<crate::triggers::BecomesBlockedTrigger>()?;
    let description = trigger.filter.description();
    if trigger.filter.has_relative_attachment_state_surface()
        && let Some(attachment) = trigger.filter.tagged_constraints.iter().find_map(|constraint| {
            (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
                .then_some(constraint.tag.as_str())
                .filter(|tag| matches!(*tag, "enchanted" | "equipped"))
        })
    {
        let without_article = description
            .strip_prefix("a ")
            .or_else(|| description.strip_prefix("an "))
            .unwrap_or(&description);
        if let Some(base) = without_article.strip_prefix(&format!("{attachment} ")) {
            return Some(format!(
                "Whenever {} that's {attachment} becomes blocked",
                with_indefinite_article(base)
            ));
        }
    }
    let subject = if description.starts_with("enchanted ")
        || description.starts_with("equipped ")
        || description.starts_with("fortified ")
    {
        description
    } else {
        with_indefinite_article(&description)
    };
    Some(format!("Whenever {subject} becomes blocked"))
}

pub(super) fn describe_this_attacks_or_dies_trigger(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let or_trigger = trigger.downcast_ref::<crate::triggers::OrTrigger>()?;
    if or_trigger.triggers.len() != 2 {
        return None;
    }
    let has_attacks = or_trigger.triggers.iter().any(trigger_is_this_attacks);
    let has_dies = or_trigger.triggers.iter().any(|trigger| {
        trigger
            .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
            .is_some_and(|zone_change| {
                zone_change.this_object
                    && zone_change.from
                        == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
                    && zone_change.to
                        == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
                    && zone_change.player == crate::triggers::zone_changes::PlayerRelation::Any
            })
    });

    (has_attacks && has_dies).then(|| "Whenever this creature attacks or dies".to_string())
}

/// A combat trigger can retain the other combatant under a tag and schedule
/// destruction of objects attached to it at end of combat.  Keep that tagged
/// attachment anchor explicit in the delayed surface instead of letting the
/// generic filter renderer produce an unbound "attached to it" pronoun.
fn describe_delayed_destroy_attached_to_triggering_creature(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let or_trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::OrTrigger>()?;
    let [first, second] = or_trigger.triggers.as_slice() else {
        return None;
    };
    let blocks = first
        .downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>())?;
    let blocked_by = first
        .downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>())?;
    if blocks.blocked_filter != blocked_by.blocker_filter
        || !blocks
            .blocked_filter
            .card_types
            .contains(&CardType::Creature)
    {
        return None;
    }

    let [tag_effect, schedule_effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if !schedule.one_shot
        || schedule.start_next_turn
        || schedule.until_end_of_turn
        || schedule
            .trigger
            .downcast_ref::<crate::triggers::EndOfCombatTrigger>()
            .is_none()
    {
        return None;
    }
    let [destroy_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let destroy = unwrap_basic_tag_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(attached_filter) = destroy.spec.base() else {
        return None;
    };
    let matching_anchors = attached_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag == tag.tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
        .count();
    if matching_anchors != 1
        || attached_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag != tag.tag
                || constraint.relation
                    != crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
    {
        return None;
    }

    let mut attachment_kind = attached_filter.clone();
    attachment_kind.tagged_constraints.clear();
    attachment_kind.zone = None;
    let attachment_description = attachment_kind.description();
    let attachment = strip_indefinite_article(&attachment_description).trim();
    if attachment.is_empty() || attachment == "permanent" {
        return None;
    }
    Some(format!(
        "destroy all {} attached to that creature at end of combat",
        pluralize_noun_phrase(attachment)
    ))
}

pub(super) fn describe_triggered_resolution_text(
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
    rewrite_it_deals: bool,
) -> Option<String> {
    if let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let [lose_effect, draw_effect] = segment.default_effects.as_slice()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && matches!(
            lose.player.unhinted(),
            ChooseSpec::Player(PlayerFilter::You)
        )
        && draw.player == PlayerFilter::You
        && draw
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
    {
        return Some(format!(
            "you lose {} life and you draw {}",
            describe_value(&lose.amount),
            describe_card_count(&draw.count)
        ));
    }

    if let Some(text) = describe_return_triggering_object_then_remove_all_abilities(triggered) {
        return Some(text);
    }

    if let Some(text) = describe_exile_triggering_object_then_return_source(triggered, subject) {
        return Some(text);
    }

    if let Some(text) = describe_delayed_destroy_attached_to_triggering_creature(triggered) {
        return Some(text);
    }

    if triggered_deals_same_damage_to_each_other_opponent(triggered) {
        return Some("it deals that much damage to each other opponent".to_string());
    }

    if let Some(text) = describe_this_attacks_target_creature_blocks_it(triggered) {
        return Some(text);
    }

    if let Some(keyword) = triggered
        .trigger
        .downcast_ref::<crate::triggers::KeywordActionTrigger>()
        && keyword.action == crate::events::KeywordActionKind::Discover
        && keyword.player == PlayerFilter::You
        && let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let [effect] = segment.default_effects.as_slice()
        && let Some(discover) = effect.downcast_ref::<crate::effects::DiscoverEffect>()
        && discover.player == PlayerFilter::You
        && matches!(
            discover.count,
            crate::effect::Value::EventValue(EventValueSpec::Amount)
        )
    {
        return Some("discover again for the same value".to_string());
    }

    if triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
        .is_some()
        && let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let [effect] = segment.default_effects.as_slice()
        && let Some(surveil) = effect.downcast_ref::<crate::effects::SurveilEffect>()
        && surveil.player == PlayerFilter::You
        && value_prefers_where_x(&surveil.count)
        && matches!(
            surveil.count.unhinted(),
            Value::EventValue(EventValueSpec::Amount)
        )
    {
        return Some(
            "surveil X, where X is the amount of damage it dealt to that player".to_string(),
        );
    }

    if triggered.effects.is_empty() {
        return None;
    }

    let mut effects = super::ast_render::describe_resolution_program(&triggered.effects);
    if effects.contains("Whenever that creature ") {
        effects = effects.replace(", draw ", ", you draw ");
    }
    effects = rewrite_damaged_player_reference_for_damage_trigger(triggered, effects);
    effects = rewrite_triggering_artifact_reference_for_tap_or_ability_trigger(triggered, effects);
    effects = rewrite_source_no_counter_resolution_surface(effects, subject);
    effects = rewrite_damage_phrases_for_permanent_abilities(&effects, subject, rewrite_it_deals);
    effects = normalize_ability_self_reference_surface(&effects, subject);
    effects = split_sacrifice_then_lose_life_resolution(effects);
    Some(effects)
}

/// Preserve the conjunction in a linked death cleanup: the trigger tags its
/// object, exiles that exact object, then returns the ability source. The tag
/// is runtime plumbing and should not force two independent sentences.
fn describe_exile_triggering_object_then_return_source(
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
) -> Option<String> {
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, move_effect, return_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let return_to_hand = return_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if move_to_zone.zone != Zone::Exile
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(key) if key == &tag.tag)
        || !matches!(return_to_hand.spec.base(), ChooseSpec::Source)
    {
        return None;
    }

    Some(format!("exile it and return {subject} to its owner's hand"))
}

pub(super) fn describe_this_attacks_target_creature_blocks_it(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !trigger_is_this_attacks(&triggered.trigger) {
        return None;
    }

    let mut triggering_tag = None;
    let mut target_tag = None;
    let mut must_block = None;
    for effect in triggered.effects.flattened_default_effects() {
        if let Some(tag_triggering) =
            effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        {
            triggering_tag = Some(tag_triggering.tag.clone());
            continue;
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(target_only) = tagged
                .effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
            && matches!(
                &target_only.target,
                ChooseSpec::Target(inner)
                    if matches!(
                        inner.as_ref(),
                        ChooseSpec::Object(filter)
                            if filter.card_types == [CardType::Creature]
                    )
            )
        {
            target_tag = Some(tagged.tag.clone());
            continue;
        }
        if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>()
            && cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
                &cant.restriction
        {
            must_block = Some((blockers, attacker));
        }
    }

    let triggering_tag = triggering_tag.as_ref()?;
    let target_tag = target_tag.as_ref()?;
    let (blockers, attacker) = must_block?;
    if object_filter_has_tagged_constraint(blockers, target_tag)
        && object_filter_has_tagged_constraint(attacker, triggering_tag)
    {
        return Some("target creature blocks it this turn if able".to_string());
    }
    None
}

pub(super) fn split_sacrifice_then_lose_life_resolution(text: String) -> String {
    let Some((sacrifice, life_loss)) = text.split_once(", then lose ") else {
        return text;
    };
    if !sacrifice.starts_with("Sacrifice ") || life_loss.trim().is_empty() {
        return text;
    }
    format!("{sacrifice}. You lose {}", life_loss.trim())
}

pub(super) fn rewrite_triggering_artifact_reference_for_tap_or_ability_trigger(
    triggered: &crate::ability::TriggeredAbility,
    text: String,
) -> String {
    if triggered.trigger.display()
        != "Whenever an artifact becomes tapped or a player activates an artifact's ability without {T} in its activation cost"
    {
        return text;
    }
    text.replace("that object's controller", "that artifact's controller")
}

pub(super) fn rewrite_source_no_counter_resolution_surface(
    mut text: String,
    subject: &str,
) -> String {
    if subject != "this creature" && subject != "this enchantment" {
        return text;
    }

    for needle in ["if there are no ", "If there are no "] {
        let mut search_from = 0;
        while let Some(relative_start) = text[search_from..].find(needle) {
            let start = search_from + relative_start;
            let counter_start = start + needle.len();
            let Some(relative_end) = text[counter_start..].find(" counters on it") else {
                break;
            };
            let counter_end = counter_start + relative_end;
            let counter_text = &text[counter_start..counter_end];
            if counter_text.starts_with("more ") {
                search_from = counter_end;
                continue;
            }
            let condition_prefix = if needle.starts_with('I') { "If" } else { "if" };
            let replacement = if subject == "this creature" {
                format!(
                    "{condition_prefix} this creature doesn't have {} on it",
                    with_indefinite_article(&format!("{counter_text} counter"))
                )
            } else {
                format!("{condition_prefix} this enchantment has no {counter_text} counters on it")
            };
            let replace_end = counter_end + " counters on it".len();
            text.replace_range(start..replace_end, &replacement);
            search_from = start + replacement.len();
        }
    }

    text
}

pub(super) fn describe_return_triggering_object_then_remove_all_abilities(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !triggered.choices.is_empty() || !trigger_is_this_dies(&triggered.trigger) {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }

    let [tag_triggering, return_effect, remove_abilities] = segment.default_effects.as_slice()
    else {
        return None;
    };
    let tag_triggering =
        tag_triggering.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    if tag_triggering.tag.as_str() != "triggering" {
        return None;
    }

    let return_effect = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if return_effect.zone != Zone::Battlefield
        || return_effect.to_top
        || return_effect.enters_tapped
        || return_effect.enters_attacking
        || return_effect.enters_face_down
        || return_effect.transfer_exiled_with_source_links
        || return_effect.battlefield_controller != ironsmith_core::BattlefieldController::Owner
        || !matches!(
            return_effect.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
        )
    {
        return None;
    }

    let remove_abilities =
        remove_abilities.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if remove_abilities.until != Until::Forever
        || !matches!(
            remove_abilities.target,
            crate::continuous::EffectTarget::Source
        )
        || !matches!(
            remove_abilities.target_spec.as_ref(),
            Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "triggering"
        )
        || remove_abilities.modification.is_some()
        || !remove_abilities.additional_modifications.is_empty()
        || !matches!(
            remove_abilities.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::RemoveAllAbilities]
        )
        || remove_abilities.condition.is_some()
    {
        return None;
    }

    Some(
        "return it to the battlefield under its owner's control and it loses all abilities"
            .to_string(),
    )
}

pub(super) fn triggered_deals_same_damage_to_each_other_opponent(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    if !triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
        .is_some_and(|trigger| trigger.player == PlayerFilter::Opponent)
    {
        return false;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return false;
    };
    let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() else {
        return false;
    };
    if !matches!(
        &for_players.filter,
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Opponent)
                && matches!(excluded.as_ref(), PlayerFilter::DamagedPlayer)
    ) {
        return false;
    }
    let [inner] = for_players.effects.as_slice() else {
        return false;
    };
    let Some(deal_damage) = inner.downcast_ref::<crate::effects::DealDamageEffect>() else {
        return false;
    };
    matches!(
        deal_damage.amount,
        Value::EventValue(EventValueSpec::Amount)
    ) && matches!(
        deal_damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) && !deal_damage.source_is_combat
}

pub(super) fn rewrite_damaged_player_reference_for_damage_trigger(
    triggered: &crate::ability::TriggeredAbility,
    effects: String,
) -> String {
    let references_damaged_player = triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
        .is_some()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::DealsDamageTrigger>()
            .is_some_and(|trigger| trigger.damaged_player.is_some());

    if !references_damaged_player {
        return effects;
    }
    effects
        .replace("the damaged player's", "their")
        .replace("The damaged player's", "Their")
        .replace("the damaged player", "that player")
        .replace("The damaged player", "That player")
}

pub(super) fn describe_unique_creature_control_leader_upkeep_control_change(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let upkeep = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    if upkeep.player != PlayerFilter::You
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let Condition::PlayerControlsMost { player, filter } = triggered.intervening_if.as_ref()?
    else {
        return None;
    };
    if *player != PlayerFilter::Any || filter.card_types != [CardType::Creature] {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let control = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if control.target != crate::continuous::EffectTarget::Source
        || control.until != Until::Forever
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || control.condition.is_some()
        || !matches!(
            control.runtime_modifications.as_slice(),
            [
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    PlayerFilter::IteratedPlayer
                )
            ]
        )
    {
        return None;
    }

    Some(
        "At the beginning of your upkeep, if a player controls more creatures than each other player, the player who controls the most creatures gains control of this creature"
            .to_string(),
    )
}

pub(super) fn is_oath_of_ghouls_player(player: &PlayerFilter) -> bool {
    matches!(player, PlayerFilter::Active | PlayerFilter::IteratedPlayer)
}

pub(super) fn describe_oath_of_ghouls_triggered_ability(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let upkeep = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    if upkeep.player != PlayerFilter::Any
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let Condition::AnOpponentHasFewerThanPlayer { player, filter } =
        triggered.intervening_if.as_ref()?
    else {
        return None;
    };
    if !is_oath_of_ghouls_player(player) || !oath_of_ghouls_creature_graveyard_filter(filter, None)
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [may_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let decider = may.decider.as_ref()?;
    if !is_oath_of_ghouls_player(decider) {
        return None;
    }
    let [return_effect] = may.effects.as_slice() else {
        return None;
    };
    let return_from_gy =
        return_effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_from_gy.random || !exact_count(&return_from_gy.target.count(), 1) {
        return None;
    }
    let ChooseSpec::Object(return_filter) = return_from_gy.target.base() else {
        return None;
    };
    if !oath_of_ghouls_creature_graveyard_filter(return_filter, Some(decider)) {
        return None;
    }

    Some(
        "At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand"
            .to_string(),
    )
}

pub(super) fn describe_flurry_copy_exile_suspend_triggered_ability(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !matches!(
        triggered.presentation_label.as_ref(),
        Some(PresentationLabel::AbilityWord(label)) if label.eq_ignore_ascii_case("Flurry")
    ) || triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
    {
        return None;
    }
    let spell_cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    if spell_cast.caster != PlayerFilter::You
        || spell_cast.during_turn.is_some()
        || spell_cast.min_spells_this_turn.is_some()
        || spell_cast.exact_spells_this_turn != Some(2)
        || spell_cast.from_not_hand
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [
        tag_effect,
        copy_effect,
        move_effect,
        put_effect,
        conditional_effect,
    ] = segment.default_effects.as_slice()
    else {
        return None;
    };
    let tag_triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let tag = &tag_triggering.tag;

    let tagged_copy = copy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged_copy.tag.as_str() != "__copied_stack_object__" {
        return None;
    }
    let with_id = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy.copier != PlayerFilter::You
        || copy.count != Value::Fixed(1)
        || !copy.removed_supertypes.is_empty()
        || !matches!(&copy.target, ChooseSpec::Tagged(found) if found == tag)
    {
        return None;
    }

    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Exile
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(found) if found == tag)
    {
        return None;
    }

    let put = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != CounterType::Time
        || put.amount != Value::Fixed(4)
        || put.target_count.is_some()
        || put.distributed
        || !matches!(&put.target, ChooseSpec::Tagged(found) if found == tag)
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !condition_is_tagged_object_without_suspend(&conditional.condition, tag)
    {
        return None;
    }
    let apply = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_grants_suspend_to_tag(apply, tag) {
        return None;
    }

    Some(
        "Flurry — Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend"
            .to_string(),
    )
}

pub(super) fn describe_tap_lands_sharing_mana_types_with_triggering_land(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::TapForManaTrigger>()?;
    if trigger.player != PlayerFilter::Any
        || trigger.filter.zone != Some(Zone::Battlefield)
        || trigger.filter.controller != Some(PlayerFilter::Opponent)
        || trigger.filter.card_types.as_slice() != [CardType::Land]
    {
        return None;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tap = effect.downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::All(filter) = tap.target.base() else {
        return None;
    };
    if filter.zone != Some(Zone::Battlefield)
        || filter.controller != Some(PlayerFilter::IteratedPlayer)
        || filter.card_types.as_slice() != [CardType::Land]
    {
        return None;
    }

    Some(
        "Whenever a land an opponent controls is tapped for mana, tap all lands that player controls that could produce any type of mana that land could produce"
            .to_string(),
    )
}

pub(super) fn describe_tap_for_mana_actual_produced_type_bonus(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::TapForManaTrigger>()?;
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let add = segment
        .default_effects
        .first()?
        .downcast_ref::<crate::effects::AddManaOfLandProducedTypesEffect>()?;
    if add.amount != Value::Fixed(1)
        || add.player != PlayerFilter::IteratedPlayer
        || add.land_filter.card_types.as_slice() != [CardType::Land]
        || !add.allow_colorless
        || add.same_type
        || add.mana_type_source != crate::effects::ManaTypeSource::TriggeringEventProduced
    {
        return None;
    }

    let resolution = lowercase_first(&super::ast_render::describe_resolution_program(
        &triggered.effects,
    ));
    if !resolution.starts_with("that player adds one mana of any type that land produced") {
        return None;
    }
    Some(format!(
        "{}, {resolution}",
        crate::triggers::TriggerMatcher::display(trigger)
    ))
}

fn additional_mana_effect_player(effect: &Effect) -> Option<&PlayerFilter> {
    if let Some(add) = effect.downcast_ref::<crate::effects::AddManaEffect>() {
        return Some(&add.player);
    }
    if let Some(add) = effect.downcast_ref::<crate::effects::AddScaledManaEffect>() {
        return Some(&add.player);
    }
    if let Some(add) = effect.downcast_ref::<crate::effects::AddManaOfAnyColorEffect>() {
        return Some(&add.player);
    }
    if let Some(add) = effect.downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>() {
        return Some(&add.player);
    }
    effect
        .downcast_ref::<crate::effects::mana::AddManaOfChosenColorEffect>()
        .map(|add| &add.player)
}

fn describe_additional_mana_amount(effect: &Effect, player: &PlayerFilter) -> Option<String> {
    let rendered = describe_effect(effect);
    let destination = describe_add_mana_destination_suffix(player);
    if let Some(amount) = rendered
        .strip_prefix("Add ")
        .and_then(|body| body.strip_suffix(&destination))
    {
        return Some(amount.to_string());
    }

    let subject_prefix = format!("{} adds ", describe_player_filter(player));
    rendered
        .strip_prefix(&subject_prefix)
        .map(ToString::to_string)
}

pub(super) fn describe_tap_for_mana_additional_mana_trigger(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }

    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::TapForManaTrigger>()?;
    if trigger.player != PlayerFilter::Any {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, add_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let player = additional_mana_effect_player(add_effect)?;
    if player != &PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag.tag.clone())) {
        return None;
    }
    let amount = describe_additional_mana_amount(add_effect, player)?;

    let active_surface = crate::triggers::TriggerMatcher::display(trigger);
    let object = active_surface
        .strip_prefix("Whenever a player taps ")?
        .strip_suffix(" for mana")?;
    let object = object
        .strip_prefix("a enchanted ")
        .or_else(|| object.strip_prefix("an enchanted "))
        .map(|rest| format!("enchanted {rest}"))
        .unwrap_or_else(|| object.to_string());

    Some(format!(
        "Whenever {object} is tapped for mana, its controller adds an additional {amount}"
    ))
}

pub(super) fn describe_triggered_inline_ability(
    triggered: &crate::ability::TriggeredAbility,
    self_subject: &str,
) -> String {
    if triggered.intervening_if.is_none()
        && triggered.presentation_label.is_none()
        && matches!(
            triggered.effects.flattened_default_effects(),
            [effect] if effect
                .downcast_ref::<crate::effects::HauntExileEffect>()
                .is_some()
        )
    {
        return "Haunt".to_string();
    }
    if let Some(rendered) = describe_structural_hideaway_keyword(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_backup_keyword(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_unique_creature_control_leader_upkeep_control_change(triggered)
    {
        return rendered;
    }
    if let Some(rendered) = describe_oath_of_ghouls_triggered_ability(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_flurry_copy_exile_suspend_triggered_ability(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_tap_for_mana_actual_produced_type_bonus(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_tap_lands_sharing_mana_types_with_triggering_land(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_tap_for_mana_additional_mana_trigger(triggered) {
        return rendered;
    }

    let (intervening_condition, trigger_frequency) = triggered
        .intervening_if
        .as_ref()
        .map(split_trigger_intervening_if)
        .unwrap_or((None, None));
    let mut intervening_condition =
        retain_state_trigger_residual_condition(&triggered.trigger, intervening_condition);
    intervening_condition = intervening_condition
        .and_then(|condition| remove_presentation_label_chosen_option(&condition, triggered));
    let mut line =
        describe_trigger_surface_with_frequency(triggered, trigger_frequency, self_subject);
    if triggered_deals_same_damage_to_each_other_opponent(triggered) {
        line = line.replace("combat damage to a player", "combat damage to an opponent");
    }
    if matches!(intervening_condition, Some(Condition::YourTurn))
        && line.to_ascii_lowercase().ends_with("becomes tapped")
    {
        line.push_str(" during your turn");
        intervening_condition = None;
    }
    if let Some(condition) = intervening_condition {
        line.push_str(", if ");
        line.push_str(&describe_trigger_intervening_condition(
            &condition,
            triggered,
            Some(self_subject),
        ));
    }

    let mut clauses = Vec::new();
    if !triggered.choices.is_empty()
        && !(!triggered.effects.is_empty() && choices_are_simple_targets(&triggered.choices))
    {
        clauses.push(format!(
            "choose {}",
            triggered
                .choices
                .iter()
                .map(describe_choose_spec)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(effects) = describe_triggered_resolution_text(triggered, self_subject, true) {
        clauses.push(effects);
    }

    if !clauses.is_empty() {
        if clauses.len() == 1 {
            let only = clauses[0].trim_start();
            if let Some(rest) = only.strip_prefix("If ") {
                line.push_str(", if ");
                line.push_str(rest.trim_start());
            } else if let Some(rest) = only.strip_prefix("if ") {
                line.push_str(", if ");
                line.push_str(rest.trim_start());
            } else if only.contains(" if ") && !only.contains(". ") && !only.contains(": ") {
                line.push_str(", ");
                line.push_str(&lowercase_first(only));
            } else if triggered.presentation_label.is_some() {
                line.push_str(", ");
                line.push_str(&lowercase_first(only));
            } else if triggered.trigger.saga_chapters().is_some() {
                line.push_str(" — ");
                line.push_str(only);
            } else {
                line.push_str(": ");
                line.push_str(only);
            }
        } else {
            line.push_str(": ");
            line.push_str(&clauses.join(": "));
        }
    }

    match trigger_frequency {
        Some(TriggerFrequencySurface::AbilityMaxTimesEachTurn(max)) => {
            if max == 1 {
                line.push_str(". This ability triggers only once each turn");
            } else if max == 2 {
                line.push_str(". This ability triggers only twice each turn");
            } else {
                line.push_str(". This ability triggers only ");
                line.push_str(&max.to_string());
                line.push_str(" times each turn");
            }
        }
        Some(TriggerFrequencySurface::DoThisMaxTimesEachTurn(max)) => {
            if max == 1 {
                line.push_str(". Do this only once each turn");
            } else if max == 2 {
                line.push_str(". Do this only twice each turn");
            } else {
                line.push_str(". Do this only ");
                line.push_str(&max.to_string());
                line.push_str(" times each turn");
            }
        }
        _ => {}
    }

    line = normalize_redundant_short_name_etb_surface(line, triggered, self_subject);
    line = normalize_modal_named_source_etb_surface(line, triggered, self_subject);
    line = normalize_spellcast_trigger_mana_value_surface(triggered, line);
    if triggered_deals_same_damage_to_each_other_opponent(triggered) {
        line = line.replace("combat damage to a player", "combat damage to an opponent");
    }
    if triggered_has_you_difference_draw(triggered) {
        line = line.replace(
            "you draw cards equal to the difference",
            "draw cards equal to the difference",
        );
    }
    if triggered.presentation_label.is_none()
        && line.starts_with("Whenever you cast a spell that targets this creature")
    {
        line = format!("Heroic — {line}");
    }
    apply_triggered_presentation_label(triggered, line)
}

pub(super) fn describe_trigger_intervening_condition(
    condition: &Condition,
    triggered: &crate::ability::TriggeredAbility,
    self_subject: Option<&str>,
) -> String {
    if matches!(condition, Condition::ThisSpellWasKicked)
        && trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return "it was kicked".to_string();
    }
    if let Condition::SourceHasNoCounter(counter_type) = condition {
        if let Some(subject) = self_subject {
            if subject == "this creature" {
                return format!(
                    "this creature doesn't have {} on it",
                    with_indefinite_article(&format!("{} counter", counter_type.description()))
                );
            }
            if subject == "this enchantment" {
                return format!(
                    "this enchantment has no {} counters on it",
                    counter_type.description()
                );
            }
        }
    }
    if let Condition::TriggeringObjectHadCounters {
        counter_type,
        min_count,
    } = condition
        && trigger_is_this_dies(&triggered.trigger)
    {
        return format!(
            "it had {} on it",
            triggering_object_counter_phrase(*counter_type, *min_count)
        );
    }
    if let Condition::PlayerCardsInHandOrFewer { player, count } = condition
        && *player == PlayerFilter::You
        && triggered_has_you_difference_draw(triggered)
    {
        let threshold = count + 1;
        if threshold > 0 {
            let count_text = number_word(threshold).unwrap_or_else(|| threshold.to_string());
            return format!("you have fewer than {count_text} cards in hand");
        }
    }
    if matches!(condition, Condition::SourceIsInZone(Zone::Graveyard))
        && let Some(subject) = source_return_from_graveyard_subject(triggered)
    {
        return format!("{subject} is in your graveyard");
    }
    describe_condition(condition)
}

pub(super) fn triggering_object_counter_phrase(
    counter_type: CounterType,
    min_count: u32,
) -> String {
    let counter = counter_type.description();
    if min_count == 1 {
        return with_indefinite_article(&format!("{counter} counter"));
    }
    format!("{min_count} or more {counter} counters")
}

pub(super) fn triggered_has_you_difference_draw(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .any(|effect| {
            effect
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some_and(|draw| {
                    draw.player == PlayerFilter::You
                        && draw.count.has_surface_hint(ValueSurfaceHint::Difference)
                })
        })
}

pub(super) fn source_return_from_graveyard_subject(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .find_map(source_return_from_graveyard_subject_in_effect)
}

pub(super) fn source_return_from_graveyard_subject_in_effect(effect: &Effect) -> Option<String> {
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        && matches!(return_to_battlefield.target.unhinted(), ChooseSpec::Source)
    {
        return Some(describe_choose_spec(&return_to_battlefield.target));
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return if_effect
            .then
            .iter()
            .chain(if_effect.else_.iter())
            .find_map(source_return_from_graveyard_subject_in_effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return source_return_from_graveyard_subject_in_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return source_return_from_graveyard_subject_in_effect(&with_id.effect);
    }
    None
}

pub(super) fn describe_delayed_coin_flip_result(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot || schedule.start_next_turn || schedule.until_end_of_turn {
        return None;
    }
    let trigger_text = schedule.trigger.display().to_ascii_lowercase();
    if !trigger_text.contains("beginning of") || !trigger_text.contains("end step") {
        return None;
    }

    let effects: &[Effect] = &schedule.effects;
    let [flip_effect, branch_effect] = effects else {
        return None;
    };
    let flip_with_id = flip_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let flip_coin = flip_with_id
        .effect
        .downcast_ref::<crate::effects::FlipCoinEffect>()?;
    if flip_coin.player != PlayerFilter::You {
        return None;
    }

    let if_effect = branch_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != flip_with_id.id
        || !matches!(if_effect.predicate, EffectPredicate::DidNotHappen)
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [choose_effect, sacrifice_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    let sacrifice_text = describe_choose_then_sacrifice(choose, sacrifice)?;
    if !sacrifice_text.starts_with("you sacrifice ") {
        return None;
    }
    if !choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str().starts_with("targeted_")
    }) {
        return None;
    }

    let object = if choose.filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if choose.filter.card_types.contains(&CardType::Artifact) {
        "artifact"
    } else if choose.filter.card_types.contains(&CardType::Enchantment) {
        "enchantment"
    } else if choose.filter.card_types.contains(&CardType::Planeswalker) {
        "planeswalker"
    } else if choose.filter.card_types.contains(&CardType::Land) {
        "land"
    } else {
        "permanent"
    };

    Some(format!(
        "Flip a coin at the beginning of the next end step. If you lose the flip, sacrifice that {object}"
    ))
}

pub(super) fn describe_delayed_each_player_discard_hand_return_exiled(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot || schedule.start_next_turn || schedule.until_end_of_turn {
        return None;
    }
    let trigger_text = schedule.trigger.display().to_ascii_lowercase();
    if !trigger_text.contains("beginning of") || !trigger_text.contains("end step") {
        return None;
    }

    let effects = schedule.effects.flattened_default_effects();
    let Some(for_players_effect) = effects.first() else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let (discard_effect, return_effect) = match (for_players.effects.as_slice(), effects) {
        ([discard_effect, return_effect], [_]) => (discard_effect, return_effect),
        ([discard_effect], [_, return_effect]) => (discard_effect, return_effect),
        _ => return None,
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardHandEffect>()?;
    if discard.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let return_spec = if let Some(return_to_hand) =
        return_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        &return_to_hand.spec
    } else {
        let move_to_hand = return_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        if move_to_hand.zone != Zone::Hand {
            return None;
        }
        &move_to_hand.target
    };
    if !choose_spec_is_all_exiled_cards_tagged_this_way(return_spec) {
        return None;
    }

    Some(
        "At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way"
            .to_string(),
    )
}

pub(super) fn choose_spec_is_all_exiled_cards_tagged_this_way(spec: &ChooseSpec) -> bool {
    match spec.base() {
        ChooseSpec::All(filter) if filter.zone == Some(Zone::Exile) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && tag_is_exiled_this_way(&constraint.tag)
            })
        }
        ChooseSpec::Tagged(tag) => tag_is_exiled_this_way(tag),
        _ => false,
    }
}

pub(super) fn tag_is_exiled_this_way(tag: &TagKey) -> bool {
    tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        || tag.as_str().starts_with("exiled_")
        || crate::cards::is_sentence_helper_tag(tag.as_str(), "exiled")
}

pub(super) fn describe_next_spell_delayed_trigger(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
    additional: bool,
) -> Option<String> {
    if !schedule.one_shot || !schedule.until_end_of_turn || schedule.start_next_turn {
        return None;
    }
    let trigger_display =
        cleanup_decompiled_text(schedule.trigger.display().trim().trim_end_matches('.'));
    let trigger_action = trigger_display
        .strip_prefix("Whenever you ")
        .or_else(|| trigger_display.strip_prefix("When you "))?
        .trim();
    if !trigger_action.starts_with("cast ") && !trigger_action.contains(" activate ") {
        return None;
    }
    let trigger_action = trigger_action
        .strip_suffix(" this turn")
        .unwrap_or(trigger_action)
        .to_string();
    let trigger_can_activate_ability = trigger_action.contains("activate ");
    let mut delayed_text = lowercase_first(&describe_effect_list(&schedule.effects));
    if let Some(rest) = delayed_text.strip_prefix("copy it") {
        let copied_object = if trigger_can_activate_ability {
            "spell or ability"
        } else {
            "spell"
        };
        delayed_text = format!("copy that {copied_object}{rest}");
    }
    let copying_twice = delayed_text.contains(" 2 time");
    delayed_text = delayed_text
        .replace(
            "copy that spell or ability 2 time(s)",
            "copy that spell or ability twice",
        )
        .replace(
            "copy that spell or ability 2 times",
            "copy that spell or ability twice",
        )
        .replace(
            "copy that spell or ability 2 time",
            "copy that spell or ability twice",
        )
        .replace("copy that spell 2 time(s)", "copy that spell twice")
        .replace("copy that spell 2 times", "copy that spell twice")
        .replace("copy that spell 2 time", "copy that spell twice");
    if copying_twice {
        delayed_text = delayed_text
            .replace(
                ", then you may choose new targets for the copy",
                ". You may choose new targets for the copies",
            )
            .replace(
                ". You may choose new targets for the copy",
                ". You may choose new targets for the copies",
            );
    } else {
        delayed_text = delayed_text
            .replace(
                "copy that spell. You may choose new targets for the copy",
                "copy it and you may choose new targets for the copy",
            )
            .replace(
                "copy that spell or ability. You may choose new targets for the copy",
                "copy it and you may choose new targets for the copy",
            );
    }
    if additional {
        delayed_text =
            delayed_text.replacen("copy that spell", "copy that spell an additional time", 1);
    }
    Some(format!(
        "When you next {trigger_action} this turn, {delayed_text}"
    ))
}

pub(super) fn describe_play_card_this_way_delayed_trigger(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot || !schedule.until_end_of_turn || schedule.start_next_turn {
        return None;
    }
    let either = schedule
        .trigger
        .downcast_ref::<crate::triggers::OrTrigger>()?;
    let [first, second] = either.triggers.as_slice() else {
        return None;
    };
    let (spell, land) = if let (Some(spell), Some(land)) = (
        first.downcast_ref::<crate::triggers::SpellCastTrigger>(),
        second.downcast_ref::<crate::triggers::PlayerPlaysLandTrigger>(),
    ) {
        (spell, land)
    } else if let (Some(land), Some(spell)) = (
        first.downcast_ref::<crate::triggers::PlayerPlaysLandTrigger>(),
        second.downcast_ref::<crate::triggers::SpellCastTrigger>(),
    ) {
        (spell, land)
    } else {
        return None;
    };
    let spell_filter = spell.filter.as_ref()?;
    if spell.caster != PlayerFilter::You
        || spell.during_turn.is_some()
        || spell.min_spells_this_turn.is_some()
        || spell.exact_spells_this_turn.is_some()
        || spell.from_not_hand
        || land.player != PlayerFilter::You
        || spell_filter != &land.filter
        || !spell_filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }
    let delayed = lowercase_first(&describe_effect_list(&schedule.effects));
    Some(format!("When you play a card this way, {delayed}"))
}

pub(super) fn describe_unless_pays_lose_game_payment(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let unless_pays = effect.downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless_pays.player != PlayerFilter::You {
        return None;
    }
    let [lose_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    let lose_game = lose_effect.downcast_ref::<crate::effects::LoseTheGameEffect>()?;
    if lose_game.player != PlayerFilter::You {
        return None;
    }
    let display = describe_total_cost_payment(&unless_pays.cost);
    Some(display.strip_prefix("Pay ").unwrap_or(&display).to_string())
}

pub(super) fn normalize_modal_named_source_etb_surface(
    line: String,
    triggered: &crate::ability::TriggeredAbility,
    _subject: &str,
) -> String {
    let Some(zone_trigger) = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
    else {
        return line;
    };
    if zone_trigger.this_object
        || zone_trigger.to
            != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || zone_trigger.object_filter.subtypes.len() != 1
        || !line.contains("choose one or both")
    {
        return line;
    }
    let subtype = format!("{:?}", zone_trigger.object_filter.subtypes[0]);
    let prefix = format!("Whenever a {subtype} enters,");
    if let Some(start) = line.find(&prefix) {
        if start == 0 || line[..start].ends_with(": ") {
            let rest = &line[start + prefix.len()..];
            return format!("{}When this creature enters,{rest}", &line[..start]);
        }
    }
    line
}
