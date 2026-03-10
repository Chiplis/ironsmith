use crate::cards::builders::{CardTextError, IT_TAG, PlayerAst, ReferenceEnv, TagKey, TargetAst};
use crate::effect::{EventValueSpec, Restriction, Value};
use crate::filter::{ObjectFilter, ObjectRef, PlayerFilter, TaggedOpbjectRelation};
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub(crate) fn is_you_player_filter(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::You => true,
        PlayerFilter::Target(inner) => is_you_player_filter(inner),
        _ => false,
    }
}

pub(crate) fn resolve_unless_player_filter(
    player: PlayerAst,
    refs: &ReferenceEnv,
    previous_last_player_filter: Option<PlayerFilter>,
) -> Result<PlayerFilter, CardTextError> {
    if matches!(player, PlayerAst::That)
        && !refs.iterated_player
        && refs
            .known_last_player_filter()
            .is_some_and(is_you_player_filter)
        && previous_last_player_filter
            .as_ref()
            .is_some_and(|filter| !is_you_player_filter(filter))
    {
        return previous_last_player_filter.ok_or_else(|| {
            CardTextError::InvariantViolation(
                "expected previous non-you player filter for unless-player resolution".to_string(),
            )
        });
    }
    resolve_non_target_player_filter(player, refs)
}

pub(crate) fn resolve_non_target_player_filter(
    player: PlayerAst,
    refs: &ReferenceEnv,
) -> Result<PlayerFilter, CardTextError> {
    match player {
        PlayerAst::You => Ok(PlayerFilter::You),
        PlayerAst::Any => Ok(PlayerFilter::Any),
        PlayerAst::Defending => Ok(PlayerFilter::Defending),
        PlayerAst::Attacking => Ok(PlayerFilter::Attacking),
        PlayerAst::Target | PlayerAst::TargetOpponent => Err(CardTextError::ParseError(
            "target player requires explicit targeting".to_string(),
        )),
        PlayerAst::Opponent => Ok(PlayerFilter::Opponent),
        PlayerAst::That => {
            if refs.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else if let Some(filter) = refs.known_last_player_filter() {
                Ok(filter.clone())
            } else {
                Ok(PlayerFilter::IteratedPlayer)
            }
        }
        PlayerAst::ThatPlayerOrTargetController => {
            Ok(PlayerFilter::TargetPlayerOrControllerOfTarget)
        }
        PlayerAst::ItsController => {
            if let Some(tag) = refs.known_last_object_tag() {
                Ok(PlayerFilter::ControllerOf(ObjectRef::tagged(tag.clone())))
            } else {
                Ok(PlayerFilter::ControllerOf(ObjectRef::Target))
            }
        }
        PlayerAst::ItsOwner => {
            if let Some(tag) = refs.known_last_object_tag() {
                Ok(PlayerFilter::OwnerOf(ObjectRef::tagged(tag.clone())))
            } else {
                Ok(PlayerFilter::OwnerOf(ObjectRef::Target))
            }
        }
        PlayerAst::Implicit => {
            if refs.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else {
                Ok(PlayerFilter::You)
            }
        }
    }
}

pub(crate) fn infer_player_filter_from_object_filter(
    filter: &ObjectFilter,
) -> Option<PlayerFilter> {
    if let Some(owner) = &filter.owner {
        return Some(owner.clone());
    }
    if let Some(controller) = &filter.controller {
        return Some(controller.clone());
    }
    for constraint in &filter.tagged_constraints {
        if matches!(
            constraint.relation,
            TaggedOpbjectRelation::SameControllerAsTagged
        ) {
            return Some(PlayerFilter::ControllerOf(ObjectRef::tagged(
                constraint.tag.clone(),
            )));
        }
    }
    filter
        .any_of
        .iter()
        .find_map(infer_player_filter_from_object_filter)
}

