use super::*;

const COPIED_STACK_OBJECT_TAG: &str = "__copied_stack_object__";

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
        let draw = normalize_imperative_you_clause(draw.trim_end_matches('.'));
        format!(
            "{}, then {}",
            selection.trim_end_matches('.'),
            lowercase_first(&draw)
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
            // Energy is a player counter, so Oracle keeps the shared player
            // subject on a coordinated life-gain clause: "you gain ... and
            // get {E}". Dropping it turns the first finite verb into an
            // imperative while the second action still belongs to that
            // player.
            // Keep it as well when the conjunction introduces a different
            // explicit subject. Otherwise the first clause becomes an
            // imperative while the second remains finite (for example,
            // "lose 1 life and this creature endures 1").
            let changes_subject = rest.split_once(" and ").is_some_and(|(_, tail)| {
                ["this ", "that ", "it ", "they ", "each ", "all ", "target "]
                    .iter()
                    .any(|subject| tail.starts_with(subject))
            });
            if rest.contains(" and get {E}") || changes_subject {
                return text.to_string();
            }
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
                "attach ",
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
    // The parse records ", then" sequencing on the trailing counter
    // placement; keep that authored surface instead of "and".
    let then_followup = second_action
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .is_some_and(|put| {
            put.amount
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupThen)
        });
    let body = if then_followup {
        format!("{}, then {last}", clauses.join(", "))
    } else if clauses.len() == 1 {
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
            .replace(" counters on this source", " counters on it");
        let first_clause = capitalize_first(&first_clause);
        return Some(format!(
            "{first_clause} and that much damage to {}",
            describe_choose_spec(&second_damage.target)
        ));
    }

    // A paired damage instruction may define X once and use a rounded
    // fraction of that same value for its second recipient:
    // "... deals X damage to ... and half X damage, rounded up, to ...,
    // where X is ...". Runtime arithmetic represents rounded-up halves as
    // floor((X + 1) / 2); recognize that reusable shape without exposing the
    // implementation formula in compiled text.
    if let Some(first_damage) = deal_damage_effect_view(first)
        && let Some(second_damage) = deal_damage_effect_view(second)
        && first_damage.source_is_combat == second_damage.source_is_combat
        && first_damage.unpreventable == second_damage.unpreventable
        && let Value::HalfRoundedDown(rounded_inner) = second_damage.amount.unhinted()
        && let Value::Add(left, right) = rounded_inner.unhinted()
    {
        let rounded_basis = match (left.unhinted(), right.unhinted()) {
            (basis, Value::Fixed(1)) | (Value::Fixed(1), basis) => Some(basis.unhinted()),
            _ => None,
        };
        if rounded_basis == Some(first_damage.amount.unhinted()) {
            let first_clause = describe_effect(first)
                .trim()
                .trim_end_matches('.')
                .to_string();
            if let Some((first_head, where_x)) = first_clause.rsplit_once(", where X is ") {
                return Some(format!(
                    "{first_head} and half X damage, rounded up, to {}, where X is {where_x}",
                    describe_damage_target(&second_damage.target)
                ));
            }
            if let Some(where_x) = describe_where_x_basis(&first_damage.amount) {
                return Some(format!(
                    "{first_clause} and half X damage, rounded up, to {}, where X is {where_x}",
                    describe_damage_target(&second_damage.target)
                ));
            }
        }
    }

    None
}

/// Render the common enter-trigger shape where the triggering object is the
/// source of two or more simultaneous damage instructions. Lowering keeps an
/// invisible triggering-object tag followed by one `ExecuteWithSourceEffect`
/// per recipient; without this structural view the generic list renderer
/// repeats "that creature" and invents sentence boundaries between the
/// independently targeted damage effects.
fn triggering_reference_tag(effect: &Effect) -> Option<&TagKey> {
    if let Some(tag) = effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>() {
        return Some(&tag.tag);
    }
    if let Some(tag) = effect.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>() {
        return Some(&tag.tag);
    }
    effect
        .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
        .map(|tag| &tag.tag)
}

pub(super) fn describe_triggering_object_coordinated_damage(effects: &[Effect]) -> Option<String> {
    let [tag_effect, trailing @ ..] = effects else {
        return None;
    };
    let triggering_tag = triggering_reference_tag(tag_effect)?;
    let nested_effects = match trailing {
        [effect] => effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .filter(|sequence| sequence.surface.is_coordinated())
            .map(|sequence| sequence.effects.as_slice())
            .unwrap_or(trailing),
        _ => trailing,
    };
    let damage_effects = nested_effects
        .iter()
        .filter(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
    if damage_effects.len() < 2 {
        return None;
    }

    let mut damages = Vec::with_capacity(damage_effects.len());
    for effect in damage_effects {
        let execute = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
        if !matches!(
            execute.source.unhinted(),
            ChooseSpec::Tagged(tag) if tag == triggering_tag
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

#[cfg(test)]
#[test]
fn triggering_object_coordinated_damage_recovers_nested_target_declarations() {
    let triggering = TagKey::from("triggering");
    let source = ChooseSpec::Tagged(triggering.clone());
    let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::AnyTarget)),
        Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            source.clone(),
            Effect::deal_damage(Value::Fixed(4), ChooseSpec::AnyTarget),
        )),
        Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            source,
            Effect::deal_damage(Value::Fixed(3), ChooseSpec::SourceController),
        )),
    ]));
    let effects = vec![Effect::tag_triggering_object(triggering), coordinated];

    assert_eq!(
        describe_effect_list(&effects),
        "it deals 4 damage to any target and 3 damage to you"
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

    if let [(_, second_damage)] = views.as_slice()
        && let Value::HalfRoundedDown(rounded_inner) = second_damage.amount.unhinted()
        && let Value::Add(left, right) = rounded_inner.unhinted()
    {
        let rounded_basis = match (left.unhinted(), right.unhinted()) {
            (basis, Value::Fixed(1)) | (Value::Fixed(1), basis) => Some(basis.unhinted()),
            _ => None,
        };
        if rounded_basis == Some(first_damage.amount.unhinted()) {
            let first_clause = describe_effect(&effects[0])
                .trim()
                .trim_end_matches('.')
                .to_string();
            if let Some((first_head, where_x)) = first_clause.rsplit_once(", where X is ") {
                return Some(format!(
                    "{first_head} and half X damage, rounded up, to {}, where X is {where_x}",
                    describe_damage_target(&second_damage.target)
                ));
            }
            if let Some(where_x) = describe_where_x_basis(&first_damage.amount) {
                return Some(format!(
                    "{first_clause} and half X damage, rounded up, to {}, where X is {where_x}",
                    describe_damage_target(&second_damage.target)
                ));
            }
        }
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
            .map(|exile| &exile.spec)
            .or_else(|| {
                effect
                    .downcast_ref::<crate::effects::MoveToZoneEffect>()
                    .filter(|moved| moved.zone == Zone::Exile)
                    .map(|moved| &moved.target)
            }),
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

/// Coordinate bulk returns that share the same owner-hand destination while
/// retaining the distinct source zones carried by each filter.
fn describe_coordinated_return_to_hand_shared_destination(effects: &[Effect]) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }

    let mut subjects = Vec::with_capacity(effects.len());
    let mut shared_destination: Option<String> = None;
    for effect in effects {
        let returned = unwrap_basic_tag_wrappers(effect)
            .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
        let ChooseSpec::All(filter) = &returned.spec else {
            return None;
        };
        let destination = owner_hand_phrase_for_spec(&returned.spec).to_string();
        if shared_destination
            .as_ref()
            .is_some_and(|expected| expected != &destination)
        {
            return None;
        }
        shared_destination.get_or_insert(destination.clone());

        let rendered = describe_effect(effect);
        let suffix = format!(" to {destination}");
        let mut subject = rendered
            .trim()
            .trim_end_matches('.')
            .strip_prefix("Return ")?
            .strip_suffix(&suffix)?
            .to_string();
        match filter.zone {
            Some(Zone::Battlefield) if !subject.contains("battlefield") => {
                subject.push_str(" on the battlefield");
            }
            Some(Zone::Graveyard) if filter.owner.is_none() && !filter.single_graveyard => {
                subject = subject
                    .replace(" from a graveyard", " in graveyards")
                    .replace(" in a graveyard", " in graveyards");
            }
            _ => {}
        }
        subjects.push(subject);
    }

    Some(format!(
        "Return {} to {}",
        join_coordinated_parts(&subjects)?,
        shared_destination?
    ))
}

/// Preserve the single authored move represented by the compiler as one
/// executable bulk move per source zone.
fn describe_coordinated_owned_commanders_to_hand(effects: &[Effect]) -> Option<String> {
    let [command_effect, graveyard_effect] = effects else {
        return None;
    };
    let expected = ObjectFilter::default()
        .owned_by(PlayerFilter::You)
        .commander();
    for (effect, expected_zone) in [
        (command_effect, Zone::Command),
        (graveyard_effect, Zone::Graveyard),
    ] {
        let returned = unwrap_basic_tag_wrappers(effect)
            .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
        if returned.actor_surface.is_some()
            || returned.destination_player_surface.is_some()
            || returned.exiled_with_source_surface.is_some()
            || returned.set_quantifier_surface.is_some()
            || returned.set_reference_surface.is_some()
        {
            return None;
        }
        let ChooseSpec::All(filter) = &returned.spec else {
            return None;
        };
        let mut semantic_filter = filter.clone();
        if semantic_filter.zone != Some(expected_zone) {
            return None;
        }
        semantic_filter.zone = None;
        if semantic_filter != expected {
            return None;
        }
    }

    Some(
        "Put all commanders you own from the command zone and from your graveyard into your hand"
            .to_string(),
    )
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
        && first.animation_pt_surface == other.animation_pt_surface
        && first.animation_duration_surface == other.animation_duration_surface
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

/// Render one authored target shared by a temporary ability grant and a
/// power/toughness modification.
///
/// The compiler must declare the target once so the two continuous effects do
/// not become independently assignable.  That declaration is lowering
/// bookkeeping, not an instruction to print "choose ... then".  Rebuild the
/// older direct-target/tagged-followup view only for rendering after proving
/// that both consumers use the declaration's exact tag and duration.
pub(super) fn describe_shared_declared_target_grant_then_pt_pump(
    target_effect: &Effect,
    grant_effect: &Effect,
    pump_effect: &Effect,
) -> Option<String> {
    let target_only = unwrap_basic_tag_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let target_tag = coordinated_effect_tag(target_effect)?;
    let grant_tag = coordinated_effect_tag(grant_effect)?;
    let pump_tag = coordinated_effect_tag(pump_effect);
    let grant = coordinated_apply_continuous(grant_effect)?;
    let pump = coordinated_apply_continuous(pump_effect)?;
    let ChooseSpec::Tagged(grant_target_tag) = grant.target_spec.as_ref()?.base() else {
        return None;
    };
    let ChooseSpec::Tagged(pump_target_tag) = pump.target_spec.as_ref()?.base() else {
        return None;
    };
    if target_only.explicit_declaration
        || target_only.chooser.is_some()
        || exact_single_target_object_filter(&target_only.target).is_none()
        || grant_target_tag != target_tag
        || (pump_target_tag != target_tag && pump_target_tag != grant_tag)
        || grant.until != pump.until
        || grant.until == Until::Forever
        || grant.target != pump.target
        || !matches!(
            grant.target,
            crate::continuous::EffectTarget::Source
                | crate::continuous::EffectTarget::AllPermanents
        )
        || grant.condition.is_some()
        || pump.condition.is_some()
        || grant.source_type.is_some()
        || pump.source_type.is_some()
        || grant.source_reference_surface.is_some()
        || pump.source_reference_surface.is_some()
        || grant.set_quantifier_surface.is_some()
        || pump.set_quantifier_surface.is_some()
        || grant.type_retention_surface.is_some()
        || pump.type_retention_surface.is_some()
        || grant.animation_pt_surface.is_some()
        || pump.animation_pt_surface.is_some()
        || grant.animation_duration_surface.is_some()
        || pump.animation_duration_surface.is_some()
        || grant.lock_filter_at_resolution
        || pump.lock_filter_at_resolution
        || grant.resolve_set_pt_values_at_resolution
        || pump.resolve_set_pt_values_at_resolution
        || grant.require_creature_target
        || !pump.require_creature_target
        || !grant.runtime_modifications.is_empty()
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
        || !matches!(
            pump.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ModifyPowerToughness { .. }]
        )
    {
        return None;
    }
    if !grant
        .modification
        .iter()
        .chain(grant.additional_modifications.iter())
        .all(|modification| matches!(modification, crate::continuous::Modification::AddAbility(_)))
        || grant.modification.is_none()
    {
        return None;
    }

    let mut direct_grant = grant.clone();
    direct_grant.target_spec = Some(target_only.target.clone());
    let mut linked_pump = pump.clone();
    linked_pump.target_spec = Some(ChooseSpec::Tagged(grant_tag.clone()));
    let linked_pump = Effect::new(linked_pump);
    let linked_pump = if let Some(pump_tag) = pump_tag {
        linked_pump.tag(pump_tag.clone())
    } else {
        linked_pump
    };
    let render_only = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        Effect::new(direct_grant).tag(grant_tag.clone()),
        linked_pump,
    ]));
    let rendered = describe_effect(&render_only);
    if rendered.is_empty()
        || rendered.to_ascii_lowercase().starts_with("choose ")
        || rendered.to_ascii_lowercase().contains(", then ")
    {
        return None;
    }

    // The generic continuous-effect coordinator preserves a leading shared
    // duration and an Oxford comma. This exact matcher has already proved
    // that the grant and pump share one declared target and one trailing
    // duration, so restore the authored single-subject surface here.
    let rendered = rendered
        .strip_prefix("Until end of turn, ")
        .unwrap_or(&rendered)
        .replace(", and gets ", " and gets ");
    Some(capitalize_first(&rendered))
}

/// Render a temporary keyword grant and mana-value pump that share one
/// explicitly declared creature target.
///
/// Lowering tags the declared target, then tags the grant's affected object so
/// both the pump target and its `ManaValueOf` basis resolve through that exact
/// chain at execution time. Requiring every typed link lets us factor the
/// common target and duration without turning independently introduced
/// targets into one.
pub(super) fn describe_shared_target_trample_mana_value_pump(
    target_effect: &Effect,
    first_effect: &Effect,
    second_effect: &Effect,
) -> Option<String> {
    let target_only = unwrap_basic_tag_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let target_tag = coordinated_effect_tag(target_effect)?;
    let first = coordinated_apply_continuous(first_effect)?;
    let second = coordinated_apply_continuous(second_effect)?;
    let first_target = first.target_spec.as_ref()?;
    let second_target = second.target_spec.as_ref()?;
    let target_filter = exact_single_target_object_filter(&target_only.target)?;
    let first_tag = coordinated_effect_tag(first_effect)?;
    let ChooseSpec::Tagged(first_target_tag) = first_target.base() else {
        return None;
    };
    let ChooseSpec::Tagged(second_target_tag) = second_target.base() else {
        return None;
    };

    if target_only.explicit_declaration
        || target_only.chooser.is_some()
        || first.until != Until::EndOfTurn
        || second.until != Until::EndOfTurn
        || first_target.is_target()
        || second_target.is_target()
        || first_target_tag != target_tag
        || (second_target_tag != target_tag && second_target_tag != first_tag)
        || target_filter.card_types.as_slice() != [CardType::Creature]
        || first.condition.is_some()
        || second.condition.is_some()
        || first.source_type.is_some()
        || second.source_type.is_some()
        || first.source_reference_surface.is_some()
        || second.source_reference_surface.is_some()
        || first.set_quantifier_surface.is_some()
        || second.set_quantifier_surface.is_some()
        || first.type_retention_surface.is_some()
        || second.type_retention_surface.is_some()
        || first.animation_pt_surface.is_some()
        || second.animation_pt_surface.is_some()
        || first.animation_duration_surface.is_some()
        || second.animation_duration_surface.is_some()
        || first.lock_filter_at_resolution
        || second.lock_filter_at_resolution
        || first.resolve_set_pt_values_at_resolution
        || second.resolve_set_pt_values_at_resolution
        || !first.additional_modifications.is_empty()
        || !second.additional_modifications.is_empty()
        || !first.runtime_modifications.is_empty()
        || second.modification.is_some()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &first.modification else {
        return None;
    };
    if ability.id() != crate::static_abilities::StaticAbilityId::Trample {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = second.runtime_modifications.as_slice()
    else {
        return None;
    };
    let Value::ManaValueOf(basis) = power.unhinted() else {
        return None;
    };
    if !matches!(toughness.unhinted(), Value::Fixed(0))
        || !matches!(basis.base(), ChooseSpec::Tagged(tag) if tag == second_target_tag)
    {
        return None;
    }

    Some(format!(
        "{} gains trample and gets +X/+0 until end of turn, where X is that creature's mana value",
        capitalize_first(&describe_choose_spec(&target_only.target))
    ))
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
    let leading_prefix = format!("{}, ", capitalize_first(&duration));
    if let Some(body) = rendered.strip_prefix(&leading_prefix) {
        let (body, trailing) = body
            .rsplit_once(", where X is ")
            .map(|(body, basis)| (body, format!(", where X is {basis}")))
            .unwrap_or((body, String::new()));
        return Some((capitalize_first(body), trailing));
    }
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

    // The leading-animation parser retains the grammatical coordination, but
    // lowering currently repeats its resolved target specification on the
    // implicit "loses"/"gains" continuations. That is still one authored
    // subject when the typed leading animation is followed only by ability
    // changes to the identical target. Keep independently authored target
    // clauses on the stricter tag/reference path below.
    let lowered_leading_animation_subject = leading_duration
        && first.animation_duration_surface
            == Some(ironsmith_core::AnimationDurationSurface::Leading)
        && first.target_spec.is_some()
        && applies.iter().skip(1).all(|apply| {
            apply.target_spec.as_ref().map(ChooseSpec::unhinted)
                == first.target_spec.as_ref().map(ChooseSpec::unhinted)
                && apply.additional_modifications.is_empty()
                && apply.modification.as_ref().is_none_or(|modification| {
                    matches!(
                        modification,
                        crate::continuous::Modification::AddAbility(_)
                            | crate::continuous::Modification::AddAbilityGeneric(_)
                            | crate::continuous::Modification::RemoveAbility(_)
                            | crate::continuous::Modification::RemoveAbilityGeneric { .. }
                            | crate::continuous::Modification::SetAbilities(_)
                    )
                })
                && apply.runtime_modifications.iter().all(|modification| {
                    matches!(
                        modification,
                        crate::effects::continuous::RuntimeModification::RemoveAllAbilities
                            | crate::effects::continuous::RuntimeModification::RemoveThisAbility
                    )
                })
                && (apply.modification.is_some() || !apply.runtime_modifications.is_empty())
        });
    if !lowered_leading_animation_subject
        && effects
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

    // A leading land-animation effect already carries the authoritative
    // power/toughness, subtype, ability, type-retention, and duration
    // surfaces. Lowering may put a final same-object keyword grant in a
    // sibling effect so the runtime can resolve the reference explicitly.
    // Merge only those proven keyword continuations into a render-only clone;
    // decomposing the animation into layer clauses loses both the compact
    // "4/4 Elemental creature" surface and "It's still a land."
    if first.animation_pt_surface == Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness)
        && first.type_retention_surface.is_some()
        && applies.iter().skip(1).all(|apply| {
            apply.condition.is_none()
                && apply.additional_modifications.is_empty()
                && apply.runtime_modifications.is_empty()
                && apply.type_retention_surface.is_none()
                && apply.animation_pt_surface.is_none()
                && apply.animation_duration_surface.is_none()
                && matches!(
                    apply.modification.as_ref(),
                    Some(
                        crate::continuous::Modification::AddAbility(_)
                            | crate::continuous::Modification::AddAbilityGeneric(_)
                    )
                )
        })
    {
        let mut merged = first.clone();
        merged.additional_modifications.extend(
            applies
                .iter()
                .skip(1)
                .filter_map(|apply| apply.modification.clone()),
        );
        return describe_apply_continuous_effect(&merged);
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

/// Render a permanent coordinated continuous-effect chain from one typed
/// subject. Unlike temporary modifiers, `Until::Forever` has no printable
/// duration suffix to factor out, so rebuild every predicate directly from
/// the typed modifications while retaining the first effect's subject.
fn describe_coordinated_permanent_same_object_modifiers(effects: &[Effect]) -> Option<String> {
    let (effects, declared_player_target) =
        if let Some(target_only) = effects.first().and_then(|effect| {
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::TargetOnlyEffect>()
        }) && let ChooseSpec::Target(inner) = target_only.target.unhinted()
            && let ChooseSpec::Player(player) = inner.unhinted()
        {
            (&effects[1..], Some(player))
        } else {
            (effects, None)
        };
    if effects.len() < 2 {
        return None;
    }
    let applies = effects
        .iter()
        .map(coordinated_apply_continuous)
        .collect::<Option<Vec<_>>>()?;
    if let Some(declared_player_target) = declared_player_target
        && applies.iter().any(|apply| {
            !matches!(
                apply_continuous_filter(apply).and_then(|filter| filter.controller.as_ref()),
                Some(PlayerFilter::Target(inner))
                    if inner.as_ref() == declared_player_target
            )
        })
    {
        return None;
    }
    if applies.iter().any(|apply| {
        apply.until != Until::Forever
            || apply.condition.is_some()
            || describe_apply_continuous_tail(apply).is_some()
    }) {
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

    let (subject, plural_subject) = describe_apply_continuous_target(applies[0]);
    if subject.is_empty() {
        return None;
    }
    // A leading-P/T animation is already one typed predicate: for example,
    // “becomes a 9/9 Construct artifact creature.” Decomposing that first
    // effect into layer-by-layer clauses exposes implementation details such
    // as subtype removal and type retention. Keep its specialized rendering
    // intact, then coordinate only the later same-object modifiers.
    if applies[0].animation_pt_surface
        == Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness)
    {
        let head = describe_effect(&effects[0])
            .trim()
            .trim_end_matches('.')
            .to_string();
        if head.is_empty() || head.contains(". ") {
            return None;
        }
        let mut parts = vec![head];
        for apply in applies.iter().skip(1) {
            let apply_clauses = describe_apply_continuous_clauses(apply, plural_subject);
            if apply_clauses.is_empty() {
                return None;
            }
            parts.extend(apply_clauses);
        }
        return Some(capitalize_first(&join_coordinated_parts(&parts)?));
    }
    let mut clauses = Vec::new();
    for apply in applies {
        let apply_clauses = describe_apply_continuous_clauses(apply, plural_subject);
        if apply_clauses.is_empty() {
            return None;
        }
        clauses.extend(apply_clauses);
    }
    Some(capitalize_first(&format!(
        "{subject} {}",
        join_coordinated_parts(&clauses)?
    )))
}

fn describe_coordinated_named_possessive_base_pt_and_grant(
    effects: &[Effect],
    _leading_duration: bool,
) -> Option<String> {
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
    let source_surface = first.source_reference_surface.as_ref().or_else(|| {
        first
            .target_spec
            .as_ref()
            .and_then(ChooseSpec::source_reference_surface)
    })?;
    let crate::target::SourceReferenceSurface::FullName(name) = source_surface else {
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
    let combined = if let Some(action) = restriction.strip_prefix("It ") {
        format!("{continuous} and {action}")
    } else {
        let (subject, action) = restriction.split_once(" must be blocked")?;
        if !continuous.starts_with(subject) {
            return None;
        }
        format!("{continuous} and must be blocked{action}")
    };
    Some(capitalize_first(&combined))
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
        || first.has_characteristic_modifiers()
        || second.has_characteristic_modifiers()
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
        capitalize_first(gain.trim().trim_end_matches('.')),
        lowercase_first(suspect.trim().trim_end_matches('.'))
    ))
}

/// An additional-draw clause describes a distinct draw-step entitlement, so
/// retain the explicit player subject on both authored actions. Accept either
/// direct siblings or the grammar-preserved coordinated sequence wrapper.
fn describe_you_lose_life_and_draw_additional(effects: &[Effect]) -> Option<String> {
    let effects = if let [effect] = effects
        && let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && !matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::Sequential
        ) {
        sequence.effects.as_slice()
    } else {
        effects
    };
    let [lose_effect, draw_effect] = effects else {
        return None;
    };
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if !matches!(
        lose.player.unhinted(),
        ChooseSpec::Player(PlayerFilter::You)
    ) || draw.player != PlayerFilter::You
        || !draw
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
    {
        return None;
    }

    Some(format!(
        "you lose {} life and you draw {}",
        describe_value(&lose.amount),
        describe_card_count(&draw.count)
    ))
}

/// Preserve established draw-followup wording inside an explicitly
/// coordinated source clause. The ordinary effect-list renderer already has
/// typed handling for both same-player gain/draw pairs and imperative actions
/// followed by a draw; route those shapes before the generic coordinated
/// fallback renders each child independently and repeats "you".
fn describe_coordinated_draw_followup(effects: &[Effect]) -> Option<String> {
    if let [first, second] = effects
        && let Some(compact) = describe_same_actor_gain_then_draw(first, second)
    {
        return Some(capitalize_first(&compact));
    }

    let effect_refs = effects.iter().collect::<Vec<_>>();
    describe_conjoined_counter_or_draw_sequence(&effect_refs)
}

/// Fold a lowering-only player target into two authored restrictions that
/// share the sentence's leading duration. The typed player reference on both
/// restrictions proves that the second clause's subject is "that player";
/// the restriction kind preserves the expanded Oracle wording for the
/// non-mana-ability exception.
pub(super) fn describe_coordinated_target_player_cast_and_activation_restrictions(
    effects: &[Effect],
    leading_duration: bool,
) -> Option<String> {
    if !leading_duration {
        return None;
    }
    let [target_effect, cast_effect, activation_effect] = effects else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.explicit_declaration || target.chooser.is_some() {
        return None;
    }
    let ChooseSpec::Target(target_inner) = &target.target else {
        return None;
    };
    let ChooseSpec::Player(target_player) = target_inner.as_ref() else {
        return None;
    };
    let references_target_player = |candidate: &PlayerFilter| {
        matches!(
            candidate,
            PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner)
                if inner.as_ref() == target_player
        )
    };

    let cast = cast_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let activation = activation_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if cast.duration != Until::EndOfTurn
        || activation.duration != Until::EndOfTurn
        || !matches!(&cast.start, crate::effect::RestrictionStart::Immediate)
        || !matches!(
            &activation.start,
            crate::effect::RestrictionStart::Immediate
        )
        || cast.duration_surface != crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
        || activation.duration_surface
            != crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
    {
        return None;
    }
    let crate::effect::Restriction::CastSpellsMatching(cast_player, _) = &cast.restriction else {
        return None;
    };
    let crate::effect::Restriction::ActivateNonManaAbilities(activation_player) =
        &activation.restriction
    else {
        return None;
    };
    if !references_target_player(cast_player) || !references_target_player(activation_player) {
        return None;
    }

    let subject = describe_choose_spec(&target.target);
    let cast_clause = describe_restriction(&cast.restriction);
    let cast_action = cast_clause.strip_prefix(&format!("{subject} "))?;
    Some(format!(
        "Until end of turn, {subject} {cast_action}, and that player can't activate abilities that aren't mana abilities"
    ))
}

/// Restore an explicit source conjunction after the ordinary effect-list
/// renderer has preserved the child clauses' established surfaces.
fn describe_typed_coordinated_clause_fallback(effects: &[Effect]) -> Option<String> {
    if let [first, second, third] = effects {
        let mut parts = [first, second, third]
            .into_iter()
            .map(describe_effect)
            .map(|part| part.trim().trim_end_matches('.').to_string())
            .collect::<Vec<_>>();
        if parts
            .iter()
            .any(|part| part.is_empty() || part.contains(". "))
        {
            return None;
        }
        if parts.first().is_some_and(|part| part == "Tap this source")
            && parts.iter().skip(1).any(|part| part.contains(" from it"))
        {
            parts[0] = "Tap it".to_string();
        }
        parts[0] = capitalize_first(&normalize_imperative_you_clause(&parts[0]));
        for part in parts.iter_mut().skip(1) {
            *part = lowercase_first(&normalize_imperative_you_clause(part));
        }
        return join_coordinated_parts(&parts);
    }

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
    if parts.first().is_some_and(|part| part == "Tap this source")
        && parts.iter().skip(1).any(|part| part.contains(" from it"))
    {
        parts[0] = "Tap it".to_string();
    }
    if let Some(first) = parts.first_mut() {
        *first = capitalize_first(&normalize_imperative_you_clause(first));
    }
    for part in parts.iter_mut().skip(1) {
        *part = lowercase_first(part);
    }
    join_coordinated_parts(&parts)
}

/// Preserve three explicitly coordinated player clauses when an each-opponent
/// action is followed by the controller drawing and gaining life. The generic
/// effect-list renderer correctly combines the two same-player actions, but
/// doing so erases the source sequence's three-clause boundary and produces
/// "each opponent ... and you ... and ...".
fn describe_each_opponent_then_you_draw_and_gain(effects: &[Effect]) -> Option<String> {
    let [opponent_effect, second, third] = effects else {
        return None;
    };
    let for_opponents = opponent_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_opponents.filter != PlayerFilter::Opponent || for_opponents.effects.len() != 1 {
        return None;
    }

    fn explicit_you_draw_or_gain(effect: &Effect) -> Option<String> {
        let is_fixed_you_draw = effect
            .downcast_ref::<crate::effects::DrawCardsEffect>()
            .is_some_and(|draw| {
                draw.player == PlayerFilter::You && matches!(draw.count, Value::Fixed(_))
            });
        let is_fixed_you_gain = effect
            .downcast_ref::<crate::effects::GainLifeEffect>()
            .is_some_and(|gain| {
                gain.player == ChooseSpec::Player(PlayerFilter::You)
                    && matches!(gain.amount, Value::Fixed(_))
            });
        if !is_fixed_you_draw && !is_fixed_you_gain {
            return None;
        }

        let rendered = describe_effect(effect);
        let action = rendered
            .trim()
            .trim_end_matches('.')
            .strip_prefix("You ")
            .or_else(|| rendered.trim().trim_end_matches('.').strip_prefix("you "))
            .unwrap_or_else(|| rendered.trim().trim_end_matches('.'));
        if action.is_empty() || action.contains(". ") {
            return None;
        }
        Some(format!("you {}", lowercase_first(action)))
    }

    let second_is_draw = second
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some();
    let third_is_draw = third
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some();
    let second_is_gain = second
        .downcast_ref::<crate::effects::GainLifeEffect>()
        .is_some();
    let third_is_gain = third
        .downcast_ref::<crate::effects::GainLifeEffect>()
        .is_some();
    if !(second_is_draw && third_is_gain || second_is_gain && third_is_draw) {
        return None;
    }

    let opponent_clause = describe_effect(opponent_effect);
    let opponent_clause = opponent_clause.trim().trim_end_matches('.');
    if opponent_clause.is_empty()
        || opponent_clause.contains(". ")
        || opponent_clause.contains(", ")
        || !opponent_clause.starts_with("Each opponent")
    {
        return None;
    }
    let second_clause = explicit_you_draw_or_gain(second)?;
    let third_clause = explicit_you_draw_or_gain(third)?;
    Some(format!(
        "{opponent_clause}, {second_clause}, and {third_clause}"
    ))
}

fn describe_source_sacrifice_then_coordinated_suffix(effects: &[Effect]) -> Option<String> {
    let [sacrifice_effect, trailing @ ..] = effects else {
        return None;
    };
    if trailing.len() < 2 {
        return None;
    }
    let sacrifice = unwrap_basic_tag_wrappers(sacrifice_effect)
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target.base(), ChooseSpec::Source) {
        return None;
    }

    // The parser may flatten an ordinary nested coordination after the
    // leading source sacrifice. Re-render only that proven trailing group so
    // its shared action and recipient fanout compact normally, then restore
    // the authored source pronoun after "sacrifice this ...".
    let trailing_sequence = Effect::new(crate::effects::SequenceEffect::coordinated(
        trailing.to_vec(),
    ));
    let mut trailing_text = describe_effect(&trailing_sequence)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if trailing_text.is_empty() || trailing_text.contains(". ") {
        return None;
    }

    let source = describe_choose_spec(&sacrifice.target);
    let source_prefix = capitalize_first(&source);
    if let Some(rest) = trailing_text.strip_prefix(&format!("{source_prefix} ")) {
        trailing_text = format!("it {rest}");
    } else if let Some(rest) = trailing_text
        .strip_prefix("This ")
        .or_else(|| trailing_text.strip_prefix("this "))
    {
        // A bare runtime source can render with a less-specific noun than
        // the sacrifice's surface hint. The typed source identity is still
        // authoritative for the anaphoric "it".
        let action = match rest.split_once(' ') {
            Some((
                "creature" | "artifact" | "enchantment" | "land" | "permanent" | "source" | "card"
                | "spell",
                action,
            )) => action,
            _ => rest,
        };
        trailing_text = format!("it {action}");
    } else {
        return None;
    }
    trailing_text = trailing_text
        .replace(&format!(" on {source}"), " on it")
        .replace(" on this source", " on it");

    let sacrifice_text = describe_effect(sacrifice_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{sacrifice_text} and {}",
        lowercase_first(&trailing_text)
    ))
}

fn describe_independent_explicit_may_coordination(effects: &[Effect]) -> Option<String> {
    if effects.len() < 2
        || !effects.iter().all(|effect| {
            unwrap_basic_tag_wrappers(effect)
                .downcast_ref::<crate::effects::MayEffect>()
                .is_some()
        })
    {
        return None;
    }
    let mut clauses = effects
        .iter()
        .map(describe_effect)
        .map(|clause| clause.trim().trim_end_matches('.').to_string())
        .collect::<Vec<_>>();
    if clauses
        .iter()
        .any(|clause| clause.is_empty() || clause.contains(". "))
    {
        return None;
    }
    clauses[0] = capitalize_first(&clauses[0]);
    for clause in clauses.iter_mut().skip(1) {
        *clause = lowercase_first(clause);
    }
    let (last, leading) = clauses.split_last()?;
    Some(format!("{}, and {last}", leading.join(", ")))
}

fn describe_damage_and_you_scry(effects: &[Effect]) -> Option<String> {
    let (damage_effect, scry_effect) = match effects {
        [damage_effect, scry_effect] => (damage_effect, scry_effect),
        [target_effect, damage_effect, scry_effect] => {
            let target = structural_unwrap_render_wrappers(target_effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            let damage = damage_effect_view(damage_effect)?;
            if target.explicit_declaration
                || !target_specs_select_same_objects(&target.target, &damage.target)
            {
                return None;
            }
            (damage_effect, scry_effect)
        }
        _ => return None,
    };
    damage_effect_view(damage_effect)?;
    let scry = structural_unwrap_render_wrappers(scry_effect)
        .downcast_ref::<crate::effects::ScryEffect>()?;
    if scry.player != PlayerFilter::You {
        return None;
    }
    let damage = describe_effect(damage_effect);
    let scry = describe_effect(scry_effect);
    Some(format!(
        "{} and you {}",
        damage.trim().trim_end_matches('.'),
        lowercase_first(scry.trim().trim_end_matches('.'))
    ))
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
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::ResultConjunction {
            leading_duration: false
        }
    ) && describe_untap_attacking_then_additional_combat(&sequence.effects).is_some()
    {
        return Some(
            "Untap all attacking creatures and after this phase, there is an additional combat phase"
                .to_string(),
        );
    }
    if let Some(compact) = describe_coordinated_create_token_then_grant_same_tag(&sequence.effects)
    {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_action_and_get_energy_pair(first, second)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_damage_and_you_scry(&sequence.effects) {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(compact) = describe_you_life_change_and_create_token(&sequence.effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_source_sacrifice_then_coordinated_suffix(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_coordinated_copy_all_stack_sets(&sequence.effects) {
        return Some(compact);
    }
    let refs = sequence.effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_conjoined_counter_or_draw_sequence(&refs) {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_joint_subject_pair(first, second)
    {
        return Some(compact);
    }
    // A result branch can hold the whole consult program ("If you do, reveal
    // cards from the top of your library until you reveal X, put it onto the
    // battlefield, then put the rest on the bottom ..." — Kethek); the
    // compact one-sentence surface must win over the generic clause split.
    {
        let refs: Vec<&Effect> = sequence.effects.iter().collect();
        if let Some(compact) = describe_consult_reveal_move_matches_then_bottom(&refs) {
            return Some(compact);
        }
    }
    describe_typed_coordinated_clause_fallback(&sequence.effects)
}

/// Keep an explicitly authored result conjunction together when later
/// effects belong to a new sentence in the same result branch.
///
/// The first sequence is the parser's structural proof that those actions
/// shared one clause (for example, revealing a hand and choosing a card from
/// it). Rendering the whole outer list at once can otherwise reassociate a
/// following action with the final member of that clause. Requiring both the
/// typed result surface and a nonempty tail keeps ordinary coordinated effect
/// lists on their existing paths.
fn describe_leading_result_conjunction_then_followups(effects: &[Effect]) -> Option<String> {
    let (leading, followups) = effects.split_first()?;
    if followups.is_empty() {
        return None;
    }

    let leading = describe_typed_coordinated_result_branch(std::slice::from_ref(leading))?;
    let followups = describe_effect_list(followups);
    let mut leading = leading.trim().trim_end_matches('.').to_string();
    for verb in [
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
        "attach ",
    ] {
        leading = leading.replace(&format!(" and you {verb}"), &format!(" and {verb}"));
    }
    let followups = followups.trim().trim_end_matches('.');
    if leading.is_empty() || leading.contains(". ") || followups.is_empty() {
        return None;
    }

    Some(format!("{leading}. {}", capitalize_first(followups)))
}

pub(super) fn describe_result_branch_effect_list(effects: &[Effect]) -> String {
    describe_typed_coordinated_result_branch(effects)
        .or_else(|| describe_leading_result_conjunction_then_followups(effects))
        .or_else(|| {
            // A flat consult program in the result branch keeps oracle's
            // one-sentence surface ("If you do, reveal ... until you reveal
            // X, put it onto the battlefield, then put the rest ..." —
            // Kethek).
            let refs: Vec<&Effect> = effects.iter().collect();
            describe_consult_reveal_move_matches_then_bottom(&refs)
        })
        .unwrap_or_else(|| describe_effect_list(effects))
}

/// Compact a coordinated control-and-lock bundle that applies one authored
/// duration to the same previously chosen permanent.
///
/// Lowering tags the control effect's result, then points both the ability
/// removal and attack/block restriction back at that exact tag. Requiring
/// those links keeps this renderer generic while preserving the leading
/// duration and shared pronoun surface from the source text.
fn describe_coordinated_control_and_lock(
    effects: &[Effect],
    leading_duration: bool,
) -> Option<String> {
    if !leading_duration {
        return None;
    }
    let [control_effect, ability_loss_effect, restriction_effect] = effects else {
        return None;
    };
    let control = coordinated_apply_continuous(control_effect)?;
    let ability_loss = coordinated_apply_continuous(ability_loss_effect)?;
    let restriction = unwrap_basic_tag_wrappers(restriction_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;

    if control.until != ability_loss.until
        || control.until != restriction.duration
        || control.condition.is_some()
        || ability_loss.condition.is_some()
        || control.modification.is_some()
        || ability_loss.modification.is_some()
        || !control.additional_modifications.is_empty()
        || !ability_loss.additional_modifications.is_empty()
        || !matches!(
            control.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
        || !matches!(
            ability_loss.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::RemoveAllAbilities]
        )
        || !coordinated_apply_targets_previous(
            control_effect,
            ability_loss_effect,
            control,
            ability_loss,
        )
    {
        return None;
    }

    let crate::effect::Restriction::AttackOrBlock(restriction_filter) = &restriction.restriction
    else {
        return None;
    };
    let controlled_tag = coordinated_effect_tag(control_effect)?;
    if !filter_is_exactly_tagged(restriction_filter, controlled_tag) {
        return None;
    }

    let duration = describe_until(&control.until);
    if duration.is_empty() {
        return None;
    }
    let (target, plural_target) = describe_apply_continuous_target(control);
    if target.is_empty() || plural_target {
        return None;
    }

    Some(format!(
        "{}, gain control of {}, it loses all abilities, and it can't attack or block",
        capitalize_first(&duration),
        lowercase_first(&target),
    ))
}

/// Preserve an inline mill/selection procedure as one authored `, then`
/// clause. The coordinated boundary is supplied only by the same-sentence
/// bundle parser, while the collection tag and selected-group loop prove the
/// follow-up is bound to that exact mill result.
fn describe_coordinated_mill_then_collection_selection(effects: &[Effect]) -> Option<String> {
    let compact = match effects {
        [milled_effect, matching_effect, move_effect]
            if matching_effect
                .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
                .is_some() =>
        {
            let (source_tag, mill) = mill_with_collection_tag(milled_effect)?;
            let matching =
                matching_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
            let (_, move_matching) = for_each_tagged_for_compaction(move_effect)?;
            describe_tagged_mill_then_put_all_matching_milled_cards(
                source_tag.as_str(),
                mill,
                matching,
                move_matching,
            )?
        }
        [milled_effect, choose_effect, move_effect] => {
            let (source_tag, mill) = mill_with_collection_tag(milled_effect)?;
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, move_chosen) = for_each_tagged_for_compaction(move_effect)?;
            describe_mill_then_put_milled_cards(source_tag.as_str(), mill, &[choose], move_chosen)?
        }
        [
            milled_effect,
            first_choice_effect,
            second_choice_effect,
            move_effect,
        ] => {
            let (source_tag, mill) = mill_with_collection_tag(milled_effect)?;
            let first_choice =
                first_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let second_choice =
                second_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, move_chosen) = for_each_tagged_for_compaction(move_effect)?;
            describe_mill_then_put_milled_cards(
                source_tag.as_str(),
                mill,
                &[first_choice, second_choice],
                move_chosen,
            )?
        }
        _ => return None,
    };
    let (mill_clause, put_clause) = compact.split_once(". ")?;
    Some(format!(
        "{mill_clause}, then {}",
        lowercase_first(put_clause)
    ))
}

/// Keep an implicit attachment choice inside the sacrifice instruction that
/// precedes it. The explicit choice effect is runtime provenance, while the
/// coordinated surface and exact shared tag prove that the attach action
/// consumes that choice.
fn describe_sacrifice_then_controller_chosen_attachment(effects: &[Effect]) -> Option<String> {
    let [sacrifice_effect, choose_effect, attach_effect] = effects else {
        return None;
    };
    sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || choose.count.is_random()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !matches!(attach.objects.base(), ChooseSpec::Source)
        || !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let sacrifice = describe_effect(sacrifice_effect)
        .trim_end_matches('.')
        .to_string();
    let object = describe_attach_objects_spec(&attach.objects);
    let selection = describe_choose_selection(choose);
    Some(format!("{sacrifice} and attach {object} to {selection}"))
}

/// Render the connective carried by `SequenceSurface::CommaThen`.
///
/// Target-only effects are lowering scaffolding rather than authored actions.
/// The common draw/discard pair is rendered from its typed player and count
/// relationships so the shared subject and "that many" back-reference stay
/// intact. Other shapes retain the boundary by treating the final visible
/// action as the `then` arm, matching the generic chain splitter's two-arm
/// representation.
fn describe_source_and_plural_cost_set_exile(effects: &[Effect]) -> Option<String> {
    let [source_effect, set_effect] = effects else {
        return None;
    };
    let source = structural_unwrap_render_wrappers(source_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let set = structural_unwrap_render_wrappers(set_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(source)
        || !move_to_zone_is_plain_exile(set)
        || !matches!(source.target.base(), ChooseSpec::Source)
    {
        return None;
    }
    let filter = match set.target.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return None,
    };
    let [constraint] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || !constraint.tag.as_str().starts_with("sacrifice_cost_")
        || !filter.has_plural_object_noun_surface()
        || !filter.has_explicit_card_noun()
    {
        return None;
    }

    let mut noun_filter = filter.clone();
    noun_filter.tagged_constraints.clear();
    noun_filter.zone = None;
    noun_filter.set_plural_object_noun_surface(false);
    let noun = pluralize_noun_phrase(strip_indefinite_article(&noun_filter.description()));
    Some(format!(
        "Exile {} and those {noun}",
        describe_choose_spec(&source.target)
    ))
}

fn synthetic_target_player_filter(effect: &Effect) -> Option<PlayerFilter> {
    let target = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.explicit_declaration || target.chooser.is_some() {
        return None;
    }
    let PlayerFilter::Target(inner) = choose_spec_player_filter(&target.target)? else {
        return None;
    };
    Some(*inner)
}

fn draw_count_uses_target_player(
    draw: &crate::effects::DrawCardsEffect,
    target: &PlayerFilter,
) -> bool {
    if draw.player != PlayerFilter::You {
        return false;
    }
    let Value::Count(filter) = draw.count.unhinted() else {
        return false;
    };
    matches!(
        filter.controller.as_ref(),
        Some(PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner))
            if inner.as_ref() == target
    )
}

/// Fold the lowering-only target declarations in an authored pair of
/// target-relative count draws back into the two draw instructions. Each
/// typed count must reference its immediately preceding player target, so
/// unrelated declarations cannot disappear into this surface.
fn describe_target_player_count_draw_pair(effects: &[Effect]) -> Option<String> {
    let [first_target, first_draw, second_target, second_draw] = effects else {
        return None;
    };
    let first_target = synthetic_target_player_filter(first_target)?;
    let second_target = synthetic_target_player_filter(second_target)?;
    let first_draw = structural_unwrap_render_wrappers(first_draw)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let second_draw = structural_unwrap_render_wrappers(second_draw)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if !draw_count_uses_target_player(first_draw, &first_target)
        || !draw_count_uses_target_player(second_draw, &second_target)
    {
        return None;
    }

    let first = describe_effect(&effects[1]);
    let second = describe_effect(&effects[3]);
    Some(format!(
        "{}, then {}",
        capitalize_first(first.trim().trim_end_matches('.')),
        lowercase_first(second.trim().trim_end_matches('.'))
    ))
}

/// Fold a shared target-player action chain whose final sacrifice is backed by
/// an explicit choice/tag pair. The choice remains executable, while the
/// renderer restores the authored "of their choice" surface instead of
/// exposing the internal choose instruction as a separate sentence.
fn describe_target_player_lose_discard_choose_sacrifice(effects: &[Effect]) -> Option<String> {
    let [
        target_effect,
        lose_effect,
        discard_effect,
        choose_effect,
        sacrifice_effect,
    ] = effects
    else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target != &crate::effects::TargetOnlyEffect::new(ChooseSpec::target_player()) {
        return None;
    }
    let lose = structural_unwrap_render_wrappers(lose_effect)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let target_player = PlayerFilter::Target(Box::new(PlayerFilter::Any));
    if lose.amount.unhinted() != &Value::Fixed(1)
        || (lose.player != ChooseSpec::target_player()
            && lose.player != ChooseSpec::Player(target_player))
    {
        return None;
    }

    let aliased_target = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let discard = structural_unwrap_render_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count.unhinted() != &Value::Fixed(1)
        || discard.player != aliased_target
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }

    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_choose = |filter: ObjectFilter| {
        crate::effects::ChooseObjectsEffect::new(
            filter.controlled_by(aliased_target.clone()),
            ChoiceCount::exactly(1),
            aliased_target.clone(),
            choose.tag.clone(),
        )
    };
    // Runtime lowering uses both equivalent permanent encodings: the compact
    // battlefield-only selector and the explicit six permanent card types on
    // the battlefield. Keep the correlation guard exact while accepting the
    // public parser's latter representation.
    if choose != &expected_choose(ObjectFilter::permanent())
        && choose != &expected_choose(ObjectFilter::permanent_card().in_zone(Zone::Battlefield))
    {
        return None;
    }
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    if sacrifice.filter != &ObjectFilter::tagged(choose.tag.clone())
        || sacrifice.count.unhinted() != &Value::Fixed(1)
        || sacrifice.player != &aliased_target
    {
        return None;
    }
    let sacrifice = describe_choose_then_sacrifice(choose, sacrifice)?;
    let sacrifice = sacrifice.strip_prefix("that player ")?;
    Some(format!(
        "Target player loses 1 life, discards a card, then {sacrifice}"
    ))
}

#[cfg(test)]
mod target_player_choice_chain_tests {
    use super::*;

    fn effects(sacrifice_tag: &str) -> Vec<Effect> {
        let target = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
        let chosen_tag = TagKey::from("chosen_permanent");
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_player(),
            )),
            Effect::new(crate::effects::LoseLifeEffect::target_player(1)),
            Effect::new(crate::effects::DiscardEffect::new(1, target.clone(), false)),
            Effect::new(crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::permanent()
                    .controlled_by(target.clone())
                    .in_zone(Zone::Battlefield),
                ChoiceCount::exactly(1),
                target.clone(),
                chosen_tag,
            )),
            Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
                ObjectFilter::tagged(sacrifice_tag),
                1,
                target,
            )),
        ]
    }

    #[test]
    fn target_player_chain_restores_choice_surface_only_for_the_exact_tagged_object() {
        assert_eq!(
            describe_target_player_lose_discard_choose_sacrifice(&effects("chosen_permanent"))
                .as_deref(),
            Some(
                "Target player loses 1 life, discards a card, then sacrifices a permanent of their choice"
            )
        );
        assert_eq!(
            describe_target_player_lose_discard_choose_sacrifice(&effects("other_permanent")),
            None
        );

        let mut lowered_target_player = effects("chosen_permanent");
        lowered_target_player[1] = Effect::new(crate::effects::LoseLifeEffect::with_filter(
            1,
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        ));
        assert_eq!(
            describe_target_player_lose_discard_choose_sacrifice(&lowered_target_player).as_deref(),
            Some(
                "Target player loses 1 life, discards a card, then sacrifices a permanent of their choice"
            )
        );

        let mut explicit_permanent_types = effects("chosen_permanent");
        let choose = explicit_permanent_types[3]
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .expect("choice effect")
            .clone();
        explicit_permanent_types[3] = Effect::new(crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::permanent_card()
                .in_zone(Zone::Battlefield)
                .controlled_by(choose.chooser.clone()),
            choose.count,
            choose.chooser.clone(),
            choose.tag.clone(),
        ));
        assert_eq!(
            describe_target_player_lose_discard_choose_sacrifice(&explicit_permanent_types)
                .as_deref(),
            Some(
                "Target player loses 1 life, discards a card, then sacrifices a permanent of their choice"
            )
        );
    }
}

fn describe_repeated_scry_comma_then(effects: &[Effect]) -> Option<String> {
    let [first, rest @ ..] = effects else {
        return None;
    };
    if rest.is_empty() {
        return None;
    }
    let first_scry = first.downcast_ref::<crate::effects::ScryEffect>()?;
    let mut clauses = Vec::with_capacity(effects.len());
    for effect in effects {
        let scry = effect.downcast_ref::<crate::effects::ScryEffect>()?;
        if scry.player != first_scry.player {
            return None;
        }
        let clause = describe_effect(effect);
        let clause = clause.trim().trim_end_matches('.');
        if clause.is_empty() || clause.contains(". ") {
            return None;
        }
        clauses.push(clause.to_string());
    }

    let mut rendered = capitalize_first(&clauses[0]);
    for clause in &clauses[1..] {
        rendered.push_str(", then ");
        rendered.push_str(&lowercase_first(clause));
    }
    Some(rendered)
}

#[cfg(test)]
mod repeated_scry_comma_then_tests {
    use super::*;

    #[test]
    fn homogeneous_scry_chain_renders_every_ordered_arm() {
        let effects = vec![
            Effect::new(crate::effects::ScryEffect::you(1)),
            Effect::new(crate::effects::ScryEffect::you(2)),
            Effect::new(crate::effects::ScryEffect::you(3)),
        ];
        assert_eq!(
            describe_repeated_scry_comma_then(&effects).as_deref(),
            Some("Scry 1, then scry 2, then scry 3")
        );
    }

    #[test]
    fn repeated_comma_then_provenance_survives_the_public_trigger_route() {
        const LINE: &str = "When this creature enters, scry 1, then scry 2, then scry 3.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cryptic Annelid")
                .card_types(vec![CardType::Creature])
                .parse_text(LINE)
                .expect("repeated scry trigger should parse");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );
    }

    #[test]
    fn mixed_player_chain_does_not_inherit_the_homogeneous_scry_surface() {
        let effects = vec![
            Effect::new(crate::effects::ScryEffect::you(1)),
            Effect::new(crate::effects::ScryEffect::new(2, PlayerFilter::Opponent)),
            Effect::new(crate::effects::ScryEffect::you(3)),
        ];
        assert_eq!(describe_repeated_scry_comma_then(&effects), None);
    }
}

/// Rejoin a fixed energy gain with a tagged sacrifice-or-energy-payment tail.
///
/// The shared tag proves that the sacrificed permanent and the mana-value
/// basis of the payment are the same object. That correlation is what makes
/// the authored `that creature` / `its mana value` wording safe; unrelated
/// sacrifice and payment tags stay on the generic renderer.
fn describe_energy_then_tagged_sacrifice_unless_payment(effects: &[Effect]) -> Option<String> {
    let [energy_effect, unless_effect] = effects else {
        return None;
    };
    let energy = structural_unwrap_render_wrappers(energy_effect)
        .downcast_ref::<crate::effects::EnergyCountersEffect>()?;
    if energy.player != PlayerFilter::You
        || !matches!(energy.count.unhinted(), Value::Fixed(amount) if *amount > 0)
    {
        return None;
    }

    let unless_pays = structural_unwrap_render_wrappers(unless_effect)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless_pays.player != PlayerFilter::You
        || unless_pays.leading_surface
        || unless_pays.before_delayed_step
    {
        return None;
    }
    let [sacrifice_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    let sacrifice = structural_unwrap_render_wrappers(sacrifice_effect)
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    let ChooseSpec::Tagged(sacrifice_tag) = sacrifice.target.base() else {
        return None;
    };

    let [cost] = unless_pays.cost.costs() else {
        return None;
    };
    let payment = cost
        .effect_ref()?
        .downcast_ref::<crate::effects::PayEnergyEffect>()?;
    if payment.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let Value::ManaValueOf(payment_object) = payment.amount.unhinted() else {
        return None;
    };
    if !matches!(payment_object.base(), ChooseSpec::Tagged(payment_tag) if payment_tag == sacrifice_tag)
    {
        return None;
    }

    Some(format!(
        "{}, then sacrifice that creature unless you pay an amount of {{E}} equal to its mana value",
        describe_effect(energy_effect).trim().trim_end_matches('.')
    ))
}

#[cfg(test)]
mod energy_sacrifice_unless_surface_tests {
    use super::*;

    fn effects(sacrifice_tag: &str, payment_tag: &str) -> Vec<Effect> {
        let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(
            ChooseSpec::Tagged(TagKey::from(sacrifice_tag)),
        ));
        let payment = Effect::new(crate::effects::PayEnergyEffect::new(
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(payment_tag)))),
            ChooseSpec::Player(PlayerFilter::You),
        ));
        let cost = crate::costs::Cost::try_effect(payment)
            .expect("dynamic energy payment should be executable as a cost");
        vec![
            Effect::new(crate::effects::EnergyCountersEffect::you(4)),
            Effect::new(crate::effects::UnlessPaysEffect::new_total_cost(
                vec![sacrifice],
                PlayerFilter::You,
                crate::cost::TotalCost::from_cost(cost),
            )),
        ]
    }

    #[test]
    fn shared_tag_restores_that_creature_and_its_mana_value() {
        assert_eq!(
            describe_energy_then_tagged_sacrifice_unless_payment(&effects(
                "exchanged",
                "exchanged"
            ))
            .as_deref(),
            Some(
                "you get {E}{E}{E}{E}, then sacrifice that creature unless you pay an amount of {E} equal to its mana value"
            )
        );
    }

    #[test]
    fn different_payment_object_does_not_inherit_the_correlated_surface() {
        assert_eq!(
            describe_energy_then_tagged_sacrifice_unless_payment(&effects(
                "exchanged",
                "different"
            )),
            None
        );
    }
}

fn dream_cache_library_move(
    mode: &ironsmith_core::EffectMode<Effect>,
) -> Option<&crate::effects::MoveToZoneEffect> {
    let [effect] = mode.effects.as_slice() else {
        return None;
    };
    let move_effect = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_effect.zone != Zone::Library
        || move_effect.library_order.is_some()
        || move_effect.remainder_surface.is_some()
        || move_effect.exiled_with_source_surface.is_some()
        || move_effect.battlefield_controller != ironsmith_core::BattlefieldController::Preserve
        || move_effect.controller_surface_explicit
        || !move_effect.enters_with_counters.is_empty()
        || move_effect.enters_tapped
        || move_effect.enters_attacking
        || move_effect.enters_face_down
        || move_effect.enters_transformed
        || move_effect.transfer_exiled_with_source_links
    {
        return None;
    }
    let ChooseSpec::WithCount(inner, count) = move_effect.target.unhinted() else {
        return None;
    };
    if count != &ChoiceCount::exactly(2)
        || inner.unhinted()
            != &ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(PlayerFilter::You),
            )
    {
        return None;
    }
    Some(move_effect)
}

/// Restore the compact same-object destination choice without exposing the
/// internal modal transport. Both modes must move the same exact two cards
/// from the controller's hand to opposite ends of that player's library.
fn describe_draw_then_same_cards_top_or_bottom(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    let [draw_effect, choose_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count.unhinted() != &Value::Fixed(3) {
        return None;
    }
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose.modes.len() != 2
        || !choose.common_prefix_effects.is_empty()
        || choose.chooser.as_ref() != Some(&PlayerFilter::You)
        || choose.min.unhinted() != &Value::Fixed(1)
        || choose.max.unhinted() != &Value::Fixed(1)
        || choose.allow_repeat
        || choose.random
        || choose.allow_repeated_modes
        || choose.spree
        || choose.tiered
        || !choose.mode_additional_mana_costs.is_empty()
        || choose.common_suffix_effect_count != 0
        || choose.disallow_previously_chosen_modes
        || choose.disallow_previously_chosen_modes_this_turn
        || choose.distinct_player_targets_per_mode
        || choose.conditional_mode_range.is_some()
    {
        return None;
    }
    let first = dream_cache_library_move(&choose.modes[0])?;
    let second = dream_cache_library_move(&choose.modes[1])?;
    if first.target != second.target || !first.to_top || second.to_top {
        return None;
    }
    Some(
        "Draw three cards, then put two cards from your hand both on top of your library or both on the bottom of your library"
            .to_string(),
    )
}

#[cfg(test)]
mod draw_then_same_cards_destination_choice_tests {
    use super::*;

    fn sequence(count: ChoiceCount) -> crate::effects::SequenceEffect {
        let target = ChooseSpec::Object(
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You),
        )
        .with_count(count);
        let choose = crate::effects::ChooseModeEffect::choose_one(vec![
            ironsmith_core::EffectMode::new(
                "Put two cards from your hand both on top of your library",
                vec![Effect::new(
                    crate::effects::MoveToZoneEffect::to_top_of_library(target.clone()),
                )],
            ),
            ironsmith_core::EffectMode::new(
                "Put them both on the bottom of your library",
                vec![Effect::new(
                    crate::effects::MoveToZoneEffect::to_bottom_of_library(target),
                )],
            ),
        ])
        .with_chooser(PlayerFilter::You);
        crate::effects::SequenceEffect::comma_then(vec![
            Effect::draw(Value::Fixed(3)),
            Effect::new(choose),
        ])
    }

    #[test]
    fn same_exact_two_cards_top_or_bottom_compacts() {
        assert_eq!(
            describe_draw_then_same_cards_top_or_bottom(&sequence(ChoiceCount::exactly(2)))
                .as_deref(),
            Some(
                "Draw three cards, then put two cards from your hand both on top of your library or both on the bottom of your library"
            )
        );
    }

    #[test]
    fn different_count_does_not_inherit_the_compact_surface() {
        assert_eq!(
            describe_draw_then_same_cards_top_or_bottom(&sequence(ChoiceCount::exactly(1))),
            None
        );
    }
}

fn describe_target_player_life_loss_then_random_hand_reveal(effects: &[Effect]) -> Option<String> {
    let [target_effect, lose_effect, choose_effect, reveal_effect] = effects else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.explicit_declaration || target.chooser.is_some() {
        return None;
    }
    let ChooseSpec::Target(declared) = target.target.unhinted() else {
        return None;
    };
    let ChooseSpec::Player(declared_player) = declared.unhinted() else {
        return None;
    };
    let subject = match declared_player {
        PlayerFilter::Opponent => "Target opponent",
        PlayerFilter::Any => "Target player",
        _ => return None,
    };

    let lose = structural_unwrap_render_wrappers(lose_effect)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let ChooseSpec::Player(losing_player) = lose.player.unhinted() else {
        return None;
    };
    if !matches!(
        losing_player,
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner)
            if inner.as_ref() == declared_player
    ) {
        return None;
    }

    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::Target(Box::new(declared_player.clone()))),
        ChoiceCount::exactly(1).at_random(),
        PlayerFilter::AliasedTarget(Box::new(declared_player.clone())),
        choose.tag.clone(),
    )
    .in_zone(Zone::Hand);
    if choose != &expected_choose {
        return None;
    }
    let reveal = structural_unwrap_render_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != choose.tag {
        return None;
    }

    Some(format!(
        "{subject} loses {} life, then reveals a card at random from their hand",
        describe_value(&lose.amount)
    ))
}

fn describe_comma_then_sequence(sequence: &crate::effects::SequenceEffect) -> Option<String> {
    if !matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::CommaThen
            | ironsmith_core::SequenceSurface::RepeatedCommaThen
    ) {
        return None;
    }
    if let Some(compact) = describe_draw_then_same_cards_top_or_bottom(sequence) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_player_count_draw_pair(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_player_lose_discard_choose_sacrifice(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_repeated_scry_comma_then(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_energy_then_tagged_sacrifice_unless_payment(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_target_player_life_loss_then_random_hand_reveal(&sequence.effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_target_player_resource_coordination(&sequence.effects) {
        return Some(compact);
    }
    if let [look_effect, exile_effect, grant_effect] = sequence.effects.as_slice()
        && let Some(look) = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(exile) = exile_effect.downcast_ref::<crate::effects::ExileEffect>()
        && let Some(grant) = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(compact) =
            describe_look_at_top_exile_face_down_then_play_while_exiled(look, exile, grant)
    {
        // A comma-then source can still carry an authored sentence boundary:
        // the look and exile are the procedure, while the persistent play
        // permission begins a new sentence. Prove the exact shared tag and
        // duration before restoring that surface so helper identities never
        // leak into rules text.
        return Some(compact);
    }
    if let Some(compact) = describe_lki_control_choose_attach_sequence(&sequence.effects) {
        return Some(compact);
    }
    let visible = sequence
        .effects
        .iter()
        .map(structural_unwrap_render_wrappers)
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_none_or(|target| target.explicit_declaration)
        })
        .collect::<Vec<_>>();
    if let Some(compact) = describe_reveal_hand_choose_discard_inline(&visible) {
        return Some(compact);
    }
    let effect_refs = sequence.effects.iter().collect::<Vec<_>>();
    if let Some((compact, consumed)) = describe_looked_card_selected_partition(&effect_refs)
        && consumed == sequence.effects.len()
    {
        return Some(compact);
    }
    if sequence.effects.len() > 2
        && let Some(compact) = describe_source_and_plural_cost_set_exile(
            &sequence.effects[sequence.effects.len() - 2..],
        )
    {
        let leading = describe_effect_list(&sequence.effects[..sequence.effects.len() - 2]);
        let leading = leading.trim().trim_end_matches('.');
        if !leading.is_empty() {
            return Some(format!(
                "{}, then {}",
                capitalize_first(leading),
                lowercase_first(&compact)
            ));
        }
    }
    // Dynamic token characteristics lower as a tagged creation followed by a
    // typed base-P/T setter. Keep that pair in the authored `then` arm so a
    // delayed cleanup carried by the creation cannot be rendered before the
    // token's P/T definition.
    if let [leading_effect, create_effect, set_pt_effect] = sequence.effects.as_slice()
        && structural_unwrap_render_wrappers(leading_effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_none()
        && let Some(token_text) =
            describe_create_token_then_set_base_pt_bundle(&[create_effect, set_pt_effect])
    {
        let leading = describe_effect(leading_effect);
        let leading = leading.trim().trim_end_matches('.');
        let token_text = token_text.trim().trim_end_matches('.');
        if !leading.is_empty() && !token_text.is_empty() {
            return Some(format!(
                "{}, then {}",
                capitalize_first(leading),
                lowercase_first(token_text)
            ));
        }
    }
    // A comma-then boundary is at least as strong a license for the
    // ", then" join as a plain coordinated one, so the mill/selection
    // compactor applies here too ("Mill four cards, then you may return a
    // permanent card from among them to your hand").
    if let Some(compact) = describe_coordinated_mill_then_collection_selection(&sequence.effects) {
        return Some(compact);
    }
    if let [look_effect, choose_effect, move_effect, shuffle_effect] = sequence.effects.as_slice()
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_looked_battlefield_then_shuffle(
            look_at_top,
            choose,
            move_effect,
            shuffle,
            false,
            true,
        )
    {
        return Some(compact);
    }
    // Give the flat, tag-aware clause compactor a chance to consume the
    // complete authored sequence before splitting off its final action.
    // This preserves producers such as an exact graveyard choice with their
    // linked consumer ("mill ..., then return ...") instead of exposing the
    // internal choose/tag scaffolding as a separate sentence.
    if let Some(compact) = describe_effect_clause_list(&sequence.effects) {
        let compact = compact.trim().trim_end_matches('.');
        if compact.contains(", then ") && !compact.contains(". ") {
            return Some(capitalize_first(compact));
        }
    }

    let visible_indices = sequence
        .effects
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_none()
                .then_some(index)
        })
        .collect::<Vec<_>>();
    if let [first_index, second_index] = visible_indices.as_slice() {
        let first = &sequence.effects[*first_index];
        let second = structural_unwrap_render_wrappers(&sequence.effects[*second_index]);
        if let Some(draw) = draw_cards_view(first)
            && let Some(discard) = second.downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(rendered) = describe_draw_then_discard(draw, discard)
        {
            return Some(rendered);
        }
    }

    let boundary = *visible_indices.last()?;
    if boundary == 0 {
        return None;
    }
    let leading = describe_effect_list(&sequence.effects[..boundary]);
    let trailing = describe_effect_list(&sequence.effects[boundary..]);
    let leading = leading.trim().trim_end_matches('.');
    let trailing = trailing.trim().trim_end_matches('.');
    if leading.is_empty() || trailing.is_empty() {
        return None;
    }
    Some(format!(
        "{}, then {}",
        capitalize_first(leading),
        lowercase_first(trailing)
    ))
}

/// A coordinated source modification followed by source damage to its
/// controller is one shared-subject predicate: "this creature gets ... and
/// deals ... to you".  Both effects must structurally point at the source,
/// while the damage effect must structurally target that source's controller.
/// The standalone damage renderer uses an imperative ("Deal ..."), so the
/// coordinator conjugates that typed predicate under the source subject.
fn describe_coordinated_source_continuous_then_damage_controller(
    effects: &[Effect],
) -> Option<String> {
    let [continuous_effect, damage_effect] = effects else {
        return None;
    };
    let continuous = structural_unwrap_render_wrappers(continuous_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(
        continuous.target_spec.as_ref().map(ChooseSpec::base),
        Some(ChooseSpec::Source)
    ) {
        return None;
    }
    let damage = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.target != ChooseSpec::SourceController {
        return None;
    }

    let continuous_text = describe_effect(continuous_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let damage_text = describe_effect(damage_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    for subject in [
        "This creature ",
        "This artifact ",
        "This enchantment ",
        "This land ",
        "This permanent ",
        "This source ",
        "It ",
        "this creature ",
        "this artifact ",
        "this enchantment ",
        "this land ",
        "this permanent ",
        "this source ",
        "it ",
    ] {
        if continuous_text.starts_with(subject) {
            let damage_predicate = [
                subject,
                "This creature ",
                "This permanent ",
                "This source ",
                "It ",
                "this creature ",
                "this permanent ",
                "this source ",
                "it ",
            ]
            .into_iter()
            .find_map(|prefix| damage_text.strip_prefix(prefix))
            .unwrap_or(&damage_text);
            let damage_predicate = if damage_predicate.starts_with("deals ") {
                damage_predicate.to_string()
            } else {
                damage_predicate
                    .strip_prefix("deal ")
                    .or_else(|| damage_predicate.strip_prefix("Deal "))
                    .map(|tail| format!("deals {tail}"))?
            };
            return Some(format!(
                "{} and {damage_predicate}",
                capitalize_first(&continuous_text)
            ));
        }
    }
    None
}

/// Fold a lowering-only target declaration into a mixed restriction and
/// subtype-modification chain. Both runtime actions must reference the exact
/// declaration tag, so this cannot conflate independently chosen targets.
fn describe_target_cant_block_then_add_subtypes(effects: &[Effect]) -> Option<String> {
    let [target_effect, cant_effect, subtype_effect] = effects else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.explicit_declaration || target.chooser.is_some() {
        return None;
    }
    let ChooseSpec::Target(inner) = target.target.unhinted() else {
        return None;
    };
    let ChooseSpec::Object(_) = inner.unhinted() else {
        return None;
    };
    let subject = describe_choose_spec(&target.target);
    if subject != "target creature" {
        return None;
    }
    let target_tag = wrapped_effect_tag(target_effect)?;

    let cant = structural_unwrap_render_wrappers(cant_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Block(restricted) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn
        || cant.start != crate::effect::RestrictionStart::Immediate
        || !object_filter_has_tagged_constraint(restricted, target_tag)
    {
        return None;
    }

    let subtype = structural_unwrap_render_wrappers(subtype_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if subtype.until != Until::EndOfTurn
        || subtype.condition.is_some()
        || !subtype.additional_modifications.is_empty()
        || !subtype.runtime_modifications.is_empty()
        || !matches!(
            subtype.modification.as_ref(),
            Some(crate::continuous::Modification::AddSubtypes(subtypes)) if !subtypes.is_empty()
        )
        || !subtype
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_has_tagged_constraint(spec, target_tag))
    {
        return None;
    }
    let subtype_clauses = describe_apply_continuous_clauses(subtype, false);
    let [subtype_clause] = subtype_clauses.as_slice() else {
        return None;
    };
    let duration = describe_apply_continuous_tail(subtype)?;
    Some(format!(
        "{} can't block this turn and {subtype_clause} {duration}",
        capitalize_first(&subject)
    ))
}

/// Rejoin a source animation and its same-source unblockable restriction.
/// The complete animation stays with the established typed renderer so its
/// power/toughness, colors, subtypes, retained types, and duration remain
/// authoritative; this helper only restores the shared source subject.
fn describe_source_animation_then_unblockable(effects: &[Effect]) -> Option<String> {
    let [animation_effect, cant_effect] = effects else {
        return None;
    };
    let animation = structural_unwrap_render_wrappers(animation_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if animation.until != Until::EndOfTurn
        || animation.condition.is_some()
        || animation.animation_pt_surface
            != Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness)
        || !matches!(
            animation.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Source)
        )
        || !matches!(
            animation.modification.as_ref(),
            Some(
                crate::continuous::Modification::AddCardTypes(types)
                    | crate::continuous::Modification::SetCardTypes(types)
            ) if types.contains(&CardType::Creature)
        )
        || !animation
            .additional_modifications
            .iter()
            .any(|modification| {
                matches!(
                    modification,
                    crate::continuous::Modification::SetPowerToughness { .. }
                )
            })
    {
        return None;
    }
    let source_reference_surface = animation.source_reference_surface.clone();

    let cant = structural_unwrap_render_wrappers(cant_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::BeBlocked(restricted) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn
        || cant.start != crate::effect::RestrictionStart::Immediate
        || !restricted.source
    {
        return None;
    }

    let animation = describe_effect(animation_effect);
    let mut animation = animation.trim().trim_end_matches('.').to_string();
    if let Some(crate::target::SourceReferenceSurface::ThisPermanentType(source_type)) =
        source_reference_surface.as_ref()
        && !source_type.to_ascii_lowercase().starts_with("this ")
        && animation
            .to_ascii_lowercase()
            .starts_with(&format!("{} ", source_type.to_ascii_lowercase()))
    {
        animation = format!("This {}", lowercase_first(&animation));
    }
    (!animation.is_empty() && !animation.contains(". "))
        .then(|| format!("{animation} and can't be blocked this turn"))
}

/// Rejoin a source animation with protection from every color of the spell
/// that triggered the ability. The executable representation expands the
/// dynamic color set into five independently gated fixed-color protection
/// grants; this matcher proves that complete WUBRG partition before restoring
/// the authored compact surface.
fn describe_source_animation_with_triggering_spell_color_protection(
    effects: &[Effect],
) -> Option<String> {
    let [animation_effect, conditional_effects @ ..] = effects else {
        return None;
    };
    if conditional_effects.len() != 5 {
        return None;
    }

    let animation = structural_unwrap_render_wrappers(animation_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if animation.until != Until::Forever
        || animation.condition.is_some()
        || !animation.runtime_modifications.is_empty()
        || animation.source_type.is_some()
        || animation.set_quantifier_surface.is_some()
        || animation.type_retention_surface.is_some()
        || animation.animation_duration_surface.is_some()
        || animation.lock_filter_at_resolution
        || !animation.resolve_set_pt_values_at_resolution
        || animation.require_creature_target
        || animation.animation_pt_surface
            != Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness)
        || !matches!(
            animation.target_spec.as_ref().map(ChooseSpec::unhinted),
            Some(ChooseSpec::Source)
        )
        || !matches!(
            animation.modification.as_ref(),
            Some(crate::continuous::Modification::SetCardTypes(types))
                if types.as_slice() == [CardType::Creature]
        )
        || animation.additional_modifications.len() != 3
        || !matches!(
            animation.additional_modifications[0],
            crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(4),
                toughness: Value::Fixed(4),
                sublayer: crate::continuous::PtSublayer::Setting,
            }
        )
        || !matches!(
            animation.additional_modifications[1],
            crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature
            )
        )
        || !matches!(
            &animation.additional_modifications[2],
            crate::continuous::Modification::AddSubtypes(subtypes)
                if subtypes.as_slice() == [crate::types::Subtype::Giant]
        )
    {
        return None;
    }

    let expected_colors = [
        crate::color::ColorSet::WHITE,
        crate::color::ColorSet::BLUE,
        crate::color::ColorSet::BLACK,
        crate::color::ColorSet::RED,
        crate::color::ColorSet::GREEN,
    ];
    let mut seen_colors = Vec::with_capacity(expected_colors.len());
    for effect in conditional_effects {
        let conditional = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ConditionalEffect>()?;
        let crate::effect::Condition::TaggedObjectMatches(tag, filter) = &conditional.condition
        else {
            return None;
        };
        let mut expected_filter = ObjectFilter::default();
        expected_filter.colors = filter.colors;
        let Some(color) = filter.colors else {
            return None;
        };
        if tag.as_str() != "triggering"
            || filter != &expected_filter
            || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
            || !conditional.if_false.is_empty()
        {
            return None;
        }
        let [grant_effect] = conditional.if_true.as_slice() else {
            return None;
        };
        let grant = structural_unwrap_render_wrappers(grant_effect)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
            return None;
        };
        if grant.until != Until::Forever
            || grant.condition.is_some()
            || !grant.additional_modifications.is_empty()
            || !grant.runtime_modifications.is_empty()
            || grant.source_type.is_some()
            || grant.source_reference_surface.is_some()
            || grant.set_quantifier_surface.is_some()
            || grant.type_retention_surface.is_some()
            || grant.animation_pt_surface.is_some()
            || grant.animation_duration_surface.is_some()
            || grant.lock_filter_at_resolution
            || grant.resolve_set_pt_values_at_resolution
            || grant.require_creature_target
            || !matches!(
                grant.target_spec.as_ref().map(ChooseSpec::unhinted),
                Some(ChooseSpec::Source)
            )
            || ability.protection_from() != Some(&crate::ability::ProtectionFrom::Color(color))
        {
            return None;
        }
        seen_colors.push(color);
    }
    if expected_colors
        .iter()
        .any(|expected| seen_colors.iter().filter(|seen| *seen == expected).count() != 1)
    {
        return None;
    }

    Some(
        "it becomes a 4/4 Giant creature with protection from each of that spell's colors"
            .to_string(),
    )
}

/// The recipient selected by one coordinated action whose following life
/// gain belongs to the effect controller instead.
enum CoordinatedActionRecipient<'a> {
    Damage(&'a ChooseSpec),
    Player(&'a PlayerFilter),
}

fn declared_target_matches_player(
    declared_target: &ChooseSpec,
    action_player: &PlayerFilter,
) -> bool {
    let ChooseSpec::Target(declared) = declared_target.unhinted() else {
        return false;
    };
    let ChooseSpec::Player(declared_player) = declared.unhinted() else {
        return false;
    };
    matches!(
        action_player,
        PlayerFilter::Target(action_player) | PlayerFilter::AliasedTarget(action_player)
            if declared_player == action_player.as_ref()
    )
}

/// Render one coordinated action followed by life gain for its controller.
/// The two effects have different grammatical subjects, so the generic
/// coordinated-clause joiner must not elide `you` from the second action. An
/// implicit target declaration is accepted only when it exactly matches the
/// action's typed recipient.
fn describe_coordinated_action_then_you_gain_life(effects: &[Effect]) -> Option<String> {
    let (target_only, action_effect, gain_effect) = match effects {
        [action_effect, gain_effect] => (None, action_effect, gain_effect),
        [target_effect, action_effect, gain_effect] => {
            let target = structural_unwrap_render_wrappers(target_effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            if target.explicit_declaration || target.chooser.is_some() {
                return None;
            }
            (Some(target), action_effect, gain_effect)
        }
        _ => return None,
    };
    let rendered_action = structural_unwrap_render_wrappers(action_effect);
    let action_recipient = if let Some(with_source) =
        rendered_action.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
    {
        let damage = structural_unwrap_render_wrappers(&with_source.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()?;
        CoordinatedActionRecipient::Damage(&damage.target)
    } else if let Some(damage) = rendered_action.downcast_ref::<crate::effects::DealDamageEffect>()
    {
        CoordinatedActionRecipient::Damage(&damage.target)
    } else if let Some(mill) = rendered_action.downcast_ref::<crate::effects::MillEffect>() {
        CoordinatedActionRecipient::Player(&mill.player)
    } else if let Some(discard) = rendered_action.downcast_ref::<crate::effects::DiscardEffect>() {
        CoordinatedActionRecipient::Player(&discard.player)
    } else {
        return None;
    };
    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::GainLifeEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    match (target_only, action_recipient) {
        (Some(target), CoordinatedActionRecipient::Damage(action_target))
            if &target.target == action_target => {}
        (Some(target), CoordinatedActionRecipient::Player(action_player))
            if declared_target_matches_player(&target.target, action_player) => {}
        (None, CoordinatedActionRecipient::Damage(_)) => {}
        _ => return None,
    }

    let action_text = describe_effect(action_effect);
    let action_text = action_text.trim().trim_end_matches('.');
    let gain_text = describe_effect(gain_effect);
    let gain_text = gain_text.trim().trim_end_matches('.');
    let gain_tail = gain_text
        .strip_prefix("You ")
        .or_else(|| gain_text.strip_prefix("you "))
        .unwrap_or(gain_text);
    let gain_tail = lowercase_first(gain_tail);
    if !gain_tail.starts_with("gain ") || action_text.contains(". ") || gain_tail.contains(". ") {
        return None;
    }

    let (action_head, action_basis) = action_text
        .split_once(", where X is ")
        .map_or((action_text, None), |(head, basis)| (head, Some(basis)));
    let (gain_head, gain_basis) = gain_tail
        .split_once(", where X is ")
        .map_or((gain_tail.as_str(), None), |(head, basis)| {
            (head, Some(basis))
        });
    let basis = match (action_basis, gain_basis) {
        (Some(left), Some(right)) if left == right => Some(left),
        (Some(basis), None) | (None, Some(basis)) => Some(basis),
        (None, None) => None,
        _ => return None,
    };
    Some(format!(
        "{} and you {gain_head}{}",
        capitalize_first(action_head),
        basis.map_or_else(String::new, |basis| format!(", where X is {basis}"))
    ))
}

pub(super) fn describe_coordinated_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::CommaThen
            | ironsmith_core::SequenceSurface::RepeatedCommaThen
    ) {
        return describe_comma_then_sequence(sequence);
    }
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::ResultConjunction {
            leading_duration: false
        }
    ) && describe_untap_attacking_then_additional_combat(&sequence.effects).is_some()
    {
        return Some(
            "Untap all attacking creatures and after this phase, there is an additional combat phase"
                .to_string(),
        );
    }
    let leading_duration = matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::CoordinatedLeadingDuration
            | ironsmith_core::SequenceSurface::ResultConjunction {
                leading_duration: true
            }
    );
    if let Some(rendered) = describe_next_turn_pt_modifier_and_activation_lock(sequence) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_single_dynamic_base_pt_leading_duration(sequence) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_shared_target_end_of_turn_modifications(sequence) {
        return Some(rendered);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(rendered) = describe_explicit_you_three_action_sequence(&sequence.effects)
    {
        return Some(rendered);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(rendered) = describe_coordinated_action_then_you_gain_life(&sequence.effects)
    {
        return Some(rendered);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(rendered) = describe_you_life_change_and_exile_top(&sequence.effects)
    {
        return Some(rendered);
    }
    // A target-damage + correlated fanout pair is more specific than the
    // broad action-fanout and coordinated-damage renderers below. Prove the
    // shared target tag and amount before either child is rendered alone,
    // because doing so repeats the damage count on the second recipient set.
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_target_creature_damage_fanout_pair(first, second)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let [return_root, venture_root] = sequence.effects.as_slice()
        && let Some(return_to_hand) =
            return_root.downcast_ref::<crate::effects::ReturnToHandEffect>()
        && let Some(venture) =
            venture_root.downcast_ref::<crate::effects::VentureIntoDungeonEffect>()
        && matches!(return_to_hand.spec.unhinted(), ChooseSpec::Source)
        && return_to_hand.actor_surface.is_none()
        && return_to_hand.destination_player_surface.is_none()
        && return_to_hand.exiled_with_source_surface.is_none()
        && return_to_hand.set_quantifier_surface.is_none()
        && return_to_hand.set_reference_surface.is_none()
        && venture.player == PlayerFilter::You
    {
        let returned = describe_effect(return_root);
        let returned = returned.trim().trim_end_matches('.');
        if !returned.is_empty() && !returned.contains(". ") {
            return Some(format!("{returned} and venture into the dungeon"));
        }
    }
    if let Some((compact, consumed)) = describe_linked_target_set_followup_prefix(&sequence.effects)
        .or_else(|| describe_same_name_exile_then_investigate_prefix(&sequence.effects))
        .or_else(|| describe_target_same_name_action_fanout_prefix(&sequence.effects))
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
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_action_and_get_energy_pair(first, second)
    {
        return Some(compact);
    }
    // Target declarations coordinated with multiple actions are retained as
    // a singleton `SequenceEffect`, so the ordinary effect-list compactor
    // never sees this exact target/loss/gain triple. Reuse its fully typed
    // matcher here: it proves one target-player declaration, the same target
    // consumer, and one shared life amount before folding the authored X
    // basis to the end of the clause.
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let [target_effect, lose_effect, gain_effect] = sequence.effects.as_slice()
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(gain) = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()
        && let Some(compact) =
            describe_target_player_lose_then_you_gain_life(target_only, lose, gain)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let [target_effect, draw_effect, lose_effect] = sequence.effects.as_slice()
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && target_only.chooser.is_none()
        && !target_only.explicit_declaration
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(compact) = describe_target_player_draw_then_lose_life(draw, target_only, lose)
    {
        return Some(compact);
    }
    // A target announced solely inside a shared dynamic value remains an
    // executable TargetOnly sibling of the coordinated actions. Keep that
    // declaration silent after proving both actions use the same count and
    // that the count is scoped to the announced player's objects.
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let [target_effect, draw_effect, lose_effect] = sequence.effects.as_slice()
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && target_only.target == ChooseSpec::target_player()
        && target_only.chooser.is_none()
        && !target_only.explicit_declaration
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Value::Count(counted) = draw.count.unhinted()
        && counted.owner.as_ref() == Some(&PlayerFilter::target_player())
        && let Some(compact) = describe_draw_then_lose_life(draw, lose)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let [first, second] = sequence.effects.as_slice()
        && let Some(compact) =
            describe_each_opponent_damage_then_controller_gain_shared_x(first, second)
    {
        return Some(compact);
    }
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Coordinated
            | ironsmith_core::SequenceSurface::ResultConjunction {
                leading_duration: false
            }
    ) && let Some(compact) = describe_source_sacrifice_then_coordinated_suffix(&sequence.effects)
    {
        return Some(compact);
    }
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Sequential
    ) {
        return None;
    }
    if let Some(compact) = describe_return_target_and_attached_objects_to_owners(&sequence.effects)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(compact) =
            describe_coordinated_mill_then_collection_selection(&sequence.effects)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let [look_effect, exile_effect, grant_effect] = sequence.effects.as_slice()
        && let Some(look) = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(exile) = exile_effect.downcast_ref::<crate::effects::ExileEffect>()
        && let Some(grant) = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(compact) =
            describe_look_at_top_exile_face_down_then_play_while_exiled(look, exile, grant)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(compact) = describe_independent_explicit_may_coordination(&sequence.effects)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(compact) = describe_each_opponent_then_you_draw_and_gain(&sequence.effects)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(compact) =
            describe_coordinated_draw_then_pump_and_grant_same_filter(&sequence.effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_gain_life_then_put_same_x_counters(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_gain_life_then_distribute_creatures_died_counters(&sequence.effects)
    {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(compact) =
            describe_sacrifice_then_controller_chosen_attachment(&sequence.effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_target_and_shared_color_inline_ability_grant(&sequence.effects)
    {
        return Some(compact);
    }
    // This pair is a shared source subject, not a generic target/action
    // fanout. Select it before the broad continuous-effect coordinator so
    // the following damage verb stays conjugated ("deals", not "deal").
    if let Some(compact) =
        describe_coordinated_source_continuous_then_damage_controller(&sequence.effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_target_cant_block_then_add_subtypes(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_source_animation_then_unblockable(&sequence.effects) {
        return Some(compact);
    }
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::ResultConjunction {
            leading_duration: false
        }
    ) && let Some(compact) = describe_tagged_untap_all_then_additional_combat(&sequence.effects)
    {
        return Some(compact);
    }
    if let Some(compact) =
        describe_source_animation_with_triggering_spell_color_protection(&sequence.effects)
    {
        return Some(compact);
    }
    let visible_effects = sequence
        .effects
        .iter()
        .filter(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
    if let Some(compact) =
        describe_declared_target_for_each_pump_unblockable_bundle(&visible_effects)
    {
        return Some(compact);
    }
    if let [target, first, second] = visible_effects.as_slice()
        && let Some(compact) = describe_shared_target_trample_mana_value_pump(target, first, second)
    {
        return Some(compact);
    }
    if let [target, grant, pump] = visible_effects.as_slice()
        && let Some(compact) =
            describe_shared_declared_target_grant_then_pt_pump(target, grant, pump)
    {
        return Some(compact);
    }
    if let [first, second] = visible_effects.as_slice()
        && let Some(compact) = describe_target_continuous_fanout_pair(first, second)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_target_player_source_control_transfer(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_declared_target_player_draw_fanout(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_declared_target_joint_draw(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_target_player_cast_and_creatures_attack_restrictions(&sequence.effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_coordinated_target_player_cast_and_activation_restrictions(
        &sequence.effects,
        leading_duration,
    ) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_coordinated_control_and_lock(&sequence.effects, leading_duration)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_you_lose_life_and_draw_additional(&sequence.effects) {
        return Some(capitalize_first(&compact));
    }
    if let [first, second] = sequence.effects.as_slice() {
        let pair = [first, second];
        if let Some(compact) = describe_target_pump_unblockable_bundle(&pair) {
            return Some(compact);
        }
    }
    if let Some(compact) = describe_coordinated_draw_followup(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_source_control_transfer_then_untap(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_damage_and_you_scry(&sequence.effects) {
        return Some(compact);
    }
    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
        && let Some(compact) = describe_you_life_change_and_create_token(&sequence.effects)
    {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice() {
        if let Some(draw) = first.downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(lose) = second.downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(compact) = describe_draw_then_lose_life(draw, lose)
        {
            return Some(compact);
        }
    }
    if let Some(compact) =
        describe_coordinated_put_counters_then_grant_same_filter(&sequence.effects)
    {
        return Some(compact);
    }
    let effect_refs = sequence.effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_tagged_pump_then_conditional_keyword(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_coordinated_create_token_then_grant_same_tag(&sequence.effects)
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
        && let Some(compact) =
            describe_player_or_planeswalker_damage_then_controlled_creature_damage(first, second)
    {
        return Some(compact);
    }
    describe_leading_then_coordinated_same_object_modifiers(&sequence.effects)
        .or_else(|| describe_coordinated_create_token_then_grant_same_tag(&sequence.effects))
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
        .or_else(|| describe_coordinated_permanent_same_object_modifiers(&sequence.effects))
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
        .or_else(|| describe_coordinated_owned_commanders_to_hand(&sequence.effects))
        .or_else(|| describe_coordinated_return_to_hand_shared_destination(&sequence.effects))
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
        .or_else(|| describe_leading_duration_typed_fallback(sequence))
        .or_else(|| describe_typed_coordinated_clause_fallback(&sequence.effects))
}

/// Preserve the authored leading duration for one global dynamic base-P/T
/// modifier. The continuous-effect renderer deliberately describes a global
/// filter as an `Each ... has` sentence; the sequence surface proves that the
/// source instead authored a leading duration over the plural set.
fn describe_single_dynamic_base_pt_leading_duration(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::CoordinatedLeadingDuration
        || sequence.result_label.is_some()
    {
        return None;
    }
    let [effect] = sequence.effects.as_slice() else {
        return None;
    };
    let apply = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || apply.source_type.is_some()
        || apply.source_reference_surface.is_some()
        || apply.set_quantifier_surface.is_some()
        || apply.type_retention_surface.is_some()
        || apply.animation_pt_surface.is_some()
        || apply.animation_duration_surface.is_some()
        || apply.lock_filter_at_resolution
    {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(filter) = &apply.target else {
        return None;
    };
    if apply.target_spec.as_ref().is_some_and(
        |spec| !matches!(spec.unhinted(), ChooseSpec::Object(spec_filter) if spec_filter == filter),
    ) {
        return None;
    }
    let Some(crate::continuous::Modification::SetPowerToughness {
        power,
        toughness,
        sublayer: crate::continuous::PtSublayer::Setting,
    }) = apply.modification.as_ref()
    else {
        return None;
    };
    if power.unhinted() != toughness.unhinted()
        || !power.has_surface_hint(ValueSurfaceHint::WhereXIs)
    {
        return None;
    }
    let where_x = describe_where_x_basis(power)?;
    let subject = pluralize_noun_phrase(strip_leading_article(
        &describe_object_filter_with_fixed_pt_shorthand(filter),
    ));
    Some(format!(
        "Until end of turn, {subject} have base power and toughness X/X, where X is {where_x}"
    ))
}

fn describe_leading_duration_typed_fallback(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::CoordinatedLeadingDuration {
        return None;
    }
    let rendered = describe_typed_coordinated_clause_fallback(&sequence.effects)?;
    let rendered = rendered.trim().trim_end_matches('.');
    let body = rendered.strip_suffix(" until end of turn")?;
    Some(format!("Until end of turn, {}", lowercase_first(body)))
}

fn describe_next_turn_pt_modifier_and_activation_lock(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::CoordinatedLeadingDuration {
        return None;
    }
    let [modifier_effect, target_effect, restriction_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let modifier_tagged = modifier_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let modifier = modifier_tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let target = modifier.target_spec.as_ref()?;
    let ChooseSpec::Object(target_filter) = target.base() else {
        return None;
    };
    let mut semantic_target_filter = target_filter.clone();
    semantic_target_filter.set_explicit_card_type_noun(None);
    if semantic_target_filter != ObjectFilter::creature()
        || !target.is_target()
        || target.count() != crate::effect::ChoiceCount::up_to(1)
        || modifier.until != Until::YourNextTurn
        || modifier.condition.is_some()
        || modifier.modification.is_some()
        || !modifier.additional_modifications.is_empty()
        || !matches!(
            modifier.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ModifyPowerToughness { .. }]
        )
        || modifier.source_type.is_some()
        || modifier.source_reference_surface.is_some()
        || modifier.set_quantifier_surface.is_some()
        || modifier.type_retention_surface.is_some()
        || modifier.animation_pt_surface.is_some()
        || modifier.animation_duration_surface.is_some()
        || modifier.lock_filter_at_resolution
        || modifier.resolve_set_pt_values_at_resolution
        || !modifier.require_creature_target
    {
        return None;
    }

    let target_tagged = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = target_tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target_only.chooser.is_some()
        || target_only.explicit_declaration
        || !matches!(
            target_only.target.unhinted(),
            ChooseSpec::Tagged(tag) if tag == &modifier_tagged.tag
        )
    {
        return None;
    }

    let restriction = restriction_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::ActivateAbilitiesOf(restricted_filter) =
        &restriction.restriction
    else {
        return None;
    };
    if !filter_is_exactly_tagged(restricted_filter, &modifier_tagged.tag)
        || restriction.duration != Until::YourNextTurn
        || restriction.start != crate::effect::RestrictionStart::Immediate
        || restriction.duration_surface
            != crate::effect::RestrictionDurationSurface::LeadingUntilYourNextTurn
    {
        return None;
    }

    let modifier_text = describe_effect(modifier_effect);
    let modifier_text = modifier_text
        .trim()
        .trim_end_matches('.')
        .strip_suffix(" until your next turn")?;
    Some(format!(
        "Until your next turn, {} and its activated abilities can't be activated",
        lowercase_first(modifier_text)
    ))
}

pub(super) fn describe_shared_target_end_of_turn_modifications(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if !matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Coordinated
            | ironsmith_core::SequenceSurface::CoordinatedLeadingDuration
    ) || sequence.effects.len() < 2
    {
        return None;
    }
    let first_tagged = sequence.effects[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let first = first_tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let target = first.target_spec.as_ref()?;
    if !target.is_target() || first.until != Until::EndOfTurn || first.condition.is_some() {
        return None;
    }
    for effect in &sequence.effects[1..] {
        let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
        let apply = tagged
            .effect
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        if apply.target_spec.as_ref().map(ChooseSpec::unhinted)
            != Some(ChooseSpec::Tagged(first_tagged.tag.clone()).unhinted())
            || apply.until != Until::EndOfTurn
            || apply.condition.is_some()
        {
            return None;
        }
    }

    let strip_duration = |text: String| {
        let text = text.trim().trim_end_matches('.');
        if let Some((body, where_clause)) = split_coordinated_duration(text, &Until::EndOfTurn) {
            let where_clause = if where_clause == ", where X is X" {
                ""
            } else {
                where_clause.as_str()
            };
            format!("{body}{where_clause}")
        } else {
            text.to_string()
        }
    };
    let mut clauses = Vec::with_capacity(sequence.effects.len());
    let mut first_clause = strip_duration(describe_effect(&sequence.effects[0]));
    if target.count().is_any_number() && !first_clause.contains(" each ") {
        for predicate in [" get ", " gain ", " become ", " lose ", " have "] {
            if let Some((head, tail)) = first_clause.split_once(predicate) {
                first_clause = format!("{head} each{predicate}{tail}");
                break;
            }
        }
    }
    let mut first_chars = first_clause.chars();
    let first_char = first_chars.next()?.to_lowercase().collect::<String>();
    clauses.push(format!("{first_char}{}", first_chars.as_str()));
    for effect in &sequence.effects[1..] {
        let clause = strip_duration(describe_effect(effect));
        let clause = clause
            .strip_prefix("It ")
            .or_else(|| clause.strip_prefix("it "))
            .unwrap_or(&clause);
        let clause = clause
            .strip_prefix("gains can attack ")
            .map(|tail| format!("can attack {tail}"))
            .unwrap_or_else(|| clause.to_string());
        let clause = if target.count().is_any_number() {
            [
                ("gains ", "gain "),
                ("gets ", "get "),
                ("has ", "have "),
                ("loses ", "lose "),
                ("becomes ", "become "),
            ]
            .into_iter()
            .find_map(|(singular, plural)| {
                clause
                    .strip_prefix(singular)
                    .map(|tail| format!("{plural}{tail}"))
            })
            .unwrap_or(clause)
        } else {
            clause
        };
        clauses.push(clause);
    }
    if clauses.len() >= 2 {
        let penultimate = clauses.len() - 2;
        if clauses[penultimate].starts_with("gains \"") && clauses[penultimate].ends_with(".\"") {
            let shortened_len = clauses[penultimate].len() - 2;
            clauses[penultimate].truncate(shortened_len);
            clauses[penultimate].push_str(",\"");
        }
    }
    let final_clause = clauses.pop()?;
    let body = if clauses.len() == 1 {
        format!("{} and {final_clause}", clauses[0])
    } else {
        format!("{}, and {final_clause}", clauses.join(", "))
    };
    if sequence.surface == ironsmith_core::SequenceSurface::CoordinatedLeadingDuration {
        Some(format!("Until end of turn, {body}"))
    } else {
        Some(format!("{} until end of turn", capitalize_first(&body)))
    }
}

#[cfg(test)]
mod coordinated_sequence_tests {
    use super::*;

    fn next_turn_modifier_and_activation_lock(
        modifier_tag: &str,
        restriction_tag: &str,
        duration: Until,
    ) -> Effect {
        let mut creature = ObjectFilter::creature();
        creature.set_explicit_card_type_noun(Some(CardType::Creature));
        let target = ChooseSpec::target(ChooseSpec::Object(creature))
            .with_count(crate::effect::ChoiceCount::up_to(1));
        let modifier = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec_runtime(
                target,
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: Value::Fixed(-3),
                    toughness: Value::Fixed(0),
                },
                Until::YourNextTurn,
            )
            .require_creature_target(),
        )
        .tag(modifier_tag);
        let target_only = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::Tagged(
            TagKey::from(modifier_tag),
        )))
        .tag("targeted_1");
        let restriction = Effect::new(
            crate::effects::CantEffect::new(
                crate::effect::Restriction::activate_abilities_of(ObjectFilter::tagged(
                    restriction_tag,
                )),
                duration,
            )
            .with_duration_surface(
                crate::effect::RestrictionDurationSurface::LeadingUntilYourNextTurn,
            ),
        );
        Effect::new(
            crate::effects::SequenceEffect::coordinated_with_leading_duration(vec![
                modifier,
                target_only,
                restriction,
            ]),
        )
    }

    #[test]
    fn next_turn_modifier_and_activation_lock_requires_one_correlated_target() {
        let exact =
            next_turn_modifier_and_activation_lock("pumped_0", "pumped_0", Until::YourNextTurn);
        assert_eq!(
            describe_effect(&exact),
            "Until your next turn, up to one target creature gets -3/-0 and its activated abilities can't be activated"
        );

        let wrong_tag =
            next_turn_modifier_and_activation_lock("pumped_0", "another_0", Until::YourNextTurn);
        assert_ne!(describe_effect(&wrong_tag), describe_effect(&exact));

        let wrong_duration =
            next_turn_modifier_and_activation_lock("pumped_0", "pumped_0", Until::EndOfTurn);
        assert_ne!(describe_effect(&wrong_duration), describe_effect(&exact));
    }

    fn coordinated_damage_life_sequence(
        announced_target: ChooseSpec,
        damage_target: ChooseSpec,
        gain_player: PlayerFilter,
    ) -> Effect {
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(announced_target)),
            Effect::new(crate::effects::DealDamageEffect::new(
                Value::Fixed(4),
                damage_target,
            )),
            Effect::new(crate::effects::GainLifeEffect::with_filter(
                Value::Fixed(2),
                gain_player,
            )),
        ]))
    }

    #[test]
    fn coordinated_damage_and_life_gain_keeps_the_changed_subject() {
        let exact = coordinated_damage_life_sequence(
            ChooseSpec::AnyTarget,
            ChooseSpec::AnyTarget,
            PlayerFilter::You,
        );
        assert_eq!(
            describe_effect(&exact),
            "Deal 4 damage to any target and you gain 2 life"
        );

        let mismatched_target = coordinated_damage_life_sequence(
            ChooseSpec::AnyTarget,
            ChooseSpec::target_player(),
            PlayerFilter::You,
        );
        assert_ne!(describe_effect(&mismatched_target), describe_effect(&exact));

        let wrong_player = coordinated_damage_life_sequence(
            ChooseSpec::AnyTarget,
            ChooseSpec::AnyTarget,
            PlayerFilter::Opponent,
        );
        assert_ne!(describe_effect(&wrong_player), describe_effect(&exact));

        let source_wrapped_effects = vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::AnyTarget)),
            Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(TagKey::from("triggering")),
                Effect::new(crate::effects::DealDamageEffect::new(
                    Value::Fixed(1),
                    ChooseSpec::AnyTarget,
                )),
            )),
            Effect::new(crate::effects::GainLifeEffect::you(Value::Fixed(1))),
        ];
        assert!(
            describe_coordinated_action_then_you_gain_life(&source_wrapped_effects).is_some(),
            "{source_wrapped_effects:#?}\ndamage={}\ngain={}",
            describe_effect(&source_wrapped_effects[1]),
            describe_effect(&source_wrapped_effects[2]),
        );
        let source_wrapped = Effect::new(crate::effects::SequenceEffect::coordinated(
            source_wrapped_effects,
        ));
        let rendered = describe_effect(&source_wrapped);
        assert!(rendered.contains(" and you gain 1 life"), "{rendered}");
        assert!(!rendered.contains(", and gain"), "{rendered}");
    }

    fn coordinated_player_action_life_sequence(
        announced_target: ChooseSpec,
        action: Effect,
    ) -> Effect {
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(announced_target)),
            action,
            Effect::new(crate::effects::GainLifeEffect::you(Value::X)),
        ]))
    }

    #[test]
    fn coordinated_target_player_mill_and_life_gain_keep_the_changed_subject() {
        let exact = coordinated_player_action_life_sequence(
            ChooseSpec::target_player(),
            Effect::new(crate::effects::MillEffect::new(
                Value::X,
                PlayerFilter::target_player(),
            )),
        );
        assert_eq!(
            describe_effect(&exact),
            "Target player mills X cards and you gain X life"
        );

        let mismatched_target = coordinated_player_action_life_sequence(
            ChooseSpec::target_opponent(),
            Effect::new(crate::effects::MillEffect::new(
                Value::X,
                PlayerFilter::target_player(),
            )),
        );
        assert_ne!(describe_effect(&mismatched_target), describe_effect(&exact));
    }

    #[test]
    fn coordinated_target_opponent_discard_and_life_gain_keep_the_changed_subject() {
        let exact = coordinated_player_action_life_sequence(
            ChooseSpec::target_opponent(),
            Effect::new(crate::effects::DiscardEffect::new_with_filter(
                Value::Fixed(2),
                PlayerFilter::target_opponent(),
                false,
                None,
            )),
        );
        assert_eq!(
            describe_effect(&exact),
            "Target opponent discards two cards and you gain X life"
        );

        let wrong_actor = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_opponent(),
            )),
            Effect::new(crate::effects::DiscardEffect::new_with_filter(
                Value::Fixed(2),
                PlayerFilter::target_opponent(),
                false,
                None,
            )),
            Effect::new(crate::effects::GainLifeEffect::with_filter(
                Value::X,
                PlayerFilter::Opponent,
            )),
        ]));
        assert_ne!(describe_effect(&wrong_actor), describe_effect(&exact));
    }

    fn tagged_power(tag: &str) -> Value {
        Value::PowerOf(Box::new(ChooseSpec::Tagged(crate::TagKey::from(tag))))
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
    }

    fn tagged_power_with_it_surface(tag: &str) -> Value {
        Value::PowerOf(Box::new(
            ChooseSpec::Tagged(crate::TagKey::from(tag)).with_surface_hint(
                crate::target::ChooseSpecSurfaceHint::SourceReference(
                    crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
                ),
            ),
        ))
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
    }

    fn target_player_life_sequence(loss: Value, gain: Value) -> Effect {
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_player(),
            )),
            Effect::new(crate::effects::LoseLifeEffect::with_filter(
                loss,
                PlayerFilter::target_player(),
            )),
            Effect::new(crate::effects::GainLifeEffect::you(gain)),
        ]))
    }

    fn target_player_graveyard_creature_count(owner: PlayerFilter) -> Value {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(owner);
        filter.card_types = vec![CardType::Creature];
        Value::Count(filter).with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
    }

    fn target_scoped_draw_and_lose(owner: PlayerFilter) -> Effect {
        let count = target_player_graveyard_creature_count(owner);
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_player(),
            )),
            Effect::new(crate::effects::DrawCardsEffect::you(count.clone())),
            Effect::new(crate::effects::LoseLifeEffect::you(count)),
        ]))
    }

    #[test]
    fn target_declared_only_in_shared_count_is_not_rendered_as_choose_prelude() {
        assert_eq!(
            describe_effect(&target_scoped_draw_and_lose(PlayerFilter::target_player())),
            "You draw X cards and you lose X life, where X is the number of creature cards in target player's graveyard"
        );
    }

    #[test]
    fn unrelated_owner_count_keeps_the_executable_target_declaration_visible() {
        assert_ne!(
            describe_effect(&target_scoped_draw_and_lose(PlayerFilter::You)),
            "You draw X cards and you lose X life, where X is the number of creature cards in target player's graveyard"
        );
    }

    #[test]
    fn coordinated_target_player_life_pair_shares_one_tagged_power_basis() {
        let amount = tagged_power_with_it_surface("triggering");
        assert_eq!(
            describe_effect(&target_player_life_sequence(amount.clone(), amount)),
            "Target player loses X life and you gain X life, where X is its power"
        );
    }

    #[test]
    fn generic_tagged_power_keeps_explicit_creature_antecedent() {
        let amount = tagged_power("triggering");
        assert_eq!(
            describe_effect(&target_player_life_sequence(amount.clone(), amount)),
            "Target player loses X life and you gain X life, where X is that creature's power"
        );
    }

    #[test]
    fn coordinated_target_player_life_pair_rejects_mismatched_lki_tags() {
        let rendered = describe_effect(&target_player_life_sequence(
            tagged_power("triggering"),
            tagged_power("other"),
        ));
        assert_ne!(
            rendered,
            "Target player loses X life and you gain X life, where X is that creature's power"
        );
    }

    #[test]
    fn coordinated_target_player_life_pair_rejects_unequal_amounts() {
        let target = crate::effects::TargetOnlyEffect::new(ChooseSpec::target_player());
        let lose = crate::effects::LoseLifeEffect::with_filter(
            tagged_power("triggering"),
            PlayerFilter::target_player(),
        );
        let gain = crate::effects::GainLifeEffect::you(Value::Fixed(2));

        assert_eq!(
            describe_target_player_lose_then_you_gain_life(&target, &lose, &gain),
            None,
            "the shared-X surface must not claim unequal executable life amounts"
        );
    }

    #[test]
    fn repeated_explicit_may_clauses_keep_the_independent_clause_comma() {
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::MayEffect::new(vec![Effect::draw(
                Value::Fixed(1),
            )])),
            Effect::new(crate::effects::MayEffect::new(vec![Effect::gain_life(1)])),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "You may draw a card, and you may gain 1 life"
        );
    }

    #[test]
    fn exactly_three_independent_clauses_use_the_oracle_list_conjunction() {
        let triggering = TagKey::from("triggering");
        let counters = Effect::new(crate::effects::ForEachObject::new(
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
            vec![Effect::new(crate::effects::PutCountersEffect::new(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            ))],
        ))
        .tag("counters_0");
        let effects = vec![
            Effect::new(crate::effects::SacrificeTargetEffect::new(
                ChooseSpec::Tagged(triggering),
            )),
            Effect::draw(Value::Fixed(1)),
            counters,
        ];

        assert_eq!(
            describe_typed_coordinated_clause_fallback(&effects).as_deref(),
            Some("Sacrifice it, draw a card, and put a +1/+1 counter on each creature you control")
        );
        assert_eq!(
            describe_effect(&Effect::new(crate::effects::SequenceEffect::coordinated(
                effects
            ))),
            "Sacrifice it, draw a card, and put a +1/+1 counter on each creature you control"
        );
    }

    #[test]
    fn coordinated_fallback_preserves_two_children_and_rejects_multisentence_third_child() {
        let pair = vec![Effect::draw(Value::Fixed(1)), Effect::gain_life(1)];
        assert_eq!(
            describe_typed_coordinated_clause_fallback(&pair).as_deref(),
            Some("Draw a card and you gain 1 life")
        );

        let nested = Effect::new(crate::effects::SequenceEffect::new(vec![
            Effect::draw(Value::Fixed(1)),
            Effect::gain_life(1),
        ]));
        let near_miss = vec![
            Effect::sacrifice_source(),
            Effect::draw(Value::Fixed(1)),
            nested,
        ];
        assert_eq!(describe_typed_coordinated_clause_fallback(&near_miss), None);
    }

    #[test]
    fn source_control_transfer_then_untap_conjugates_the_shared_player_subject() {
        let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
            crate::continuous::EffectTarget::Source,
            crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                PlayerFilter::Attacking,
            ),
            Until::Forever,
        );
        control.target_spec = Some(ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this artifact".to_string(),
                ),
            ),
        ));
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(control).tag(TagKey::from("controlled")),
            Effect::untap(ChooseSpec::Source).tag(TagKey::from("untapped")),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "The attacking player gains control of this artifact and untaps it"
        );
    }

    #[test]
    fn leading_if_consequence_stays_in_the_same_lowercase_clause() {
        let conditional = Effect::new(crate::effects::ConditionalEffect::new(
            Condition::ValueComparison {
                left: Value::Fixed(5),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(5),
            },
            vec![Effect::sacrifice_source()],
            Vec::new(),
        ));

        let rendered = describe_effect(&conditional);
        assert!(rendered.contains(", sacrifice this source"), "{rendered}");
    }

    #[test]
    fn coordinated_non_target_exiles_share_the_authored_verb() {
        let human = ObjectFilter::default()
            .with_subtype(crate::types::Subtype::Human)
            .you_control()
            .in_zone(Zone::Battlefield);
        let artifact = ObjectFilter::artifact()
            .you_control()
            .in_zone(Zone::Battlefield);
        let exile = |filter| {
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Object(filter).with_count(crate::effect::ChoiceCount::exactly(1)),
                Zone::Exile,
                true,
            ))
        };
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            exile(human),
            exile(artifact),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Exile a Human you control and an artifact you control"
        );
    }

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
            "Deal 2 damage to target creature and 2 damage to target player or planeswalker"
        );
    }

    #[test]
    fn coordinated_opponent_damage_and_life_gain_share_one_dynamic_amount() {
        let amount = Value::Count(
            ObjectFilter::creature()
                .in_zone(Zone::Battlefield)
                .controlled_by(PlayerFilter::You),
        )
        .with_surface_hint(ValueSurfaceHint::WhereXIs);
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::for_players(
                PlayerFilter::Opponent,
                vec![Effect::deal_damage(
                    amount.clone(),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                )],
            ),
            Effect::gain_life(amount),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "it deals X damage to each opponent and you gain X life, where X is the number of creatures you control"
        );
    }

    #[test]
    fn coordinated_tagged_damage_and_life_gain_share_one_dynamic_amount() {
        let amount = Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You)
                .with_ability_marker("cycling"),
        )
        .with_surface_hint(ValueSurfaceHint::WhereXIs);
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::deal_damage(amount.clone(), ChooseSpec::AnyTarget)
                .tag(TagKey::from("damaged_0")),
            Effect::gain_life(amount),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Deal X damage to any target and you gain X life, where X is the number of cards with cycling in your graveyard"
        );
    }

    #[test]
    fn permanent_same_subject_continuous_chain_renders_each_typed_action() {
        let filter = ObjectFilter::creature()
            .controlled_by(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)));
        let remove = Effect::new(
            crate::effects::ApplyContinuousEffect::new_runtime(
                crate::continuous::EffectTarget::Filter(filter.clone()),
                crate::effects::continuous::RuntimeModification::RemoveAllAbilities,
                Until::Forever,
            )
            .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each))
            .lock_filter_at_resolution(),
        );
        let add_subtype = Effect::new(crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Filter(filter.clone()),
            crate::continuous::Modification::AddSubtypes(vec![crate::types::Subtype::Coward]),
            Until::Forever,
        ));
        let set_pt = Effect::new(crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Filter(filter),
            crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(1),
                toughness: Value::Fixed(1),
                sublayer: crate::continuous::PtSublayer::Setting,
            },
            Until::Forever,
        ));
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
                ChooseSpec::Player(PlayerFilter::Opponent),
            ))),
            remove,
            add_subtype,
            set_pt,
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1"
        );
    }

    #[test]
    fn mixed_cant_block_and_subtype_chain_folds_the_shared_target_declaration() {
        let target_tag = TagKey::from("targeted_0");
        let target_spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
        let target =
            Effect::new(crate::effects::TargetOnlyEffect::new(target_spec)).tag(target_tag.clone());
        let cant = Effect::new(crate::effects::CantEffect::until_end_of_turn(
            crate::effect::Restriction::block(ObjectFilter::tagged(target_tag.clone())),
        ));
        let subtype = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(target_tag),
            crate::continuous::Modification::AddSubtypes(vec![crate::types::Subtype::Coward]),
            Until::EndOfTurn,
        ))
        .tag("subtyped_1");
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            target, cant, subtype,
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Target creature can't block this turn and becomes a Coward in addition to its other types until end of turn"
        );
    }

    #[test]
    fn source_animation_and_unblockable_chain_keeps_all_characteristics() {
        let mut animation = crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Source,
            crate::continuous::Modification::AddCardTypes(vec![
                CardType::Artifact,
                CardType::Creature,
            ]),
            Until::EndOfTurn,
        )
        .with_source_reference_surface(crate::target::SourceReferenceSurface::ThisPermanentType(
            "artifact".to_string(),
        ))
        .with_animation_pt_surface(Some(
            ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
        ));
        animation.additional_modifications.extend([
            crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(2),
                toughness: Value::Fixed(2),
                sublayer: crate::continuous::PtSublayer::Setting,
            },
            crate::continuous::Modification::SetColors(
                crate::color::ColorSet::BLUE.union(crate::color::ColorSet::BLACK),
            ),
            crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature,
            ),
            crate::continuous::Modification::AddSubtypes(vec![crate::types::Subtype::Horror]),
        ]);
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(animation),
            Effect::new(crate::effects::CantEffect::until_end_of_turn(
                crate::effect::Restriction::be_blocked(ObjectFilter::source()),
            )),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "This artifact becomes a 2/2 blue and black Horror artifact creature until end of turn and can't be blocked this turn"
        );
    }

    fn triggering_spell_color_protection_bundle(
        colors: [crate::color::ColorSet; 5],
    ) -> Vec<Effect> {
        let mut animation = crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Source,
            crate::continuous::Modification::SetCardTypes(vec![CardType::Creature]),
            Until::Forever,
        )
        .with_animation_pt_surface(Some(
            ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
        ))
        .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(4),
            toughness: Value::Fixed(4),
            sublayer: crate::continuous::PtSublayer::Setting,
        })
        .resolve_set_pt_values_at_resolution();
        animation.additional_modifications.extend([
            crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature,
            ),
            crate::continuous::Modification::AddSubtypes(vec![crate::types::Subtype::Giant]),
        ]);

        let mut effects = vec![Effect::new(animation)];
        for color in colors {
            let mut filter = ObjectFilter::default();
            filter.colors = Some(color);
            let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::Source,
                crate::continuous::Modification::AddAbility(
                    crate::static_abilities::StaticAbility::protection(
                        crate::ability::ProtectionFrom::Color(color),
                    ),
                ),
                Until::Forever,
            ));
            effects.push(Effect::new(crate::effects::ConditionalEffect::if_only(
                crate::effect::Condition::TaggedObjectMatches(TagKey::from("triggering"), filter),
                vec![grant],
            )));
        }
        effects
    }

    #[test]
    fn triggering_spell_color_protection_bundle_requires_all_five_exact_grants() {
        let colors = [
            crate::color::ColorSet::WHITE,
            crate::color::ColorSet::BLUE,
            crate::color::ColorSet::BLACK,
            crate::color::ColorSet::RED,
            crate::color::ColorSet::GREEN,
        ];
        let effects = triggering_spell_color_protection_bundle(colors);
        assert_eq!(
            describe_source_animation_with_triggering_spell_color_protection(&effects),
            Some(
                "it becomes a 4/4 Giant creature with protection from each of that spell's colors"
                    .to_string()
            )
        );

        let duplicate_red = [
            crate::color::ColorSet::WHITE,
            crate::color::ColorSet::BLUE,
            crate::color::ColorSet::BLACK,
            crate::color::ColorSet::RED,
            crate::color::ColorSet::RED,
        ];
        assert_eq!(
            describe_source_animation_with_triggering_spell_color_protection(
                &triggering_spell_color_protection_bundle(duplicate_red)
            ),
            None
        );
    }

    #[test]
    fn coordinated_sacrifice_then_attachment_keeps_the_choice_implicit() {
        let triggering = TagKey::from("triggering");
        let destination = TagKey::from("attachment_target_0");
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::SacrificeTargetEffect::new(
                ChooseSpec::Tagged(triggering),
            )),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::creature().controlled_by(PlayerFilter::You),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    destination.clone(),
                )
                .in_zone(Zone::Battlefield),
            ),
            Effect::attach_objects(ChooseSpec::Source, ChooseSpec::Tagged(destination)),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Sacrifice it and attach this source to a creature you control"
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
    fn coordinated_draw_followup_elides_repeated_you_subject() {
        let counters = Effect::new(crate::effects::PutCountersEffect::new(
            CounterType::Stun,
            3,
            ChooseSpec::Source,
        ))
        .tag("stunned_source");
        let counter_then_draw = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            counters,
            Effect::draw(Value::Fixed(3)),
        ]));
        let sacrifice_then_draw = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::sacrifice_source(),
            Effect::draw(Value::Fixed(3)),
        ]));

        assert_eq!(
            describe_effect(&counter_then_draw),
            "Put three stun counters on this source and draw three cards"
        );
        assert_eq!(
            describe_effect(&sacrifice_then_draw),
            "Sacrifice this source and draw three cards"
        );

        let result_branch = Effect::new(crate::effects::SequenceEffect::result_conjunction(
            vec![Effect::sacrifice_source(), Effect::draw(Value::Fixed(3))],
            false,
        ));
        assert_eq!(
            describe_result_branch_effect_list(std::slice::from_ref(&result_branch)),
            "Sacrifice this source and draw three cards"
        );
    }

    #[test]
    fn leading_result_conjunction_keeps_followup_in_a_new_sentence() {
        let result = Effect::new(crate::effects::SequenceEffect::result_conjunction(
            vec![
                Effect::draw(Value::Fixed(1)),
                Effect::gain_life(Value::Fixed(2)),
            ],
            false,
        ));
        let followup = Effect::scry(Value::Fixed(1));

        assert_eq!(
            describe_result_branch_effect_list(&[result, followup]),
            "Draw a card and gain 2 life. Scry 1"
        );
    }

    #[test]
    fn ordinary_coordination_does_not_inherit_result_sentence_boundary() {
        let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::draw(Value::Fixed(1)),
            Effect::gain_life(Value::Fixed(2)),
        ]));
        let followup = Effect::scry(Value::Fixed(1));

        assert_eq!(
            describe_leading_result_conjunction_then_followups(&[coordinated, followup]),
            None,
            "only an explicitly typed result conjunction may force this sentence boundary"
        );
    }

    #[test]
    fn counted_sacrifice_reflexive_rejoins_source_sentences() {
        let oracle = "Whenever this creature attacks, sacrifice any number of artifacts. When you sacrifice one or more artifacts this way, tap up to that many target creatures and draw that many cards.";
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Counted Sacrifice Reflexive",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("the counted sacrifice reflexive trigger should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()]
        );
    }

    #[test]
    fn coordinated_equal_draws_use_iterated_player_back_reference() {
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::draw(Value::Fixed(1)),
            Effect::new(crate::effects::DrawCardsEffect::new(
                Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "You and that player each draw a card"
        );
    }

    #[test]
    fn coordinated_player_restrictions_share_target_and_leading_duration() {
        let target_player = PlayerFilter::target_player();
        let leading = crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn;
        let sequence = Effect::new(
            crate::effects::SequenceEffect::coordinated_with_leading_duration(vec![
                Effect::new(crate::effects::TargetOnlyEffect::new(
                    ChooseSpec::target_player(),
                )),
                Effect::new(
                    crate::effects::CantEffect::until_end_of_turn(
                        crate::effect::Restriction::cast_spells_matching(
                            target_player.clone(),
                            ObjectFilter::instant_or_sorcery(),
                        ),
                    )
                    .with_duration_surface(leading),
                ),
                Effect::new(
                    crate::effects::CantEffect::until_end_of_turn(
                        crate::effect::Restriction::activate_non_mana_abilities(target_player),
                    )
                    .with_duration_surface(leading),
                ),
            ]),
        );

        assert_eq!(
            describe_effect(&sequence),
            "Until end of turn, target player can't cast instant or sorcery spells, and that player can't activate abilities that aren't mana abilities"
        );
        let sequence = sequence
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("expected the coordinated restriction sequence");
        assert_eq!(
            describe_effect_list(&sequence.effects),
            "Until end of turn, target player can't cast instant or sorcery spells, and that player can't activate abilities that aren't mana abilities",
            "migrated source-line lowering may expose the same typed members directly"
        );
        assert_eq!(
            describe_pre_clause_structural_effect_list(&sequence.effects).as_deref(),
            Some(
                "Until end of turn, target player can't cast instant or sorcery spells, and that player can't activate abilities that aren't mana abilities"
            )
        );
    }

    #[test]
    fn coordinated_additional_draw_keeps_both_you_subjects() {
        let additional = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::lose_life(Value::Fixed(1)),
            Effect::draw(
                Value::Fixed(1)
                    .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards),
            ),
        ]));
        let ordinary = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::lose_life(Value::Fixed(1)),
            Effect::draw(Value::Fixed(1)),
        ]));

        assert_eq!(
            describe_effect(&additional),
            "You lose 1 life and you draw an additional card"
        );
        let ordinary_sequence = ordinary
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("ordinary control should remain a coordinated sequence");
        assert!(
            describe_you_lose_life_and_draw_additional(&ordinary_sequence.effects).is_none(),
            "ordinary draw must not opt into the explicit AdditionalCards surface"
        );
    }

    #[test]
    fn coordinated_fixed_draw_and_life_loss_keeps_both_you_subjects() {
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::draw(Value::Fixed(2)),
            Effect::lose_life(Value::Fixed(2)),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "You draw two cards and you lose 2 life"
        );

        let mismatched = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::draw(Value::Fixed(2)),
            Effect::lose_life(Value::Fixed(1)),
        ]));
        assert_ne!(
            describe_effect(&mismatched),
            "You draw two cards and you lose 1 life",
            "the exact shared-count coordinator must not claim mismatched values"
        );
    }

    #[test]
    fn coordinated_gain_then_draw_keeps_one_actor_subject() {
        let fixed = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::gain_life(Value::Fixed(3)),
            Effect::draw(Value::Fixed(3)),
        ]));
        let variable = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::gain_life(Value::X),
            Effect::draw(Value::X),
        ]));

        assert_eq!(
            describe_effect(&fixed),
            "You gain 3 life and draw three cards"
        );
        assert_eq!(
            describe_effect(&variable),
            "You gain X life and draw X cards"
        );
    }

    #[test]
    fn coordinated_opponent_action_draw_and_gain_keeps_three_authored_clauses() {
        let discard_draw_gain = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::for_players(
                PlayerFilter::Opponent,
                vec![Effect::new(crate::effects::DiscardEffect::new(
                    1,
                    PlayerFilter::IteratedPlayer,
                    false,
                ))],
            ),
            Effect::draw(Value::Fixed(1)),
            Effect::gain_life(Value::Fixed(2)),
        ]));
        let lose_gain_draw = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::for_players(
                PlayerFilter::Opponent,
                vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                    1,
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
            Effect::gain_life(Value::Fixed(1)),
            Effect::draw(Value::Fixed(1)),
        ]));

        assert_eq!(
            describe_effect(&discard_draw_gain),
            "Each opponent discards a card, you draw a card, and you gain 2 life"
        );
        assert_eq!(
            describe_effect(&lose_gain_draw),
            "Each opponent loses 1 life, you gain 1 life, and you draw a card"
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
    fn coordinated_owned_commanders_from_command_and_graveyard_keep_one_move_surface() {
        let command = Effect::new(crate::effects::ReturnToHandEffect::all(
            ObjectFilter::default()
                .owned_by(PlayerFilter::You)
                .commander()
                .in_zone(Zone::Command),
        ));
        let graveyard = Effect::new(crate::effects::ReturnToHandEffect::all(
            ObjectFilter::default()
                .owned_by(PlayerFilter::You)
                .commander()
                .in_zone(Zone::Graveyard),
        ));
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            command.clone(),
            graveyard.clone(),
        ]));

        assert_eq!(
            describe_effect(&sequence),
            "Put all commanders you own from the command zone and from your graveyard into your hand"
        );

        let wrong_zone = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            command,
            Effect::new(crate::effects::ReturnToHandEffect::all(
                ObjectFilter::default()
                    .owned_by(PlayerFilter::You)
                    .commander()
                    .in_zone(Zone::Exile),
            )),
        ]));
        assert_ne!(
            describe_effect(&wrong_zone),
            "Put all commanders you own from the command zone and from your graveyard into your hand"
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

    fn declared_target_grant_then_dynamic_pump(pump_target: &str) -> Vec<Effect> {
        let target_tag = TagKey::from("targeted_0");
        let mut creature = ObjectFilter::creature().controlled_by(PlayerFilter::You);
        creature.other = true;
        creature.set_explicit_card_type_noun(Some(CardType::Creature));
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
            ChooseSpec::Object(creature),
        )))
        .tag(target_tag.clone());
        let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(target_tag),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::trample(),
            ),
            Until::EndOfTurn,
        ))
        .tag("granted_0");
        let x = Value::PowerOf(Box::new(ChooseSpec::Source.with_surface_hint(
            ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
                ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            ),
        )))
        .with_surface_hint(ValueSurfaceHint::WhereXIs);
        let pump = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec_runtime(
                ChooseSpec::Tagged(TagKey::from(pump_target)),
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: x.clone(),
                    toughness: x,
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        );
        vec![target, grant, pump]
    }

    #[test]
    fn declared_target_shared_by_grant_and_pump_keeps_subject_surface() {
        let effects = declared_target_grant_then_dynamic_pump("targeted_0");
        assert_eq!(
            describe_effect_list(&effects),
            "Another target creature you control gains trample and gets +X/+X until end of turn, where X is this creature's power"
        );

        let changed_tag = declared_target_grant_then_dynamic_pump("different_target");
        assert!(
            describe_shared_declared_target_grant_then_pt_pump(
                &changed_tag[0],
                &changed_tag[1],
                &changed_tag[2],
            )
            .is_none(),
            "independent target identities must not be merged"
        );
    }

    fn shared_target_trample_mana_value_pair(
        pump_target: ChooseSpec,
        grant_duration: Until,
        mana_value_tag: &str,
        where_x_surface: bool,
    ) -> Vec<Effect> {
        let target_tag = TagKey::from("targeted_0");
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_creature(),
        ))
        .tag(target_tag.clone());
        let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(target_tag),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::trample(),
            ),
            grant_duration,
        ))
        .tag("granted_0");
        let power = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(mana_value_tag))));
        let power = if where_x_surface {
            power.with_surface_hint(ValueSurfaceHint::WhereXIs)
        } else {
            power
        };
        let pump = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec_runtime(
                pump_target,
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power,
                    toughness: Value::Fixed(0),
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        )
        .tag("pumped_1");
        vec![target, grant, pump]
    }

    #[test]
    fn shared_target_trample_mana_value_pump_factors_target_and_eot_duration() {
        let effects = shared_target_trample_mana_value_pair(
            ChooseSpec::Tagged(TagKey::from("granted_0")),
            Until::EndOfTurn,
            "granted_0",
            true,
        );
        let expected = "Target creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value";

        assert_eq!(
            describe_shared_target_trample_mana_value_pump(&effects[0], &effects[1], &effects[2],)
                .as_deref(),
            Some(expected)
        );
        assert_eq!(describe_effect_list(&effects), expected);
        let expected_clause = lowercase_first(expected);
        assert_eq!(
            describe_effect_clause_list(&effects).as_deref(),
            Some(expected_clause.as_str())
        );
        assert_eq!(
            describe_effect(&Effect::new(crate::effects::SequenceEffect::coordinated(
                effects
            ))),
            expected
        );

        let direct_target_value = shared_target_trample_mana_value_pair(
            ChooseSpec::Tagged(TagKey::from("targeted_0")),
            Until::EndOfTurn,
            "targeted_0",
            false,
        );
        assert_eq!(
            describe_effect_list(&direct_target_value),
            expected,
            "a direct ManaValueOf still proves X when both consumers share the declared target"
        );
    }

    #[test]
    fn shared_target_trample_mana_value_pump_rejects_unproven_identity_or_duration() {
        let different_target = ChooseSpec::Tagged(TagKey::from("different_target"));
        let mismatched_target = shared_target_trample_mana_value_pair(
            different_target,
            Until::EndOfTurn,
            "granted_0",
            true,
        );
        let permanent_grant = shared_target_trample_mana_value_pair(
            ChooseSpec::Tagged(TagKey::from("granted_0")),
            Until::Forever,
            "granted_0",
            true,
        );
        let unrelated_basis = shared_target_trample_mana_value_pair(
            ChooseSpec::Tagged(TagKey::from("granted_0")),
            Until::EndOfTurn,
            "another_effect",
            true,
        );

        for effects in [mismatched_target, permanent_grant, unrelated_basis] {
            assert_eq!(
                describe_shared_target_trample_mana_value_pump(
                    &effects[0],
                    &effects[1],
                    &effects[2],
                ),
                None
            );
        }
    }

    #[test]
    fn coordinated_source_pump_and_damage_share_the_source_subject() {
        let pump = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
            ChooseSpec::Source,
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: Value::Fixed(1),
                toughness: Value::Fixed(0),
            },
            Until::EndOfTurn,
        ));
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            pump,
            Effect::deal_damage(Value::Fixed(1), ChooseSpec::SourceController),
        ]));
        let expected = "This source gets +1/+0 until end of turn and deals 1 damage to you";

        let coordinated = sequence
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("coordinated sequence");
        let continuous = structural_unwrap_render_wrappers(&coordinated.effects[0])
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("continuous source modification");
        assert_eq!(continuous.target_spec.as_ref(), Some(&ChooseSpec::Source));
        let damage = structural_unwrap_render_wrappers(&coordinated.effects[1])
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .expect("source damage");
        assert_eq!(damage.target, ChooseSpec::SourceController);
        assert_eq!(
            describe_coordinated_source_continuous_then_damage_controller(&coordinated.effects,)
                .as_deref(),
            Some(expected),
        );

        assert_eq!(describe_effect(&sequence), expected);
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

        let ordinary_coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::Source.with_surface_hint(
                    crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::FullName(
                            "Moon Girl and Devil Dinosaur".to_string(),
                        ),
                    ),
                ),
                crate::continuous::Modification::SetPowerToughness {
                    power: Value::Fixed(6),
                    toughness: Value::Fixed(6),
                    sublayer: crate::continuous::PtSublayer::Setting,
                },
                Until::EndOfTurn,
            )),
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    ChooseSpec::Source,
                    crate::continuous::Modification::AddAbility(
                        crate::static_abilities::StaticAbility::trample(),
                    ),
                    Until::EndOfTurn,
                )
                .with_source_reference_surface(
                    crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
                ),
            ),
        ]));
        assert_eq!(
            describe_effect(&ordinary_coordinated),
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
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
        let animation = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                target.clone(),
                crate::continuous::Modification::SetCardTypes(vec![CardType::Creature]),
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
                crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                    crate::types::SubtypeFamily::Creature,
                ),
            )
            .with_additional_modification(crate::continuous::Modification::AddSubtypes(vec![
                Subtype::Dragon,
                Subtype::Illusion,
            ]))
            .with_animation_pt_surface(Some(
                ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness,
            ))
            .with_animation_duration_surface(Some(
                ironsmith_core::AnimationDurationSurface::Leading,
            )),
        )
        .tag(animated.clone());
        let remove = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
            target.clone(),
            crate::effects::continuous::RuntimeModification::RemoveAllAbilities,
            Until::EndOfTurn,
        ))
        .tag(removed.clone());
        let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            target,
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
        assert_eq!(
            rendered,
            "Until end of turn, target creature you control becomes a blue Dragon Illusion with base power and toughness 4/4, loses all abilities, and gains flying"
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

fn declared_target_player_filter(
    target_only: &crate::effects::TargetOnlyEffect,
) -> Option<&PlayerFilter> {
    match target_only.target.base() {
        ChooseSpec::Player(filter) => Some(filter),
        _ => None,
    }
}

fn declared_target_player_surface(
    target_only: &crate::effects::TargetOnlyEffect,
) -> Option<String> {
    let declared = declared_target_player_filter(target_only)?;
    if target_only.target.is_target() {
        return Some(describe_choose_spec(&target_only.target));
    }
    if matches!(
        declared,
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Any)
                && matches!(excluded.as_ref(), PlayerFilter::You)
    ) {
        return Some("another target player".to_string());
    }
    Some(format!(
        "target {}",
        strip_leading_article(&describe_player_filter(declared))
    ))
}

fn player_reference_matches_declared_target(
    reference: &PlayerFilter,
    declared: &PlayerFilter,
) -> bool {
    match reference {
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            player_reference_matches_declared_target(inner, declared)
        }
        _ => reference == declared,
    }
}

/// A synthetic target declaration and its source-control application are one
/// authored clause: "Target player ... gains control of it." Keep the exact
/// target filter instead of letting the application render only its anaphoric
/// player subject.
pub(super) fn describe_target_player_source_control_transfer(effects: &[Effect]) -> Option<String> {
    let [target_effect, control_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let declared = declared_target_player_filter(target_only)?;
    if target_only.chooser.is_some() || !target_only.target.count().is_single() {
        return None;
    }

    let control = structural_unwrap_render_wrappers(control_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)] =
        control.runtime_modifications.as_slice()
    else {
        return None;
    };
    if control.target != crate::continuous::EffectTarget::Source
        || control.until != Until::Forever
        || control.condition.is_some()
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || !control
            .target_spec
            .as_ref()
            .is_some_and(|spec| matches!(spec.base(), ChooseSpec::Source))
        || !player_reference_matches_declared_target(player, declared)
    {
        return None;
    }

    Some(format!(
        "{} gains control of it",
        capitalize_first(&describe_choose_spec(&target_only.target))
    ))
}

/// A permanent control transfer followed by untapping that same source has
/// the player as the grammatical subject of both coordinated verbs:
/// "that player gains control of this artifact and untaps it."
fn describe_source_control_transfer_then_untap(effects: &[Effect]) -> Option<String> {
    let [control_effect, untap_effect] = effects else {
        return None;
    };
    let control = structural_unwrap_render_wrappers(control_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let untap = structural_unwrap_render_wrappers(untap_effect)
        .downcast_ref::<crate::effects::UntapEffect>()?;
    if control.target != crate::continuous::EffectTarget::Source
        || control.until != Until::Forever
        || control.condition.is_some()
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || !matches!(
            control.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(_)]
        )
        || !control
            .target_spec
            .as_ref()
            .is_some_and(|target| matches!(target.base(), ChooseSpec::Source))
        || !matches!(untap.target.base(), ChooseSpec::Source)
    {
        return None;
    }

    let control_text = describe_effect(control_effect);
    let control_text = control_text.trim().trim_end_matches('.');
    (!control_text.is_empty() && !control_text.contains(". "))
        .then(|| format!("{control_text} and untaps it"))
}

/// A target-only prelude followed by the two affected-player draw effects is
/// the executable form of "you and another target player each draw ...".
/// Render the declared target surface directly so its exclusion is not
/// weakened to the later alias "that player".
pub(super) fn describe_declared_target_joint_draw(effects: &[Effect]) -> Option<String> {
    let [target_effect, first_effect, second_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let declared = declared_target_player_filter(target_only)?;
    if target_only.chooser.is_some() || !target_only.target.count().is_single() {
        return None;
    }
    let first = unwrap_basic_tag_wrappers(first_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let second = unwrap_basic_tag_wrappers(second_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if first.player != PlayerFilter::You
        || first.count != second.count
        || !player_reference_matches_declared_target(&second.player, declared)
    {
        return None;
    }

    Some(format!(
        "You and {} each draw {}",
        declared_target_player_surface(target_only)?,
        describe_card_count(&first.count)
    ))
}

/// Preserve a counted target-player declaration when lowering executes the
/// action through a `ForPlayersEffect`. If the counted objects are attached
/// to the same player excluded from the target set, the two structured
/// references share the oracle anaphor "that player".
pub(super) fn describe_declared_target_player_draw_fanout(effects: &[Effect]) -> Option<String> {
    let [target_effect, fanout_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let declared = declared_target_player_filter(target_only)?;
    let count = target_only.target.count();
    if target_only.chooser.is_some() || count.is_single() {
        return None;
    }
    let PlayerFilter::Excluding { base, excluded } = declared else {
        return None;
    };
    if !matches!(base.as_ref(), PlayerFilter::Any) {
        return None;
    }

    let fanout = structural_unwrap_render_wrappers(fanout_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if fanout.starting_with_controller
        || fanout.stop_after_first_happened
        || !player_reference_matches_declared_target(&fanout.filter, declared)
    {
        return None;
    }
    let [draw_effect] = fanout.effects.as_slice() else {
        return None;
    };
    let draw =
        unwrap_basic_tag_wrappers(draw_effect).downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let Value::Count(attached_filter) = &draw.count else {
        return None;
    };
    if attached_filter.attached_to_object.is_some()
        || attached_filter.attached_to_player.as_ref() != Some(excluded.as_ref())
    {
        return None;
    }

    let mut unattached = attached_filter.clone();
    unattached.attached_to_player = None;
    unattached.zone = None;
    let counted = describe_count_filter_value_subject(&unattached);
    let target_surface = match (count.min, count.max) {
        (0, None) => "Any number of target players other than that player".to_string(),
        _ => describe_choose_spec(&target_only.target),
    };
    Some(format!(
        "{target_surface} each draw cards equal to the number of {counted} attached to that player"
    ))
}

/// "You and that player each <verb> ..." for adjacent same-payload effects
/// whose only difference is the affected player (you + a back-reference).
pub(in crate::compiled_text) fn describe_must_block_then_control_block_assignments(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let must_block =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let control = unwrap_basic_tag_wrappers(second)
        .downcast_ref::<crate::effects::ControlCombatChoicesThisTurnEffect>()?;
    if control.attackers || !control.blockers || control.this_combat {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(filter) = &must_block.target else {
        return None;
    };
    if !filter.card_types.contains(&CardType::Creature) {
        return None;
    }
    let (mut target, plural_target) = describe_apply_continuous_target(must_block);
    describe_attack_block_if_able_apply_continuous(must_block, &target)?;
    // An object filter controlled by `Opponent` ranges across every opponent;
    // "an opponent" would incorrectly narrow the multiplayer set to one
    // player once the subject is pluralized.
    if plural_target && filter.controller == Some(PlayerFilter::Opponent) {
        target = target.replace("an opponent controls", "your opponents control");
    }

    Some(format!(
        "{} {} this turn if able, and you choose how those creatures block",
        capitalize_first(&target),
        if plural_target { "block" } else { "blocks" },
    ))
}

pub(in crate::compiled_text) fn describe_joint_subject_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    fn join_shared_where_x(first: &Effect, second: &Effect) -> Option<String> {
        let split = |rendered: String| {
            let rendered = rendered.trim().trim_end_matches('.').to_string();
            let (head, basis) = rendered.rsplit_once(", where X is ")?;
            Some((head.to_string(), basis.to_string()))
        };
        let (first_head, first_basis) = split(describe_effect(first))?;
        let (second_head, second_basis) = split(describe_effect(second))?;
        if first_basis != second_basis {
            return None;
        }
        Some(format!(
            "{first_head} and {}, where X is {first_basis}",
            lowercase_first(&second_head)
        ))
    }

    fn joint_other_surface(player: &PlayerFilter) -> Option<&'static str> {
        match player {
            PlayerFilter::DamagedPlayer
            | PlayerFilter::TaggedPlayer(_)
            | PlayerFilter::ChosenPlayer
            | PlayerFilter::IteratedPlayer => Some("that player"),
            PlayerFilter::Target(inner) if **inner == PlayerFilter::Opponent => {
                Some("target opponent")
            }
            PlayerFilter::Target(inner) if **inner == PlayerFilter::Any => Some("target player"),
            PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner)
                if matches!(
                    inner.as_ref(),
                    PlayerFilter::Excluding { base, excluded }
                        if matches!(base.as_ref(), PlayerFilter::Any)
                            && matches!(excluded.as_ref(), PlayerFilter::You)
                ) =>
            {
                Some("another target player")
            }
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

    if let Some(compact) = describe_must_block_then_control_block_assignments(first, second) {
        return Some(compact);
    }

    if let Some(compact) = describe_linked_same_source_damage_pair(first, second) {
        return Some(compact);
    }

    // A coordinated damage/life pair can share one authored dynamic amount
    // definition. Lowering keeps the damage tagged for runtime provenance,
    // but that wrapper must not force each action to repeat its own
    // `where X is ...` clause. Requiring the same typed amount and an
    // explicit controller life-gain subject keeps this compaction local to
    // the structurally proven shared-X sentence.
    if let Some(damage) = deal_damage_effect_view(first)
        && let Some(gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && matches!(gain.player.base(), ChooseSpec::Player(PlayerFilter::You))
        && damage.amount.unhinted() == gain.amount.unhinted()
        && let Some(compact) = join_shared_where_x(first, second)
    {
        return Some(compact);
    }

    if let Some(for_players) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::ForPlayersEffect>()
        && for_players.filter == PlayerFilter::Opponent
        && !for_players.starting_with_controller
        && !for_players.stop_after_first_happened
        && let [inner] = for_players.effects.as_slice()
        && let Some(lose) =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::LoseLifeEffect>()
        && matches!(
            lose.player,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        && let Some(gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
        && lose.amount.unhinted() == gain.amount.unhinted()
        && let Some(basis) = describe_where_x_basis(&lose.amount)
    {
        return Some(format!(
            "Each opponent loses X life and you gain X life, where X is {basis}"
        ));
    }

    if let Some(for_players) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::ForPlayersEffect>()
        && for_players.filter == PlayerFilter::Opponent
        && !for_players.starting_with_controller
        && !for_players.stop_after_first_happened
        && let [inner] = for_players.effects.as_slice()
        && let Some(lose) =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::LoseLifeEffect>()
        && matches!(
            lose.player,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        && let Some(gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
        && let Some(compact) = join_shared_where_x(first, second)
    {
        return Some(compact);
    }

    if let Some(_lose) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
        && let Some(compact) = join_shared_where_x(first, second)
    {
        return Some(compact);
    }

    if let Some(lose) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(create) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::CreateTokenEffect>()
        && matches!(lose.player, ChooseSpec::Player(PlayerFilter::You))
        && create.controller == PlayerFilter::You
        && let Some(compact) = join_shared_where_x(first, second)
    {
        return Some(compact);
    }

    // Multiple actions can share one authored `where X is ...` definition.
    // Keep that definition once at the end of the coordinated sentence rather
    // than independently expanding it on both executable effects.
    if let Some(gain) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::GainLifeEffect>()
        && matches!(gain.player.base(), ChooseSpec::Player(PlayerFilter::You))
        && let Some(put) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::PutCountersEffect>()
        && gain.amount.unhinted() == put.amount.unhinted()
        && let Some(compact) = join_shared_where_x(first, second)
    {
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

pub(in crate::compiled_text) fn describe_player_or_planeswalker_damage_then_controlled_creature_damage(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let (first_source, first_damage) = coordinated_damage_view(first)?;
    if describe_choose_spec(&first_damage.target) != "target player or planeswalker" {
        return None;
    }

    let mut expected_filter = ObjectFilter::creature();
    expected_filter.controller = Some(PlayerFilter::TargetPlayerOrControllerOfTarget);
    let (second_source, second_damage, second_recipient) = if let Some(for_each) =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::ForEachObject>()
    {
        if for_each.filter != expected_filter {
            return None;
        }
        let [inner] = for_each.effects.as_slice() else {
            return None;
        };
        let (source, damage) = coordinated_damage_view(inner)?;
        if !matches!(damage.target, ChooseSpec::Iterated) {
            return None;
        }
        (
            source,
            damage,
            "each creature that player or that planeswalker's controller controls",
        )
    } else {
        let (source, damage) = coordinated_damage_view(second)?;
        if !damage.target.is_target()
            || damage.target.count() != ChoiceCount::up_to(1)
            || damage.target.count_value().is_some()
        {
            return None;
        }
        let ChooseSpec::Object(filter) = damage.target.base() else {
            return None;
        };
        let mut normalized = filter.clone();
        normalized.union_surface = Default::default();
        normalized.source_surface = None;
        if normalized != expected_filter {
            return None;
        }
        (
            source,
            damage,
            "up to one target creature that player or that planeswalker's controller controls",
        )
    };
    if first_source.map(ChooseSpec::unhinted) != second_source.map(ChooseSpec::unhinted) {
        return None;
    }

    let (amount_text, where_x) = describe_damage_amount_clause(&first_damage.amount);
    let mut text = if second_damage.amount == first_damage.amount {
        if second_recipient.starts_with("up to one") {
            format!(
                "Deal {amount_text} to target player or planeswalker and {amount_text} to {second_recipient}"
            )
        } else {
            format!("Deal {amount_text} to target player or planeswalker and {second_recipient}")
        }
    } else {
        let (fanout_amount_text, fanout_where_x) =
            describe_damage_amount_clause(&second_damage.amount);
        if where_x.is_some() || fanout_where_x.is_some() {
            return None;
        }
        format!(
            "Deal {amount_text} to target player or planeswalker and {fanout_amount_text} to {second_recipient}"
        )
    };
    if let Some(where_x) = where_x {
        text.push_str(&format!(", where X is {where_x}"));
    }
    Some(text)
}

/// Recombine an explicit player-owned object choice with the linked attach
/// action that consumes it. Lowering keeps these separate so the named player
/// actually makes the choice at runtime; Oracle expresses both operations as
/// one "that player attaches ... of their choice" clause.
pub(super) fn describe_player_chosen_attachment(effects: &[Effect]) -> Option<String> {
    let [choose_effect, attach_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || choose.count.is_random()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let actor = describe_player_filter(&choose.chooser);
    let verb = player_verb(&actor, "attach", "attaches");
    let object = describe_attach_objects_spec(&attach.objects);
    let selection = describe_choose_selection(choose);
    let possessive = describe_possessive_player_filter(&choose.chooser);
    Some(format!(
        "{} {verb} {object} to {selection} of {possessive} choice",
        capitalize_first(&actor)
    ))
}

pub(crate) fn describe_false_only_conditional(
    condition: &crate::effect::Condition,
    false_branch: &str,
) -> String {
    if let crate::effect::Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
        mode,
    } = condition
        && *mode == crate::effect::TaggedObjectMatchMode::CurrentOrLastKnown
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

fn battlefield_move_verb_and_destination(
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> (&'static str, &'static str) {
    match move_to_zone.verb_surface {
        ironsmith_core::MoveToZoneVerbSurface::Put => ("put", "onto"),
        ironsmith_core::MoveToZoneVerbSurface::Canonical
        | ironsmith_core::MoveToZoneVerbSurface::Return => ("return", "to"),
    }
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
    let target_is_source = matches!(exile_move.target.unhinted(), ChooseSpec::Source);
    let target = if target_is_source {
        describe_source_motion_reference(&exile_move.target, "this")
    } else {
        describe_choose_spec(&exile_move.target)
    };
    let return_object = match move_back.target_reference_surface {
        Some(ironsmith_core::SearchResultReferenceSurface::It) => "it",
        Some(ironsmith_core::SearchResultReferenceSurface::ThatCard) => "that card",
        Some(ironsmith_core::SearchResultReferenceSurface::Them) => "them",
        Some(ironsmith_core::SearchResultReferenceSurface::ThoseCards) => "those cards",
        Some(ironsmith_core::SearchResultReferenceSurface::TheCard) => "the card",
        None if target_is_source => "it",
        None if choose_spec_allows_multiple(&exile_move.target) => "those cards",
        None => "that card",
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
    let face_down_suffix = if move_back.enters_face_down {
        " face down"
    } else {
        ""
    };
    let transformed_suffix = if move_back.enters_transformed {
        " transformed"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    // A fused entry counter ("with a +1/+1 counter on it") lowers onto the
    // return move; keep its authored surface.
    let counters_suffix = describe_entry_counters_suffix(&move_back.enters_with_counters);
    let (move_verb, destination) = battlefield_move_verb_and_destination(move_back);
    Some(format!(
        "Exile {target}, then {move_verb} {return_object} {destination} the battlefield{tapped_suffix}{face_down_suffix}{transformed_suffix}{controller_suffix}{counters_suffix}"
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
    let face_down_suffix = if move_back.enters_face_down {
        " face down"
    } else {
        ""
    };
    let transformed_suffix = if move_back.enters_transformed {
        " transformed"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    // A fused entry counter ("with a +1/+1 counter on it") lowers onto the
    // return move; keep its authored surface.
    let counters_suffix = describe_entry_counters_suffix(&move_back.enters_with_counters);
    let (move_verb, destination) = battlefield_move_verb_and_destination(move_back);
    Some(format!(
        "Exile {target}, then {move_verb} {return_object} {destination} the battlefield{tapped_suffix}{face_down_suffix}{transformed_suffix}{controller_suffix}{counters_suffix}"
    ))
}

pub(super) fn describe_source_motion_reference(spec: &ChooseSpec, named_fallback: &str) -> String {
    let Some(surface) = spec.source_reference_surface() else {
        return named_fallback.to_string();
    };
    match surface {
        crate::target::SourceReferenceSurface::ThisPermanentType(text)
            if text.eq_ignore_ascii_case("it") =>
        {
            text.clone()
        }
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
        // Authored name references ("exile Grist, then return it ...") keep
        // their surface; oracle-faithful motion clauses name the source.
        crate::target::SourceReferenceSurface::FullName(name)
        | crate::target::SourceReferenceSurface::ShortName(name) => name.clone(),
    }
}

pub(super) fn describe_exile_then_return_transformed_with_counter(
    exile_effect: &Effect,
    return_effect: &Effect,
    transform_effect: Option<&Effect>,
    put_counter_effect: &Effect,
) -> Option<String> {
    let exile_tag = wrapped_effect_tag(exile_effect);
    let exile_move = unwrap_basic_tag_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_back = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
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
    let transformed_target_matches = if let Some(transform_effect) = transform_effect {
        let transform = unwrap_basic_tag_wrappers(transform_effect)
            .downcast_ref::<crate::effects::TransformEffect>()?;
        matches!(&transform.target, ChooseSpec::Tagged(tag) if tag == return_tag)
    } else {
        move_back.enters_transformed
    };
    if !transformed_target_matches
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
    // This legacy four-effect shape historically implied `put` when no
    // explicit verb surface was retained. Preserve that canonical fallback,
    // while still honoring an explicitly authored `return`.
    let (move_verb, destination) = match move_back.verb_surface {
        ironsmith_core::MoveToZoneVerbSurface::Return => ("return", "to"),
        ironsmith_core::MoveToZoneVerbSurface::Canonical
        | ironsmith_core::MoveToZoneVerbSurface::Put => ("put", "onto"),
    };
    Some(format!(
        "Exile {target}, then {move_verb} {return_object} {destination} the battlefield{tapped_suffix} transformed{controller_suffix} with {counter_text} on it"
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
    let (move_verb, destination) = battlefield_move_verb_and_destination(move_back);
    Some(format!(
        "Exile {target}, then {move_verb} {return_object} {destination} the battlefield{tapped_suffix} transformed{controller_suffix}"
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

pub(super) fn face_change_targets_returned_object(
    move_back: &crate::effects::MoveToZoneEffect,
    returned_tag: Option<&crate::tag::TagKey>,
    face_change_target: &ChooseSpec,
) -> bool {
    if let Some(returned_tag) = returned_tag
        && matches!(face_change_target, ChooseSpec::Tagged(face_change_tag) if face_change_tag == returned_tag)
    {
        return true;
    }
    move_back.target.unhinted() == face_change_target.unhinted()
}

pub(crate) fn describe_return_then_transform_or_convert(
    return_effect: &Effect,
    face_change_effect: &Effect,
) -> Option<String> {
    let (move_back, returned_tag) = move_to_zone_for_transform_compaction(return_effect)?;
    let (face_change_target, face_change_surface) = if let Some(transform) =
        face_change_effect.downcast_ref::<crate::effects::TransformEffect>()
    {
        (&transform.target, "transformed")
    } else if let Some(convert) = face_change_effect.downcast_ref::<crate::effects::ConvertEffect>()
    {
        (&convert.target, "converted")
    } else {
        return None;
    };
    if move_back.zone != Zone::Battlefield {
        return None;
    }
    if !face_change_targets_returned_object(move_back, returned_tag, face_change_target) {
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
    let (move_verb, destination) = battlefield_move_verb_and_destination(move_back);
    Some(format!(
        "{} {return_object} {destination} the battlefield{tapped_suffix} {face_change_surface}{controller_suffix}",
        capitalize_first(move_verb)
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
        ) && let Some(rest) = reveal_subject.strip_prefix("the ")
        {
            reveal_subject = rest.to_string();
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
        return ("that much damage".to_string(), None);
    }
    let desc = describe_value(amount);
    // A counting phrase reads as oracle's "damage equal to ..." tail, not as
    // an inline determiner.
    for prefix in ["the number of ", "the amount of ", "the total "] {
        if desc.starts_with(prefix) {
            return (format!("damage equal to {desc}"), None);
        }
    }
    (format!("{desc} damage"), None)
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

pub(in crate::compiled_text) fn describe_damage_target(target: &ChooseSpec) -> String {
    if let Some(text) = describe_counted_any_damage_target(target) {
        return text;
    }
    if let ChooseSpec::Object(filter) = target.unhinted()
        && filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Each)
    {
        let mut singular = filter.clone();
        singular.set_set_quantifier_surface(None);
        let described = describe_choose_spec(&ChooseSpec::Object(singular));
        if let Some(noun) = described.strip_prefix("another ") {
            return format!("each other {noun}");
        }
        return format!("each {}", strip_leading_article(described.trim()).trim());
    }
    if let ChooseSpec::ObjectOrPlayer(filter, PlayerFilter::DamagedPlayer) = target.unhinted() {
        let mut expected = ObjectFilter::permanent();
        expected
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::TagKey::from("damaged"),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let expected_explicit = ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .match_tagged(
                crate::TagKey::from("damaged"),
                crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            );
        if filter == &expected || filter == &expected_explicit {
            return "that permanent or player".to_string();
        }
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

#[cfg(test)]
mod quantified_damage_target_tests {
    use super::*;

    #[test]
    fn each_other_surface_is_not_collapsed_to_another_object() {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.other = true;
        filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));
        assert_eq!(
            describe_damage_target(&ChooseSpec::Object(filter)),
            "each other creature"
        );
    }

    #[test]
    fn ordinary_other_target_keeps_singular_surface() {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.other = true;
        assert_eq!(
            describe_damage_target(&ChooseSpec::Object(filter)),
            "another creature"
        );
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
        ("same name as that cards", "same name as that card"),
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
            "where X is the number of counters removed this way",
            &format!("where X is {x_phrase}"),
        );
        effects = effects.replace(
            "deals X damage to",
            &format!("deals damage equal to {x_phrase} to"),
        );
        effects = effects.replace(
            "Deal X damage to",
            &format!("Deal damage equal to {x_phrase} to"),
        );
        effects = effects.replace(
            "deals damage equal to X to",
            &format!("deals damage equal to {x_phrase} to"),
        );
        effects = effects.replace(
            "Deals damage equal to X to",
            &format!("Deals damage equal to {x_phrase} to"),
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
            let cost_text = describe_total_cost(&activated.mana_cost);
            if !cost_text.is_empty() {
                line.push_str(&cost_text);
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
                        describe_total_cost(&activated.mana_cost)
                    );
                }
            }
            if let Some(prefix) = describe_loyalty_activation_prefix_for_activated(activated) {
                let effects = super::ast_render::describe_resolution_program(&activated.effects);
                let effects = replace_this_spell_self_reference(effects, self_subject);
                let effects = normalize_ability_self_reference_surface(&effects, self_subject);
                return format!("[{prefix}]: {effects}");
            }
            let mut line = String::new();
            let mut pre = Vec::new();
            let mut trailing_x_definition = None;
            let waterbend_label = activated_presentation_label(activated)
                .filter(|label| label.starts_with("Waterbend {") && label.ends_with('}'));
            if let Some(label) = waterbend_label {
                pre.push(label.to_string());
            } else {
                let cost_text = describe_total_cost(&activated.mana_cost);
                if !cost_text.is_empty() {
                    let (cost_text, x_definition) =
                        describe_total_cost_with_trailing_x_definition(&activated.mana_cost);
                    pre.push(cost_text);
                    trailing_x_definition = x_definition;
                }
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
                effects = rewrite_cost_bound_x_phrases(
                    effects,
                    activated.mana_cost.as_all().unwrap_or(&[]),
                );
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
                &activated.activation_restrictions,
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
    for cost in activated.mana_cost.as_all()? {
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
        .as_all()?
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

pub(crate) fn describe_as_enters_counter_phrase_on_it(
    amount: &Value,
    counter_type: CounterType,
) -> String {
    if amount.has_surface_hint(ValueSurfaceHint::WhereXIs) {
        return format!(
            "X {} counters on it, where X is {}",
            counter_type.description(),
            describe_value(amount)
        );
    }
    if let Some((multiplier, basis)) = describe_for_each_multiplier_and_basis(amount)
        && multiplier > 0
    {
        let counter_name = counter_type.description();
        let counter_phrase = if multiplier == 1 {
            with_indefinite_article(&format!("{counter_name} counter"))
        } else {
            let amount = number_word(multiplier).unwrap_or_else(|| multiplier.to_string());
            format!("{amount} {counter_name} counters")
        };
        return format!("{counter_phrase} on it for each {basis}");
    }
    let phrase = describe_put_counter_phrase(amount, counter_type);
    let lower = phrase.to_ascii_lowercase();
    let qualifier = [" for each ", " equal to "]
        .into_iter()
        .filter_map(|marker| lower.find(marker))
        .min();
    if let Some(qualifier) = qualifier {
        format!("{} on it{}", &phrase[..qualifier], &phrase[qualifier..])
    } else {
        format!("{phrase} on it")
    }
}

fn restore_trailing_source_counter_as_enters_surface(
    program: &crate::resolution::ResolutionProgram,
    subject: &str,
) -> Option<String> {
    let trailing = program.segments.last()?.default_effects.last()?;
    let put =
        unwrap_basic_tag_wrappers(trailing).downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(put.target.unhinted(), ChooseSpec::Source)
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }

    let mut prefix_program = program.clone();
    prefix_program.pop()?;
    let prefix = lowercase_first(
        super::ast_render::describe_resolution_program(&prefix_program)
            .trim()
            .trim_end_matches('.'),
    );
    let enters_with = format!(
        "{} enters with {}",
        if prefix.is_empty() {
            subject.to_ascii_lowercase()
        } else {
            capitalize_first(&subject.to_ascii_lowercase())
        },
        describe_as_enters_counter_phrase_on_it(&put.amount, put.counter_type)
    );
    Some(if prefix.is_empty() {
        enters_with
    } else {
        format!("{prefix}. {enters_with}")
    })
}

fn restore_conditional_source_counter_grant_as_enters_surface(
    program: &crate::resolution::ResolutionProgram,
    subject: &str,
) -> Option<String> {
    fn contains_effect_id(effect: &Effect, id: crate::effect::EffectId) -> bool {
        if effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .is_some_and(|with_id| with_id.id == id)
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            found |= contains_effect_id(child, id);
        });
        found
    }

    let conditional_segment = program.segments.last()?;
    let conditional_effect = conditional_segment.default_effects.last()?;
    if !conditional_segment.self_replacements.is_empty() {
        return None;
    }
    let mut prefix_program = program.clone();
    prefix_program.pop()?;
    let conditional =
        unwrap_basic_tag_wrappers(conditional_effect).downcast_ref::<crate::effects::IfEffect>()?;
    if !matches!(
        conditional.predicate,
        crate::effect::EffectPredicate::Happened | crate::effect::EffectPredicate::Chosen
    ) || !conditional.else_.is_empty()
        || conditional.then.len() != 1
        || !prefix_program
            .iter()
            .any(|effect| contains_effect_id(effect, conditional.condition))
    {
        return None;
    }
    let apply = unwrap_basic_tag_wrappers(&conditional.then[0])
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::Forever
        || !apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| matches!(spec.unhinted(), ChooseSpec::Source))
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
    {
        return None;
    }
    let crate::continuous::Modification::AddAbility(ability) = apply.modification.as_ref()? else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } =
        &ability.compiled_model()?.payload
    else {
        return None;
    };

    let prefix_text = lowercase_first(
        super::ast_render::describe_resolution_program(&prefix_program)
            .trim()
            .trim_end_matches('.'),
    );
    let counter_phrase = describe_as_enters_counter_phrase_on_it(count, *counter);
    Some(format!(
        "{prefix_text}. If you do, {} enters with {counter_phrase}",
        subject.to_ascii_lowercase()
    ))
}

fn restore_as_enters_with_counter_surface(
    program: &crate::resolution::ResolutionProgram,
    body: &str,
    subject: &str,
) -> String {
    if let Some(restored) =
        restore_conditional_source_counter_grant_as_enters_surface(program, subject)
    {
        return restored;
    }
    if let Some(restored) = restore_trailing_source_counter_as_enters_surface(program, subject) {
        return restored;
    }

    let lower = body.to_ascii_lowercase();
    let Some(put_idx) = lower.rfind("put ") else {
        return body.to_string();
    };
    let after_put_idx = put_idx + "put ".len();
    let after_put_lower = &lower[after_put_idx..];
    let subject_lower = subject.to_ascii_lowercase();
    let source_marker = format!(" on {subject_lower}");
    let (marker_idx, marker_len) = after_put_lower
        .find(&source_marker)
        .map(|idx| (idx, source_marker.len()))
        .or_else(|| {
            after_put_lower
                .find(" on it")
                .map(|idx| (idx, " on it".len()))
        })
        .unwrap_or((usize::MAX, 0));
    if marker_idx == usize::MAX {
        return body.to_string();
    }
    let counter_phrase = body[after_put_idx..after_put_idx + marker_idx].trim();
    if !counter_phrase.to_ascii_lowercase().contains("counter") {
        return body.to_string();
    }

    let prefix = &body[..put_idx];
    let suffix = &body[after_put_idx + marker_idx + marker_len..];
    let rendered_subject = if matches!(prefix.trim_end().chars().last(), Some('.' | '!' | '?')) {
        capitalize_first(&subject_lower)
    } else {
        subject_lower
    };
    format!("{prefix}{rendered_subject} enters with {counter_phrase} on it{suffix}")
}

fn describe_exile_would_die_with_follow_up(
    static_ability: &crate::static_abilities::StaticAbility,
) -> Option<String> {
    let (_, _, _, _, follow_up_effects) = static_ability.exile_would_die_instead_spec()?;
    if follow_up_effects.is_empty() {
        return None;
    }

    let display = static_ability.display();
    let (condition, replacement) = display.trim().trim_end_matches('.').split_once(", ")?;
    let exile = replacement.strip_suffix(" instead")?;
    let exile = exile
        .strip_prefix("exile it")
        .map(|suffix| format!("exile that card{suffix}"))?;
    let follow_up = describe_effect_list(follow_up_effects);
    let follow_up = follow_up.trim().trim_end_matches('.');
    if follow_up.is_empty() || follow_up.contains(". ") {
        return None;
    }

    Some(format!(
        "{condition}, instead {exile} and {}",
        lowercase_first(follow_up)
    ))
}

#[cfg(test)]
#[test]
fn exile_would_die_follow_up_renders_as_one_typed_replacement_clause() {
    let zombie =
        crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![crate::types::Subtype::Zombie])
            .color_indicator(crate::color::ColorSet::BLACK)
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
    let ability = crate::static_abilities::StaticAbility::exile_would_die_instead_with_damage_source_and_follow_up(
        ObjectFilter::creature()
            .nontoken()
            .controlled_by(PlayerFilter::Opponent)
            .in_zone(Zone::Battlefield),
        None,
        vec![Effect::create_tokens(zombie, 1)],
    );

    assert_eq!(
        describe_static_ability_with_subject(&ability, "this creature"),
        "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token"
    );
}

fn characteristic_value_is_source_power(value: &Value) -> bool {
    match value.unhinted() {
        Value::SourcePower => true,
        Value::PowerOf(spec) => matches!(spec.unhinted(), ChooseSpec::Source),
        _ => false,
    }
}

fn characteristic_value_is_source_toughness(value: &Value) -> bool {
    match value.unhinted() {
        Value::SourceToughness => true,
        Value::ToughnessOf(spec) => matches!(spec.unhinted(), ChooseSpec::Source),
        _ => false,
    }
}

pub(crate) fn describe_static_ability_with_subject(
    static_ability: &crate::static_abilities::StaticAbility,
    subject: &str,
) -> String {
    if let Some(ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt {
        power,
        toughness,
    }) = static_ability.compiled_model().map(|model| &model.payload)
    {
        let capitalized_subject = capitalize_first(subject);
        // Card names always form the Oracle possessive with "'s", including
        // names that already end in s ("Plague Rats's"). Ordinary nouns keep
        // the grammatical plural possessive supplied by `possessive_subject`.
        let uses_named_source_subject = power
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject)
            || toughness.has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject);
        let possessive = if uses_named_source_subject {
            format!("{capitalized_subject}'s")
        } else {
            possessive_subject(&capitalized_subject)
        };
        let power_text = describe_value(power);
        if power.unhinted() == toughness.unhinted() {
            return format!("{possessive} power and toughness are each equal to {power_text}");
        }
        let offset = match toughness.unhinted() {
            Value::Add(left, right) if left.unhinted() == power.unhinted() => {
                match right.unhinted() {
                    Value::Fixed(offset) if *offset > 0 => Some(*offset),
                    _ => None,
                }
            }
            Value::Add(left, right) if right.unhinted() == power.unhinted() => {
                match left.unhinted() {
                    Value::Fixed(offset) if *offset > 0 => Some(*offset),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(offset) = offset {
            return format!(
                "{possessive} power is equal to {power_text} and its toughness is equal to that number plus {offset}"
            );
        }
        if characteristic_value_is_source_power(power) {
            return format!(
                "{possessive} toughness is equal to {}",
                describe_value(toughness)
            );
        }
        if characteristic_value_is_source_toughness(toughness) {
            return format!("{possessive} power is equal to {power_text}");
        }
        return format!(
            "{possessive} power is {power_text}, and its toughness is {}",
            describe_value(toughness)
        );
    }
    if matches!(
        static_ability.compiled_model().map(|model| &model.payload),
        Some(ironsmith_core::StaticAbilityPayload::Companion(_))
    ) {
        return format!(
            "Companion — {}",
            capitalize_first(static_ability.display().trim())
        );
    }
    let authored_subject = static_ability.compiled_model().and_then(|model| {
        let ironsmith_core::StaticAbilityPayload::SelfSubjectSurface { surface } = &model.payload
        else {
            return None;
        };
        Some(surface.display_text())
    });
    let subject = authored_subject.as_deref().unwrap_or(subject);
    if let Some(ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count }) =
        static_ability.compiled_model().map(|model| &model.payload)
    {
        return format!(
            "{} enters with {}",
            capitalize_first(subject),
            describe_as_enters_counter_phrase_on_it(count, *counter)
        );
    }
    if let Some(model) = static_ability.compiled_model()
        && let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
            &model.payload
        && grant.condition.is_none()
        && grant.additional_abilities.is_empty()
        && grant.set_quantifier_surface.is_none()
        && grant.ability.functional_zones.as_slice() == [Zone::Battlefield]
        && matches!(
            &grant.ability.kind,
            ironsmith_core::AbilityKind::Triggered(_)
        )
    {
        let granted = crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(
            &grant.ability,
        );
        if model.label.eq_ignore_ascii_case("prowess")
            || match &granted.kind {
                AbilityKind::Triggered(triggered) => {
                    describe_structural_prowess_keyword(triggered).is_some()
                }
                AbilityKind::Static(static_ability) => {
                    static_ability.id() == crate::static_abilities::StaticAbilityId::Prowess
                }
                _ => false,
            }
        {
            let (grant_subject, _) =
                crate::static_abilities::grant_subject_with_set_quantifier(&grant.filter, None);
            return format!("{} have prowess", capitalize_first(&grant_subject));
        }
    }
    if let Some(ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
        program,
        subject: authored_subject,
        also_turns_face_up,
        turns_face_up_only,
        uses_enters_with_counter_surface,
        transforms_into,
        presentation_label,
    }) = static_ability.compiled_model().map(|model| &model.payload)
    {
        let timing = if let Some(destination) = transforms_into {
            format!("As {authored_subject} transforms into {destination}")
        } else if *turns_face_up_only {
            format!("As {authored_subject} is turned face up")
        } else if *also_turns_face_up {
            format!("As {authored_subject} enters or is turned face up")
        } else {
            format!("As {authored_subject} enters")
        };
        let mut body = lowercase_first(
            super::ast_render::describe_resolution_program(program)
                .trim()
                .trim_end_matches('.'),
        )
        .replace("if this spell was kicked", "if it was kicked");
        if let Some(choice) = body.strip_prefix("you choose ") {
            body = format!("choose {choice}");
        }
        // A hand-selection filter may preserve its leading preposition while
        // the reveal bundle supplies the quantified `of`. Collapse that seam
        // before adapting the as-enters body's hand-zone wording.
        body = body.replace("reveal any number of of ", "reveal any number of ");
        if (body.starts_with("you may reveal ")
            || body.starts_with("you reveal ")
            || body.starts_with("reveal "))
            && body.contains(" in your hand")
        {
            body = body.replacen(" in your hand", " from your hand", 1);
        }
        if *uses_enters_with_counter_surface {
            body = restore_as_enters_with_counter_surface(program, &body, authored_subject);
        }
        if let Some(destination) = transforms_into {
            body = body
                .replace("exiled with this source", "exiled with it")
                .replace("with this permanent", "with it")
                .replace(" on this source ", &format!(" on {destination} "));
        }
        let line = if body.is_empty() {
            timing
        } else {
            format!("{timing}, {body}")
        };
        return presentation_label
            .as_ref()
            .and_then(crate::ability::PresentationLabel::display_prefix)
            .map(|label| format!("{label} — {line}"))
            .unwrap_or(line);
    }
    if let Some(rendered) = describe_exile_would_die_with_follow_up(static_ability) {
        return rendered;
    }
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
    if static_ability.id() == crate::static_abilities::StaticAbilityId::AttachedAbilityGrant
        && let Some(attached_subject) = match subject {
            "this Aura" => Some("Enchanted permanent"),
            "this Equipment" => Some("Equipped creature"),
            "this Fortification" => Some("Fortified land"),
            _ => None,
        }
        && let Some(Ability {
            kind: AbilityKind::Static(granted),
            ..
        }) = static_ability.granted_inline_ability()
    {
        if granted.id() == crate::static_abilities::StaticAbilityId::Unblockable {
            return format!("{attached_subject} can't be blocked");
        }
        let grant_display = static_ability
            .display()
            .trim()
            .trim_end_matches('.')
            .to_string();
        let keyword = granted
            .display()
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if granted.is_keyword() && grant_display.eq_ignore_ascii_case(&keyword) {
            return format!("{attached_subject} has {keyword}");
        }
    }

    // Rules text granted by an Aura, Equipment, or Fortification is quoted in
    // Oracle text. Keep keyword grants unquoted above, but restore the quotes
    // for an exact executable activated or triggered ability even when it is
    // the only granted ability on the line.
    if static_ability.id() == crate::static_abilities::StaticAbilityId::AttachedAbilityGrant
        && let Some(granted) = static_ability.granted_inline_ability()
        && matches!(
            &granted.kind,
            AbilityKind::Activated(_) | AbilityKind::Triggered(_)
        )
    {
        let display = static_ability.display();
        let trimmed = display.trim().trim_end_matches('.');
        if !trimmed.contains('"') {
            for delimiter in [" has ", " have "] {
                if let Some((grant_subject, _body)) = trimmed.split_once(delimiter) {
                    let typed_body = describe_inline_ability(granted);
                    let body = typed_body.trim().trim_end_matches('.');
                    if !grant_subject.is_empty() && !body.is_empty() {
                        let terminal = if body.ends_with('?') || body.ends_with('!') {
                            ""
                        } else {
                            "."
                        };
                        return format!(
                            "{}{}\"{body}{terminal}\"",
                            capitalize_first(grant_subject),
                            delimiter
                        );
                    }
                }
            }
        }
    }

    let rendered = restore_modeled_value_surface(static_ability, static_ability.display());
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
        } else if !subject.starts_with("this ")
            && let Some(tail) = [
                "artifact",
                "battle",
                "card",
                "creature",
                "enchantment",
                "land",
                "permanent",
                "planeswalker",
                "source",
                "spell",
            ]
            .into_iter()
            .find_map(|generic| {
                rest.strip_prefix(generic)
                    .filter(|tail| tail.starts_with(' ') || tail.starts_with("'s"))
            })
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

#[cfg(test)]
#[test]
fn characteristic_defining_single_axis_omits_the_unchanged_axis() {
    let power_only = crate::static_abilities::StaticAbility::from_model(
        crate::static_abilities::CompiledStaticAbility::characteristic_defining_pt(
            Value::PartySize(PlayerFilter::You),
            Value::SourceToughness,
        ),
    );
    assert_eq!(
        describe_static_ability_with_subject(&power_only, "this creature"),
        "This creature's power is equal to the number of creatures in your party"
    );

    let toughness_only = crate::static_abilities::StaticAbility::from_model(
        crate::static_abilities::CompiledStaticAbility::characteristic_defining_pt(
            Value::PowerOf(Box::new(ChooseSpec::Source)),
            Value::Fixed(5),
        ),
    );
    assert_eq!(
        describe_static_ability_with_subject(&toughness_only, "this creature"),
        "This creature's toughness is equal to 5"
    );
}

#[cfg(test)]
#[test]
fn named_battlefield_count_characteristic_uses_the_typed_value_on_the_public_route() {
    let oracle = "Plague Rats's power and toughness are each equal to the number of creatures named Plague Rats on the battlefield.";
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Plague Rats",
    )
    .card_types(vec![CardType::Creature])
    .subtypes(vec![crate::types::Subtype::Rat])
    .power_toughness(crate::card::PowerToughness::new(
        crate::card::PtValue::Star,
        crate::card::PtValue::Star,
    ))
    .parse_text(oracle)
    .expect("named characteristic count should compile");

    let [ability] = definition.abilities.as_slice() else {
        panic!(
            "expected one characteristic ability: {:#?}",
            definition.abilities
        )
    };
    let AbilityKind::Static(static_ability) = &ability.kind else {
        panic!("expected a static characteristic ability: {ability:#?}")
    };
    let model = static_ability
        .compiled_model()
        .unwrap_or_else(|| panic!("characteristic ability lost its compiled model: {ability:#?}"));
    assert!(
        matches!(
            model.payload,
            ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { .. }
        ),
        "unexpected characteristic payload: {:#?}",
        model.payload
    );
    assert!(
        static_ability.prefers_card_name_subject(),
        "named-source surface hint was lost: {model:#?}"
    );
    assert_eq!(
        describe_static_ability_with_subject(static_ability, "Plague Rats"),
        oracle.trim_end_matches('.')
    );
    assert_eq!(
        super::abilities_and_costs::describe_ability(1, ability, "Plague Rats", true),
        vec![format!(
            "Static ability 1: {}",
            oracle.trim_end_matches('.')
        )]
    );
    assert_eq!(
        crate::compiled_text::debug_compiled_lines(&definition),
        vec![oracle.to_string()]
    );

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

/// Replace value debug placeholders emitted by runtime-owned ability labels
/// with the canonical compiled-text description of the same typed value.
///
/// Dynamic entry-counter and anthem abilities still execute in the engine,
/// but their rich surface vocabulary belongs to this crate. After the parser
/// migration the lightweight runtime display deliberately renders a `Value`
/// with its compact debug shape; allowing that placeholder to escape here
/// produced text such as `where X is SurfaceHinted { value: Count, hints: }`.
/// Match only payload fields whose identity is proven by the compiled model,
/// leaving unrelated authored text and nested quoted abilities untouched.
pub(crate) fn restore_modeled_value_surface(
    static_ability: &crate::static_abilities::StaticAbility,
    mut rendered: String,
) -> String {
    fn replace_value(rendered: &mut String, value: &Value) {
        let debug = format!("{value:?}");
        if rendered.contains(&debug) {
            *rendered = rendered.replace(&debug, &describe_value(value));
            return;
        }

        // Runtime-owned labels intentionally use a compact debug formatter
        // for dynamic values. Match the variant as well as the wrapper so a
        // model-proven Count cannot replace an unrelated Add placeholder in
        // a multi-value ability.
        let unhinted_debug = format!("{:?}", value.unhinted());
        let variant = unhinted_debug
            .split(['(', ' ', '{'])
            .next()
            .unwrap_or(unhinted_debug.as_str());
        let compact_prefix = format!("SurfaceHinted {{ value: {variant}");
        let compact_suffix = ", hints: }";
        while let Some(start) = rendered.find(&compact_prefix) {
            let Some(relative_end) = rendered[start..].find(compact_suffix) else {
                break;
            };
            let end = start + relative_end + compact_suffix.len();
            rendered.replace_range(start..end, &describe_value(value));
        }
    }

    fn replace_condition(rendered: &mut String, condition: &Condition) {
        let debug = format!("{condition:?}");
        if rendered.contains(&debug) {
            *rendered = rendered.replace(&debug, &describe_condition(condition));
        }
    }

    fn restore_for_each_cost_reduction(rendered: &mut String, value: &Value, unit: &str) {
        fn singularize_first_word(phrase: &str) -> String {
            let Some((first, rest)) = phrase.split_once(' ') else {
                return phrase.strip_suffix('s').unwrap_or(phrase).to_string();
            };
            let singular = first
                .strip_suffix("ies")
                .map(|stem| format!("{stem}y"))
                .or_else(|| first.strip_suffix('s').map(str::to_string))
                .unwrap_or_else(|| first.to_string());
            format!("{singular} {rest}")
        }
        fn legacy_for_each_surface(value: &Value) -> Option<(i32, String)> {
            fn factor(value: &Value) -> Option<(i32, &Value)> {
                match value {
                    Value::SurfaceHinted { value, hints }
                        if hints.contains(&ironsmith_core::ValueSurfaceHint::ForEach) =>
                    {
                        Some((1, value.unhinted()))
                    }
                    Value::Scaled(inner, multiplier) if *multiplier > 0 => {
                        let (inner_multiplier, basis) = factor(inner)?;
                        Some((inner_multiplier.checked_mul(*multiplier)?, basis))
                    }
                    Value::Add(left, right) if left == right => {
                        let (multiplier, basis) = factor(left)?;
                        Some((multiplier.checked_mul(2)?, basis))
                    }
                    _ => None,
                }
            }
            let (multiplier, basis) = factor(value)?;
            let described = describe_value(basis);
            let basis = if let Some(counted) = described.strip_prefix("the number of ") {
                singularize_first_word(counted)
            } else if let Some(history) = described.strip_prefix("the amount of life ") {
                format!("1 life {history}")
            } else {
                return None;
            };
            Some((multiplier, basis))
        }
        fn collect_clauses(value: &Value, clauses: &mut Vec<(i32, String)>) -> Option<()> {
            if let Some(clause) = describe_for_each_multiplier_and_basis(value)
                .or_else(|| legacy_for_each_surface(value))
            {
                clauses.push(clause);
                return Some(());
            }
            let Value::Add(left, right) = value else {
                return None;
            };
            collect_clauses(left, clauses)?;
            collect_clauses(right, clauses)
        }
        let marker = format!("{unit} less to cast");
        if !rendered.contains(&marker) || rendered.contains(" for each ") {
            return;
        }
        if let Some((multiplier, basis)) =
            describe_for_each_multiplier_and_basis(value).or_else(|| legacy_for_each_surface(value))
        {
            let repeated_unit = if unit == "{X}" {
                format!("{{{multiplier}}}")
            } else {
                unit.repeat(multiplier.max(1) as usize)
            };
            *rendered = rendered.replace(
                &marker,
                &format!("{repeated_unit} less to cast for each {basis}"),
            );
            return;
        }
        let mut clauses = Vec::new();
        if collect_clauses(value, &mut clauses).is_none() || clauses.len() < 2 {
            return;
        }
        let replacement = clauses
            .into_iter()
            .map(|(multiplier, basis)| {
                let repeated_unit = if unit == "{X}" {
                    format!("{{{multiplier}}}")
                } else {
                    unit.repeat(multiplier.max(1) as usize)
                };
                format!("{repeated_unit} less to cast for each {basis}")
            })
            .collect::<Vec<_>>()
            .join(" and ");
        *rendered = rendered.replace(&marker, &replacement);
    }

    fn restore_for_each_anthem(rendered: &mut String, anthem: &ironsmith_core::Anthem) {
        fn signed(value: i32) -> String {
            if value >= 0 {
                format!("+{value}")
            } else {
                value.to_string()
            }
        }

        fn replace_where_x_tail(rendered: &mut String, old_pt: &str, new_pt: String) {
            let Some(where_x) = rendered.rfind(", where X is ") else {
                return;
            };
            let head = &rendered[..where_x];
            if !head.ends_with(old_pt) {
                return;
            }
            let start = where_x - old_pt.len();
            rendered.replace_range(start.., &new_pt);
        }

        match (&anthem.power, &anthem.toughness) {
            (
                ironsmith_core::AnthemValue::Dynamic(power),
                ironsmith_core::AnthemValue::Dynamic(toughness),
            ) if power == toughness => {
                if let Some((multiplier, basis)) = describe_for_each_multiplier_and_basis(power) {
                    replace_where_x_tail(
                        rendered,
                        "+X/+X",
                        format!("+{multiplier}/+{multiplier} for each {basis}"),
                    );
                }
            }
            (
                ironsmith_core::AnthemValue::Dynamic(power),
                ironsmith_core::AnthemValue::Fixed(toughness),
            ) => {
                if let Some((multiplier, basis)) = describe_for_each_multiplier_and_basis(power) {
                    replace_where_x_tail(
                        rendered,
                        &format!("+X/{}", signed(*toughness)),
                        format!("+{multiplier}/{} for each {basis}", signed(*toughness)),
                    );
                }
            }
            (
                ironsmith_core::AnthemValue::Fixed(power),
                ironsmith_core::AnthemValue::Dynamic(toughness),
            ) => {
                if let Some((multiplier, basis)) = describe_for_each_multiplier_and_basis(toughness)
                {
                    replace_where_x_tail(
                        rendered,
                        &format!("{}/+X", signed(*power)),
                        format!("{}/+{multiplier} for each {basis}", signed(*power)),
                    );
                }
            }
            _ => {}
        }
    }

    let Some(model) = static_ability.compiled_model() else {
        return rendered;
    };
    match &model.payload {
        ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { count, .. }
        | ironsmith_core::StaticAbilityPayload::EntersWithCountersIfCondition { count, .. } => {
            replace_value(&mut rendered, count)
        }
        ironsmith_core::StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
            count,
            count_condition,
            otherwise_count,
            ..
        } => {
            replace_value(&mut rendered, count);
            if let Some(condition) = count_condition {
                replace_condition(&mut rendered, condition);
            }
            if let Some(otherwise_count) = otherwise_count {
                replace_value(&mut rendered, otherwise_count);
            }
        }
        ironsmith_core::StaticAbilityPayload::Anthem(anthem) => {
            restore_for_each_anthem(&mut rendered, anthem);
            if let ironsmith_core::AnthemValue::Dynamic(value) = &anthem.power {
                replace_value(&mut rendered, value);
            }
            if let ironsmith_core::AnthemValue::Dynamic(value) = &anthem.toughness {
                replace_value(&mut rendered, value);
            }
            if let Some(condition) = &anthem.condition {
                replace_condition(&mut rendered, condition);
            }
        }
        ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => {
            restore_for_each_cost_reduction(&mut rendered, &reduction.amount, "{X}");
        }
        ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) => {
            restore_for_each_cost_reduction(&mut rendered, &reduction.amount, "{X}");
        }
        ironsmith_core::StaticAbilityPayload::ThisSpellCostReductionManaCost(reduction) => {
            if let Some(repetitions) = &reduction.repetitions {
                restore_for_each_cost_reduction(
                    &mut rendered,
                    repetitions,
                    &reduction.cost.to_oracle(),
                );
            }
        }
        ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { increase, .. } => {
            let cost = describe_total_cost(increase);
            if !cost.is_empty() && !cost.contains("Effect") {
                let prefix = "cost an additional \"";
                let suffix = "\" to activate";
                if let Some(start) = rendered.find(prefix)
                    && let Some(relative_end) = rendered[start + prefix.len()..].find(suffix)
                {
                    let value_start = start + prefix.len();
                    let value_end = value_start + relative_end;
                    rendered.replace_range(value_start..value_end, &cost);
                }
            }
        }
        ironsmith_core::StaticAbilityPayload::GrantAbility(grant) => {
            let granted =
                crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(
                    &grant.ability,
                );
            if let Some(keyword) = describe_keyword_ability(&granted)
                && keyword.starts_with("Ward—")
                && !keyword.contains("Effect")
            {
                let (grant_subject, singular) =
                    crate::static_abilities::grant_subject_with_set_quantifier(
                        &grant.filter,
                        grant.set_quantifier_surface,
                    );
                let verb = if singular { "has" } else { "have" };
                rendered = format!("{} {verb} \"{keyword}.\"", capitalize_first(&grant_subject));
            }
        }
        ironsmith_core::StaticAbilityPayload::Grants(spec) => {
            if let ironsmith_core::Grantable::DerivedAlternativeCast(
                ironsmith_core::DerivedAlternativeCast::GraveyardCastFromCardManaCost {
                    additional_costs,
                    ..
                },
            ) = &spec.grantable
                && !additional_costs.is_empty()
            {
                let additional = describe_additional_costs(additional_costs);
                let additional = [
                    ("sacrifice ", "sacrificing "),
                    ("exile ", "exiling "),
                    ("discard ", "discarding "),
                    ("pay ", "paying "),
                    ("reveal ", "revealing "),
                    ("return ", "returning "),
                    ("tap ", "tapping "),
                ]
                .into_iter()
                .find_map(|(verb, gerund)| {
                    additional
                        .strip_prefix(verb)
                        .map(|rest| format!("{gerund}{rest}"))
                })
                .unwrap_or(additional);
                if !additional.is_empty() && !additional.contains("Effect") {
                    rendered = rendered.replace(
                        "by paying its mana cost plus Effect",
                        &format!("by {additional} in addition to paying its other costs"),
                    );
                }
            }
        }
        ironsmith_core::StaticAbilityPayload::Conditional { condition, .. } => {
            replace_condition(&mut rendered, condition);
        }
        ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
            if let Some(condition) = &grant.condition {
                replace_condition(&mut rendered, condition);
            }
        }
        _ => {}
    }
    rendered
}

#[cfg(test)]
#[test]
fn dynamic_entry_counter_value_uses_typed_text_instead_of_runtime_debug() {
    let count = Value::LifeTotal(PlayerFilter::You)
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);
    let model = crate::static_abilities::CompiledStaticAbility::enters_with_counters_value(
        CounterType::Charge,
        count,
    );
    let ability = crate::static_abilities::StaticAbility::from_model(model);

    assert_eq!(
        describe_static_ability_with_subject(&ability, "this artifact"),
        "This artifact enters with X charge counters on it, where X is your life total"
    );
}

#[cfg(test)]
#[test]
fn dynamic_entry_counter_value_preserves_for_each_surface() {
    let count = Value::SpellsCastThisTurnMatching {
        player: PlayerFilter::Any,
        filter: ObjectFilter::spell(),
        exclude_source: true,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let model = crate::static_abilities::CompiledStaticAbility::enters_with_counters_value(
        CounterType::PlusOnePlusOne,
        count,
    );
    let ability = crate::static_abilities::StaticAbility::from_model(model);

    assert_eq!(
        describe_static_ability_with_subject(&ability, "this creature"),
        "This creature enters with a +1/+1 counter on it for each other spell cast this turn"
    );
}

#[cfg(test)]
#[test]
fn compact_dynamic_entry_counter_debug_uses_the_typed_count() {
    let count = Value::Count(ObjectFilter::creature().you_control())
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);
    let model = crate::static_abilities::CompiledStaticAbility::enters_with_counters_value(
        CounterType::PlusOnePlusOne,
        count,
    );
    let ability = crate::static_abilities::StaticAbility::from_model(model);

    let rendered = describe_static_ability_with_subject(&ability, "this creature");
    assert!(!rendered.contains("SurfaceHinted"), "{rendered}");
    assert!(
        rendered.contains("the number of creatures you control"),
        "{rendered}"
    );
}

#[cfg(test)]
#[test]
fn parsed_dynamic_entry_counter_never_exposes_compact_value_debug() {
    let oracle = "Draft this card face up.\nAs you draft a card, you may remove it from the draft face down.\nThis creature enters with X +1/+1 counters on it, where X is the number of cards you removed from the draft with cards named Cogwork Grinder.";
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Cogwork Grinder",
    )
    .card_types(vec![CardType::Artifact, CardType::Creature])
    .parse_text(oracle)
    .expect("dynamic draft counter card should compile");

    let ability = definition
        .abilities
        .last()
        .expect("entry-counter static ability");
    let AbilityKind::Static(static_ability) = &ability.kind else {
        panic!("entry-counter ability should be static");
    };
    let direct = describe_static_ability_with_subject(static_ability, "this creature");
    assert!(!direct.contains("SurfaceHinted"), "{direct}");
    assert!(describe_keyword_ability(ability).is_none());
    let ability_lines = super::abilities_and_costs::describe_ability(
        definition.abilities.len(),
        ability,
        "this creature",
        true,
    );
    assert!(
        ability_lines
            .iter()
            .all(|line| !line.contains("SurfaceHinted")),
        "{ability_lines:#?}"
    );

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(!rendered.contains("SurfaceHinted"), "{rendered}");
    assert!(
        rendered.contains("the number of cards named Cogwork Grinder"),
        "{rendered}"
    );
}

#[cfg(test)]
#[test]
fn parsed_for_each_anthem_preserves_the_authored_unit_modifier() {
    let oracle = "Equipped creature gets +1/+1 for each color among permanents you control.\nAs long as this Equipment is attached to a creature, your opponents can't cast spells during your turn.\nEquip {2}";
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Conqueror's Flail",
    )
    .card_types(vec![CardType::Artifact])
    .subtypes(vec![crate::types::Subtype::Equipment])
    .parse_text(oracle)
    .expect("for-each anthem should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[cfg(test)]
#[test]
fn self_cost_reduction_uses_typed_for_each_multiplier_and_basis() {
    let amount = Value::MaxCardsDrawnThisTurn(PlayerFilter::You)
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
    let model = crate::static_abilities::CompiledStaticAbility {
        id: Some(crate::static_abilities::StaticAbilityId::ThisSpellCostReduction),
        label: "this spell cost reduction".to_string(),
        payload: ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(
            ironsmith_core::ThisSpellCostReduction::new(
                amount,
                crate::static_abilities::ThisSpellCostCondition::Always,
            ),
        ),
    };
    let ability = crate::static_abilities::StaticAbility::from_model(model);

    assert_eq!(
        describe_static_ability_with_subject(&ability, "this spell"),
        "This spell costs {1} less to cast for each card you've drawn this turn"
    );
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
        && zone_change.object_filter.union_is_one_or_more()
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

    if let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        && zone_change.player == crate::triggers::zone_changes::PlayerRelation::Any
        && zone_change.count_mode == crate::triggers::zone_changes::CountMode::Each
        && zone_change.from == crate::triggers::zone_changes::ZonePattern::Any
        && zone_change.to == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        && zone_change.object_filter.owner == Some(PlayerFilter::Opponent)
        && zone_change.object_filter.card_types == [CardType::Creature]
    {
        return "Whenever a creature card is put into an opponent's graveyard from anywhere"
            .to_string();
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
        describe_upkeep_and_attached_land_becomes_tapped_trigger(&triggered.trigger)
            .or_else(|| {
                describe_this_enters_and_your_upkeep_trigger(&triggered.trigger, self_subject)
            })
            .or_else(|| describe_this_attacks_or_dies_trigger(&triggered.trigger))
            .or_else(|| describe_this_blocks_or_becomes_blocked_by_trigger(&triggered.trigger))
            .or_else(|| describe_becomes_blocked_trigger(&triggered.trigger))
            .or_else(|| describe_attack_with_serial_keyword_filter(&triggered.trigger))
            .unwrap_or_else(|| triggered.trigger.display());
    if triggered
        .trigger
        .downcast_ref::<crate::triggers::combat::AttacksTrigger>()
        .is_some_and(|attacks| {
            !attacks.one_or_more
                && !attacks.filter.source
                && !attacks.filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
                })
        })
        && let Some(rest) = trigger_surface.strip_prefix("Whenever ")
        && let Some((subject, tail)) = rest.split_once(" attacks")
        && !matches!(
            subject.split_whitespace().next(),
            Some("a" | "an" | "the" | "this")
        )
    {
        trigger_surface = format!(
            "Whenever {} attacks{tail}",
            with_indefinite_article(subject)
        );
    }
    if matches!(
        trigger_frequency,
        Some(TriggerFrequencySurface::FirstTimeThisTurn)
    ) {
        if triggered
            .trigger
            .downcast_ref::<crate::triggers::YouGainLifeTrigger>()
            .is_some_and(|trigger| trigger.during_turn == Some(PlayerFilter::You))
            && let Some(base) = trigger_surface.strip_suffix(" during your turn")
        {
            trigger_surface = format!("{base} for the first time during each of your turns");
        } else {
            trigger_surface.push_str(" for the first time each turn");
        }
    }
    rewrite_source_bound_trigger_subject(triggered, trigger_surface, self_subject)
}

pub(super) fn apply_attacks_while_most_life_surface(
    triggered: &crate::ability::TriggeredAbility,
    trigger_surface: &mut String,
    intervening_condition: &mut Option<Condition>,
) {
    if triggered.presentation_label.is_none()
        && triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
            .is_some()
        && matches!(
            intervening_condition.as_ref(),
            Some(Condition::PlayerHasNoOpponentWithMoreLifeThan {
                player: PlayerFilter::You,
            })
        )
        && trigger_surface.ends_with(" attacks")
    {
        trigger_surface.push_str(" while you have the most life or are tied for most life");
        *intervening_condition = None;
    }
}

/// Render a thresholded attack group whose object predicate is an inclusive
/// serial keyword list. The typed `any_of` branches prove both the inclusive
/// predicate and each keyword arm, so Oracle's relative-clause surface is
/// unambiguous: "at least N creatures that have A, B, and/or C."
fn describe_attack_with_serial_keyword_filter(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let attacks = trigger.downcast_ref::<crate::triggers::combat::AttacksTrigger>()?;
    if !attacks.one_or_more
        || attacks.min_total_attackers < 2
        || attacks.max_total_attackers.is_some()
        || !attacks.filter.union_is_one_or_more()
        || attacks.filter.controller != Some(PlayerFilter::You)
        || attacks
            .filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || attacks.filter.targets_only_player.is_some()
        || attacks.filter.any_of.len() < 2
        || !attacks.filter.any_of.iter().all(is_positive_keyword_branch)
    {
        return None;
    }

    let minimum = ironsmith_core::cardinal_word(attacks.min_total_attackers as u32)
        .unwrap_or_else(|| attacks.min_total_attackers.to_string());
    let displayed = trigger.display();
    let canonical_prefix = format!("Whenever you attack with {minimum} or more ");
    let subject = displayed.strip_prefix(&canonical_prefix)?;
    let (object_subject, keyword_list) = subject.rsplit_once(" with ")?;
    Some(format!(
        "Whenever you attack with at least {minimum} {object_subject} that have {keyword_list}"
    ))
}

fn is_positive_keyword_branch(filter: &ObjectFilter) -> bool {
    if filter.static_abilities.len() + filter.ability_markers.len() != 1
        || !filter.excluded_static_abilities.is_empty()
        || !filter.excluded_ability_markers.is_empty()
    {
        return false;
    }
    let mut semantic_remainder = filter.clone();
    semantic_remainder.static_abilities.clear();
    semantic_remainder.ability_markers.clear();
    semantic_remainder == ObjectFilter::default()
}

#[cfg(test)]
mod serial_keyword_attack_surface_tests {
    use super::*;

    #[test]
    fn renders_thresholded_attack_keyword_union_as_relative_clause() {
        use crate::static_abilities::StaticAbilityId::{
            DoubleStrike, FirstStrike, Haste, Vigilance,
        };

        let mut filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
        filter.any_of = [FirstStrike, DoubleStrike, Vigilance, Haste]
            .into_iter()
            .map(|ability| {
                let mut branch = ObjectFilter::default();
                branch.static_abilities.push(ability);
                branch
            })
            .collect();
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        filter.set_union_one_or_more(true);
        let trigger = crate::triggers::Trigger::attacks_one_or_more_with_min_total(filter, 2);

        assert_eq!(
            describe_attack_with_serial_keyword_filter(&trigger).as_deref(),
            Some(
                "Whenever you attack with at least two creatures that have first strike, double strike, vigilance, and/or haste"
            )
        );
    }
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

fn describe_upkeep_and_attached_land_becomes_tapped_trigger(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let or_trigger = trigger.downcast_ref::<crate::triggers::OrTrigger>()?;
    let [first, second] = or_trigger.triggers.as_slice() else {
        return None;
    };
    let (upkeep, tapped) = if let (Some(upkeep), Some(tapped)) = (
        first.downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>(),
        second.downcast_ref::<crate::triggers::PermanentBecomesTappedTrigger>(),
    ) {
        (upkeep, tapped)
    } else if let (Some(tapped), Some(upkeep)) = (
        first.downcast_ref::<crate::triggers::PermanentBecomesTappedTrigger>(),
        second.downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>(),
    ) {
        (upkeep, tapped)
    } else {
        return None;
    };
    if upkeep.player != PlayerFilter::You {
        return None;
    }

    let mut actual = tapped.filter.clone();
    actual.zone = None;
    let mut expected = ObjectFilter::default();
    expected.card_types.push(CardType::Land);
    expected = expected.match_tagged(
        TagKey::from("enchanted"),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    if actual != expected {
        return None;
    }

    Some("At the beginning of your upkeep and whenever enchanted land becomes tapped".to_string())
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

    #[test]
    fn joins_upkeep_and_attached_land_tap_with_authored_introductions() {
        let mut attached_land = ObjectFilter::default();
        attached_land.card_types.push(CardType::Land);
        attached_land = attached_land.match_tagged(
            TagKey::from("enchanted"),
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        let trigger = crate::triggers::Trigger::new(crate::triggers::OrTrigger::two(
            crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::You),
            crate::triggers::Trigger::new(crate::triggers::PermanentBecomesTappedTrigger::new(
                attached_land,
            )),
        ));

        assert_eq!(
            describe_upkeep_and_attached_land_becomes_tapped_trigger(&trigger).as_deref(),
            Some("At the beginning of your upkeep and whenever enchanted land becomes tapped")
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

    for source_surface in ["this creature", "this permanent", "this card"] {
        for frequency in ["Whenever ", "When "] {
            if surface.starts_with(&format!("{frequency}{source_surface}")) {
                return surface.replacen(source_surface, self_subject, 1);
            }
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
        && let Some(attachment) = trigger
            .filter
            .tagged_constraints
            .iter()
            .find_map(|constraint| {
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

fn blocks_or_becomes_blocked_by_creature(triggered: &crate::ability::TriggeredAbility) -> bool {
    let or_trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::OrTrigger>();
    let Some(or_trigger) = or_trigger else {
        return false;
    };
    let [first, second] = or_trigger.triggers.as_slice() else {
        return false;
    };
    let blocks = first
        .downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>());
    let blocked_by = first
        .downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>());
    let (Some(blocks), Some(blocked_by)) = (blocks, blocked_by) else {
        return false;
    };
    blocks.blocked_filter == blocked_by.blocker_filter
        && blocks
            .blocked_filter
            .card_types
            .contains(&CardType::Creature)
}

fn describe_destroy_attached_to_tagged_creature(
    destroy_effect: &Effect,
    tag: &TagKey,
) -> Option<String> {
    let destroy = unwrap_basic_tag_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(attached_filter) = destroy.spec.base() else {
        return None;
    };
    let matching_anchors = attached_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag == *tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
        .count();
    if matching_anchors != 1
        || attached_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag != *tag
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
        "destroy all {} attached to that creature",
        pluralize_noun_phrase(attachment)
    ))
}

/// A combat trigger can retain the other combatant under a tag. Keep that
/// attachment anchor explicit instead of emitting the ambiguous pronoun `it`.
fn describe_destroy_attached_to_triggering_creature(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !blocks_or_becomes_blocked_by_creature(triggered) {
        return None;
    }
    let [tag_effect, destroy_effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    describe_destroy_attached_to_tagged_creature(destroy_effect, &tag.tag)
}

/// The same combat relationship can schedule attachment destruction for end
/// of combat; preserve the typed anchor through the delayed wrapper as well.
fn describe_delayed_destroy_attached_to_triggering_creature(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !blocks_or_becomes_blocked_by_creature(triggered) {
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
    describe_destroy_attached_to_tagged_creature(destroy_effect, &tag.tag)
        .map(|text| format!("{text} at end of combat"))
}

fn describe_manifest_dread_graveyard_card_to_hand(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let keyword = triggered
        .trigger
        .downcast_ref::<crate::triggers::KeywordActionTrigger>()?;
    if keyword.action != crate::events::KeywordActionKind::ManifestDread
        || keyword.player != PlayerFilter::You
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, move_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    if tag.tag.as_str() != crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG {
        return None;
    }
    let move_to_hand = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_hand.zone != Zone::Hand || move_to_hand.to_top {
        return None;
    }
    let moves_manifest_dread_card = match move_to_hand.target.base() {
        ChooseSpec::Tagged(move_tag) => move_tag == &tag.tag,
        ChooseSpec::Object(filter) => {
            filter.zone == Some(Zone::Graveyard)
                && filter.tagged_constraints.len() == 1
                && filter.tagged_constraints[0].tag == tag.tag
                && filter.tagged_constraints[0].relation
                    == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }
        _ => false,
    };
    if !moves_manifest_dread_card {
        return None;
    }

    Some("put a card you put into your graveyard this way into your hand".to_string())
}

fn describe_draw_step_player_life_loss_then_search(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let draw_step = triggered
        .trigger
        .downcast_ref::<crate::triggers::phase_step::BeginningOfDrawStepTrigger>()?;
    if draw_step.player != PlayerFilter::Any {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [sequence_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let [life_effect, search_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let life_loss = life_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let search = search_effect.downcast_ref::<crate::effects::SearchLibraryEffect>()?;
    let ChooseSpec::Player(life_player) = life_loss.player.base() else {
        return None;
    };
    if life_player != &PlayerFilter::IteratedPlayer
        || &search.chooser != life_player
        || &search.player != life_player
        || search.filter.owner.as_ref() != Some(life_player)
        || search.destination != Zone::Hand
        || search.reveal
        || search.library_position_from_top.is_some()
    {
        return None;
    }

    let life_text = describe_effect(life_effect);
    let search_text = describe_effect(search_effect);
    let search_tail = search_text.strip_prefix("That player ")?;
    Some(format!(
        "{}, {}",
        life_text.trim_end_matches('.'),
        search_tail.replacen(" into hand", " into their hand", 1)
    ))
}

fn describe_optional_source_cant_attack_then_vigilance_rule(
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
) -> Option<String> {
    if triggered
        .effects
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return None;
    }
    let effects = triggered.effects.flattened_default_effects();
    let [optional_effect, result_effect] = effects else {
        return None;
    };

    let optional = optional_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = optional
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [restriction_effect] = may.effects.as_slice() else {
        return None;
    };
    let restriction = restriction_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Attack(attacker) = &restriction.restriction else {
        return None;
    };
    let mut normalized_attacker = attacker.clone();
    normalized_attacker.source_surface = None;
    normalized_attacker.union_surface = Default::default();
    normalized_attacker.set_explicit_card_type_noun(None);
    if normalized_attacker.source
        && normalized_attacker.zone == Some(Zone::Battlefield)
        && normalized_attacker.card_types.as_slice() == [CardType::Creature]
    {
        // The named-source parser retains the authored permanent domain on a
        // source identity filter. For this structural recognizer, that
        // redundant creature/battlefield scope is equivalent to `Source`.
        normalized_attacker.zone = None;
        normalized_attacker.card_types.clear();
    }
    if !may
        .decider
        .as_ref()
        .is_none_or(|decider| decider == &PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || normalized_attacker != ObjectFilter::source()
        || restriction.duration != Until::EndOfCombat
        || restriction.start != crate::effect::RestrictionStart::Immediate
    {
        return None;
    }

    let result = result_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if result.condition != optional.id
        || result.predicate != EffectPredicate::Happened
        || !result.else_.is_empty()
    {
        return None;
    }
    let [vigilance_effect] = result.then.as_slice() else {
        return None;
    };
    let vigilance = vigilance_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let crate::continuous::EffectTarget::Filter(filter) = &vigilance.target else {
        return None;
    };
    let mut normalized_filter = filter.clone();
    normalized_filter.union_surface = Default::default();
    normalized_filter.set_explicit_card_type_noun(None);
    let expected_filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    let Some(crate::continuous::Modification::AddAbility(ability)) = &vigilance.modification else {
        return None;
    };
    if normalized_filter != expected_filter
        || ability.id() != crate::static_abilities::StaticAbilityId::Vigilance
        || !vigilance.additional_modifications.is_empty()
        || !vigilance.runtime_modifications.is_empty()
        || vigilance.condition != Some(Condition::SourceIsUntapped)
        || vigilance.until != Until::EndOfCombat
        || vigilance.lock_filter_at_resolution
    {
        return None;
    }

    Some(format!(
        "you may have {subject} gain \"{subject} can't attack\" until end of combat. If you do, attacking doesn't cause creatures you control to tap this combat if {subject} is untapped"
    ))
}

#[cfg(test)]
mod optional_source_cant_attack_then_vigilance_rule_tests {
    #[test]
    fn legendary_source_round_trips_the_old_vigilance_surface() {
        let oracle = "At the beginning of combat on your turn, you may have Johan gain \"Johan can't attack\" until end of combat. If you do, attacking doesn't cause creatures you control to tap this combat if Johan is untapped.";
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Johan",
        )
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![crate::types::CardType::Creature])
        .parse_text(oracle)
        .expect("Johan-style combat choice should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()],
            "{definition:#?}"
        );
    }
}

/// ETB trigger bodies use `it` for ordinary references to the object that
/// entered. A continuous-effect duration, however, can explicitly refer to
/// the source rather than the affected object. Preserve that distinction
/// before the broader resolution-subject rewrite turns `this source` into
/// `it`, which would otherwise make the duration read as though it were tied
/// to the most recently mentioned target or exiled card.
fn preserve_source_bound_reference_phrases(line: String, source_subject: &str) -> String {
    line.replace("this source remains", &format!("{source_subject} remains"))
        .replace(
            "this permanent leaves the battlefield",
            &format!("{source_subject} leaves the battlefield"),
        )
        .replace(
            "this source leaves the battlefield",
            &format!("{source_subject} leaves the battlefield"),
        )
        .replace(
            "you control this source",
            &format!("you control {source_subject}"),
        )
        .replace(
            "this source exiles another card",
            &format!("{source_subject} exiles another card"),
        )
        .replace(
            "Attach this source to",
            &format!("Attach {source_subject} to"),
        )
        .replace(
            "attach this source to",
            &format!("attach {source_subject} to"),
        )
}

#[cfg(test)]
#[test]
fn source_bound_durations_survive_etb_resolution_pronoun_rewriting() {
    let animation = preserve_source_bound_reference_phrases(
        "target artifact becomes an artifact creature for as long as this source remains on the battlefield"
            .to_string(),
        "this creature",
    );
    assert_eq!(
        normalize_ability_self_reference_surface(&animation, "it"),
        "target artifact becomes an artifact creature for as long as this creature remains on the battlefield"
    );

    let permission = preserve_source_bound_reference_phrases(
        "you may play that card for as long as you control this source".to_string(),
        "this creature",
    );
    assert_eq!(
        normalize_ability_self_reference_surface(&permission, "it"),
        "you may play that card for as long as you control this creature"
    );

    let linked_exile = preserve_source_bound_reference_phrases(
        "Exile target creature until this permanent leaves the battlefield".to_string(),
        "this enchantment",
    );
    assert_eq!(
        normalize_ability_self_reference_surface(&linked_exile, "it"),
        "Exile target creature until this enchantment leaves the battlefield"
    );

    let attachment = preserve_source_bound_reference_phrases(
        "Cloak the top card of your library, then Attach this source to it".to_string(),
        "this Equipment",
    );
    assert_eq!(
        normalize_ability_self_reference_surface(&attachment, "it"),
        "Cloak the top card of your library, then Attach this Equipment to it"
    );
}

fn describe_destroy_source_and_triggering_attacker(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let blocks = triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>()?;
    if blocks.min_blocked_objects.is_some() {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let (tag, destroy_source, destroy_attacker) = match segment.default_effects.as_slice() {
        [tag, destroy_source, destroy_attacker] => (tag, destroy_source, destroy_attacker),
        [tag, coordinated] => {
            let sequence = coordinated.downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
                return None;
            }
            let [destroy_source, destroy_attacker] = sequence.effects.as_slice() else {
                return None;
            };
            (tag, destroy_source, destroy_attacker)
        }
        _ => return None,
    };
    let tag = tag.downcast_ref::<crate::effects::TagTriggeringAttackerEffect>()?;
    if tag.filter.as_ref() != Some(&blocks.blocked_filter) {
        return None;
    }
    let destroy_source = unwrap_basic_tag_wrappers(destroy_source)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let destroy_attacker = unwrap_basic_tag_wrappers(destroy_attacker)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    if !matches!(destroy_source.spec.base(), ChooseSpec::Source)
        || !matches!(destroy_attacker.spec.base(), ChooseSpec::Tagged(candidate) if candidate == &tag.tag)
    {
        return None;
    }
    Some("Destroy both creatures".to_string())
}

/// Recombine an attack-triggered temporary quoted restriction with the
/// defending player's sacrifice payment. The compiler stores the sacrifice
/// as an executable `UnlessPays` cost and the quoted rule as a typed static
/// ability grant; this surface is valid only after proving both structures.
fn describe_attack_grant_unblockable_unless_defender_sacrifices(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisAttacksTrigger>()?;
    if triggered.intervening_if.is_some() || !triggered.choices.is_empty() {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let unless =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless.player != PlayerFilter::Defending
        || unless.leading_surface
        || unless.before_delayed_step
    {
        return None;
    }
    let [cost] = unless.cost.as_all()? else {
        return None;
    };
    let sacrifice = sacrifice_view(cost.effect_ref()?)?;
    if sacrifice.filter != &ObjectFilter::creature()
        || sacrifice.count.unhinted() != &Value::Fixed(1)
        || sacrifice.player != &PlayerFilter::You
    {
        return None;
    }

    let [grant_effect] = unless.effects.as_slice() else {
        return None;
    };
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.until != Until::EndOfTurn
        || grant.condition.is_some()
        || !grant.additional_modifications.is_empty()
        || !grant.runtime_modifications.is_empty()
        || !(matches!(grant.target, crate::continuous::EffectTarget::Source)
            || grant
                .target_spec
                .as_ref()
                .is_some_and(|target| matches!(target.unhinted(), ChooseSpec::Source)))
    {
        return None;
    }
    let crate::continuous::Modification::AddAbility(ability) = grant.modification.as_ref()? else {
        return None;
    };
    let (crate::effect::Restriction::BlockSpecificAttacker { blockers, attacker }, _, condition) =
        ability.rule_restriction_parts()?
    else {
        return None;
    };
    if condition.is_some()
        || blockers != &ObjectFilter::creature()
        || attacker != &ObjectFilter::source()
    {
        return None;
    }

    Some(
        "it gains \"this creature can't be blocked\" until end of turn unless defending player sacrifices a creature of their choice"
            .to_string(),
    )
}

/// Render an excess-damage trigger whose follow-up reuses both typed pieces
/// of the triggering event: its numeric excess and its damaged permanent.
fn describe_excess_noncombat_damage_to_other_target(
    triggered: &crate::ability::TriggeredAbility,
    fallback_subject: &str,
) -> Option<String> {
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::IsDealtDamageTrigger>()?;
    if trigger.combat_only || !trigger.noncombat_only || !trigger.excess_only {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, damage_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let damaged = tag_effect.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>()?;
    let unwrapped_damage = unwrap_basic_tag_wrappers(damage_effect);
    let (damage, source_subject) = if let Some(with_source) =
        unwrapped_damage.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
    {
        if !matches!(with_source.source.unhinted(), ChooseSpec::Source) {
            return None;
        }
        let damage = unwrap_basic_tag_wrappers(&with_source.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()?;
        let source_subject = with_source
            .source
            .source_reference_surface()
            .map(crate::target::SourceReferenceSurface::display_text)
            .unwrap_or_else(|| fallback_subject.to_string());
        (damage, source_subject)
    } else {
        (
            unwrapped_damage.downcast_ref::<crate::effects::DealDamageEffect>()?,
            fallback_subject.to_string(),
        )
    };
    if damage.source_is_combat
        || damage.unpreventable
        || !damage
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
        || !matches!(
            damage.amount.unhinted(),
            Value::EventValue(EventValueSpec::Amount)
        )
    {
        return None;
    }
    let ChooseSpec::Target(target) = damage.target.unhinted() else {
        return None;
    };
    let ChooseSpec::ObjectOrPlayer(object_filter, PlayerFilter::Any) = target.unhinted() else {
        return None;
    };
    let expected_filter = ObjectFilter::permanent().not_tagged(damaged.tag.clone());
    if object_filter != &expected_filter {
        return None;
    }

    Some(format!(
        "{source_subject} deals damage equal to the excess to any target other than that permanent"
    ))
}

#[cfg(test)]
#[test]
fn attack_grant_unblockable_unless_defender_sacrifices_is_structural() {
    let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature(),
                    ObjectFilter::source(),
                ),
                "This creature can't be blocked.".to_string(),
            ),
        ),
        Until::EndOfTurn,
    ));
    let unless = Effect::new(crate::effects::UnlessPaysEffect::new_total_cost(
        vec![grant],
        PlayerFilter::Defending,
        crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::sacrifice(
            ObjectFilter::creature(),
        )]),
    ));
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_attacks(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![unless]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_attack_grant_unblockable_unless_defender_sacrifices(&triggered).as_deref(),
        Some(
            "it gains \"this creature can't be blocked\" until end of turn unless defending player sacrifices a creature of their choice"
        )
    );
    assert_eq!(
        describe_triggered_resolution_text(&triggered, "this creature", true).as_deref(),
        Some(
            "it gains \"this creature can't be blocked\" until end of turn unless defending player sacrifices a creature of their choice"
        )
    );
}

#[cfg(test)]
#[test]
fn excess_noncombat_damage_followup_reuses_event_amount_and_permanent_identity() {
    let damaged = TagKey::from("damaged");
    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::FullName("Excess Herald".to_string()),
        ),
    );
    let target = ChooseSpec::target(ChooseSpec::ObjectOrPlayer(
        ObjectFilter::permanent().not_tagged(damaged.clone()),
        PlayerFilter::Any,
    ));
    let damage = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
        source,
        Effect::deal_damage(
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
            target,
        ),
    ));
    let mut trigger_filter = ObjectFilter::default();
    trigger_filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
    trigger_filter.controller = Some(PlayerFilter::Opponent);
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::is_dealt_excess_noncombat_damage(ChooseSpec::Object(
            trigger_filter,
        )),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::tag_triggering_damage_target(damaged),
            damage,
        ]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    let expected =
        "Excess Herald deals damage equal to the excess to any target other than that permanent";
    assert_eq!(
        describe_excess_noncombat_damage_to_other_target(&triggered, "this creature").as_deref(),
        Some(expected)
    );
    assert_eq!(
        describe_triggered_resolution_text(&triggered, "this creature", true).as_deref(),
        Some(expected)
    );

    let mut implicit_source = triggered.clone();
    implicit_source.effects.segments[0].default_effects[1] = Effect::deal_damage(
        Value::EventValue(EventValueSpec::Amount)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
        ChooseSpec::target(ChooseSpec::ObjectOrPlayer(
            ObjectFilter::permanent().not_tagged(TagKey::from("damaged")),
            PlayerFilter::Any,
        )),
    );
    assert_eq!(
        describe_excess_noncombat_damage_to_other_target(&implicit_source, "Excess Herald")
            .as_deref(),
        Some(expected)
    );

    let mut changed_target = implicit_source;
    changed_target.effects.segments[0].default_effects[1] = Effect::deal_damage(
        Value::EventValue(EventValueSpec::Amount)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
        ChooseSpec::target(ChooseSpec::ObjectOrPlayer(
            ObjectFilter::permanent(),
            PlayerFilter::Any,
        )),
    );
    assert_eq!(
        describe_excess_noncombat_damage_to_other_target(&changed_target, "Excess Herald"),
        None
    );
}

/// Render the linked optional-damage/no-combat-assignment program only after
/// proving the same unblocked attacker supplies the damage, its power basis,
/// the deciding controller, and the conditional followup.
fn describe_unblocked_attacker_controller_damage_offer(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    triggered
        .trigger
        .downcast_ref::<crate::triggers::combat::AttacksAndIsntBlockedTrigger>()?;
    if triggered.intervening_if.is_some() || triggered.presentation_label.is_some() {
        return None;
    }
    let [offer_segment, result_segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !offer_segment.self_replacements.is_empty()
        || !result_segment.self_replacements.is_empty()
        || offer_segment.starts_new_source_line
        || result_segment.starts_new_source_line
    {
        return None;
    }

    let [tag_effect, offer_effect] = offer_segment.default_effects.as_slice() else {
        return None;
    };
    let triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let offer = offer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = offer.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.fallback != crate::decision::FallbackStrategy::Decline
        || !matches!(
            may.decider.as_ref(),
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)))
                if tag.as_str() == triggering.tag.as_str()
        )
    {
        return None;
    }
    let [announced_target] = triggered.choices.as_slice() else {
        return None;
    };
    let damage_effect = match may.effects.as_slice() {
        [damage_effect] => damage_effect,
        [target_effect, damage_effect] => {
            let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            if &target_only.target != announced_target
                || target_only.chooser.is_some()
                || target_only.explicit_declaration
            {
                return None;
            }
            damage_effect
        }
        _ => return None,
    };
    let with_source = unwrap_basic_tag_wrappers(damage_effect)
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(
        with_source.source.unhinted(),
        ChooseSpec::Tagged(tag) if tag.as_str() == triggering.tag.as_str()
    ) {
        return None;
    }
    let damage = with_source
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat || damage.unpreventable {
        return None;
    }
    let Value::Add(power, offset) = damage.amount.unhinted() else {
        return None;
    };
    if !matches!(
        power.unhinted(),
        Value::PowerOf(spec)
            if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == triggering.tag.as_str())
    ) || offset.unhinted() != &Value::Fixed(2)
    {
        return None;
    }
    let ChooseSpec::Target(target) = damage.target.unhinted() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target.unhinted() else {
        return None;
    };
    if announced_target != &damage.target {
        return None;
    }
    let mut target_filter = target_filter.clone();
    let source_exclusions = target_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag.as_str() == triggering.tag.as_str()
        })
        .count();
    if source_exclusions != 1 {
        return None;
    }
    target_filter.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            && constraint.tag.as_str() == triggering.tag.as_str())
    });
    target_filter.union_surface = Default::default();
    target_filter.set_explicit_card_type_noun(None);
    target_filter.source_surface = None;
    if target_filter != ObjectFilter::creature().other() {
        return None;
    }

    let [result_effect] = result_segment.default_effects.as_slice() else {
        return None;
    };
    let result = result_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if result.condition != offer.id
        || result.predicate != EffectPredicate::Happened
        || !result.else_.is_empty()
        || result.prior_result_replacement_surface
    {
        return None;
    }
    let [assignment_effect] = result.then.as_slice() else {
        return None;
    };
    let assignment =
        assignment_effect.downcast_ref::<crate::effects::AssignNoCombatDamageEffect>()?;
    if assignment.until != Until::EndOfTurn
        || !matches!(
            assignment.source.unhinted(),
            ChooseSpec::Tagged(tag) if tag.as_str() == triggering.tag.as_str()
        )
    {
        return None;
    }

    Some(
        "its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn"
            .to_string(),
    )
}

#[cfg(test)]
mod unblocked_attacker_controller_damage_offer_tests {
    use super::*;

    fn fixture(offset: i32) -> crate::ability::TriggeredAbility {
        let tag = TagKey::from("triggering");
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().other().not_tagged(tag.clone()),
        ));
        let damage = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Tagged(tag.clone()),
            Effect::deal_damage(
                Value::Add(
                    Box::new(Value::PowerOf(Box::new(ChooseSpec::Tagged(tag.clone())))),
                    Box::new(Value::Fixed(offset)),
                ),
                target.clone(),
            ),
        ));
        let offer_id = ironsmith_core::EffectId(17);
        let offer = Effect::with_id(
            offer_id.0,
            Effect::may_player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(tag.clone())),
                vec![damage],
            ),
        );
        let result = Effect::if_then(
            offer_id,
            EffectPredicate::Happened,
            vec![Effect::assign_no_combat_damage(
                ChooseSpec::Tagged(tag.clone()),
                Until::EndOfTurn,
            )],
        );
        crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::attacks_and_isnt_blocked(
                ObjectFilter::creature().match_tagged(
                    "enchanted",
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                ),
            ),
            effects: crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment::from_effects(vec![
                    Effect::tag_triggering_object(tag),
                    offer,
                ]),
                crate::resolution::ResolutionSegment::from_effects(vec![result]),
            ]),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }
    }

    #[test]
    fn renders_linked_controller_choice_damage_and_combat_assignment() {
        let triggered = fixture(2);
        let expected = "its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn";
        assert_eq!(
            describe_unblocked_attacker_controller_damage_offer(&triggered).as_deref(),
            Some(expected)
        );
        assert_eq!(
            describe_triggered_resolution_text(&triggered, "this Aura", false).as_deref(),
            Some(expected)
        );
    }

    #[test]
    fn does_not_invent_the_oracle_surface_for_a_different_offset() {
        assert!(describe_unblocked_attacker_controller_damage_offer(&fixture(3)).is_none());
    }
}

/// Render an exact singular creature choice followed by the complement of the
/// source and that choice. The source surface and the tagged exclusion are
/// independent identity proofs; neither may be inferred from a generic
/// `other` filter.
fn describe_choose_creature_then_debuff_source_and_chosen_complement(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let [choice_segment, debuff_segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if choice_segment.starts_new_source_line
        || !choice_segment.self_replacements.is_empty()
        || !debuff_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let choice = unwrap_basic_tag_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choice.count.min != 1
        || choice.count.max != Some(1)
        || choice.count.dynamic_x
        || choice.count.up_to_x
        || choice.count.random
        || choice.count.explicit_exactly
        || choice.count_value.is_some()
        || choice.aggregate_constraint.is_some()
        || choice.chooser != PlayerFilter::You
        || choice.zone != Some(Zone::Battlefield)
        || !choice.additional_zones.is_empty()
        || choice.description != "Choose"
        || choice.is_search
        || choice.reveal
        || choice.search_mode != crate::effect::SearchSelectionMode::Exact
        || choice.search_reveal_reference_surface.is_some()
        || choice.search_result_reference_surface.is_some()
        || choice.search_top_in_any_order_surface.is_some()
        || choice.top_only
        || choice.bottom_only
        || choice.replace_tagged_objects
        || !choice.remember_as_chosen_object
    {
        return None;
    }
    let mut choice_filter = choice.filter.clone();
    choice_filter.union_surface = Default::default();
    if choice_filter != ObjectFilter::creature().controlled_by(PlayerFilter::Opponent) {
        return None;
    }

    let [debuff_effect] = debuff_segment.default_effects.as_slice() else {
        return None;
    };
    let debuff_effect = if let Some(sequence) =
        unwrap_basic_tag_wrappers(debuff_effect).downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.surface == ironsmith_core::SequenceSurface::CoordinatedLeadingDuration
        && sequence.result_label.is_none()
        && let [debuff] = sequence.effects.as_slice()
    {
        debuff
    } else {
        debuff_effect
    };
    let debuff = unwrap_basic_tag_wrappers(debuff_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let crate::continuous::EffectTarget::Filter(filter) = &debuff.target else {
        return None;
    };
    let [chosen_exclusion] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    if chosen_exclusion.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        || !(chosen_exclusion.tag == choice.tag
            || chosen_exclusion.tag.as_str() == ironsmith_core::CHOSEN_OBJECTS_TAG)
        || !filter.other
    {
        return None;
    }
    let source = filter
        .source_surface
        .as_ref()
        .map(crate::target::SourceReferenceSurface::display_text)?;
    let mut base_filter = filter.clone();
    base_filter.other = false;
    base_filter.source_surface = None;
    base_filter.tagged_constraints.clear();
    base_filter.union_surface = Default::default();
    if base_filter != ObjectFilter::creature() {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = debuff.runtime_modifications.as_slice()
    else {
        return None;
    };
    if power.unhinted() != &Value::Fixed(-2)
        || toughness.unhinted() != &Value::Fixed(-2)
        || debuff.until != crate::effect::Until::EndOfTurn
        || debuff.target_spec.is_some()
        || debuff.modification.is_some()
        || !debuff.additional_modifications.is_empty()
        || debuff.condition.is_some()
        || debuff.source_type.is_some()
        || debuff.source_reference_surface.is_some()
        || debuff.set_quantifier_surface.is_some()
        || debuff.type_retention_surface.is_some()
        || debuff.animation_pt_surface.is_some()
        || debuff.animation_duration_surface.is_some()
        || !debuff.lock_filter_at_resolution
        || debuff.resolve_set_pt_values_at_resolution
        || debuff.require_creature_target
    {
        return None;
    }

    let choice_text = describe_effect(choice_effect);
    let choice_text = choice_text
        .strip_prefix("You choose ")
        .map(|selection| format!("choose {selection}"))?;
    Some(format!(
        "{choice_text}. Until end of turn, creatures other than {source} and the chosen creature get -2/-2"
    ))
}

#[cfg(test)]
#[test]
fn chosen_creature_complement_renderer_requires_both_excluded_identities() {
    let chosen = TagKey::from(ironsmith_core::CHOSEN_OBJECTS_TAG);
    let choice = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
            1,
            PlayerFilter::You,
            chosen.clone(),
        )
        .in_zone(Zone::Battlefield)
        .remember_as_chosen_object(),
    );
    let source_surface =
        crate::target::SourceReferenceSurface::FullName("Linked Rival".to_string());
    let mut filter = ObjectFilter::creature().not_tagged(chosen);
    filter.other = true;
    filter.source_surface = Some(source_surface);
    let mut debuff = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Filter(filter),
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(-2),
            toughness: Value::Fixed(-2),
        },
        crate::effect::Until::EndOfTurn,
    );
    debuff.lock_filter_at_resolution = true;
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_enters_battlefield(),
        effects: crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![choice]),
            crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(debuff)]),
        ]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_choose_creature_then_debuff_source_and_chosen_complement(&triggered).as_deref(),
        Some(
            "choose a creature an opponent controls. Until end of turn, creatures other than Linked Rival and the chosen creature get -2/-2"
        )
    );

    let mut duration_wrapped = triggered.clone();
    let debuff = duration_wrapped.effects.segments[1].default_effects[0].clone();
    duration_wrapped.effects.segments[1].default_effects[0] = Effect::new(
        crate::effects::SequenceEffect::coordinated_with_leading_duration(vec![debuff]),
    );
    assert_eq!(
        describe_choose_creature_then_debuff_source_and_chosen_complement(&duration_wrapped)
            .as_deref(),
        Some(
            "choose a creature an opponent controls. Until end of turn, creatures other than Linked Rival and the chosen creature get -2/-2"
        )
    );

    let mut sentence_split = duration_wrapped.clone();
    sentence_split.effects.segments[1].starts_new_source_line = true;
    assert_eq!(
        describe_choose_creature_then_debuff_source_and_chosen_complement(&sentence_split)
            .as_deref(),
        Some(
            "choose a creature an opponent controls. Until end of turn, creatures other than Linked Rival and the chosen creature get -2/-2"
        )
    );

    let mut near_miss = triggered.clone();
    let debuff = near_miss.effects.segments[1].default_effects[0]
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("fixture debuff")
        .clone();
    let crate::continuous::EffectTarget::Filter(mut filter) = debuff.target.clone() else {
        unreachable!();
    };
    filter.tagged_constraints.clear();
    let mut debuff = debuff;
    debuff.target = crate::continuous::EffectTarget::Filter(filter);
    near_miss.effects.segments[1].default_effects[0] = Effect::new(debuff);
    assert!(
        describe_choose_creature_then_debuff_source_and_chosen_complement(&near_miss).is_none(),
        "the source-only complement must not invent a chosen-object exclusion"
    );
}

/// Render the exact optional self-exile/collect-evidence procedure while
/// retaining both tagged sets: the source card returned by the reflexive
/// branch and the independent graveyard cards exiled as evidence.
fn describe_optional_self_exile_collect_evidence_then_return(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let (tag_triggering, optional_with_id, if_effect) = match triggered.effects.segments.as_slice()
    {
        [segment] if segment.self_replacements.is_empty() => {
            let [tag_triggering, optional_with_id, if_effect] = segment.default_effects.as_slice()
            else {
                return None;
            };
            (tag_triggering, optional_with_id, if_effect)
        }
        [first_segment, second_segment]
            if first_segment.self_replacements.is_empty()
                && second_segment.self_replacements.is_empty() =>
        {
            let [tag_triggering, optional_with_id] = first_segment.default_effects.as_slice()
            else {
                return None;
            };
            let [if_effect] = second_segment.default_effects.as_slice() else {
                return None;
            };
            (tag_triggering, optional_with_id, if_effect)
        }
        _ => return None,
    };
    let Some(triggering) =
        tag_triggering.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    else {
        return None;
    };
    let with_id = optional_with_id.downcast_ref::<crate::effects::WithIdEffect>()?;
    let optional = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [choose_effect, source_exile_effect, evidence_exile_effect] = optional.effects.as_slice()
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let source_exile = source_exile_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let source_move = source_exile
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let evidence_loop =
        evidence_exile_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [evidence_move_effect] = evidence_loop.effects.as_slice() else {
        return None;
    };
    let evidence_move = evidence_move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    let expected_evidence_filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .match_tagged(
            triggering.tag.clone(),
            crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
        );
    let minimum = choose
        .aggregate_constraint
        .as_ref()
        .and_then(|constraint| {
            (constraint.metric == crate::effect::ChoiceAggregateMetric::ManaValue
                && matches!(constraint.maximum.unhinted(), Value::Fixed(i32::MAX)))
            .then_some(constraint.minimum.as_ref())
            .flatten()
        })?;
    let Value::Fixed(minimum) = minimum.unhinted() else {
        return None;
    };
    if *minimum < 0
        || choose.filter != expected_evidence_filter
        || !choose.count.is_any_number()
        || choose.count_value.is_some()
        || choose.chooser != PlayerFilter::You
        || choose.tag != evidence_loop.tag
        || choose.is_search
        || choose.reveal
        || source_exile.tag.as_str() == evidence_loop.tag.as_str()
        || source_move.target != ChooseSpec::Tagged(triggering.tag.clone())
        || source_move.zone != Zone::Exile
        || evidence_move.target != ChooseSpec::Iterated
        || evidence_move.zone != Zone::Exile
    {
        return None;
    }

    let conditional = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [return_effect] = conditional.then.as_slice() else {
        return None;
    };
    let returned = return_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if conditional.condition != with_id.id
        || conditional.predicate != crate::effect::EffectPredicate::Happened
        || !conditional.else_.is_empty()
        || returned.target != ChooseSpec::Tagged(source_exile.tag.clone())
        || returned.zone != Zone::Battlefield
        || !returned.enters_tapped
    {
        return None;
    }

    Some(format!(
        "you may exile it and collect evidence {minimum}. If you do, return this card to the battlefield tapped"
    ))
}

/// Preserve the revealed set as the comparison domain for a per-card
/// same-mana-value test. The runtime loop binds each current card to `__it__`;
/// the condition must compare it with a *different* member of the original
/// revealed tag.
fn describe_reveal_hand_cards_then_create_for_duplicate_mana_values(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.trigger.saga_chapters()? != [3] {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [choose_effect, reveal_effect, iterator_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect
        .downcast_ref::<crate::effects::RevealTaggedEffect>()
        .or_else(|| {
            reveal_effect
                .downcast_ref::<crate::effects::WithIdEffect>()?
                .effect
                .downcast_ref::<crate::effects::RevealTaggedEffect>()
        })?;
    let iterator = iterator_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [conditional_effect] = iterator.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(iterated_tag, comparison_filter) = &conditional.condition
    else {
        return None;
    };
    let mut expected_comparison = ObjectFilter::default().match_tagged(
        choose.tag.clone(),
        crate::filter::TaggedOpbjectRelation::SameManaValueAsAnotherTagged,
    );
    expected_comparison.zone = None;

    let mut expected_choice = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You);
    expected_choice.excluded_card_types = vec![CardType::Land];
    expected_choice.set_explicit_card_noun(true);
    if choose.filter != expected_choice
        || choose.count != crate::effect::ChoiceCount::up_to(5)
        || choose.count_value.is_some()
        || choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Hand)
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || reveal.tag != choose.tag
        || iterator.tag != choose.tag
        || iterated_tag.as_str() != "__it__"
        || comparison_filter != &expected_comparison
        || conditional.surface != ironsmith_core::ConditionalSurface::TrailingIf
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let [create_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let create = create_effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
        || create.token.card.card_types.as_slice() != [CardType::Artifact]
        || create.token.card.subtypes.as_slice() != [Subtype::Treasure]
        || create.enters_tapped
        || create.enters_attacking
    {
        return None;
    }

    Some(
        "reveal up to five nonland cards from your hand. For each of those cards that has the same mana value as another card revealed this way, create a Treasure token"
            .to_string(),
    )
}

fn describe_last_counter_destroy_attached_land_and_damage_controller(
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
) -> Option<String> {
    let counter = triggered
        .trigger
        .downcast_ref::<crate::triggers::CounterRemovedFromTrigger>()?;
    if !counter.last
        || !counter.one_or_more
        || counter.caused_by_source
        || counter.counter_type.is_none()
        || !counter.filter.source
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let effects = segment.default_effects.as_slice();
    let (destroy_effect, damage_effect) = if let [destroy_effect, damage_effect] = effects
        && unwrap_tag_wrapped_effect(destroy_effect)
            .downcast_ref::<crate::effects::DestroyEffect>()
            .is_some()
        && damage_with_source_view(damage_effect).is_some()
    {
        (destroy_effect, damage_effect)
    } else {
        let [tag_effect, sequence_effect] = effects else {
            return None;
        };
        let tag_attached =
            tag_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
        let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        let [destroy_effect, damage_effect] = sequence.effects.as_slice() else {
            return None;
        };
        if tag_attached.tag.as_str() != "enchanted"
            || sequence.surface != ironsmith_core::SequenceSurface::Coordinated
            || sequence.result_label.is_some()
        {
            return None;
        }
        (destroy_effect, damage_effect)
    };
    let destroy_tag = coordinated_effect_tag(destroy_effect);
    let destroy = unwrap_tag_wrapped_effect(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::Object(attached_filter) = destroy.spec.base() else {
        return None;
    };
    let mut actual = attached_filter.clone();
    actual.zone = None;
    let mut expected = ObjectFilter::default();
    expected.card_types.push(CardType::Land);
    expected = expected.match_tagged(
        TagKey::from("enchanted"),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    if actual != expected {
        return None;
    }

    let (damage_source, damage) = damage_with_source_view(damage_effect)?;
    if damage.source_is_combat || damage.unpreventable {
        return None;
    }
    if damage_source.is_some_and(|source| !matches!(source.unhinted(), ChooseSpec::Source)) {
        return None;
    }
    let controller_tag = match damage.target.base() {
        ChooseSpec::Player(
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            | PlayerFilter::AliasedControllerOf(crate::target::ObjectRef::Tagged(tag)),
        ) => tag,
        _ => return None,
    };
    if controller_tag.as_str() != "enchanted" && destroy_tag != Some(controller_tag) {
        return None;
    }

    Some(format!(
        "destroy enchanted land and {subject} deals {} damage to that land's controller",
        describe_value(&damage.amount)
    ))
}

#[cfg(test)]
mod last_counter_attached_land_resolution_tests {
    use super::*;

    fn ability(controller_tag: &str) -> crate::ability::TriggeredAbility {
        let mut attached_land = ObjectFilter::default();
        attached_land.card_types.push(CardType::Land);
        attached_land = attached_land.match_tagged(
            TagKey::from("enchanted"),
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        let destroy = Effect::new(crate::effects::DestroyEffect::with_spec(
            ChooseSpec::Object(attached_land),
        ))
        .tag("destroyed");
        let damage = Effect::deal_damage(
            Value::Fixed(2),
            ChooseSpec::Player(PlayerFilter::ControllerOf(
                crate::target::ObjectRef::Tagged(TagKey::from(controller_tag)),
            )),
        );
        crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::new(
                crate::triggers::CounterRemovedFromTrigger::new(ObjectFilter::source())
                    .counter_type(crate::object::CounterType::Named("ore".into()))
                    .one_or_more()
                    .last(),
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![destroy, damage]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }
    }

    fn attachment_setup_and_coordinated_ability() -> crate::ability::TriggeredAbility {
        let mut attached_land = ObjectFilter::default();
        attached_land.card_types.push(CardType::Land);
        attached_land = attached_land.match_tagged(
            TagKey::from("enchanted"),
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        let destroyed_tag = TagKey::from("destroyed_0");
        let destroy = Effect::new(crate::effects::DestroyEffect::with_spec(
            ChooseSpec::Object(attached_land),
        ))
        .tag(destroyed_tag.clone());
        let damage = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Source,
            Effect::deal_damage(
                Value::Fixed(2),
                ChooseSpec::Player(PlayerFilter::ControllerOf(
                    crate::target::ObjectRef::Tagged(destroyed_tag),
                )),
            ),
        ));
        crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::new(
                crate::triggers::CounterRemovedFromTrigger::new(ObjectFilter::source())
                    .counter_type(crate::object::CounterType::Named("ore".into()))
                    .one_or_more()
                    .last(),
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::new(crate::effects::TagAttachedToSourceEffect::new(
                    TagKey::from("enchanted"),
                )),
                Effect::new(crate::effects::SequenceEffect::coordinated(vec![
                    destroy, damage,
                ])),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }
    }

    #[test]
    fn last_counter_payload_preserves_attached_land_and_source_subject() {
        assert_eq!(
            describe_last_counter_destroy_attached_land_and_damage_controller(
                &ability("enchanted"),
                "this Aura",
            )
            .as_deref(),
            Some("destroy enchanted land and this Aura deals 2 damage to that land's controller")
        );
        assert_eq!(
            describe_last_counter_destroy_attached_land_and_damage_controller(
                &attachment_setup_and_coordinated_ability(),
                "this Aura",
            )
            .as_deref(),
            Some("destroy enchanted land and this Aura deals 2 damage to that land's controller")
        );
    }

    #[test]
    fn last_counter_payload_rejects_an_unrelated_controller_tag() {
        assert!(
            describe_last_counter_destroy_attached_land_and_damage_controller(
                &ability("another"),
                "this Aura",
            )
            .is_none()
        );
    }
}

/// Rejoin an attack trigger's optional dynamic subtype sacrifice with its
/// reflexive same-set pump and keyword grant. Every relationship is carried
/// by typed effect IDs, tags, and target specs; the renderer only restores the
/// authored compact "When you do" and plural coordinated-verb surface.
fn describe_attack_optional_dynamic_sacrifice_then_group_grant(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !triggered.choices.is_empty()
        || triggered.intervening_if.is_some()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let attacks = triggered
        .trigger
        .downcast_ref::<crate::triggers::AttacksTrigger>()?;
    if !attacks.one_or_more
        || attacks.min_total_attackers != 1
        || attacks.max_total_attackers.is_some()
        || attacks.filter != ObjectFilter::creature().controlled_by(PlayerFilter::You)
    {
        return None;
    }

    let [sacrifice_segment, reflexive_segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if [sacrifice_segment, reflexive_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return None;
    }
    let [sacrifice_root] = sacrifice_segment.default_effects.as_slice() else {
        return None;
    };
    let with_id = sacrifice_root.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [choose_effect, sacrifice_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let [_sacrificed_subtype] = choose.filter.subtypes.as_slice() else {
        return None;
    };
    let mut normalized_choose_filter = choose.filter.clone();
    normalized_choose_filter.subtypes.clear();
    if normalized_choose_filter != ObjectFilter::default()
        || choose.count.min != 0
        || choose.count.max.is_some()
        || !choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count.random
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Battlefield)
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.remember_as_chosen_object
    {
        return None;
    }
    let sacrifice =
        sacrifice_effect.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()?;
    let Value::Count(count_filter) = sacrifice.count.unhinted() else {
        return None;
    };
    if sacrifice.player != PlayerFilter::You
        || sacrifice.filter != ObjectFilter::tagged(choose.tag.clone())
        || *count_filter != ObjectFilter::tagged(choose.tag.clone())
    {
        return None;
    }

    let [reflexive_root] = reflexive_segment.default_effects.as_slice() else {
        return None;
    };
    let reflexive = reflexive_root.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()?;
    if reflexive.condition != with_id.id
        || reflexive.predicate != crate::effect::EffectPredicate::Happened
    {
        return None;
    }
    let [sequence_effect] = reflexive.effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if !matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::ResultConjunction {
            leading_duration: false
        }
    ) || sequence.result_label.is_some()
    {
        return None;
    }
    let [pump_effect, _] = sequence.effects.as_slice() else {
        return None;
    };
    let pump = unwrap_basic_tag_wrappers(pump_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if reflexive.choices.as_slice() != [pump.target_spec.as_ref()?.clone()] {
        return None;
    }
    let result = describe_targeted_pump_then_grant_same_objects(&sequence.effects)?;
    let sacrifice_text = lowercase_first(describe_effect(sacrifice_root).trim_end_matches('.'));
    if sacrifice_text.contains(". ") {
        return None;
    }
    Some(format!(
        "{sacrifice_text}. When you do, {}",
        lowercase_first(&result)
    ))
}

/// Rejoin an intervening-if legal-target choice with the exact spell copy and
/// fixed retarget that consume it. Lowering keeps the three authored
/// instructions in separate source segments, so the ordinary per-segment
/// list renderer cannot see that "those creatures" is the same set proved by
/// the intervening condition or that the final assignment consumes the exact
/// chosen-object tag.
fn describe_intervening_legal_target_copy_assignment(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let Condition::PlayerControls {
        player: PlayerFilter::You,
        filter: condition_filter,
    } = triggered.intervening_if.as_ref()?
    else {
        return None;
    };
    let [choice_segment, copy_segment, retarget_segment] = triggered.effects.segments.as_slice()
    else {
        return None;
    };
    if choice_segment.starts_new_source_line
        || !copy_segment.starts_new_source_line
        || !retarget_segment.starts_new_source_line
        || [choice_segment, copy_segment, retarget_segment]
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let [triggering_effect, choice_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let [copy_effect] = copy_segment.default_effects.as_slice() else {
        return None;
    };
    let [retarget_effect] = retarget_segment.default_effects.as_slice() else {
        return None;
    };
    let triggering =
        triggering_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let choice = choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if &choice.filter != condition_filter
        || choice.filter.zone != Some(Zone::Battlefield)
        || choice.filter.controller != Some(PlayerFilter::You)
        || choice.filter.card_types.as_slice() != [CardType::Creature]
        || !choice.filter.other
        || !matches!(
            choice.filter.could_be_targeted_by.as_ref(),
            Some(constraint)
                if matches!(&constraint.stack_object, crate::filter::ObjectRef::Tagged(tag) if tag == &triggering.tag)
        )
        || choice.chooser != PlayerFilter::You
        || !choice.count.is_single()
        || choice.count_value.is_some()
        || choice.aggregate_constraint.is_some()
        || choice.zone != Some(Zone::Battlefield)
        || !choice.additional_zones.is_empty()
        || choice.is_search
    {
        return None;
    }
    let (copied_tag, copy) = tagged_copy_spell_from_effect(copy_effect)?;
    if copy.count != Value::Fixed(1)
        || copy.copier != PlayerFilter::You
        || copy.target_reference_kind != Some(crate::filter::StackObjectKind::Spell)
        || copy.target_reference_pronoun
        || !matches!(&copy.target, ChooseSpec::Tagged(tag) if tag == &triggering.tag)
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
    {
        return None;
    }
    let retarget = retarget_effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if retarget.chooser != PlayerFilter::You
        || retarget.require_change
        || retarget.new_target_restriction.is_some()
        || !matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag == copied_tag)
    {
        return None;
    }
    let crate::effects::RetargetMode::OneToFixed(fixed) = &retarget.mode else {
        return None;
    };
    if !retarget_fixed_spec_uses_chosen_tag(fixed, &choice.tag) {
        return None;
    }

    Some(
        "Choose one of those creatures. Copy that spell. The copy targets the chosen creature"
            .to_string(),
    )
}

pub(super) fn describe_triggered_resolution_text(
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
    rewrite_it_deals: bool,
) -> Option<String> {
    if let Some(text) = describe_intervening_legal_target_copy_assignment(triggered) {
        return Some(text);
    }
    if let Some(text) = describe_attack_optional_dynamic_sacrifice_then_group_grant(triggered) {
        return Some(text);
    }
    if let Some(text) = describe_etb_copy_next_spell_when_cast(triggered) {
        return Some(text);
    }
    if let Some(text) =
        describe_last_counter_destroy_attached_land_and_damage_controller(triggered, subject)
    {
        return Some(text);
    }
    if let Some(text) = describe_optional_self_exile_collect_evidence_then_return(triggered) {
        return Some(text);
    }
    if let Some(text) = describe_reveal_hand_cards_then_create_for_duplicate_mana_values(triggered)
    {
        return Some(text);
    }
    if let Some(text) = describe_control_tagged_artifact_then_attach_if_equipment(triggered) {
        return Some(text);
    }
    if let Some(text) = describe_choose_creature_then_debuff_source_and_chosen_complement(triggered)
    {
        return Some(text);
    }
    if let Some(text) = describe_unblocked_attacker_controller_damage_offer(triggered) {
        return Some(text);
    }
    if let Some(text) = describe_destroy_source_and_triggering_attacker(triggered) {
        return Some(text);
    }
    if let Some(text) = describe_attack_grant_unblockable_unless_defender_sacrifices(triggered) {
        return Some(text);
    }
    if let Some(text) = describe_excess_noncombat_damage_to_other_target(triggered, subject) {
        return Some(text);
    }
    if let Some(text) = describe_optional_source_cant_attack_then_vigilance_rule(triggered, subject)
    {
        return Some(text);
    }

    if let Some(text) = describe_upkeep_choose_pay_each_then_untap(triggered) {
        return Some(text);
    }

    if let Some(text) = describe_draw_step_player_life_loss_then_search(triggered) {
        return Some(text);
    }

    if triggered.trigger.saga_chapters().is_some()
        && let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let [first, second] = segment.default_effects.as_slice()
        && let Some(compact) = describe_tap_then_put_counters_same_target(first, second)
        && let Some((tap, counter)) = compact.split_once(" and put ")
    {
        return Some(format!("{tap}. Put {counter}"));
    }

    if let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let Some(text) = describe_you_lose_life_and_draw_additional(&segment.default_effects)
    {
        return Some(text);
    }

    // Preserve the finite player subject on a coordinated trigger whose
    // second action switches to the source object. The general spell-clause
    // renderer intentionally turns leading "you" actions into imperatives,
    // which is not grammatical for "you lose ... and this creature ...".
    if let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let Some(text) = describe_lose_life_then_endure(&segment.default_effects)
    {
        return Some(text);
    }

    if let Some(text) = describe_manifest_dread_graveyard_card_to_hand(triggered) {
        return Some(text);
    }

    if let Some(text) = describe_return_triggering_object_then_remove_all_abilities(triggered) {
        return Some(text);
    }

    if let Some(text) = describe_exile_triggering_object_then_return_source(triggered, subject) {
        return Some(text);
    }

    if let Some(text) = describe_destroy_attached_to_triggering_creature(triggered) {
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
    effects = rewrite_exact_attacked_player_references(triggered, effects);
    effects = rewrite_each_upkeep_active_player_reference(triggered, effects);
    effects = rewrite_typed_triggering_object_player_reference(triggered, effects);
    effects = rewrite_damaged_player_reference_for_damage_trigger(triggered, effects);
    effects = rewrite_triggering_artifact_reference_for_tap_or_ability_trigger(triggered, effects);
    effects = rewrite_source_no_counter_resolution_surface(effects, subject);
    let source_bound_subject = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        .filter(|zone_change| zone_change.this_object)
        .and_then(|zone_change| zone_change.this_object_surface.as_ref())
        .map(crate::target::SourceReferenceSurface::display_text)
        .unwrap_or_else(|| subject.to_string());
    effects = preserve_source_bound_reference_phrases(effects, &source_bound_subject);
    let resolution_subject = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        .filter(|zone_change| zone_change.this_object)
        .map_or(subject, |_| "it");
    effects = rewrite_damage_phrases_for_permanent_abilities(
        &effects,
        resolution_subject,
        rewrite_it_deals,
    );
    effects = rewrite_triggering_source_damage_subject(triggered, effects);
    effects = rewrite_self_attack_damage_subject(triggered, effects, subject);
    effects = normalize_ability_self_reference_surface(&effects, resolution_subject);
    effects = split_sacrifice_then_lose_life_resolution(effects);
    if let Some(participant) = relative_power_block_destroy_participant(triggered) {
        effects = effects
            .replace("destroy that creature", &format!("destroy {participant}"))
            .replace("Destroy that creature", &format!("Destroy {participant}"));
    }
    if !effects.contains("counts toward your devotion")
        && let Some(color) = triggered_search_devotion_color(triggered)
    {
        effects = format!(
            "{}. {STANDARD_REMINDER_OPEN_SENTINEL}Each {} in the mana costs of permanents you control counts toward your devotion to {}.{STANDARD_REMINDER_CLOSE_SENTINEL}",
            effects.trim_end_matches('.'),
            describe_mana_symbol(ManaSymbol::from_color(color)),
            color.name()
        );
    }
    Some(effects)
}

/// A dynamic mana-value bound can be rendered by a structural search bundle
/// rather than the single-effect search renderer. Preserve the standard
/// devotion reminder at the triggered-resolution boundary after proving that
/// the executable search filter is actually bounded by the controller's
/// devotion.
pub(crate) fn triggered_search_devotion_color(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<crate::color::Color> {
    fn search_devotion_color(effect: &Effect) -> Option<crate::color::Color> {
        if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>()
            && let Some(crate::filter::Comparison::LessThanOrEqualExpr(limit)) =
                &search.filter.mana_value
            && let Value::Devotion {
                player: PlayerFilter::You,
                color,
            } = limit.unhinted()
        {
            return Some(*color);
        }
        if let Some(search) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && search.is_search
            && search.chooser == PlayerFilter::You
            && search.zone == Some(Zone::Library)
            && let Some(crate::filter::Comparison::LessThanOrEqualExpr(limit)) =
                &search.filter.mana_value
            && let Value::Devotion {
                player: PlayerFilter::You,
                color,
            } = limit.unhinted()
        {
            return Some(*color);
        }

        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = search_devotion_color(child);
            }
        });
        found
    }

    triggered
        .effects
        .all_effects()
        .into_iter()
        .find_map(search_devotion_color)
}

fn describe_control_tagged_artifact_then_attach_if_equipment(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [control_effect, conditional_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let tagged = control_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let control = tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if control.target != crate::continuous::EffectTarget::Source
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || control.runtime_modifications.as_slice()
            != [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        || control.until != Until::Forever
        || control.condition.is_some()
        || control.lock_filter_at_resolution
        || control.resolve_set_pt_values_at_resolution
        || control.require_creature_target
    {
        return None;
    }
    let target_spec = control.target_spec.as_ref()?;
    if !target_spec.is_target()
        || describe_choose_spec(target_spec) != "target artifact with mana value X or less"
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    if condition_tag != &tagged.tag
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !conditional.if_false.is_empty()
        || filter.zone.is_some()
        || !filter.card_types.is_empty()
        || filter.subtypes.as_slice() != [Subtype::Equipment]
        || filter.union_surface.demonstrative_antecedent()
            != Some(ironsmith_core::DemonstrativeAntecedentSurface::Artifact)
    {
        return None;
    }
    let [attach_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if attach.individual_targets
        || !choose_spec_references_exact_tag(&attach.objects, &tagged.tag)
        || !matches!(attach.target.base(), ChooseSpec::Source)
    {
        return None;
    }

    Some(
        "gain control of target artifact with mana value X or less. If that artifact is an Equipment, attach it to this creature"
            .to_string(),
    )
}

/// A relative-power block trigger binds two distinct event participants.
/// Preserve the destroy filter's exact tagged identity in the surface instead
/// of collapsing both directions to the ambiguous "that creature."
fn relative_power_block_destroy_participant(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<&'static str> {
    let (tag, participant) = if triggered
        .trigger
        .downcast_ref::<crate::triggers::BecomesBlockedByObjectWithLesserPowerTrigger>()
        .is_some()
    {
        ("blocking", "the blocking creature")
    } else if triggered
        .trigger
        .downcast_ref::<crate::triggers::BlocksObjectWithLesserPowerTrigger>()
        .is_some()
    {
        ("blocked", "the attacking creature")
    } else {
        return None;
    };
    let destroy = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DestroyEffect>())?;
    let filter = match destroy.spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return None,
    };
    let [constraint] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        && constraint.tag.as_str() == tag)
        .then_some(participant)
}

fn rewrite_exact_attacked_player_references(
    triggered: &crate::ability::TriggeredAbility,
    effects: String,
) -> String {
    let Some(attacks) = triggered
        .trigger
        .downcast_ref::<crate::triggers::combat::AttacksTrigger>()
    else {
        return effects;
    };
    if attacks
        .filter
        .attacking_player_or_planeswalker_controlled_by
        .is_none()
        || attacks.filter.targets_only_player.is_none()
    {
        return effects;
    }

    // A player-only attack event binds one concrete attacked player. Object
    // filters retain that identity as `Defending`, while Oracle uses the
    // trigger's demonstrative antecedent ("that player") in its effect.
    effects
        .replace(
            " that are attacking the defending player",
            " attacking that player",
        )
        .replace(
            " that are attacking defending players",
            " attacking that player",
        )
        .replace(
            " that's attacking the defending player",
            " attacking that player",
        )
}

fn describe_upkeep_choose_pay_each_then_untap(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let [setup_segment, result_segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !setup_segment.self_replacements.is_empty() || !result_segment.self_replacements.is_empty() {
        return None;
    }

    let [setup_effect] = setup_segment.default_effects.as_slice() else {
        return None;
    };
    let setup = setup_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = setup.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let [sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, pay_each_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::IteratedPlayer
        || !choose.count.is_any_number()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || !choose.filter.tapped
    {
        return None;
    }
    let pay_each = pay_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [payment_effect] = pay_each.effects.as_slice() else {
        return None;
    };
    let payment = payment_effect.downcast_ref::<crate::effects::PayManaEffect>()?;
    if pay_each.tag != choose.tag
        || payment.player.base() != &ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    {
        return None;
    }

    let [result_effect] = result_segment.default_effects.as_slice() else {
        return None;
    };
    let result = result_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if result.condition != setup.id
        || result.predicate != EffectPredicate::Happened
        || !result.else_.is_empty()
    {
        return None;
    }
    let [untap_effect] = result.then.as_slice() else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    let ChooseSpec::All(untap_filter) = untap.target.base() else {
        return None;
    };
    if !object_filter_has_tag(untap_filter, &choose.tag) {
        return None;
    }

    let mut selected_filter = choose.filter.clone();
    selected_filter.zone = None;
    selected_filter.controller = None;
    selected_filter.tapped = false;
    let selected =
        pluralize_noun_phrase(strip_leading_article(&selected_filter.description()).trim());
    if selected.is_empty() {
        return None;
    }
    let paid_for = if selected_filter.card_types.as_slice() == [CardType::Creature] {
        "creature"
    } else {
        "object"
    };
    Some(format!(
        "That player may choose any number of tapped {selected} they control and pay {} for each {paid_for} chosen this way. If the player does, untap those creatures",
        describe_pay_mana_cost(payment)
    ))
}

fn rewrite_each_upkeep_iterated_player_choice_surface(mut text: String) -> String {
    fn rewrite_single_choice(
        text: &mut String,
        imperative: &str,
        finite_subject: &str,
        required_suffix: &str,
    ) {
        let controlled = " that player controls";
        let mut search_from = 0;
        while let Some(relative_start) = text[search_from..].find(imperative) {
            let start = search_from + relative_start;
            let object_start = start + imperative.len();
            let Some(relative_controlled) = text[object_start..].find(controlled) else {
                break;
            };
            let object_end = object_start + relative_controlled;
            let object = text[object_start..object_end].trim();
            let controlled_end = object_end + controlled.len();
            if !(object.starts_with("a ") || object.starts_with("an "))
                || !text[controlled_end..].starts_with(required_suffix)
            {
                search_from = controlled_end;
                continue;
            }

            let replace_end = controlled_end + required_suffix.len();
            let replacement = format!("{finite_subject}{object} they control{required_suffix}");
            text.replace_range(start..replace_end, &replacement);
            search_from = start + replacement.len();
        }
    }

    // A single indefinite controlled object is chosen by the upkeep player at
    // resolution. The generic effect renderer faithfully describes the filter
    // ("a permanent that player controls") but has no standalone actor field;
    // recover the finite event-player surface while leaving mass instructions
    // such as Noetic Scales' "each creature that player controls" unchanged.
    for (imperative, finite_subject) in [
        ("Return ", "That player returns "),
        ("return ", "that player returns "),
    ] {
        rewrite_single_choice(&mut text, imperative, finite_subject, " to ");
    }
    for (imperative, finite_subject) in [
        ("Untap ", "That player untaps "),
        ("untap ", "that player untaps "),
        ("Tap ", "That player taps "),
        ("tap ", "that player taps "),
    ] {
        rewrite_single_choice(&mut text, imperative, finite_subject, "");
    }
    text
}

/// Contextualize the event participant only when an each-player upkeep
/// trigger proves that the active player is the player just introduced by the
/// trigger. Outside that typed context, "the active player" remains the
/// correct standalone rules term and must not be rewritten.
pub(super) fn rewrite_each_upkeep_active_player_reference(
    triggered: &crate::ability::TriggeredAbility,
    mut text: String,
) -> String {
    let Some(upkeep) = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
    else {
        return text;
    };
    if upkeep.player != PlayerFilter::Any {
        return text;
    }

    text = rewrite_each_upkeep_iterated_player_choice_surface(text);

    // The damage grammar binds both "that player" and the object pronoun
    // "them" to the upkeep participant. The typed target alone cannot
    // distinguish those authored surfaces, so only use the object pronoun
    // when lowering preserved that explicit presentation hint.
    let damages_upkeep_player_with_object_pronoun = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .any(|effect| {
            effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .is_some_and(|damage| {
                    damage.target == ChooseSpec::Player(PlayerFilter::IteratedPlayer)
                        && damage.amount.has_surface_hint(
                            ironsmith_core::ValueSurfaceHint::DamageRecipientPronoun,
                        )
                })
        });
    if damages_upkeep_player_with_object_pronoun {
        text = text.replace(" to that player", " to them");
    }

    // Some single-object actions encode the active player in the controlled
    // object filter rather than in an actor presentation field. Recover the
    // authored subject before applying the ordinary pronoun substitutions.
    for (needle, subject) in [
        ("Return the active player's ", "That player returns "),
        ("return the active player's ", "that player returns "),
    ] {
        while let Some(start) = text.find(needle) {
            let object_start = start + needle.len();
            let Some(to_offset) = text[object_start..].find(" to ") else {
                break;
            };
            let object_end = object_start + to_offset;
            let object = text[object_start..object_end].trim();
            if object.is_empty() {
                break;
            }
            let replacement = format!(
                "{subject}{} they control to ",
                with_indefinite_article(object)
            );
            text.replace_range(start..object_end + " to ".len(), &replacement);
        }
    }

    for (needle, subject) in [
        ("Untap the active player's ", "That player untaps "),
        ("untap the active player's ", "that player untaps "),
    ] {
        while let Some(start) = text.find(needle) {
            let object_start = start + needle.len();
            let object_end = [". ", ", then ", "; "]
                .into_iter()
                .filter_map(|delimiter| {
                    text[object_start..]
                        .find(delimiter)
                        .map(|offset| object_start + offset)
                })
                .min()
                .unwrap_or(text.len());
            let object = text[object_start..object_end].trim().trim_end_matches('.');
            if object.is_empty() {
                break;
            }
            let replacement = format!("{subject}{} they control", with_indefinite_article(object));
            text.replace_range(start..object_end, &replacement);
        }
    }

    text.replace(
        "If that player doesn't, that player returns ",
        "If they don't, they return ",
    )
    .replace(
        "if that player doesn't, that player returns ",
        "if they don't, they return ",
    )
    .replace(
        "If that player doesn't, that player ",
        "If they don't, they ",
    )
    .replace(
        "if that player doesn't, that player ",
        "if they don't, they ",
    )
    .replace("If that player does, that player ", "If they do, they ")
    .replace("if that player does, that player ", "if they do, they ")
    .replace("The active player's", "Their")
    .replace("the active player's", "their")
    .replace("Active player's", "Their")
    .replace("active player's", "their")
    .replace("The active player", "That player")
    .replace("the active player", "that player")
    .replace("If that player doesn't", "If they don't")
    .replace("if that player doesn't", "if they don't")
    .replace("If that player does", "If they do")
    .replace("if that player does", "if they do")
    .replace(" that player controls", " they control")
}

/// Replace the generic triggering-object noun with the concrete card type
/// carried by a typed permanent trigger. The damage renderer cannot recover
/// that type from `ControllerOf(Tagged("triggering"))` alone, but the trigger
/// matcher still has the exact filter that established the reference.
pub(super) fn rewrite_typed_triggering_object_player_reference(
    triggered: &crate::ability::TriggeredAbility,
    text: String,
) -> String {
    let Some(tapped) = triggered
        .trigger
        .downcast_ref::<crate::triggers::PermanentBecomesTappedTrigger>()
    else {
        return text;
    };
    let Some(noun) = simple_filter_singular_noun(&tapped.filter) else {
        return text;
    };

    fn references_triggering_object_player(effect: &Effect) -> bool {
        if effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .is_some_and(|damage| {
                matches!(
                    damage.target.base(),
                    ChooseSpec::Player(
                        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                            | PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag))
                    ) if tag.as_str() == "triggering"
                )
            })
        {
            return true;
        }

        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            if !found && references_triggering_object_player(child) {
                found = true;
            }
        });
        found
    }

    if !triggered
        .effects
        .all_effects()
        .into_iter()
        .any(references_triggering_object_player)
    {
        return text;
    }

    text.replace(
        "that creature's controller",
        &format!("that {noun}'s controller"),
    )
    .replace("that creature's owner", &format!("that {noun}'s owner"))
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

/// For a self trigger ("Whenever ~ deals combat damage to a player"), the
/// tagged triggering object IS this ability's source, so oracle names the
/// follow-up damage subject "it" ("it deals X damage to any target"). The
/// generic damage renderer only sees the tagged source and defaults to the
/// demonstrative "that creature"; restore the authored pronoun here where the
/// trigger context proves they coincide.
pub(super) fn rewrite_triggering_source_damage_subject(
    triggered: &crate::ability::TriggeredAbility,
    effects: String,
) -> String {
    let combat_damage_self_trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
        .is_some();
    let triggering_source_damage_count =
        triggering_reference_damage_source_count(&triggered.effects).or_else(|| {
            triggered
                .trigger
                .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
                .and_then(|_| {
                    damage_source_count_for_tag(&triggered.effects, &TagKey::from("triggering"))
                })
        });
    if !combat_damage_self_trigger && triggering_source_damage_count.is_none() {
        return effects;
    }
    if let Some(source_count) = triggering_source_damage_count {
        let rendered_count = effects.matches("that creature deals ").count()
            + effects.matches("That creature deals ").count();
        if rendered_count > source_count {
            return effects;
        }
    }
    if let Some(rest) = effects.strip_prefix("that creature deals ") {
        return format!("it deals {rest}");
    }
    if let Some(rest) = effects.strip_prefix("That creature deals ") {
        return format!("It deals {rest}");
    }
    effects
}

/// A self-attack trigger already identifies the permanent that owns the
/// ability. When that permanent is also the leading damage source, Oracle uses
/// the pronoun "it" rather than repeating either its permanent type or name.
fn rewrite_self_attack_damage_subject(
    triggered: &crate::ability::TriggeredAbility,
    effects: String,
    subject: &str,
) -> String {
    if triggered
        .trigger
        .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
        .is_none()
    {
        return effects;
    }

    let lower_prefix = format!("{subject} deals ");
    if let Some(rest) = effects.strip_prefix(&lower_prefix) {
        return format!("it deals {rest}");
    }
    let upper_prefix = format!("{} deals ", capitalize_first(subject));
    if let Some(rest) = effects.strip_prefix(&upper_prefix) {
        return format!("It deals {rest}");
    }
    effects
}

fn triggering_reference_damage_source_count(
    program: &crate::resolution::ResolutionProgram,
) -> Option<usize> {
    let triggering_tags = program
        .flattened_default_effects()
        .iter()
        .filter_map(triggering_reference_tag)
        .collect::<Vec<_>>();
    let [triggering_tag] = triggering_tags.as_slice() else {
        return None;
    };

    damage_source_count_for_tag(program, triggering_tag)
}

fn damage_source_count_for_tag(
    program: &crate::resolution::ResolutionProgram,
    triggering_tag: &TagKey,
) -> Option<usize> {
    fn inspect(
        effect: &Effect,
        triggering_tag: &TagKey,
        matching: &mut usize,
        incompatible: &mut bool,
    ) {
        if let Some(distributed) =
            effect.downcast_ref::<crate::effects::DealDistributedDamageEffect>()
        {
            if matches!(
                distributed.source.unhinted(),
                ChooseSpec::Tagged(tag) if tag == triggering_tag
            ) {
                *matching += 1;
            } else {
                *incompatible = true;
            }
            return;
        }
        if let Some(execute) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            && damage_effect_view(&execute.effect).is_some()
        {
            if matches!(
                execute.source.unhinted(),
                ChooseSpec::Tagged(tag) if tag == triggering_tag
            ) {
                *matching += 1;
            } else {
                *incompatible = true;
            }
            return;
        }
        effect.visit_child_effects(&mut |child| {
            inspect(child, triggering_tag, matching, incompatible);
        });
    }

    let mut matching = 0usize;
    let mut incompatible = false;
    for effect in program.flattened_default_effects() {
        inspect(effect, triggering_tag, &mut matching, &mut incompatible);
    }
    (matching > 0 && !incompatible).then_some(matching)
}

#[cfg(test)]
#[test]
fn triggering_reference_damage_source_uses_pronoun_for_single_damage_clause() {
    let triggering = TagKey::from("triggering");
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_attacks(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::tag_triggering_object(triggering.clone()),
            Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(triggering),
                Effect::deal_damage(Value::Fixed(3), ChooseSpec::AnyTarget),
            )),
        ]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        rewrite_triggering_source_damage_subject(
            &triggered,
            "that creature deals 3 damage to any target".to_string(),
        ),
        "it deals 3 damage to any target"
    );
}

#[cfg(test)]
#[test]
fn self_attack_triggering_tag_damage_source_uses_pronoun_without_tag_scaffold() {
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_attacks(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(TagKey::from("triggering")),
                Effect::deal_damage(Value::Fixed(1), ChooseSpec::AnyTarget),
            ),
        )]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        rewrite_triggering_source_damage_subject(
            &triggered,
            "that creature deals 1 damage to any target".to_string(),
        ),
        "it deals 1 damage to any target"
    );
}

#[cfg(test)]
#[test]
fn self_attack_damage_source_uses_pronoun_after_trigger_subject() {
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_attacks(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
            Value::Fixed(3),
            ChooseSpec::AnyTarget,
        )]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        rewrite_self_attack_damage_subject(
            &triggered,
            "this creature deals 3 damage to any target".to_string(),
            "this creature",
        ),
        "it deals 3 damage to any target"
    );
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
    let Condition::PlayerControlsMoreThanEachOtherPlayer { player, filter } =
        triggered.intervening_if.as_ref()?
    else {
        return None;
    };
    if *player != PlayerFilter::Any {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let control = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let [
        crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
            PlayerFilter::ControlsMost {
                filter: leader_filter,
            },
        ),
    ] = control.runtime_modifications.as_slice()
    else {
        return None;
    };
    if control.target != crate::continuous::EffectTarget::Source
        || control.until != Until::Forever
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || control.condition.is_some()
        || leader_filter.as_ref() != filter
    {
        return None;
    }

    let leader = PlayerFilter::ControlsMost {
        filter: Box::new(filter.clone()),
    }
    .description();
    let objects = leader.strip_prefix("the player who controls the most ")?;
    Some(format!(
        "At the beginning of your upkeep, if a player controls more {objects} than each other player, {leader} gains control of this creature"
    ))
}

/// A player-or-permanent target trigger names the triggering stack object as
/// "that spell or ability" and its controller as "that player". Keep those
/// authored roles when the executable consequence is an unless-life-payment
/// wrapper around countering that exact triggering source.
pub(super) fn describe_targeted_player_or_permanent_counter_unless_life(
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
        .downcast_ref::<crate::triggers::PlayerOrObjectBecomesTargetedBySourceControllerTrigger>(
    )?;
    let mut object_filter = trigger.object_filter.clone();
    let exact_scope = object_filter.zone == Some(Zone::Battlefield)
        && object_filter.controller == Some(PlayerFilter::You);
    object_filter.zone = None;
    object_filter.controller = None;
    object_filter.set_explicit_card_noun(false);
    object_filter.set_explicit_card_type_noun(None);
    if trigger.player_filter != PlayerFilter::You
        || trigger.source_controller != PlayerFilter::Opponent
        || !exact_scope
        || object_filter != ObjectFilter::default()
    {
        return None;
    }
    let [tag_effect, unless_effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tag = &tag_effect
        .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()?
        .tag;
    let unless = unless_effect.downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless.leading_surface
        || unless.before_delayed_step
        || unless.player
            != PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag.clone()))
    {
        return None;
    }
    let [counter_effect] = unless.effects.as_slice() else {
        return None;
    };
    let counter = counter_effect.downcast_ref::<crate::effects::CounterEffect>()?;
    if counter.target != ChooseSpec::Tagged(tag.clone()) {
        return None;
    }
    let [cost] = unless.cost.as_all()? else {
        return None;
    };
    let lose_life = cost
        .effect_ref()?
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let Value::Fixed(amount) = lose_life.amount else {
        return None;
    };
    if lose_life.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }

    Some(format!(
        "{}, counter that spell or ability unless that player pays {amount} life",
        crate::triggers::TriggerMatcher::display(trigger)
    ))
}

/// Preserve the target opponent as both copier and retargeting decision owner
/// for "up to one target opponent may also copy that spell". The explicit
/// target declaration is execution machinery for the embedded Oracle clause,
/// not a standalone "choose target opponent" sentence.
pub(super) fn describe_target_opponent_may_copy_triggering_spell(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some() || triggered.presentation_label.is_some() {
        return None;
    }
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCopiedTrigger>()?;
    if trigger.copier != PlayerFilter::You || trigger.filter.is_some() {
        return None;
    }
    let [tag_effect, target_effect, may_effect] = triggered.effects.flattened_default_effects()
    else {
        return None;
    };
    let triggering_tag = &tag_effect
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?
        .tag;
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let count = target.target.count();
    if !target.explicit_declaration
        || target.chooser.is_some()
        || !target.target.is_target()
        || !matches!(
            target.target.base(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        )
        || count.min != 0
        || count.max != Some(1)
    {
        return None;
    }
    let [declared_target, embedded_target] = triggered.choices.as_slice() else {
        return None;
    };
    let expected_embedded_target = ChooseSpec::target(target.target.base().clone());
    if declared_target != &target.target || embedded_target != &expected_embedded_target {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::target_opponent()) {
        return None;
    }
    let [copy_effect, retarget_effect] = may.effects.as_slice() else {
        return None;
    };
    let tagged = copy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged.tag.as_str() != COPIED_STACK_OBJECT_TAG {
        return None;
    }
    let with_id = tagged
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy.target != ChooseSpec::Tagged(triggering_tag.clone())
        || copy.target_reference_kind != Some(crate::filter::StackObjectKind::Spell)
        || copy.count.unhinted() != &Value::Fixed(1)
        || copy.count_surface.is_some()
        || copy.copier != PlayerFilter::target_opponent()
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
    {
        return None;
    }
    let retarget = retarget_effect.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if retarget.from_effect != with_id.id
        || !retarget.may
        || retarget.chooser.as_ref() != Some(&PlayerFilter::target_opponent())
        || retarget.single_target_surface
    {
        return None;
    }

    Some(
        "Whenever you copy a spell, up to one target opponent may also copy that spell. They may choose new targets for that copy"
            .to_string(),
    )
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod target_opponent_may_copy_triggering_spell_tests {
    use super::*;

    fn parsed() -> crate::ability::TriggeredAbility {
        let oracle = "Whenever you copy a spell, up to one target opponent may also copy that spell. They may choose new targets for that copy.";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Target Opponent Copy Probe",
        )
        .parse_text(oracle)
        .expect("typed target-opponent copy offer");
        let [ability] = definition.abilities.as_slice() else {
            panic!("expected one ability: {definition:#?}");
        };
        let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("expected triggered ability: {ability:#?}");
        };
        triggered.clone()
    }

    #[test]
    fn folds_the_embedded_target_choice_but_rejects_a_nontarget_actor() {
        let triggered = parsed();
        assert_eq!(
            describe_target_opponent_may_copy_triggering_spell(&triggered).as_deref(),
            Some(
                "Whenever you copy a spell, up to one target opponent may also copy that spell. They may choose new targets for that copy"
            )
        );

        let mut nontarget_actor = triggered;
        nontarget_actor.choices[1] = ChooseSpec::Player(PlayerFilter::Opponent);
        assert!(
            describe_target_opponent_may_copy_triggering_spell(&nontarget_actor).is_none(),
            "a nontarget opponent choice must not inherit the target-opponent compaction"
        );
    }
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
    let (tag_effect, copy_effect, move_effect, put_effect, conditional_effect) = match triggered
        .effects
        .segments
        .as_slice()
    {
        [segment] if segment.self_replacements.is_empty() && !segment.starts_new_source_line => {
            let [tag, copy, move_to_exile, put, conditional] = segment.default_effects.as_slice()
            else {
                return None;
            };
            (tag, copy, move_to_exile, put, conditional)
        }
        [action_segment, condition_segment]
            if action_segment.self_replacements.is_empty()
                && condition_segment.self_replacements.is_empty()
                && !action_segment.starts_new_source_line =>
        {
            let [tag, sequence_effect] = action_segment.default_effects.as_slice() else {
                return None;
            };
            let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::RepeatedCommaThen
                || sequence.result_label.is_some()
            {
                return None;
            }
            let [copy, move_to_exile, put] = sequence.effects.as_slice() else {
                return None;
            };
            let [conditional] = condition_segment.default_effects.as_slice() else {
                return None;
            };
            (tag, copy, move_to_exile, put, conditional)
        }
        _ => return None,
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
        || copy.has_characteristic_modifiers()
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

    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    let ChooseSpec::Tagged(counter_target_tag) = &put.target else {
        return None;
    };
    if put.counter_type != CounterType::Time
        || put.amount.unhinted() != &Value::Fixed(4)
        || put.target_count.is_some()
        || put.distributed
        || !(counter_target_tag == tag || counter_target_tag.as_str() == "__source_exiled__")
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !condition_is_tagged_object_without_suspend(&conditional.condition, counter_target_tag)
    {
        return None;
    }
    let apply = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_grants_suspend_to_tag(apply, counter_target_tag) {
        return None;
    }

    Some(
        "Flurry — Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend"
            .to_string(),
    )
}

#[cfg(test)]
mod flurry_copy_exile_suspend_tests {
    use super::*;

    const LINE: &str = "Flurry — Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend.";

    #[test]
    fn migrated_two_segment_route_rejoins_only_with_the_ordered_surface() {
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Taigam, Master Opportunist",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(LINE)
        .expect("flurry copy-exile-suspend line should parse");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );

        let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected spell-cast trigger");
        };
        assert_eq!(
            describe_flurry_copy_exile_suspend_triggered_ability(triggered).as_deref(),
            Some(LINE.trim_end_matches('.'))
        );

        let mut changed = triggered.clone();
        let sequence = changed.effects.segments[0].default_effects[1]
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("ordered action sequence");
        let mut sequence = sequence.clone();
        sequence.surface = ironsmith_core::SequenceSurface::Coordinated;
        changed.effects.segments[0].default_effects[1] = Effect::new(sequence);
        assert!(describe_flurry_copy_exile_suspend_triggered_ability(&changed).is_none());
    }

    #[test]
    fn migrated_condition_source_boundary_keeps_the_correlated_spell_reference() {
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Taigam, Master Opportunist",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flurry — Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend. (At the beginning of its owner's upkeep, they remove a time counter. When the last is removed, they may play it without paying its mana cost. If it's a creature, it has haste.)",
        )
        .expect("reminder-expanded flurry line should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );
    }
}

#[cfg(test)]
mod target_opponent_villainous_choice_tests {
    use super::*;

    const LINE: &str = "Draw three cards. Then target opponent faces a villainous choice — They discard three cards, or you may cast a spell from your hand without paying its mana cost.";

    #[test]
    fn public_route_preserves_the_targeted_chooser_and_leading_then() {
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Great Intelligence's Plan",
        )
        .card_types(vec![CardType::Sorcery])
        .parse_text(LINE)
        .expect("target-opponent villainous choice should parse");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );

        let program = definition.spell_effect.as_ref().expect("sorcery effects");
        assert_eq!(program.segments.len(), 2, "{program:#?}");
        let [ordered_choice] = program.segments[1].default_effects.as_slice() else {
            panic!("expected one sentence-leading choice sequence: {program:#?}");
        };
        let sequence = ordered_choice
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("second sentence should retain its leading then");
        assert_eq!(
            sequence.surface,
            ironsmith_core::SequenceSurface::SentenceLeadingThen
        );
        assert!(
            sequence.effects[0]
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
        );
        let choice = sequence.effects[1]
            .downcast_ref::<crate::effects::VillainousChoiceEffect>()
            .expect("targeted chooser should retain villainous-choice semantics");
        assert_eq!(choice.player, PlayerFilter::target_opponent());
    }
}

#[cfg(test)]
mod sacrifice_then_destroy_no_regeneration_tests {
    use super::*;

    const LINE: &str = "At the beginning of your upkeep, if there are four or more creatures on the battlefield, sacrifice this enchantment and destroy all creatures. They can't be regenerated.";

    #[test]
    fn coordinated_destroy_keeps_its_regeneration_followup_sentence() {
        let mut definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Planar Collapse")
                .card_types(vec![CardType::Enchantment])
                .parse_text(LINE)
                .expect("coordinated sacrifice and destroy should parse");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );

        let AbilityKind::Triggered(triggered) = &mut definition.abilities[0].kind else {
            panic!("expected upkeep trigger");
        };
        let sequence = triggered.effects.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("coordinated resolution")
            .clone();
        let mut changed = sequence;
        changed.surface = ironsmith_core::SequenceSurface::Sequential;
        triggered.effects.segments[0].default_effects[0] = Effect::new(changed);
        assert_ne!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );
    }
}

pub(super) fn describe_convoke_cast_damage_opponents_and_protected_battles(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || triggered.presentation_label.is_some()
        || !triggered.choices.is_empty()
    {
        return None;
    }
    let cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    if cast.caster != PlayerFilter::You
        || cast.mana_source_filter.is_some()
        || cast.timing.is_some()
        || cast.during_turn.is_some()
        || cast.min_spells_this_turn.is_some()
        || cast.exact_spells_this_turn.is_some()
        || cast.count_all_spells_this_turn
        || cast.from_not_hand
        || cast.first_spell_of_game
    {
        return None;
    }
    let mut spell_filter = cast.filter.clone()?;
    if spell_filter.static_abilities != [crate::static_abilities::StaticAbilityId::Convoke]
        || !spell_filter.ability_markers.is_empty()
    {
        return None;
    }
    spell_filter.static_abilities.clear();
    // The object-filter grammar records that an authored spell "has convoke"
    // as both the exact static-ability predicate and the implied presence of a
    // mana cost.  The latter is redundant for this already-proved Convoke
    // shape, but ObjectFilter::spell() intentionally leaves it unset.
    spell_filter.has_mana_cost = false;
    if spell_filter != ObjectFilter::spell() {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [root] = segment.default_effects.as_slice() else {
        return None;
    };
    let sequence = root.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.result_label.is_some()
    {
        return None;
    }
    let [players_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let players = players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if players.filter != PlayerFilter::Opponent
        || players.starting_with_controller
        || players.stop_after_first_happened
    {
        return None;
    }
    let [player_damage, battle_loop] = players.effects.as_slice() else {
        return None;
    };
    let player_damage = unwrap_basic_tag_wrappers(player_damage)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if player_damage.amount != Value::Fixed(1)
        || player_damage.target != ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        || player_damage.source_is_combat
        || player_damage.unpreventable
    {
        return None;
    }
    let battles =
        unwrap_basic_tag_wrappers(battle_loop).downcast_ref::<crate::effects::ForEachObject>()?;
    let mut battle_filter = battles.filter.clone();
    if battle_filter.protected_by != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    battle_filter.protected_by = None;
    let mut expected_battle = ObjectFilter::default()
        .with_type(CardType::Battle)
        .in_zone(Zone::Battlefield);
    expected_battle.set_explicit_card_type_noun(Some(CardType::Battle));
    if battle_filter != expected_battle {
        return None;
    }
    let [battle_damage] = battles.effects.as_slice() else {
        return None;
    };
    let battle_damage = unwrap_basic_tag_wrappers(battle_damage)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if battle_damage.amount != Value::Fixed(1)
        || battle_damage.target != ChooseSpec::Iterated
        || battle_damage.source_is_combat
        || battle_damage.unpreventable
    {
        return None;
    }

    Some(
        "Whenever you cast a spell that has convoke, this creature deals 1 damage to each opponent and each battle they protect"
            .to_string(),
    )
}

/// Render an optional spell copy followed by a successful-result keyword
/// grant to the exact original-and-copy pair. Two equal typed grants prove
/// the plural subject; the shared result id proves neither grant can happen
/// when the copy is declined.
pub(super) fn describe_optional_copy_plural_keyword_grant(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || triggered.presentation_label.is_some()
        || !triggered.choices.is_empty()
    {
        return None;
    }
    let cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    if cast.caster != PlayerFilter::You
        || cast.timing.is_some()
        || cast.during_turn.is_some()
        || cast.min_spells_this_turn.is_some()
        || cast.exact_spells_this_turn.is_some()
        || cast.count_all_spells_this_turn
        || cast.from_not_hand
        || cast.first_spell_of_game
        || cast.mana_source_filter.is_some()
    {
        return None;
    }
    let mut spell_filter = cast.filter.clone()?;
    if spell_filter.card_types.as_slice() != [CardType::Instant, CardType::Sorcery]
        || spell_filter.target_count != Some(ironsmith_core::ChoiceCount::exactly(1))
    {
        return None;
    }
    spell_filter.card_types.clear();
    spell_filter.target_count = None;
    spell_filter.set_explicit_card_type_noun(None);
    let mut expected_spell_filter = ObjectFilter::spell();
    // The public filter parser retains this executable discriminator for an
    // explicitly authored spell noun. It is redundant with the Stack/Spell
    // domain for this renderer, but must not make the otherwise exact
    // original-and-copy program miss its plural surface.
    expected_spell_filter.has_mana_cost = true;
    if spell_filter != expected_spell_filter {
        return None;
    }

    let [copy_segment, grant_segment, retarget_segment] = triggered.effects.segments.as_slice()
    else {
        return None;
    };
    if [copy_segment, grant_segment, retarget_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let [tag_triggering, copy_root] = copy_segment.default_effects.as_slice() else {
        return None;
    };
    let tag_triggering =
        tag_triggering.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let copy_result = copy_root.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may_copy = copy_result
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [tagged_copy] = may_copy.effects.as_slice() else {
        return None;
    };
    let tagged_copy = tagged_copy.downcast_ref::<crate::effects::TaggedEffect>()?;
    let copy_with_id = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if may_copy.decider != Some(PlayerFilter::You)
        || tagged_copy.tag.as_str() != COPIED_STACK_OBJECT_TAG
        || copy_with_id.id != copy_result.id
        || copy.copier != PlayerFilter::You
        || copy.target_reference_kind != Some(crate::filter::StackObjectKind::Spell)
        || copy.count != Value::Fixed(1)
        || copy.count_surface.is_some()
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
        || !matches!(copy.target.base(), ChooseSpec::Tagged(tag) if tag == &tag_triggering.tag)
    {
        return None;
    }

    let [result_root] = grant_segment.default_effects.as_slice() else {
        return None;
    };
    let result = result_root.downcast_ref::<crate::effects::IfEffect>()?;
    let [original_grant, copied_grant] = result.then.as_slice() else {
        return None;
    };
    let original_grant = original_grant.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let copied_grant = copied_grant.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let Some(crate::continuous::Modification::AddAbility(ability)) = &original_grant.modification
    else {
        return None;
    };
    if result.condition != copy_result.id
        || result.predicate != crate::effect::EffectPredicate::Happened
        || !result.else_.is_empty()
        || original_grant.until != crate::effect::Until::Forever
        || original_grant.set_quantifier_surface
            != Some(ironsmith_core::SetQuantifierSurface::Those)
        || !matches!(
            original_grant.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag == &tag_triggering.tag
        )
        || !matches!(
            copied_grant.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag == &tagged_copy.tag
        )
    {
        return None;
    }
    let mut original_base = original_grant.clone();
    original_base.target_spec = None;
    let mut copied_base = copied_grant.clone();
    copied_base.target_spec = None;
    if original_base != copied_base || !ability.id().is_keyword() {
        return None;
    }

    let [retarget_root] = retarget_segment.default_effects.as_slice() else {
        return None;
    };
    let may_retarget = retarget_root.downcast_ref::<crate::effects::MayEffect>()?;
    let [retarget] = may_retarget.effects.as_slice() else {
        return None;
    };
    let retarget = retarget.downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if may_retarget.decider != Some(PlayerFilter::You)
        || retarget.chooser != PlayerFilter::You
        || retarget.require_change
        || retarget.copy_reference_plural
        || retarget.new_target_restriction.is_some()
        || retarget.mode != crate::effects::RetargetMode::All
        || !matches!(retarget.target.base(), ChooseSpec::Tagged(tag) if tag == &tagged_copy.tag)
    {
        return None;
    }

    let keyword = describe_static_ability_with_subject(ability, "this spell")
        .trim_end_matches('.')
        .to_ascii_lowercase();
    (!keyword.is_empty()).then(|| {
        format!(
            "Whenever you cast an instant or sorcery spell with a single target, you may copy it. If you do, those spells gain {keyword}. You may choose new targets for the copy"
        )
    })
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

/// Preserve the authored relative-power surface after the executable program
/// binds the later choice to the first tagged attacker. The generic filter
/// renderer necessarily spells out that comparison source; this exact four-
/// segment shape originated from the shorter Oracle phrase "with lesser
/// power".
pub(super) fn describe_kamiz_relational_attacker_sequence(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some() || triggered.presentation_label.is_some() {
        return None;
    }
    let attacks = triggered
        .trigger
        .downcast_ref::<crate::triggers::AttacksTrigger>()?;
    if attacks.filter.card_types.as_slice() != [CardType::Creature]
        || attacks.filter.controller != Some(PlayerFilter::You)
        || !attacks.one_or_more
        || attacks.min_total_attackers != 1
        || attacks.max_total_attackers.is_some()
    {
        return None;
    }
    let [
        target_segment,
        connive_segment,
        choice_segment,
        grant_segment,
    ] = triggered.effects.segments.as_slice()
    else {
        return None;
    };
    if triggered
        .effects
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return None;
    }

    let [target_effect, restriction_effect] = target_segment.default_effects.as_slice() else {
        return None;
    };
    let targeted = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = targeted
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::Object(target_filter) = target_only.target.inner() else {
        return None;
    };
    if targeted.tag.as_str() != "targeted_0"
        || target_filter.card_types.as_slice() != [CardType::Creature]
        || !target_filter.attacking
        || triggered.choices.as_slice() != [target_only.target.clone()]
    {
        return None;
    }
    let cant = restriction_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::BeBlocked(restricted) = &cant.restriction else {
        return None;
    };
    if !restricted.tagged_constraints.iter().any(|constraint| {
        constraint.tag == targeted.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) || cant.duration != crate::effect::Until::EndOfTurn
    {
        return None;
    }

    let [connive_effect] = connive_segment.default_effects.as_slice() else {
        return None;
    };
    let connived = connive_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let connive = connived
        .effect
        .downcast_ref::<crate::effects::ConniveEffect>()?;
    if connive.target != ChooseSpec::Tagged(targeted.tag.clone())
        || connive.count != crate::effect::Value::Fixed(1)
    {
        return None;
    }

    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let with_source = choice_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if with_source.source.base() != &ChooseSpec::Tagged(targeted.tag.clone()) {
        return None;
    }
    let sequence = with_source
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen
        || sequence.result_label.is_some()
        || choose.filter.card_types.as_slice() != [CardType::Creature]
        || !choose.filter.attacking
        || !choose.filter.other
        || choose.filter.power_relative_to_source
            != Some(crate::filter::SourcePowerRelation::LessThanSource)
        || choose.count != ironsmith_core::ChoiceCount::exactly(1)
        || choose.chooser != PlayerFilter::You
        || choose.tag.as_str() != "__it__"
        || choose.zone != Some(Zone::Battlefield)
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
    {
        return None;
    }

    let [grant_effect] = grant_segment.default_effects.as_slice() else {
        return None;
    };
    let grant = grant_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.until != crate::effect::Until::EndOfTurn
        || !grant.target_spec.as_ref().is_some_and(
            |spec| matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"),
        )
        || !matches!(
            grant.modification.as_ref(),
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::DoubleStrike
        )
    {
        return None;
    }

    Some("Whenever you attack, target attacking creature can't be blocked this turn. It connives. Then choose another attacking creature with lesser power. That creature gains double strike until end of turn".to_string())
}

/// Preserve a grammar-proven named source across an Explore instruction and
/// its following source-power gate. The two source sentences deliberately
/// remain separate resolution segments; matching their exact typed shape here
/// prevents the generic trigger join from turning the retained leading `Then`
/// into a second inline connective.
fn describe_named_source_explore_then_exact_power_destruction(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || triggered.presentation_label.is_some()
        || !triggered.choices.is_empty()
    {
        return None;
    }
    let [explore_segment, destroy_segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if [explore_segment, destroy_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty() || segment.starts_new_source_line)
    {
        return None;
    }

    let [explore_root] = explore_segment.default_effects.as_slice() else {
        return None;
    };
    let explore = structural_unwrap_render_wrappers(explore_root)
        .downcast_ref::<crate::effects::ExploreEffect>()?;
    let surface = explore.target.source_reference_surface()?;
    if !matches!(
        surface,
        crate::target::SourceReferenceSurface::FullName(_)
            | crate::target::SourceReferenceSurface::ShortName(_)
    ) {
        return None;
    }

    let [destroy_root] = destroy_segment.default_effects.as_slice() else {
        return None;
    };
    let sequence = destroy_root.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen
        || sequence.result_label.is_some()
    {
        return None;
    }
    let [conditional_root] = sequence.effects.as_slice() else {
        return None;
    };
    let conditional = structural_unwrap_render_wrappers(conditional_root)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.surface != ironsmith_core::ConditionalSurface::TrailingIf
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let Condition::ValueComparison {
        left,
        operator: ironsmith_core::ValueComparisonOperator::Equal,
        right,
    } = &conditional.condition
    else {
        return None;
    };
    if !matches!(
        left.unhinted(),
        Value::PowerOf(spec) if matches!(spec.unhinted(), ChooseSpec::Source)
    ) || right.unhinted() != &Value::Fixed(20)
        || !right.has_surface_hint(ironsmith_core::ValueSurfaceHint::ExactComparison)
    {
        return None;
    }
    let [destroy_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    if destroy.spec != ChooseSpec::all(ObjectFilter::creature().other()) {
        return None;
    }

    Some(format!(
        "{} explores. Then destroy all other creatures if its power is exactly 20",
        capitalize_first(&surface.display_text())
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
    if let Some(rendered) = describe_kamiz_relational_attacker_sequence(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_named_source_explore_then_exact_power_destruction(triggered) {
        let trigger = describe_trigger_surface_with_frequency(triggered, None, self_subject);
        return format!("{trigger}, {rendered}");
    }
    if let Some(rendered) = describe_backup_keyword(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_unique_creature_control_leader_upkeep_control_change(triggered)
    {
        return rendered;
    }
    if let Some(rendered) = describe_targeted_player_or_permanent_counter_unless_life(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_target_opponent_may_copy_triggering_spell(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_oath_of_ghouls_triggered_ability(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_flurry_copy_exile_suspend_triggered_ability(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_convoke_cast_damage_opponents_and_protected_battles(triggered)
    {
        return rendered;
    }
    if let Some(rendered) = describe_optional_copy_plural_keyword_grant(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_etb_source_power_reciprocal_damage(triggered) {
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
    if let Some(rendered) = describe_each_player_first_main_counter_then_scaled_mana(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_each_player_first_main_scaled_artifact_mana(triggered) {
        return rendered;
    }
    if let Some(rendered) =
        describe_active_player_postcombat_opponents_lost_life_mana_trigger(triggered)
    {
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
    apply_attacks_while_most_life_surface(triggered, &mut line, &mut intervening_condition);
    if line.to_ascii_lowercase().ends_with("becomes tapped") {
        let mut conjuncts = Vec::new();
        if let Some(condition) = intervening_condition.take() {
            flatten_condition_and_expr(&condition, &mut conjuncts);
        }
        let had_your_turn = conjuncts
            .iter()
            .any(|condition| matches!(condition, Condition::YourTurn));
        conjuncts.retain(|condition| !matches!(condition, Condition::YourTurn));
        intervening_condition = fold_condition_exprs(conjuncts);
        if had_your_turn {
            line.push_str(" during your turn");
        }
    }
    let saga_intervening_condition = if triggered.trigger.saga_chapters().is_some() {
        intervening_condition.take()
    } else {
        None
    };
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
        if let Some(condition) = saga_intervening_condition.as_ref() {
            line.push_str(" — If ");
            line.push_str(&describe_trigger_intervening_condition(
                condition,
                triggered,
                Some(self_subject),
            ));
            line.push_str(", ");
            line.push_str(&lowercase_first(&clauses.join(": ")));
        } else if clauses.len() == 1 {
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
            } else if triggered.trigger.saga_chapters().is_some() {
                line.push_str(" — ");
                line.push_str(&capitalize_first(only));
            } else if triggered.presentation_label.is_some() {
                line.push_str(", ");
                line.push_str(&lowercase_first(only));
            } else {
                // A triggered ability's trigger and instruction are one
                // sentence in Oracle text. The colon is reserved for costs
                // and labels, not a generic trigger/effect boundary.
                line.push_str(", ");
                line.push_str(&lowercase_first(only));
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
    let mut line = apply_triggered_presentation_label(triggered, line);
    let condition_qualified = triggered
        .trigger
        .downcast_ref::<crate::triggers::ConditionQualifiedTrigger>();
    line = line.replace("you draw a card and you lose", "you draw a card and lose");
    if condition_qualified.is_some_and(|qualified| {
        matches!(
            &qualified.condition,
            Condition::YouControl(filter)
                | Condition::PlayerControls {
                    player: PlayerFilter::You,
                    filter,
                } if filter.subtypes.contains(&crate::types::Subtype::Dinosaur)
        )
    }) {
        line = line.replace(
            "while you control a dinosaur",
            "while you control a Dinosaur",
        );
    }
    fn puts_stun_counter(effect: &Effect) -> bool {
        if effect
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .is_some_and(|put| put.counter_type == crate::object::CounterType::Stun)
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            if !found {
                found = puts_stun_counter(child);
            }
        });
        found
    }
    let puts_stun_counter = triggered
        .effects
        .all_effects()
        .into_iter()
        .any(puts_stun_counter);
    if condition_qualified.is_some_and(|qualified| qualified.stun_counter_reminder_surface)
        || (condition_qualified.is_some() && puts_stun_counter)
    {
        line.push_str(". ");
        line.push_str(STANDARD_REMINDER_OPEN_SENTINEL);
        line.push_str(
            "If a permanent with a stun counter would become untapped, remove one from it instead.",
        );
        line.push_str(STANDARD_REMINDER_CLOSE_SENTINEL);
    }
    line
}

/// Render the reciprocal ETB damage shape only after the runtime program has
/// retained all three identities: the entering source, the chosen opposing
/// creature, and the latter creature as the source of the return damage.
/// The Battlefield condition is semantic, not presentation metadata; this
/// matcher deliberately rejects the formerly produced unconstrained gate.
fn describe_etb_source_power_reciprocal_damage(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let enters = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !enters.this_object
        || enters.from != crate::triggers::zone_changes::ZonePattern::Any
        || enters.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || enters.player != crate::triggers::zone_changes::PlayerRelation::Any
        || enters.cause_filter.is_some()
        || enters.during_turn.is_some()
        || enters.timing.is_some()
        || enters.origin_condition.is_some()
        || enters.count_mode != crate::triggers::zone_changes::CountMode::Each
        || enters.this_object_surface
            != Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this creature".to_string(),
            ))
        || triggered.intervening_if != Some(Condition::SourceIsInZone(Zone::Battlefield))
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let [declared_choice] = triggered.choices.as_slice() else {
        return None;
    };
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if segment.starts_new_source_line || !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, sequence_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if !sequence.surface.is_coordinated() {
        return None;
    }
    let [target_effect, source_damage_effect, return_damage_effect] = sequence.effects.as_slice()
    else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.chooser.is_some()
        || target.explicit_declaration
        || target.target.unhinted() != declared_choice.unhinted()
    {
        return None;
    }
    let ChooseSpec::Target(target_inner) = target.target.unhinted() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target_inner.unhinted() else {
        return None;
    };
    let mut expected_target = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::Opponent);
    expected_target.set_explicit_card_type_noun(Some(CardType::Creature));
    if target_filter != &expected_target {
        return None;
    }

    let source_damage = unwrap_basic_tag_wrappers(source_damage_effect)
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let source_damage_inner = unwrap_basic_tag_wrappers(&source_damage.effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        source_damage.source.unhinted(),
        ChooseSpec::Tagged(tag) if tag == &triggering.tag
    ) || source_damage_inner.target.unhinted() != target.target.unhinted()
        || !matches!(
            source_damage_inner.amount.unhinted(),
            Value::PowerOf(spec) if spec.unhinted() == source_damage.source.unhinted()
        )
        || source_damage_inner.source_is_combat
        || source_damage_inner.unpreventable
    {
        return None;
    }

    let return_damage = unwrap_basic_tag_wrappers(return_damage_effect)
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let return_damage_inner = unwrap_basic_tag_wrappers(&return_damage.effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if return_damage.source.unhinted() != target.target.unhinted()
        || !matches!(return_damage_inner.target.base(), ChooseSpec::Source)
        || !matches!(
            return_damage_inner.amount.unhinted(),
            Value::PowerOf(spec) if spec.unhinted() == return_damage.source.unhinted()
        )
        || return_damage_inner.source_is_combat
        || return_damage_inner.unpreventable
    {
        return None;
    }

    Some(
        "When this creature enters, if it's on the battlefield, it deals damage equal to its power to target creature an opponent controls and that creature deals damage equal to its power to this creature"
            .to_string(),
    )
}

/// Preserve the event player as both the mana recipient and controller of the
/// counted artifacts for an each-player first-main trigger. The generic mana
/// renderer otherwise repeats the player at the end of the instruction and
/// loses the natural `that player ... they control` correlation.
pub(super) fn describe_each_player_first_main_scaled_artifact_mana(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !triggered.choices.is_empty() || triggered.presentation_label.is_some() {
        return None;
    }
    let source_is_untapped = match triggered.intervening_if.as_ref()? {
        Condition::SourceIsUntapped => true,
        Condition::Not(inner) => matches!(inner.as_ref(), Condition::SourceIsTapped),
        _ => false,
    };
    if !source_is_untapped {
        return None;
    }
    let phase = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()?;
    if phase.player != PlayerFilter::Any
        || phase.phase_type != crate::triggers::phase_step::MainPhaseType::Precombat
        || phase.main_phase_surface != ironsmith_core::trigger_model::MainPhaseSurface::MainPhase
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if segment.starts_new_source_line || !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let add = effect.downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    if add.player != PlayerFilter::IteratedPlayer
        || add.mana.as_slice() != [crate::mana::ManaSymbol::Colorless]
    {
        return None;
    }
    let Value::Count(filter) = add.amount.unhinted() else {
        return None;
    };
    let mut expected = ObjectFilter::artifact().controlled_by(PlayerFilter::IteratedPlayer);
    expected.set_explicit_card_type_noun(Some(CardType::Artifact));
    if filter != &expected {
        return None;
    }

    Some(
        "At the beginning of each player's first main phase, if this artifact is untapped, that player adds {C} for each artifact they control"
            .to_string(),
    )
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod each_player_first_main_scaled_artifact_mana_tests {
    use super::*;

    fn parsed() -> crate::ability::TriggeredAbility {
        let oracle = "At the beginning of each player's first main phase, if this artifact is untapped, that player adds {C} for each artifact they control.";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "First Main Artifact Mana Probe",
        )
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("typed each-player scaled-mana trigger");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()]
        );
        let [ability] = definition.abilities.as_slice() else {
            panic!("expected one ability: {definition:#?}");
        };
        let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("expected triggered ability: {ability:#?}");
        };
        triggered.clone()
    }

    #[test]
    fn exact_shape_keeps_the_event_player_and_artifact_count_correlated() {
        let triggered = parsed();
        assert_eq!(
            describe_each_player_first_main_scaled_artifact_mana(&triggered).as_deref(),
            Some(
                "At the beginning of each player's first main phase, if this artifact is untapped, that player adds {C} for each artifact they control"
            )
        );

        let mut changed_actor = triggered.clone();
        let add = changed_actor.effects.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::AddScaledManaEffect>()
            .expect("scaled mana")
            .clone();
        changed_actor.effects.segments[0].default_effects[0] =
            Effect::new(crate::effects::AddScaledManaEffect {
                player: PlayerFilter::You,
                ..add
            });
        assert!(
            describe_each_player_first_main_scaled_artifact_mana(&changed_actor).is_none(),
            "a different recipient must not inherit the event-player pronoun"
        );

        let mut changed_condition = triggered;
        changed_condition.intervening_if = None;
        assert!(
            describe_each_player_first_main_scaled_artifact_mana(&changed_condition).is_none(),
            "the source-untapped condition is part of the compacted shape"
        );
    }
}

/// Preserve the event player across an optional counter action and its
/// result-gated mana payoff. The runtime program deliberately uses
/// `IteratedPlayer` for both the choice and the mana recipient; rendering the
/// second segment as an imperative loses that shared actor.
pub(super) fn describe_each_player_first_main_counter_then_scaled_mana(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let phase = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()?;
    if phase.player != PlayerFilter::Any
        || phase.phase_type != crate::triggers::phase_step::MainPhaseType::Precombat
        || phase.main_phase_surface != ironsmith_core::trigger_model::MainPhaseSurface::MainPhase
    {
        return None;
    }
    let [counter_segment, mana_segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if counter_segment.starts_new_source_line
        || mana_segment.starts_new_source_line
        || !counter_segment.self_replacements.is_empty()
        || !mana_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [with_id_effect] = counter_segment.default_effects.as_slice() else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [put_effect] = may.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != crate::object::CounterType::PlusOnePlusOne
        || put.amount.unhinted() != &Value::Fixed(1)
        || !matches!(put.target.base(), ChooseSpec::Source)
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }
    let [if_effect] = mana_segment.default_effects.as_slice() else {
        return None;
    };
    let result = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if result.condition != with_id.id
        || result.predicate != EffectPredicate::Happened
        || !result.else_.is_empty()
        || result.per_player_result
        || result.prior_result_replacement_surface
    {
        return None;
    }
    let [add_effect] = result.then.as_slice() else {
        return None;
    };
    let add = add_effect.downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    if add.player != PlayerFilter::IteratedPlayer
        || add.mana.as_slice() != [crate::mana::ManaSymbol::Colorless]
        || !matches!(
            add.amount.unhinted(),
            Value::CountersOn(spec, None) if matches!(spec.base(), ChooseSpec::Source)
        )
    {
        return None;
    }

    Some(
        "At the beginning of each player's first main phase, that player may put a +1/+1 counter on this creature. If they do, they add {C} for each counter on it"
            .to_string(),
    )
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod each_player_first_main_counter_mana_surface_tests {
    use super::*;

    fn parsed() -> crate::ability::TriggeredAbility {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Each Main Counter Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of each player's first main phase, that player may put a +1/+1 counter on this creature. If they do, they add {C} for each counter on it.",
        )
        .expect("typed per-player counter/mana trigger");
        let [ability] = definition.abilities.as_slice() else {
            panic!("expected one ability: {definition:#?}");
        };
        let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("expected triggered ability: {ability:#?}");
        };
        triggered.clone()
    }

    #[test]
    fn exact_shape_compacts_but_changed_mana_recipient_does_not() {
        let triggered = parsed();
        assert_eq!(
            describe_each_player_first_main_counter_then_scaled_mana(&triggered).as_deref(),
            Some(
                "At the beginning of each player's first main phase, that player may put a +1/+1 counter on this creature. If they do, they add {C} for each counter on it"
            )
        );

        let mut near_miss = triggered.clone();
        let result = near_miss.effects.segments[1].default_effects[0]
            .downcast_ref::<crate::effects::IfEffect>()
            .expect("result gate")
            .clone();
        let mut changed = result.then[0]
            .downcast_ref::<crate::effects::AddScaledManaEffect>()
            .expect("scaled mana")
            .clone();
        changed.player = PlayerFilter::You;
        let mut result = result;
        result.then[0] = Effect::new(changed);
        near_miss.effects.segments[1].default_effects[0] = Effect::new(result);
        assert!(
            describe_each_player_first_main_counter_then_scaled_mana(&near_miss).is_none(),
            "a different mana recipient must not inherit the event-player pronoun"
        );
    }
}

/// Preserve the combat-relative phase wording and event-participant actor for
/// the reusable "active player gets mana for opponents who lost life" shape.
/// The generic phase renderer deliberately prefers first/second-main wording,
/// while the typed value distinguishes affected opponents from total life
/// lost.
pub(super) fn describe_active_player_postcombat_opponents_lost_life_mana_trigger(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let phase = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()?;
    if phase.player != PlayerFilter::Any
        || phase.phase_type != crate::triggers::phase_step::MainPhaseType::Postcombat
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() || segment.starts_new_source_line {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let add = effect.downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    if add.player != PlayerFilter::Active
        || !matches!(
            add.amount.unhinted(),
            Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersLostLife(
                PlayerFilter::Opponent
            ))
        )
    {
        return None;
    }
    let mana = add
        .mana
        .iter()
        .copied()
        .map(describe_mana_symbol)
        .collect::<Vec<_>>()
        .join("");
    Some(format!(
        "At the beginning of each postcombat main phase, the active player adds {} for each of your opponents who lost life this turn",
        if mana.is_empty() { "{0}" } else { &mana }
    ))
}

#[cfg(test)]
mod active_player_postcombat_lost_life_mana_tests {
    use super::*;

    #[test]
    fn belbe_surface_keeps_active_player_and_distinct_opponent_count() {
        let oracle = "At the beginning of each postcombat main phase, the active player adds {C}{C} for each of your opponents who lost life this turn.";
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Belbe, Corrupted Observer",
        )
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![crate::types::CardType::Creature])
        .parse_text(oracle)
        .expect("Belbe-style postcombat mana trigger should compile");

        let [ability] = definition.abilities.as_slice() else {
            panic!("expected one triggered ability: {definition:#?}");
        };
        let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("expected a triggered ability: {ability:#?}");
        };
        let [effect] = triggered.effects.flattened_default_effects() else {
            panic!("expected one mana effect: {triggered:#?}");
        };
        let add = effect
            .downcast_ref::<crate::effects::AddScaledManaEffect>()
            .expect("expected scaled mana");
        assert_eq!(add.player, PlayerFilter::Active);
        assert!(matches!(
            add.amount.unhinted(),
            Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersLostLife(
                PlayerFilter::Opponent
            ))
        ));
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()]
        );
    }
}

pub(super) fn describe_trigger_intervening_condition(
    condition: &Condition,
    triggered: &crate::ability::TriggeredAbility,
    self_subject: Option<&str>,
) -> String {
    if matches!(
        condition,
        Condition::TriggeringObjectBecameTappedFirstTimeThisTurn
    ) && let Some(tapped) = triggered
        .trigger
        .downcast_ref::<crate::triggers::PermanentBecomesTappedTrigger>()
        && let Some(noun) = simple_filter_singular_noun(&tapped.filter)
    {
        return format!("it's the first time that {noun} has become tapped this turn");
    }
    if matches!(
        condition,
        Condition::TriggeringObjectHadCountersPutFirstTimeThisTurn
    ) && let Some(counters) = triggered
        .trigger
        .downcast_ref::<crate::triggers::CounterPutOnTrigger>()
        && let Some(noun) = simple_filter_singular_noun(&counters.filter)
    {
        return format!("it's the first time counters have been put on that {noun} this turn");
    }
    if matches!(condition, Condition::ThisSpellWasKicked)
        && trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return "it was kicked".to_string();
    }
    if let Condition::SourceHasNoCounter(counter_type) = condition
        && let Some(subject) = self_subject
    {
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
    if let Condition::Not(inner) = condition
        && let Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } = inner.as_ref()
        && trigger_is_this_dies(&triggered.trigger)
    {
        let counter = counter_type.description();
        return if *min_count == 1 {
            format!("it had no {counter} counters on it")
        } else {
            format!("it had fewer than {min_count} {counter} counters on it")
        };
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

pub(super) fn describe_delayed_life_loss_and_source_return(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot || schedule.start_next_turn || schedule.until_end_of_turn {
        return None;
    }
    let trigger_text = schedule.trigger.display().to_ascii_lowercase();
    if !trigger_text.contains("beginning of") || !trigger_text.contains("end step") {
        return None;
    }
    let [lose_effect, return_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let lose =
        unwrap_basic_tag_wrappers(lose_effect).downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let returned = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if !matches!(lose.player.base(), ChooseSpec::Player(PlayerFilter::You))
        || !matches!(returned.spec.base(), ChooseSpec::Source)
    {
        return None;
    }

    let lose_text = lowercase_first(describe_effect(lose_effect).trim().trim_end_matches('.'));
    let return_text = lowercase_first(describe_effect(return_effect).trim().trim_end_matches('.'));
    Some(format!(
        "At the beginning of the next end step, {lose_text} and {return_text}"
    ))
}

/// A dies-trigger delayed return is authored action-first because the upkeep
/// belongs to the returned card's owner: "return it ... at the beginning of
/// their next upkeep." The schedule stores the same relationship as an
/// owner-scoped upkeep trigger around a tagged-object move.
pub(super) fn describe_delayed_owned_object_return_at_next_upkeep(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot
        || !schedule.start_next_turn
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
    {
        return None;
    }
    let upkeep = schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    let PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(owner_tag)) = &upkeep.player else {
        return None;
    };
    let [return_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let returned = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if returned.zone != Zone::Battlefield
        || returned.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Return
        || returned.battlefield_controller != crate::effects::BattlefieldController::Owner
        || !returned.controller_surface_explicit
        || !matches!(returned.target.base(), ChooseSpec::Tagged(return_tag) if return_tag == owner_tag)
    {
        return None;
    }

    let mut action = "return it to the battlefield".to_string();
    if returned.enters_tapped {
        action.push_str(" tapped");
    }
    action.push_str(" under its owner's control at the beginning of their next upkeep");
    Some(action)
}

/// Preserve an action-first delayed return whose upkeep player and battlefield
/// controller are the same previously targeted opponent.
///
/// The schedule and the nested generic battlefield move independently retain
/// the aliased player. Requiring both links prevents an unrelated delayed move
/// from borrowing the reflexive "that player's/their" surface.
pub(super) fn describe_delayed_source_return_under_target_player_control_at_next_upkeep(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot
        || !schedule.start_next_turn
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
        || schedule.leading_duration_surface
        || schedule.watch_ability_source
        || schedule.watch_all_object_targets
        || schedule.either_of_watched_objects
        || schedule.duration != ironsmith_core::DelayedTriggerDuration::Forever
        || schedule.while_any_tagged_object_in_zone.is_some()
        || !schedule.target_objects.is_empty()
        || schedule.target_tag.is_some()
        || schedule.target_filter.is_some()
        || schedule.controller != PlayerFilter::You
        || schedule.prepayment.is_some()
        || schedule.event_value_from_prior_prevention
    {
        return None;
    }
    let upkeep = schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    let expected_player = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent));
    if upkeep.player != expected_player {
        return None;
    }
    let [return_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let returned = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    if returned.controller != expected_player
        || !matches!(returned.target.unhinted(), ChooseSpec::Source)
        || !returned.enters_with_counters.is_empty()
    {
        return None;
    }

    let mut action = format!(
        "Return {} to the battlefield",
        describe_choose_spec(&returned.target)
    );
    if returned.tapped {
        action.push_str(" tapped");
    }
    action.push_str(" under that player's control at the beginning of their next upkeep");
    Some(action)
}

/// Dynamic token counts inside a one-shot upkeep schedule are evaluated when
/// that schedule resolves. Preserve the common action-first Oracle surface
/// and make the evaluation point explicit in the trailing X definition.
pub(super) fn describe_delayed_dynamic_token_creation_at_next_upkeep(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot
        || !schedule.start_next_turn
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
    {
        return None;
    }
    let upkeep = schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    if upkeep.player != PlayerFilter::You {
        return None;
    }
    let [create_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let create = unwrap_basic_tag_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if matches!(create.count.unhinted(), Value::Fixed(_)) {
        return None;
    }

    let created = describe_effect(create_effect);
    let created = created.trim().trim_end_matches('.');
    let (action, where_x) = created.split_once(", where X is ")?;
    let where_x = where_x
        .strip_suffix(" at that time")
        .unwrap_or(where_x)
        .trim();
    Some(format!(
        "{action} at the beginning of your next upkeep, where X is {where_x} at that time"
    ))
}

/// Preserve the action-first surface for a delayed draw-step life payment:
/// "that player loses N life at ... unless they pay ... before that draw
/// step." The executable model stores the advance payment window on the
/// delayed registration itself; legacy nested-payment definitions remain
/// renderable while old snapshots migrate. This renderer only recombines the
/// clause after proving the draw-step owner, payer, and life-loss recipient are
/// all the player damaged by the enclosing trigger.
pub(super) fn describe_delayed_draw_step_life_loss_unless_payment(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot
        || !schedule.start_next_turn
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
    {
        return None;
    }
    let draw_step = schedule
        .trigger
        .downcast_ref::<crate::triggers::phase_step::BeginningOfDrawStepTrigger>()?;
    if draw_step.player != PlayerFilter::DamagedPlayer {
        return None;
    }

    let flattened = schedule.effects.flattened_default_effects();
    let (payer, cost, life_effect) = if let Some(prepayment) = &schedule.prepayment {
        let [life_effect] = flattened else {
            return None;
        };
        (&prepayment.player, &prepayment.cost, life_effect)
    } else {
        let [unless_effect] = flattened else {
            return None;
        };
        let unless = unwrap_basic_tag_wrappers(unless_effect)
            .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
        if unless.leading_surface {
            return None;
        }
        let [life_effect] = unless.effects.as_slice() else {
            return None;
        };
        (&unless.player, &unless.cost, life_effect)
    };
    if payer != &draw_step.player {
        return None;
    }
    let life_loss =
        unwrap_basic_tag_wrappers(life_effect).downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if !matches!(life_loss.player.base(), ChooseSpec::Player(player) if player == &draw_step.player)
    {
        return None;
    }

    let life_text = lowercase_first(describe_effect(life_effect).trim().trim_end_matches('.'));
    let payment = describe_total_cost_payment(cost);
    let payment = payment.strip_prefix("Pay ").unwrap_or(&payment);
    Some(format!(
        "{life_text} at the beginning of their next draw step unless they pay {payment} before that draw step"
    ))
}

/// Recombine a poison counter followed by a prepayable delayed second poison
/// counter into the action-first Oracle surface used by Asp-style abilities.
pub(super) fn describe_delayed_upkeep_additional_poison_unless_payment(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot
        || !schedule.start_next_turn
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
    {
        return None;
    }
    let upkeep = schedule
        .trigger
        .downcast_ref::<crate::triggers::phase_step::BeginningOfUpkeepTrigger>()?;
    if upkeep.player != PlayerFilter::DamagedPlayer {
        return None;
    }
    let prepayment = schedule.prepayment.as_ref()?;
    if prepayment.player != upkeep.player {
        return None;
    }
    let [poison_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let poison = unwrap_basic_tag_wrappers(poison_effect)
        .downcast_ref::<crate::effects::PoisonCountersEffect>()?;
    if poison.player != upkeep.player || poison.count != Value::Fixed(1) {
        return None;
    }

    let payment = describe_total_cost_payment(&prepayment.cost);
    let payment = payment.strip_prefix("Pay ").unwrap_or(&payment);
    Some(format!(
        "the player gets another poison counter at the beginning of their next upkeep unless they pay {payment} before that step"
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
    let coordinated = match for_players.effects.as_slice() {
        [sequence_effect] => sequence_effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .filter(|sequence| sequence.surface == ironsmith_core::SequenceSurface::Coordinated)
            .map(|sequence| sequence.effects.as_slice()),
        _ => None,
    };
    let (discard_effect, return_effect) =
        match (coordinated, for_players.effects.as_slice(), effects) {
            (Some([discard_effect, return_effect]), _, [_]) => (discard_effect, return_effect),
            (None, [discard_effect, return_effect], [_]) => (discard_effect, return_effect),
            (None, [discard_effect], [_, return_effect]) => (discard_effect, return_effect),
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

/// Return whether a delayed trigger can observe a cast spell, an activated
/// ability, or both. The copy effect itself stores only a generic triggering
/// stack-object reference, so its Oracle noun must come from this typed
/// trigger provenance rather than from the copy effect's fallback surface.
fn delayed_copy_trigger_stack_object_kinds(trigger: &crate::triggers::Trigger) -> (bool, bool) {
    if trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .is_some()
    {
        return (true, false);
    }
    if trigger
        .downcast_ref::<crate::triggers::AbilityActivatedTrigger>()
        .is_some()
    {
        return (false, true);
    }
    if let Some(or_trigger) = trigger.downcast_ref::<crate::triggers::OrTrigger>() {
        return or_trigger
            .triggers
            .iter()
            .map(delayed_copy_trigger_stack_object_kinds)
            .fold((false, false), |(spells, abilities), (spell, ability)| {
                (spells || spell, abilities || ability)
            });
    }
    (false, false)
}

fn describe_next_cast_entry_counter_replacement(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if schedule.until_end_of_combat
        || schedule.leading_duration_surface
        || schedule.watch_ability_source
        || schedule.watch_all_object_targets
        || schedule.either_of_watched_objects
        || schedule.while_any_tagged_object_in_zone.is_some()
        || !schedule.target_objects.is_empty()
        || schedule.target_tag.is_some()
        || schedule.target_filter.is_some()
        || schedule.controller != PlayerFilter::You
        || schedule.prepayment.is_some()
        || schedule.event_value_from_prior_prevention
    {
        return None;
    }
    let trigger = schedule
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    let spell_filter = trigger.filter.as_ref()?;
    if trigger.caster != PlayerFilter::You
        || trigger.mana_source_filter.is_some()
        || trigger.timing.is_some()
        || trigger.during_turn.is_some()
        || trigger.min_spells_this_turn.is_some()
        || trigger.exact_spells_this_turn.is_some()
        || trigger.count_all_spells_this_turn
        || trigger.from_not_hand
        || trigger.first_spell_of_game
        || spell_filter.card_types.as_slice() != [CardType::Creature]
    {
        return None;
    }
    let [tag_effect, register_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let register = register_effect
        .downcast_ref::<crate::effects::RegisterNextBatchEnterWithCountersEffect>()?;
    let mut expected_filter = spell_filter.clone();
    expected_filter.zone = Some(Zone::Battlefield);
    expected_filter.stack_kind = None;
    expected_filter.has_mana_cost = false;
    if register.filter != expected_filter
        || register.same_stable_id_tag.as_ref() != Some(&tag.tag)
        || register.count.unhinted() != &Value::Fixed(1)
        || !register
            .count
            .has_surface_hint(ValueSurfaceHint::InlineBattlefieldEntryCounter)
        || !register
            .count
            .has_surface_hint(ValueSurfaceHint::AdditionalEntryCounter)
    {
        return None;
    }
    Some(format!(
        "that creature enters with an additional {} counter on it",
        describe_counter_type(register.counter_type)
    ))
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
    let (trigger_can_cast_spell, trigger_can_activate_ability) =
        delayed_copy_trigger_stack_object_kinds(&schedule.trigger);
    if let Some(delayed_text) = describe_next_cast_entry_counter_replacement(schedule) {
        return Some(format!(
            "When you next {trigger_action} this turn, {delayed_text}"
        ));
    }
    let copying_multiple = schedule
        .effects
        .flattened_default_effects()
        .iter()
        .any(|effect| {
            copy_spell_from_effect(effect)
                .is_some_and(|copy| copy.count.unhinted() != &Value::Fixed(1))
        });
    let mut delayed_text = lowercase_first(&describe_effect_list(&schedule.effects));
    let copied_object = match (trigger_can_cast_spell, trigger_can_activate_ability) {
        (true, true) => "spell or ability",
        (false, true) => "ability",
        _ => "spell",
    };
    if let Some(rest) = delayed_text.strip_prefix("copy it") {
        delayed_text = format!("copy that {copied_object}{rest}");
    } else if copied_object != "spell"
        && !delayed_text.starts_with(&format!("copy that {copied_object}"))
        && let Some(rest) = delayed_text.strip_prefix("copy that spell")
    {
        delayed_text = format!("copy that {copied_object}{rest}");
    }
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
    if copying_multiple {
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
        let copied_object_surface = format!("copy that {copied_object}");
        delayed_text = delayed_text.replacen(
            &copied_object_surface,
            &format!("{copied_object_surface} an additional time"),
            1,
        );
    }
    Some(format!(
        "When you next {trigger_action} this turn, {delayed_text}"
    ))
}

/// Render the exact one-shot copy loop that chooses a new legal recipient for
/// each opponent other than the opponent singled out by the triggering spell.
/// Every relationship here is executable: the cast trigger owns the original
/// target, the player loop excludes that target/controller, and the copy is
/// retargeted only to the choice made inside that same loop iteration.
pub(super) fn describe_next_spell_each_other_opponent_copy_loop(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot
        || !schedule.until_end_of_turn
        || schedule.start_next_turn
        || schedule.until_end_of_combat
        || schedule.leading_duration_surface
        || schedule.watch_ability_source
        || schedule.watch_all_object_targets
        || schedule.either_of_watched_objects
        || schedule.duration != ironsmith_core::DelayedTriggerDuration::EndOfTurn
        || schedule.while_any_tagged_object_in_zone.is_some()
        || !schedule.target_objects.is_empty()
        || schedule.target_tag.is_some()
        || schedule.target_filter.is_some()
        || schedule.controller != PlayerFilter::You
        || schedule.prepayment.is_some()
        || schedule.event_value_from_prior_prevention
    {
        return None;
    }

    let trigger = schedule
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    let mut expected_spell = ObjectFilter::instant_or_sorcery()
        .targeting_only(
            Some(PlayerFilter::Opponent),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
        )
        .target_count_exact(1);
    expected_spell.has_mana_cost = true;
    if trigger.filter.as_ref()? != &expected_spell
        || trigger.caster != PlayerFilter::You
        || trigger.mana_source_filter.is_some()
        || trigger.timing.is_some()
        || trigger.during_turn.is_some()
        || trigger.min_spells_this_turn.is_some()
        || trigger.exact_spells_this_turn.is_some()
        || trigger.count_all_spells_this_turn
        || trigger.from_not_hand
        || trigger.first_spell_of_game
    {
        return None;
    }

    let [tag_triggering_effect, for_players_effect] = schedule.effects.flattened_default_effects()
    else {
        return None;
    };
    let tag_triggering =
        tag_triggering_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    if tag_triggering.tag.as_str() != "triggering" {
        return None;
    }
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter
        != PlayerFilter::excluding(
            PlayerFilter::Opponent,
            PlayerFilter::TargetPlayerOrControllerOfTarget,
        )
    {
        return None;
    }

    let [choice_effect, copy_effect, retarget_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let tagged_choice = choice_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let choice = tagged_choice
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let expected_permanent = ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer);
    if choice.chooser.is_some()
        || choice.explicit_declaration
        || !matches!(
            choice.target.base(),
            ChooseSpec::ObjectOrPlayer(object, PlayerFilter::IteratedPlayer)
                if object == &expected_permanent
        )
    {
        return None;
    }

    let (copied_tag, copy) = tagged_copy_spell_from_effect(copy_effect)?;
    if copied_tag.as_str() != "__copied_stack_object__"
        || copy.count != Value::Fixed(1)
        || copy.copier != PlayerFilter::You
        || copy.target_reference_kind != Some(crate::filter::StackObjectKind::Spell)
        || copy.target_reference_pronoun
        || !matches!(&copy.target, ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
    {
        return None;
    }

    let retarget = retarget_effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if retarget.chooser != PlayerFilter::You
        || retarget.require_change
        || retarget.new_target_restriction.is_some()
        || !matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag == copied_tag)
    {
        return None;
    }
    let crate::effects::RetargetMode::OneToFixed(fixed) = &retarget.mode else {
        return None;
    };
    let ChooseSpec::ObjectOrPlayer(chosen_object, PlayerFilter::IteratedPlayer) = fixed.base()
    else {
        return None;
    };
    let mut expected_chosen_object = expected_permanent;
    expected_chosen_object
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: tagged_choice.tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    if chosen_object != &expected_chosen_object {
        return None;
    }

    Some(
        "When you next cast an instant or sorcery spell that targets only a single opponent or a single permanent an opponent controls this turn, for each other opponent, choose that player or a permanent they control, copy that spell, and the copy targets the chosen player or permanent"
            .to_string(),
    )
}

/// An enters trigger may create a one-shot watcher using the authored
/// object-first surface, `copy the next spell ... when you cast it`. Keep that
/// wording only for the exact typed watcher; other delayed spell-copy effects
/// retain the ordinary `When you next cast ...` rendering.
fn describe_etb_copy_next_spell_when_cast(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let enters = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !enters.this_object
        || enters.from != crate::triggers::zone_changes::ZonePattern::Any
        || enters.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || !triggered.choices.is_empty()
        || triggered.intervening_if.is_some()
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, schedule_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    if triggering.tag.as_str() != "triggering" {
        return None;
    }
    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if describe_next_spell_delayed_trigger(schedule, false)?.as_str()
        != "When you next cast a spell this turn, copy it and you may choose new targets for the copy"
    {
        return None;
    }
    Some(format!(
        "copy the next spell you cast this turn when you cast it. You may choose new targets for the copy. {STANDARD_REMINDER_OPEN_SENTINEL}A copy of a permanent spell becomes a token.{STANDARD_REMINDER_CLOSE_SENTINEL}"
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
    if let Some(start) = line.find(&prefix)
        && (start == 0 || line[..start].ends_with(": "))
    {
        let rest = &line[start + prefix.len()..];
        return format!("{}When this creature enters,{rest}", &line[..start]);
    }
    line
}

/// Suffix for entry counters fused onto a return-to-battlefield move
/// ("with a +1/+1 counter on it").
fn describe_entry_counters_suffix(
    counters: &[ironsmith_core::BattlefieldEntryCounterSpec],
) -> String {
    if counters.is_empty() {
        return String::new();
    }
    if counters
        .iter()
        .any(|spec| spec.condition.is_some() || spec.object_filter.is_some())
    {
        return String::new();
    }
    let parts = counters
        .iter()
        .map(|spec| {
            let type_text = describe_counter_type(spec.counter_type);
            match &spec.amount {
                Value::Fixed(1) => format!("a {type_text} counter"),
                Value::Fixed(n) => {
                    let count_word = crate::compiled_text::normalize_common::number_word(*n)
                        .unwrap_or_else(|| n.to_string());
                    format!("{count_word} {type_text} counters")
                }
                amount => format!("{} {type_text} counters", describe_value(amount)),
            }
        })
        .collect::<Vec<_>>();
    format!(" with {} on it", join_with_and(&parts))
}
