use super::*;

struct TargetDeclaration<'a> {
    tags: Vec<&'a TagKey>,
    target: &'a ChooseSpec,
}

enum ForcedBlockSurface {
    Lure {
        attacker: String,
        duration: &'static str,
    },
    Specific {
        blocker: String,
        attacker: String,
        duration: &'static str,
    },
}

impl ForcedBlockSurface {
    fn render(&self) -> String {
        match self {
            Self::Lure { attacker, duration } => {
                format!("All creatures able to block {attacker} {duration} do so")
            }
            Self::Specific {
                blocker,
                attacker,
                duration,
            } => format!(
                "{} blocks {attacker} {duration} if able",
                capitalize_first(blocker)
            ),
        }
    }

    fn render_optional(&self) -> Option<String> {
        let Self::Specific {
            blocker,
            attacker,
            duration,
        } = self
        else {
            return None;
        };
        Some(format!(
            "You may have {} block {attacker} {duration} if able",
            lowercase_first(blocker)
        ))
    }
}

fn exact_tagged_reference(filter: &ObjectFilter) -> Option<&TagKey> {
    let [constraint] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || filter != &ObjectFilter::tagged(constraint.tag.clone())
    {
        return None;
    }
    Some(&constraint.tag)
}

fn exact_source_reference(filter: &ObjectFilter) -> bool {
    let mut normalized = filter.clone();
    normalized.source_surface = None;
    normalized == ObjectFilter::source()
}

fn is_all_creatures_filter(filter: &ObjectFilter) -> bool {
    filter == &ObjectFilter::creature()
        || filter == &ObjectFilter::creature().in_zone(Zone::Battlefield)
}

fn singular_target_text(target: &ChooseSpec) -> Option<String> {
    let count = target.count();
    if !target.is_target() || count.max != Some(1) || count.dynamic_x || count.random {
        return None;
    }
    let mut text = describe_choose_spec(target).replace(
        "target defending player's creature",
        "target creature defending player controls",
    );
    if count.min == 0 {
        text = text.replace("target another ", "other target ");
    } else {
        text = text.replace("target another ", "another target ");
    }
    Some(text)
}

fn target_for_tag<'a>(
    declarations: &[TargetDeclaration<'a>],
    tag: &TagKey,
) -> Option<&'a ChooseSpec> {
    let mut matching = declarations
        .iter()
        .filter(|declaration| declaration.tags.contains(&tag));
    let target = matching.next()?.target;
    matching.next().is_none().then_some(target)
}

fn tagged_target_declaration(effect: &Effect) -> Option<TargetDeclaration<'_>> {
    let mut current = effect;
    let mut tags = Vec::new();
    while let Some(tagged) = current.downcast_ref::<crate::effects::TaggedEffect>() {
        tags.push(&tagged.tag);
        current = &tagged.effect;
    }
    let target_only = current.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    (!tags.is_empty()).then_some(TargetDeclaration {
        tags,
        target: &target_only.target,
    })
}

fn forced_block_duration(duration: &Until) -> Option<&'static str> {
    match duration {
        Until::EndOfTurn => Some("this turn"),
        Until::EndOfCombat => Some("this combat"),
        _ => None,
    }
}

fn tagged_forced_block_surface(effects: &[Effect]) -> Option<ForcedBlockSurface> {
    let (triggering_tag, effects) = effects
        .first()
        .and_then(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .map(|tagged| (&tagged.tag, &effects[1..]))
        })
        .map_or((None, effects), |(tag, remaining)| (Some(tag), remaining));

    let (cant_effect, target_effects) = effects.split_last()?;
    if target_effects.is_empty() || target_effects.len() > 2 {
        return None;
    }
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
        &cant.restriction
    else {
        return None;
    };
    let duration = forced_block_duration(&cant.duration)?;

    let declarations = target_effects
        .iter()
        .map(tagged_target_declaration)
        .collect::<Option<Vec<_>>>()?;

    if is_all_creatures_filter(blockers) {
        if triggering_tag.is_some() || declarations.len() != 1 {
            return None;
        }
        let attacker_tag = exact_tagged_reference(attacker)?;
        let attacker = singular_target_text(target_for_tag(&declarations, attacker_tag)?)?;
        return Some(ForcedBlockSurface::Lure { attacker, duration });
    }

    let blocker_tag = exact_tagged_reference(blockers)?;
    let blocker = singular_target_text(target_for_tag(&declarations, blocker_tag)?)?;
    let attacker = if let Some(attacker_tag) = exact_tagged_reference(attacker) {
        if let Some(target) = target_for_tag(&declarations, attacker_tag) {
            if triggering_tag.is_some() || declarations.len() != 2 || attacker_tag == blocker_tag {
                return None;
            }
            singular_target_text(target)?
        } else {
            if declarations.len() != 1 || triggering_tag.is_some_and(|tag| tag != attacker_tag) {
                return None;
            }
            "it".to_string()
        }
    } else if exact_source_reference(attacker) {
        if triggering_tag.is_some() || declarations.len() != 1 {
            return None;
        }
        "this creature".to_string()
    } else {
        return None;
    };

    Some(ForcedBlockSurface::Specific {
        blocker,
        attacker,
        duration,
    })
}