fn resolve_object_ref(reference: &ObjectRef, refs: &ReferenceEnv) -> ObjectRef {
    match reference {
        ObjectRef::Tagged(tag) if tag.as_str() == IT_TAG => refs
            .known_last_object_tag()
            .cloned()
            .map(ObjectRef::tagged)
            .unwrap_or(ObjectRef::Target),
        _ => reference.clone(),
    }
}

fn resolve_contextual_player_filter(
    filter: &PlayerFilter,
    refs: &ReferenceEnv,
) -> Result<PlayerFilter, CardTextError> {
    Ok(match filter {
        PlayerFilter::IteratedPlayer if !refs.iterated_player => refs
            .known_last_player_filter()
            .cloned()
            .unwrap_or(PlayerFilter::IteratedPlayer),
        PlayerFilter::Target(inner) => {
            PlayerFilter::Target(Box::new(resolve_contextual_player_filter(inner, refs)?))
        }
        PlayerFilter::Excluding { base, excluded } => PlayerFilter::Excluding {
            base: Box::new(resolve_contextual_player_filter(base, refs)?),
            excluded: Box::new(resolve_contextual_player_filter(excluded, refs)?),
        },
        PlayerFilter::ControllerOf(reference) => {
            PlayerFilter::ControllerOf(resolve_object_ref(reference, refs))
        }
        PlayerFilter::OwnerOf(reference) => {
            PlayerFilter::OwnerOf(resolve_object_ref(reference, refs))
        }
        _ => filter.clone(),
    })
}

fn resolve_object_filter_player_refs(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    let mut resolved = filter.clone();
    if let Some(controller) = resolved.controller.as_mut() {
        *controller = resolve_contextual_player_filter(controller, refs)?;
    }
    if let Some(cast_by) = resolved.cast_by.as_mut() {
        *cast_by = resolve_contextual_player_filter(cast_by, refs)?;
    }
    if let Some(owner) = resolved.owner.as_mut() {
        *owner = resolve_contextual_player_filter(owner, refs)?;
    }
    if let Some(targets_player) = resolved.targets_player.as_mut() {
        *targets_player = resolve_contextual_player_filter(targets_player, refs)?;
    }
    if let Some(targets_object) = resolved.targets_object.as_mut() {
        **targets_object = resolve_object_filter_player_refs(targets_object, refs)?;
    }
    if let Some(targets_only_player) = resolved.targets_only_player.as_mut() {
        *targets_only_player = resolve_contextual_player_filter(targets_only_player, refs)?;
    }
    if let Some(targets_only_object) = resolved.targets_only_object.as_mut() {
        **targets_only_object = resolve_object_filter_player_refs(targets_only_object, refs)?;
    }
    if let Some(attacking_player) = resolved
        .attacking_player_or_planeswalker_controlled_by
        .as_mut()
    {
        *attacking_player = resolve_contextual_player_filter(attacking_player, refs)?;
    }
    if let Some(entered_controller) = resolved.entered_battlefield_controller.as_mut() {
        *entered_controller = resolve_contextual_player_filter(entered_controller, refs)?;
    }
    for nested in &mut resolved.any_of {
        *nested = resolve_object_filter_player_refs(nested, refs)?;
    }
    Ok(resolved)
}

pub(crate) fn resolve_it_tag(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    let mut resolved = resolve_object_filter_player_refs(filter, refs)?;
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        return Ok(resolved);
    }

    let Some(tag) = refs.known_last_object_tag() else {
        let mut saw_it_constraint = false;
        resolved.tagged_constraints.retain(|constraint| {
            if constraint.tag.as_str() != IT_TAG {
                return true;
            }
            saw_it_constraint = true;
            false
        });

        if saw_it_constraint
            && matches!(
                resolved.zone,
                Some(Zone::Hand | Zone::Library | Zone::Graveyard | Zone::Exile)
            )
            && let Some(player_filter) = refs.known_last_player_filter().cloned()
        {
            if resolved.owner.is_none() {
                resolved.owner = Some(player_filter);
            }
            return Ok(resolved);
        }
        if saw_it_constraint
            && resolved == ObjectFilter::default()
            && let Some(player_filter) = refs.known_last_player_filter().cloned()
        {
            resolved.zone = Some(Zone::Hand);
            resolved.owner = Some(player_filter);
            return Ok(resolved);
        }
        if saw_it_constraint && resolved == ObjectFilter::default() {
            resolved.source = true;
            return Ok(resolved);
        }
        if saw_it_constraint {
            return Ok(resolved);
        }

        return Err(CardTextError::ParseError(
            "unable to resolve 'it' without prior reference".to_string(),
        ));
    };

    for constraint in &mut resolved.tagged_constraints {
        if constraint.tag.as_str() == IT_TAG {
            constraint.tag = tag.clone();
        }
    }
    Ok(resolved)
}

