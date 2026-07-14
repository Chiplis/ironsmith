use crate::cards::CardRegistry;
use crate::decisions::context::TextInputContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, StableId};
use crate::object::ObjectKind;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;
pub use ironsmith_core::ChooseCardNameEffect;

pub(crate) fn synthetic_chosen_name_snapshot(
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
        first_printed_set_name: None,
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
        abilities: std::sync::Arc::new(Vec::new()),
        aura_attach_filter: None,
        copiable_values: crate::snapshot::CopiableValues::default(),
        x_value: None,
        cast_order_this_turn: None,
        mana_spent_to_cast: crate::player::ManaPool::default(),
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
    }
}

pub(crate) fn split_chosen_card_names(names: &str) -> Vec<String> {
    names
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
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
        let choice_ctx = TextInputContext::new(chooser, Some(ctx.source), "Choose a card name")
            .with_placeholder("Enter a card name")
            .require_known_value(true);
        let chosen_name = ctx.decision_maker.decide_text(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let chosen_name = chosen_name.trim();
        if chosen_name.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded([chosen_name]);
        let canonical_name = registry
            .get(chosen_name)
            .map(|definition| definition.name().to_string())
            .unwrap_or_else(|| chosen_name.to_string());

        let mut chosen_names = game
            .chosen_named_option(ctx.source)
            .map(split_chosen_card_names)
            .unwrap_or_default();
        if !chosen_names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&canonical_name))
        {
            chosen_names.push(canonical_name);
        }
        game.set_chosen_named_option(ctx.source, chosen_names.join("\n"));
        let snapshots = chosen_names
            .into_iter()
            .map(|name| synthetic_chosen_name_snapshot(ctx.source, chooser, name))
            .collect();
        ctx.set_tagged_objects(self.tag.clone(), snapshots);
        Ok(EffectOutcome::count(1))
    }
}