fn render_unaffected_forced_block_side(effects: &[Effect]) -> Option<String> {
    if effects.is_empty() {
        return None;
    }
    let rendered =
        describe_effect_clause_list(effects).unwrap_or_else(|| describe_effect_list(effects));
    let rendered = rendered.trim().trim_end_matches('.');
    (!rendered.is_empty()).then(|| capitalize_first(rendered))
}

fn tagged_forced_block_window(effects: &[Effect]) -> Option<(usize, usize, ForcedBlockSurface)> {
    for start in 0..effects.len() {
        let max_end = effects.len().min(start + 4);
        for end in ((start + 2)..=max_end).rev() {
            if let Some(surface) = tagged_forced_block_surface(&effects[start..end]) {
                return Some((start, end, surface));
            }
        }
    }
    None
}

pub(super) fn describe_tagged_forced_block_effect_list(effects: &[Effect]) -> Option<String> {
    let (start, end, surface) = tagged_forced_block_window(effects)?;
    let mut parts = Vec::with_capacity(3);
    if let Some(prefix) = render_unaffected_forced_block_side(&effects[..start]) {
        parts.push(prefix);
    }
    parts.push(surface.render());
    if let Some(suffix) = render_unaffected_forced_block_side(&effects[end..]) {
        parts.push(suffix);
    }
    Some(parts.join(". "))
}

