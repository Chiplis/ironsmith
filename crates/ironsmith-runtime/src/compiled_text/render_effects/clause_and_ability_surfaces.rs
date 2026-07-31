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
fn describe_coordinated_target_player_cast_and_activation_restrictions(
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
            Some((noun, action))
                if matches!(
                    noun,
                    "creature"
                        | "artifact"
                        | "enchantment"
                        | "land"
                        | "permanent"
                        | "source"
                        | "card"
                        | "spell"
                ) =>
            {
                action
            }
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
    if let Some(compact) = describe_source_sacrifice_then_coordinated_suffix(&sequence.effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_coordinated_copy_all_stack_sets(&sequence.effects) {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_joint_subject_pair(first, second)
    {
        return Some(compact);
    }
    describe_typed_coordinated_clause_fallback(&sequence.effects)
}

pub(super) fn describe_result_branch_effect_list(effects: &[Effect]) -> String {
    describe_typed_coordinated_result_branch(effects)
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

fn describe_comma_then_sequence(sequence: &crate::effects::SequenceEffect) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
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
            !effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some_and(|target| !target.explicit_declaration)
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

pub(super) fn describe_coordinated_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if std::env::var("IRONSMITH_SEQ_TRACE").is_ok() {
        eprintln!(
            "coordinated-sequence: surface={:?} len={}",
            sequence.surface,
            sequence.effects.len()
        );
    }
    if sequence.surface == ironsmith_core::SequenceSurface::CommaThen {
        return describe_comma_then_sequence(sequence);
    }
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
    if let [first, second] = sequence.effects.as_slice()
        && let Some(compact) = describe_action_and_get_energy_pair(first, second)
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
    let visible_effects = sequence
        .effects
        .iter()
        .filter(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
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
    if let Some(compact) = describe_damage_and_you_scry(&sequence.effects) {
        return Some(compact);
    }
    if let [first, second] = sequence.effects.as_slice() {
        if std::env::var("IRONSMITH_SEQ_TRACE").is_ok() {
            eprintln!(
                "coordinated-sequence pair: draw={} lose={} compact={:?}",
                first
                    .downcast_ref::<crate::effects::DrawCardsEffect>()
                    .is_some(),
                second
                    .downcast_ref::<crate::effects::LoseLifeEffect>()
                    .is_some(),
                first
                    .downcast_ref::<crate::effects::DrawCardsEffect>()
                    .zip(second.downcast_ref::<crate::effects::LoseLifeEffect>())
                    .and_then(|(draw, lose)| describe_draw_then_lose_life(draw, lose))
            );
        }
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
        && let Some(compact) = describe_target_creature_damage_fanout_pair(first, second)
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
        .or_else(|| describe_typed_coordinated_clause_fallback(&sequence.effects))
}

#[cfg(test)]
mod coordinated_sequence_tests {
    use super::*;

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
    let (second_source, second_damage) = coordinated_damage_view(inner)?;
    if first_source.map(ChooseSpec::unhinted) != second_source.map(ChooseSpec::unhinted) {
        return None;
    }
    if !matches!(second_damage.target, ChooseSpec::Iterated) {
        return None;
    }

    let (amount_text, where_x) = describe_damage_amount_clause(&first_damage.amount);
    let mut text = if second_damage.amount == first_damage.amount {
        format!(
            "Deal {amount_text} to target player or planeswalker and each creature that player or that planeswalker's controller controls"
        )
    } else {
        let (fanout_amount_text, fanout_where_x) =
            describe_damage_amount_clause(&second_damage.amount);
        if where_x.is_some() || fanout_where_x.is_some() {
            return None;
        }
        format!(
            "Deal {amount_text} to target player or planeswalker and {fanout_amount_text} to each creature that player or that planeswalker's controller controls"
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
    let return_object = if target_is_source {
        "it"
    } else if choose_spec_allows_multiple(&exile_move.target) {
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
    let (move_verb, destination) = battlefield_move_verb_and_destination(move_back);
    Some(format!(
        "{} {return_object} {destination} the battlefield{tapped_suffix} transformed{controller_suffix}",
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

fn describe_as_enters_counter_phrase_on_it(amount: &Value, counter_type: CounterType) -> String {
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

pub(crate) fn describe_static_ability_with_subject(
    static_ability: &crate::static_abilities::StaticAbility,
    subject: &str,
) -> String {
    if let Some(ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
        program,
        subject: authored_subject,
        also_turns_face_up,
        uses_enters_with_counter_surface,
        transforms_into,
        presentation_label,
    }) = static_ability.compiled_model().map(|model| &model.payload)
    {
        let timing = if let Some(destination) = transforms_into {
            format!("As {authored_subject} transforms into {destination}")
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
        describe_this_enters_and_your_upkeep_trigger(&triggered.trigger, self_subject)
            .or_else(|| describe_this_attacks_or_dies_trigger(&triggered.trigger))
            .or_else(|| describe_this_blocks_or_becomes_blocked_by_trigger(&triggered.trigger))
            .or_else(|| describe_becomes_blocked_trigger(&triggered.trigger))
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
        let definition =
            crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Johan")
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

pub(super) fn describe_triggered_resolution_text(
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
    rewrite_it_deals: bool,
) -> Option<String> {
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
    Some(effects)
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
        triggering_reference_damage_source_count(&triggered.effects);
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

    fn inspect(
        effect: &Effect,
        triggering_tag: &TagKey,
        matching: &mut usize,
        incompatible: &mut bool,
    ) {
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
    apply_triggered_presentation_label(triggered, line)
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
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
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
/// step." The executable model deliberately nests the life loss under the
/// payment and then the delayed trigger, so this renderer only recombines the
/// clause after proving the draw-step owner, payer, and life-loss recipient
/// are all the player damaged by the enclosing trigger.
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

    let [unless_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let unless = unwrap_basic_tag_wrappers(unless_effect)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless.leading_surface || unless.player != draw_step.player {
        return None;
    }
    let [life_effect] = unless.effects.as_slice() else {
        return None;
    };
    let life_loss =
        unwrap_basic_tag_wrappers(life_effect).downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if !matches!(life_loss.player.base(), ChooseSpec::Player(player) if player == &draw_step.player)
    {
        return None;
    }

    let life_text = lowercase_first(describe_effect(life_effect).trim().trim_end_matches('.'));
    let payment = describe_total_cost_payment(&unless.cost);
    let payment = payment.strip_prefix("Pay ").unwrap_or(&payment);
    Some(format!(
        "{life_text} at the beginning of their next draw step unless they pay {payment} before that draw step"
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
