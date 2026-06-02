//! Goad effect implementation.

use crate::effect::{EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::TaggedOpbjectRelation;
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that goads creature(s).
#[derive(Debug, Clone, PartialEq)]
pub struct GoadEffect {
    /// Creature target specification.
    pub target: ChooseSpec,
}

impl GoadEffect {
    /// Create a new goad effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for GoadEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        restore_source_chosen_name_tags(game, ctx, &self.target);
        let objects = resolve_objects_for_effect(game, ctx, &self.target)?;
        let mut count = 0_i32;
        for object_id in objects {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Battlefield || !game.current_is_creature(object_id) {
                continue;
            }
            game.add_goad_effect(object_id, ctx.controller, Until::YourNextTurn, ctx.source);
            count += 1;
        }
        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to goad"
    }
}

fn restore_source_chosen_name_tags(
    game: &GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
) {
    let Some(chosen_names) = game
        .chosen_named_option(ctx.source)
        .map(crate::effects::player::split_chosen_card_names)
    else {
        return;
    };
    if chosen_names.is_empty() {
        return;
    }
    let Some(filter) = choose_spec_object_filter(spec) else {
        return;
    };
    let missing_chosen_name_tags = filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == TaggedOpbjectRelation::SameNameAsTagged
                && constraint.tag.as_str() == "__chosen_name__"
                && ctx.get_tagged_all(&constraint.tag).is_none()
        })
        .map(|constraint| constraint.tag.clone())
        .collect::<Vec<TagKey>>();

    if missing_chosen_name_tags.is_empty() {
        return;
    }

    let snapshots = chosen_names
        .into_iter()
        .map(|name| {
            crate::effects::player::synthetic_chosen_name_snapshot(ctx.source, ctx.controller, name)
        })
        .collect::<Vec<_>>();
    for tag in missing_chosen_name_tags {
        ctx.set_tagged_objects(tag, snapshots.clone());
    }
}

fn choose_spec_object_filter(spec: &ChooseSpec) -> Option<&crate::target::ObjectFilter> {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Some(filter),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::AutoPassDecisionMaker;
    use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::ObjectFilter;
    use crate::types::CardType;

    fn battlefield_creature(
        id: u32,
        name: &str,
        owner: PlayerId,
        power: i32,
        toughness: i32,
    ) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        Object::from_card(
            ObjectId::from_raw(u64::from(id)),
            &card,
            owner,
            Zone::Battlefield,
        )
    }

    #[test]
    fn goad_all_with_same_name_as_tagged_name_choice() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(100);
        let memnite = battlefield_creature(1, "Memnite", bob, 1, 1);
        let vanguard = battlefield_creature(2, "Elite Vanguard", bob, 2, 1);
        game.add_object(memnite);
        game.add_object(vanguard);

        let mut filter = ObjectFilter::creature();
        filter.zone = Some(Zone::Battlefield);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("__chosen_name__"),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
        let effect = GoadEffect::new(ChooseSpec::All(filter));

        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(
            "__chosen_name__",
            vec![ObjectSnapshot {
                object_id: source,
                stable_id: crate::ids::StableId::from(source),
                kind: crate::object::ObjectKind::Card,
                card: None,
                controller: alice,
                owner: alice,
                name: "Memnite".to_string(),
                mana_cost: None,
                colors: crate::color::ColorSet::default(),
                supertypes: Vec::new(),
                card_types: Vec::new(),
                subtypes: Vec::new(),
                compiled_card_text: String::new(),
                other_face: None,
                other_face_name: None,
                linked_face_layout: crate::card::LinkedFaceLayout::None,
                power: None,
                toughness: None,
                base_power: None,
                base_toughness: None,
                loyalty: None,
                defense: None,
                abilities: Vec::new(),
                aura_attach_filter: None,
                x_value: None,
                cast_order_this_turn: None,
                counters: std::collections::HashMap::new(),
                is_token: false,
                tapped: false,
                attacking: false,
                flipped: false,
                face_down: false,
                transform_count: 0,
                attached_to: None,
                attachments: Vec::new(),
                was_enchanted: false,
                is_monstrous: false,
                is_commander: false,
                zone: Zone::Command,
            }],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("goad should resolve against matching chosen names");

        assert!(game.is_goaded(ObjectId::from_raw(1)));
        assert!(!game.is_goaded(ObjectId::from_raw(2)));
    }
}