pub(crate) fn describe_may_have_tagged_forced_block(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You)) {
        return None;
    }
    tagged_forced_block_surface(&may.effects)?.render_optional()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tagged_target(tag: &TagKey, target: ChooseSpec) -> Effect {
        Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(tag.clone())
    }

    fn must_block(blockers: ObjectFilter, attacker: ObjectFilter, duration: Until) -> Effect {
        Effect::new(crate::effects::CantEffect::new(
            crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker },
            duration,
        ))
    }

    #[test]
    fn target_lure_consumes_the_target_declaration() {
        let attacker_tag = TagKey::from("targeted_attacker");
        let inner_tag = TagKey::from("targeted_0");
        let effects = vec![
            tagged_target(
                &inner_tag,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            )
            .tag(attacker_tag.clone()),
            must_block(
                ObjectFilter::creature().in_zone(Zone::Battlefield),
                ObjectFilter::tagged(attacker_tag),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(
            describe_tagged_forced_block_effect_list(&effects).as_deref(),
            Some("All creatures able to block target creature this turn do so")
        );
    }

    #[test]
    fn two_targets_render_the_specific_blocker_attacker_relation() {
        let blocker_tag = TagKey::from("targeted_blocker");
        let attacker_tag = TagKey::from("targeted_attacker");
        let effects = vec![
            tagged_target(
                &blocker_tag,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            ),
            tagged_target(
                &attacker_tag,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            ),
            must_block(
                ObjectFilter::tagged(blocker_tag),
                ObjectFilter::tagged(attacker_tag),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(
            describe_tagged_forced_block_effect_list(&effects).as_deref(),
            Some("Target creature blocks target creature this turn if able")
        );
    }

    #[test]
    fn triggering_attacker_preserves_up_to_one_and_end_of_combat() {
        let triggering_tag = TagKey::from("triggering");
        let blocker_tag = TagKey::from("targeted_blocker");
        let blocker = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::Defending),
        ))
        .with_count(ChoiceCount::up_to(1));
        let effects = vec![
            Effect::tag_triggering_object(triggering_tag.clone()),
            tagged_target(&blocker_tag, blocker),
            must_block(
                ObjectFilter::tagged(blocker_tag),
                ObjectFilter::tagged(triggering_tag),
                Until::EndOfCombat,
            ),
        ];

        assert_eq!(
            describe_tagged_forced_block_effect_list(&effects).as_deref(),
            Some(
                "Up to one target creature defending player controls blocks it this combat if able"
            )
        );
    }

    #[test]
    fn prior_attacker_reference_preserves_other_target_surface() {
        let blocker_tag = TagKey::from("targeted_blocker");
        let blocker = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().other()))
            .with_count(ChoiceCount::up_to(1));
        let effects = vec![
            tagged_target(&blocker_tag, blocker),
            must_block(
                ObjectFilter::tagged(blocker_tag),
                ObjectFilter::tagged("prior_target"),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(
            describe_tagged_forced_block_effect_list(&effects).as_deref(),
            Some("Up to one other target creature blocks it this turn if able")
        );
    }

    #[test]
    fn optional_triggering_relation_uses_causative_block_surface() {
        let blocker_tag = TagKey::from("targeted_blocker");
        let may = Effect::may(vec![
            tagged_target(
                &blocker_tag,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            ),
            must_block(
                ObjectFilter::tagged(blocker_tag),
                ObjectFilter::tagged("triggering"),
                Until::EndOfTurn,
            ),
        ]);

        assert_eq!(
            describe_effect(&may),
            "You may have target creature block it this turn if able"
        );
    }

    #[test]
    fn unmatched_extra_target_declaration_is_not_consumed() {
        let blocker_tag = TagKey::from("targeted_blocker");
        let unused_tag = TagKey::from("unused");
        let effects = vec![
            tagged_target(
                &blocker_tag,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            ),
            tagged_target(
                &unused_tag,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            ),
            must_block(
                ObjectFilter::tagged(blocker_tag),
                ObjectFilter::source(),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(describe_tagged_forced_block_effect_list(&effects), None);
    }

    #[test]
    fn forced_block_bundle_composes_after_a_prior_counter_target() {
        let attacker_tag = TagKey::from("counters_0");
        let blocker_tag = TagKey::from("targeted_blocker");
        let inner_blocker_tag = TagKey::from("targeted_1");
        let counter_target =
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
        let counter = Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            counter_target,
        ))
        .tag(attacker_tag.clone());
        let blocker = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().other()));
        let effects = vec![
            counter,
            tagged_target(&inner_blocker_tag, blocker).tag(blocker_tag.clone()),
            must_block(
                ObjectFilter::tagged(blocker_tag),
                ObjectFilter::tagged(attacker_tag),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(
            describe_tagged_forced_block_effect_list(&effects).as_deref(),
            Some(
                "Put a +1/+1 counter on target creature you control. Another target creature blocks it this turn if able"
            )
        );
    }

    #[test]
    fn forced_block_bundle_composes_after_a_prior_pump_target() {
        let attacker_tag = TagKey::from("pumped_0");
        let blocker_tag = TagKey::from("targeted_blocker");
        let pump_target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
        let pump = Effect::pump(7, 7, pump_target, Until::EndOfTurn).tag(attacker_tag.clone());
        let blocker = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().other()))
            .with_count(ChoiceCount::up_to(1));
        let effects = vec![
            pump,
            tagged_target(&blocker_tag, blocker),
            must_block(
                ObjectFilter::tagged(blocker_tag),
                ObjectFilter::tagged(attacker_tag),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(
            describe_tagged_forced_block_effect_list(&effects).as_deref(),
            Some(
                "Target creature gets +7/+7 until end of turn. Up to one other target creature blocks it this turn if able"
            )
        );
    }

    #[test]
    fn lure_bundle_composes_before_a_trailing_conditional() {
        let attacker_tag = TagKey::from("targeted_attacker");
        let conditional = Effect::new(crate::effects::ConditionalEffect::new(
            Condition::ControlCreaturesTotalPowerAtLeast(4),
            vec![Effect::draw(1)],
            Vec::new(),
        ));
        let effects = vec![
            tagged_target(
                &attacker_tag,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            ),
            must_block(
                ObjectFilter::creature(),
                ObjectFilter::tagged(attacker_tag),
                Until::EndOfTurn,
            ),
            conditional,
        ];

        let rendered = describe_tagged_forced_block_effect_list(&effects)
            .expect("lure prefix should compose with its conditional follow-up");
        assert!(
            rendered
                .starts_with("All creatures able to block target creature this turn do so. If "),
            "{rendered}"
        );
        assert!(rendered.ends_with("draw a card"), "{rendered}");
        assert!(!rendered.contains("Choose target"), "{rendered}");
    }
}