pub(crate) fn resolve_it_tag_key(
    tag: &TagKey,
    refs: &ReferenceEnv,
) -> Result<TagKey, CardTextError> {
    if tag.as_str() != IT_TAG {
        return Ok(tag.clone());
    }
    let resolved = refs.known_last_object_tag().ok_or_else(|| {
        CardTextError::ParseError("unable to resolve 'it' without prior reference".to_string())
    })?;
    Ok(TagKey::from(resolved.as_str()))
}

pub(crate) fn object_filter_as_tagged_reference(filter: &ObjectFilter) -> Option<TagKey> {
    if filter.tagged_constraints.len() != 1 {
        return None;
    }
    let constraint = &filter.tagged_constraints[0];
    if !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject) {
        return None;
    }

    let mut bare = filter.clone();
    bare.tagged_constraints.clear();
    if bare == ObjectFilter::default() {
        Some(constraint.tag.clone())
    } else {
        None
    }
}

pub(crate) fn watch_tag_from_filter(filter: &ObjectFilter) -> Option<TagKey> {
    let mut tag: Option<TagKey> = None;
    for constraint in &filter.tagged_constraints {
        if !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject) {
            continue;
        }
        match &tag {
            Some(existing) if existing.as_str() != constraint.tag.as_str() => return None,
            Some(_) => {}
            None => tag = Some(constraint.tag.clone()),
        }
    }
    tag
}

