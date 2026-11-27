//! Bind the defining clause of a face-down instruction to the physical action.
//! This also handles catalogs that encoded the clause as a separate instruction.
use crate::continuous::Modification;
use crate::effect::{Effect, Until, Value};
use crate::effects::{
    ApplyContinuousEffect, MayEffect, MoveToZoneEffect, TaggedEffect, TurnFaceDownEffect,
};
use crate::filter::TaggedOpbjectRelation;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;
use ironsmith_core::effect::FaceDownForm;
use std::collections::HashMap;

#[derive(Clone)]
struct Definition {
    form: FaceDownForm,
    grants: Vec<Modification>,
}

fn definition(effect: &ApplyContinuousEffect) -> Option<(&TagKey, Definition)> {
    if effect.until != Until::Forever
        || effect.condition.is_some()
        || !effect.runtime_modifications.is_empty()
    {
        return None;
    }
    let tag = match effect.target_spec.as_ref()?.base() {
        ChooseSpec::Tagged(tag) => tag,
        ChooseSpec::Object(filter) => {
            let [constraint] = filter.tagged_constraints.as_slice() else {
                return None;
            };
            if constraint.relation != TaggedOpbjectRelation::IsTaggedObject
                || *filter != crate::filter::ObjectFilter::tagged(constraint.tag.clone())
            {
                return None;
            }
            &constraint.tag
        }
        _ => return None,
    };
    if !tag.as_str().starts_with("turned_face_down_") && !tag.as_str().starts_with("returned_") {
        return None;
    }
    // This shape declares the face-down object's types. Independent grants
    // and later modifications to a returned object keep their normal layers.
    if !effect
        .modification
        .iter()
        .chain(&effect.additional_modifications)
        .any(|m| {
            matches!(
                m,
                Modification::SetCardTypes(_) | Modification::AddCardTypes(_)
            )
        })
    {
        return None;
    }
    let mut grants = Vec::new();
    let mut types = vec![CardType::Creature];
    let mut subtypes: Vec<crate::types::Subtype> = Vec::new();
    let mut pt = (2, 2);
    for modification in effect
        .modification
        .iter()
        .chain(&effect.additional_modifications)
    {
        match modification {
            Modification::SetCardTypes(set) => types.clone_from(set),
            Modification::AddCardTypes(add) => {
                for ty in add {
                    if !types.contains(ty) {
                        types.push(*ty);
                    }
                }
            }
            Modification::RemoveAllSubtypesOfFamily(family) => {
                subtypes.retain(|ty| !ty.belongs_to_family(*family))
            }
            Modification::AddAbility(_) | Modification::AddAbilityGeneric(_) => {
                grants.push(modification.clone())
            }
            Modification::AddSubtypes(add) => subtypes.extend_from_slice(add),
            Modification::SetPowerToughness {
                power: Value::Fixed(p),
                toughness: Value::Fixed(t),
                ..
            } => pt = (*p, *t),
            _ => return None,
        }
    }
    Some((
        tag,
        Definition {
            form: FaceDownForm {
                power_toughness: types.contains(&CardType::Creature).then_some(pt),
                card_types: types,
                subtypes,
            },
            grants,
        },
    ))
}

// The compiler may wrap the producing operation in an optional instruction,
// a sequence, or an iteration while its defining clause follows outside it.
fn rewrite(effect: &mut Effect, change: &mut impl FnMut(&Effect) -> Option<Effect>) -> bool {
    if let Some(updated) = change(effect) {
        *effect = updated;
        return true;
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        let mut child = (*tagged.effect).clone();
        if rewrite(&mut child, change) {
            *effect = Effect::new(tagged.with_effect(child));
            return true;
        }
        return false;
    }
    macro_rules! in_body {
        ($kind:ty) => {
            if let Some(container) = effect.downcast_ref::<$kind>() {
                let mut body = container.clone();
                let mut changed = false;
                for child in &mut body.effects {
                    changed |= rewrite(child, change);
                }
                if changed {
                    *effect = Effect::new(body);
                }
                return changed;
            }
        };
    }
    in_body!(MayEffect);
    in_body!(crate::effects::SequenceEffect);
    in_body!(crate::effects::ForPlayersEffect);
    false
}

pub(crate) fn bind<'a>(effects: impl DoubleEndedIterator<Item = &'a mut Effect>) {
    // Only programs with an actual defining clause need to touch the producer.
    let mut defining_clauses = Vec::new();
    let mut definitions = HashMap::new();
    let mut used = std::collections::HashSet::new();
    for effect in effects.rev() {
        let inner = effect
            .downcast_ref::<TaggedEffect>()
            .map_or(&*effect, |t| t.effect.as_ref());
        if let Some((tag, form)) = inner
            .downcast_ref::<ApplyContinuousEffect>()
            .and_then(definition)
        {
            definitions.insert(tag.clone(), form);
            defining_clauses.push(effect);
        } else if !definitions.is_empty() {
            rewrite(effect, &mut |effect| {
                let tagged = effect.downcast_ref::<TaggedEffect>()?;
                let def = definitions.get(&tagged.tag)?;
                let inner = if let Some(turn) = tagged.effect.downcast_ref::<TurnFaceDownEffect>() {
                    if !def.grants.is_empty() {
                        return None;
                    }
                    let mut turn = turn.clone();
                    turn.face_down_form = Some(def.form.clone());
                    Effect::new(turn)
                } else if let Some(movement) = tagged.effect.downcast_ref::<MoveToZoneEffect>() {
                    if !movement.enters_face_down || movement.zone != Zone::Battlefield {
                        return None;
                    }
                    let mut movement = movement.clone();
                    movement.face_down_form = Some(def.form.clone());
                    movement.face_down_ability_grants.clone_from(&def.grants);
                    Effect::new(movement)
                } else {
                    let mut group = tagged
                        .effect
                        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?
                        .clone();
                    if !group.face_down || !def.grants.is_empty() {
                        return None;
                    }
                    group.face_down_form = Some(def.form.clone());
                    Effect::new(group)
                };
                let mut result = tagged.with_effect(inner);
                result.outcome_only = true;
                used.insert(tagged.tag.clone());
                Some(Effect::new(result))
            });
        }
    }
    if !used.is_empty() {
        for effect in defining_clauses {
            rewrite(effect, &mut |effect| {
                let apply = effect.downcast_ref::<ApplyContinuousEffect>()?;
                let (tag, _) = definition(apply)?;
                if !used.contains(tag) {
                    return None;
                }
                let mut apply = apply.clone();
                apply.defines_face_down_form = true;
                Some(Effect::new(apply))
            });
        }
    }
}
