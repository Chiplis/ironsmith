use crate::cards::CardRegistry;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, StableId};
use crate::object::ObjectKind;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardNameEffect {
    pub chooser: PlayerFilter,
    pub filter: Option<ObjectFilter>,
    pub tag: TagKey,
}

impl ChooseCardNameEffect {
    pub fn new(
        chooser: PlayerFilter,
        filter: Option<ObjectFilter>,
        tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            chooser,
            filter,
            tag: tag.into(),
        }
    }

    fn choice_options(filter: Option<&ObjectFilter>) -> Vec<String> {
        let mut registry = CardRegistry::with_builtin_cards();
        registry.ensure_all_generated_cards_loaded();
        let mut names = CardRegistry::supported_card_names();
        if let Some(filter) = filter && !filter.card_types.is_empty() {
            names.retain(|name| {
                registry.get(name).is_some_and(|definition| {
                    filter
                        .card_types
                        .iter()
                        .all(|card_type| definition.card.card_types.contains(card_type))
                })
            });
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    fn synthetic_snapshot(
        source: ObjectId,
        chooser: crate::ids::PlayerId,
        name: String,
    ) -> ObjectSnapshot {
        ObjectSnapshot {
            object_id: source,
            stable_id: StableId::from(source),
            kind: ObjectKind::Card,
            card: None,
            controller: chooser,
            owner: chooser,
            name,
            mana_cost: None,
            colors: crate::color::ColorSet::default(),
            supertypes: Vec::new(),
            card_types: Vec::new(),
            subtypes: Vec::new(),
            oracle_text: String::new(),
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
            max_saga_chapter: None,
            x_value: None,
            cast_order_this_turn: None,
            counters: std::collections::HashMap::new(),
            is_token: false,
            tapped: false,
            flipped: false,
            face_down: false,
            attached_to: None,
            attachments: Vec::new(),
            was_enchanted: false,
            is_monstrous: false,
            is_commander: false,
            zone: Zone::Command,
        }
    }
}

impl EffectExecutor for ChooseCardNameEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let names = Self::choice_options(self.filter.as_ref());
        if names.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let options: Vec<SelectableOption> = names
            .iter()
            .enumerate()
            .map(|(idx, name)| SelectableOption::new(idx, name.clone()))
            .collect();
        let choice_ctx = SelectOptionsContext::new(
            chooser,
            Some(ctx.source),
            "Choose a card name",
            options,
            1,
            1,
        );
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(chosen_idx) = selected.into_iter().next().filter(|idx| *idx < names.len()) else {
            return Ok(EffectOutcome::count(0));
        };

        let snapshot = Self::synthetic_snapshot(ctx.source, chooser, names[chosen_idx].clone());
        ctx.set_tagged_objects(self.tag.clone(), vec![snapshot]);
        Ok(EffectOutcome::count(1))
    }
}