pub(crate) fn resolve_restriction_it_tag(
    restriction: &Restriction,
    refs: &ReferenceEnv,
) -> Result<Restriction, CardTextError> {
    let resolved = match restriction {
        Restriction::Attack(filter) => Restriction::attack(resolve_it_tag(filter, refs)?),
        Restriction::Block(filter) => Restriction::block(resolve_it_tag(filter, refs)?),
        Restriction::BlockSpecificAttacker { blockers, attacker } => {
            Restriction::block_specific_attacker(
                resolve_it_tag(blockers, refs)?,
                resolve_it_tag(attacker, refs)?,
            )
        }
        Restriction::MustBlockSpecificAttacker { blockers, attacker } => {
            Restriction::must_block_specific_attacker(
                resolve_it_tag(blockers, refs)?,
                resolve_it_tag(attacker, refs)?,
            )
        }
        Restriction::Untap(filter) => Restriction::untap(resolve_it_tag(filter, refs)?),
        Restriction::BeBlocked(filter) => Restriction::be_blocked(resolve_it_tag(filter, refs)?),
        Restriction::BeDestroyed(filter) => {
            Restriction::be_destroyed(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeRegenerated(filter) => {
            Restriction::be_regenerated(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeSacrificed(filter) => {
            Restriction::be_sacrificed(resolve_it_tag(filter, refs)?)
        }
        Restriction::HaveCountersPlaced(filter) => {
            Restriction::have_counters_placed(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeTargeted(filter) => Restriction::be_targeted(resolve_it_tag(filter, refs)?),
        Restriction::BeCountered(filter) => {
            Restriction::be_countered(resolve_it_tag(filter, refs)?)
        }
        Restriction::Transform(filter) => Restriction::transform(resolve_it_tag(filter, refs)?),
        Restriction::AttackOrBlock(filter) => {
            Restriction::attack_or_block(resolve_it_tag(filter, refs)?)
        }
        Restriction::ActivateAbilitiesOf(filter) => {
            Restriction::activate_abilities_of(resolve_it_tag(filter, refs)?)
        }
        Restriction::ActivateTapAbilitiesOf(filter) => {
            Restriction::activate_tap_abilities_of(resolve_it_tag(filter, refs)?)
        }
        Restriction::ActivateNonManaAbilitiesOf(filter) => {
            Restriction::activate_non_mana_abilities_of(resolve_it_tag(filter, refs)?)
        }
        _ => restriction.clone(),
    };
    Ok(resolved)
}

pub(crate) fn resolve_choose_spec_it_tag(
    spec: &ChooseSpec,
    refs: &ReferenceEnv,
) -> Result<ChooseSpec, CardTextError> {
    match spec {
        ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG => {
            if let Some(resolved) = refs.known_last_object_tag() {
                return Ok(ChooseSpec::Tagged(TagKey::from(resolved.as_str())));
            }
            if let Some(player_filter) = refs.known_last_player_filter().cloned() {
                let filter = ObjectFilter {
                    zone: Some(Zone::Hand),
                    owner: Some(player_filter),
                    ..Default::default()
                };
                return Ok(ChooseSpec::Object(filter));
            }
            Ok(ChooseSpec::Source)
        }
        ChooseSpec::Tagged(tag) => Ok(ChooseSpec::Tagged(tag.clone())),
        ChooseSpec::Object(filter) => {
            let resolved = resolve_it_tag(filter, refs)?;
            if let Some(tag) = object_filter_as_tagged_reference(&resolved) {
                Ok(ChooseSpec::Tagged(tag))
            } else {
                Ok(ChooseSpec::Object(resolved))
            }
        }
        ChooseSpec::Target(inner) => Ok(ChooseSpec::Target(Box::new(resolve_choose_spec_it_tag(
            inner, refs,
        )?))),
        ChooseSpec::WithCount(inner, count) => Ok(ChooseSpec::WithCount(
            Box::new(resolve_choose_spec_it_tag(inner, refs)?),
            *count,
        )),
        ChooseSpec::All(filter) => Ok(ChooseSpec::All(resolve_it_tag(filter, refs)?)),
        ChooseSpec::Player(filter) => Ok(ChooseSpec::Player(resolve_contextual_player_filter(
            filter, refs,
        )?)),
        ChooseSpec::PlayerOrPlaneswalker(filter) => Ok(ChooseSpec::PlayerOrPlaneswalker(
            resolve_contextual_player_filter(filter, refs)?,
        )),
        ChooseSpec::AttackedPlayerOrPlaneswalker => Ok(ChooseSpec::AttackedPlayerOrPlaneswalker),
        ChooseSpec::SpecificObject(id) => Ok(ChooseSpec::SpecificObject(*id)),
        ChooseSpec::SpecificPlayer(id) => Ok(ChooseSpec::SpecificPlayer(*id)),
        ChooseSpec::AnyTarget => Ok(ChooseSpec::AnyTarget),
        ChooseSpec::AnyOtherTarget => Ok(ChooseSpec::AnyOtherTarget),
        ChooseSpec::Source => Ok(ChooseSpec::Source),
        ChooseSpec::SourceController => Ok(ChooseSpec::SourceController),
        ChooseSpec::SourceOwner => Ok(ChooseSpec::SourceOwner),
        ChooseSpec::EachPlayer(filter) => Ok(ChooseSpec::EachPlayer(
            resolve_contextual_player_filter(filter, refs)?,
        )),
        ChooseSpec::Iterated => Ok(ChooseSpec::Iterated),
    }
}

pub(crate) fn resolve_value_it_tag(
    value: &Value,
    refs: &ReferenceEnv,
) -> Result<Value, CardTextError> {
    match value {
        Value::X if refs.bind_unbound_x_to_last_effect => {
            if let Some(id) = refs.known_last_effect_id() {
                Ok(Value::EffectValue(id))
            } else {
                Ok(Value::X)
            }
        }
        Value::Add(left, right) => Ok(Value::Add(
            Box::new(resolve_value_it_tag(left, refs)?),
            Box::new(resolve_value_it_tag(right, refs)?),
        )),
        Value::Count(filter) => Ok(Value::Count(resolve_it_tag(filter, refs)?)),
        Value::CountScaled(filter, multiplier) => Ok(Value::CountScaled(
            resolve_it_tag(filter, refs)?,
            *multiplier,
        )),
        Value::TotalPower(filter) => Ok(Value::TotalPower(resolve_it_tag(filter, refs)?)),
        Value::TotalToughness(filter) => Ok(Value::TotalToughness(resolve_it_tag(filter, refs)?)),
        Value::TotalManaValue(filter) => Ok(Value::TotalManaValue(resolve_it_tag(filter, refs)?)),
        Value::GreatestPower(filter) => Ok(Value::GreatestPower(resolve_it_tag(filter, refs)?)),
        Value::GreatestManaValue(filter) => {
            Ok(Value::GreatestManaValue(resolve_it_tag(filter, refs)?))
        }
        Value::BasicLandTypesAmong(filter) => {
            Ok(Value::BasicLandTypesAmong(resolve_it_tag(filter, refs)?))
        }
        Value::ColorsAmong(filter) => Ok(Value::ColorsAmong(resolve_it_tag(filter, refs)?)),
        Value::PowerOf(spec) => Ok(Value::PowerOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ToughnessOf(spec) => Ok(Value::ToughnessOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ManaValueOf(spec) => Ok(Value::ManaValueOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => {
            if !refs.allow_life_event_value {
                if let Some(id) = refs.known_last_effect_id() {
                    return Ok(Value::EffectValue(id));
                }
                return Err(CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger".to_string(),
                ));
            }
            Ok(value.clone())
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            if !refs.allow_life_event_value {
                if let Some(id) = refs.known_last_effect_id() {
                    return Ok(Value::EffectValueOffset(id, *offset));
                }
                return Err(CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger".to_string(),
                ));
            }
            Ok(value.clone())
        }
        _ => Ok(value.clone()),
    }
}

pub(crate) fn choose_spec_targets_object(spec: &ChooseSpec) -> bool {
    match spec.base() {
        ChooseSpec::Object(_)
        | ChooseSpec::Tagged(_)
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::Source => true,
        _ => false,
    }
}

pub(crate) fn choose_spec_for_target(target: &TargetAst) -> ChooseSpec {
    match target {
        TargetAst::Source(_) => ChooseSpec::Source,
        TargetAst::AnyTarget(_) => ChooseSpec::AnyTarget,
        TargetAst::AnyOtherTarget(_) => ChooseSpec::AnyOtherTarget,
        TargetAst::PlayerOrPlaneswalker(filter, _) => {
            ChooseSpec::PlayerOrPlaneswalker(filter.clone())
        }
        TargetAst::AttackedPlayerOrPlaneswalker(_) => ChooseSpec::AttackedPlayerOrPlaneswalker,
        TargetAst::Spell(_) => ChooseSpec::target_spell(),
        TargetAst::Player(filter, explicit_target_span) => {
            if *filter == PlayerFilter::You {
                ChooseSpec::SourceController
            } else if *filter == PlayerFilter::IteratedPlayer {
                ChooseSpec::Player(filter.clone())
            } else if explicit_target_span.is_some() {
                ChooseSpec::target(ChooseSpec::Player(filter.clone()))
            } else {
                ChooseSpec::Player(filter.clone())
            }
        }
        TargetAst::Object(filter, explicit_target_span, _) => {
            if explicit_target_span.is_some() {
                ChooseSpec::target(ChooseSpec::Object(filter.clone()))
            } else {
                ChooseSpec::Object(filter.clone())
            }
        }
        TargetAst::Tagged(tag, _) => ChooseSpec::Tagged(tag.clone()),
        TargetAst::WithCount(inner, count) => choose_spec_for_target(inner).with_count(*count),
    }
}

pub(crate) fn resolve_target_spec_with_choices(
    target: &TargetAst,
    refs: &ReferenceEnv,
) -> Result<(ChooseSpec, Vec<ChooseSpec>), CardTextError> {
    let mut spec = choose_spec_for_target(target);
    if let TargetAst::Player(filter, explicit_target_span) = target
        && explicit_target_span.is_none()
        && matches!(filter, PlayerFilter::Target(_))
    {
        if let Some(last_filter) = refs.known_last_player_filter() {
            spec = ChooseSpec::Player(last_filter.clone());
        } else if refs.iterated_player {
            spec = ChooseSpec::Player(PlayerFilter::IteratedPlayer);
        }
    }
    let spec = resolve_choose_spec_it_tag(&spec, refs)?;
    let choices = if spec.is_target() {
        vec![spec.clone()]
    } else {
        Vec::new()
    };
    Ok((spec, choices))
}

pub(crate) fn resolve_attach_object_spec(
    object: &TargetAst,
    refs: &ReferenceEnv,
) -> Result<(ChooseSpec, Vec<ChooseSpec>), CardTextError> {
    match object {
        TargetAst::Source(_) => Ok((ChooseSpec::Source, Vec::new())),
        TargetAst::Tagged(tag, _) => {
            let resolved_tag = if tag.as_str() == IT_TAG {
                refs.known_last_object_tag()
                    .map(|tag| tag.as_str().to_string())
                    .ok_or_else(|| {
                        CardTextError::ParseError(
                            "cannot resolve 'it/them' in attach object clause without prior tagged object"
                                .to_string(),
                        )
                    })?
            } else {
                tag.as_str().to_string()
            };
            Ok((
                ChooseSpec::All(ObjectFilter::tagged(TagKey::from(resolved_tag.as_str()))),
                Vec::new(),
            ))
        }
        TargetAst::Object(filter, explicit_target_span, _) => {
            let resolved = resolve_it_tag(filter, refs)?;
            if explicit_target_span.is_some() {
                let spec = ChooseSpec::target(ChooseSpec::Object(resolved));
                Ok((spec.clone(), vec![spec]))
            } else {
                Ok((ChooseSpec::All(resolved), Vec::new()))
            }
        }
        TargetAst::WithCount(inner, count) => {
            let (base, _) = resolve_attach_object_spec(inner, refs)?;
            let spec = base.with_count(*count);
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            Ok((spec, choices))
        }
        _ => Err(CardTextError::ParseError(
            "unsupported attach object reference".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_it_tag_rewrites_contextual_owner_filters() {
        let refs = ReferenceEnv {
            last_player_filter: crate::cards::builders::RefState::Known(
                PlayerFilter::ControllerOf(ObjectRef::tagged("exiled_1")),
            ),
            ..ReferenceEnv::default()
        };
        let filter = ObjectFilter {
            owner: Some(PlayerFilter::IteratedPlayer),
            any_of: vec![
                ObjectFilter::default().in_zone(Zone::Hand),
                ObjectFilter::default().in_zone(Zone::Graveyard),
            ],
            ..Default::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("resolve contextual owner filter");
        assert_eq!(
            resolved.owner,
            Some(PlayerFilter::ControllerOf(ObjectRef::tagged("exiled_1")))
        );
    }

    #[test]
    fn resolve_choose_spec_it_tag_rewrites_contextual_player_specs() {
        let refs = ReferenceEnv {
            last_player_filter: crate::cards::builders::RefState::Known(
                PlayerFilter::ControllerOf(ObjectRef::tagged("exiled_1")),
            ),
            ..ReferenceEnv::default()
        };

        let resolved =
            resolve_choose_spec_it_tag(&ChooseSpec::Player(PlayerFilter::IteratedPlayer), &refs)
                .expect("resolve player choose spec");

        assert_eq!(
            resolved,
            ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::tagged("exiled_1")))
        );
    }
}
