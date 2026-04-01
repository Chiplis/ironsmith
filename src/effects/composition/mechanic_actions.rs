//! Explicit mechanic effects used by parser/rendering for supported wording.
//!
//! These mechanics are represented as first-class effects so parser output does
//! not depend on raw oracle text passthrough for rendering.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{ChoiceCount, EffectOutcome, ExecutionFact, OutcomeValue, Until, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{normalize_object_selection, resolve_value};
use crate::effects::player::CastTaggedEffect;
use crate::effects::zones::apply_zone_change;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::event_processor::EventOutcome;
use crate::events::permanents::SacrificeEvent;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::ids::{ObjectId, StableId};
use crate::object::{CounterType, ObjectKind};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct BackupEffect {
    pub amount: u32,
    pub granted_abilities: Vec<crate::ability::Ability>,
}

impl BackupEffect {
    pub fn new(amount: u32, granted_abilities: Vec<crate::ability::Ability>) -> Self {
        Self {
            amount,
            granted_abilities,
        }
    }
}

impl EffectExecutor for BackupEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target = crate::effects::helpers::resolve_single_object_from_spec(
            game,
            &ChooseSpec::target_creature(),
            ctx,
        )?;

        let mut outcomes = vec![
            crate::effects::PutCountersEffect::new(
                CounterType::PlusOnePlusOne,
                self.amount,
                ChooseSpec::SpecificObject(target),
            )
            .execute(game, ctx)?,
        ];

        if target != ctx.source {
            for ability in &self.granted_abilities {
                let granted = match &ability.kind {
                    crate::ability::AbilityKind::Static(static_ability) => static_ability.clone(),
                    _ => crate::static_abilities::StaticAbility::grant_object_ability_for_filter(
                        crate::target::ObjectFilter::source(),
                        ability.clone(),
                        ability.text.clone().unwrap_or_default(),
                    ),
                };
                outcomes.push(
                    crate::effects::ApplyContinuousEffect::new(
                        crate::continuous::EffectTarget::Specific(target),
                        crate::continuous::Modification::AddAbility(granted),
                        Until::EndOfTurn,
                    )
                    .execute(game, ctx)?,
                );
            }
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        static TARGET: std::sync::OnceLock<ChooseSpec> = std::sync::OnceLock::new();
        Some(TARGET.get_or_init(ChooseSpec::target_creature))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExploreEffect {
    pub target: ChooseSpec,
}

impl ExploreEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for ExploreEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Runtime explore behavior is handled separately; this preserves
        // parser/render semantics without oracle-text fallback.
        Ok(EffectOutcome::resolved())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAttractionEffect;

impl OpenAttractionEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for OpenAttractionEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::resolved())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDreadEffect;

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestTopCardOfLibraryEffect {
    pub player: PlayerFilter,
}

impl ManifestTopCardOfLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl ManifestDreadEffect {
    pub fn new() -> Self {
        Self
    }
}

fn manifest_card(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card_id: ObjectId,
    controller: crate::ids::PlayerId,
) -> Result<EffectOutcome, ExecutionError> {
    let Some(original_controller) = game.object(card_id).map(|obj| obj.controller) else {
        return Ok(EffectOutcome::count(0));
    };

    if let Some(card) = game.object_mut(card_id) {
        card.controller = controller;
        card.apply_face_down_cast_overlay();
    }

    let outcome = match move_to_battlefield_with_options(
        game,
        ctx,
        card_id,
        BattlefieldEntryOptions::specific(controller, false),
    ) {
        BattlefieldEntryOutcome::Moved(new_id) => {
            game.set_manifested(new_id);
            EffectOutcome::with_objects(vec![new_id]).with_event(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(KeywordActionKind::Manifest, controller, ctx.source, 1),
                ctx.provenance,
            ))
        }
        BattlefieldEntryOutcome::Prevented => {
            if let Some(card) = game.object_mut(card_id) {
                card.controller = original_controller;
                card.end_face_down_cast_overlay();
            }
            EffectOutcome::count(0)
        }
    };

    Ok(outcome)
}

impl EffectExecutor for ManifestDreadEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::resolved())
    }
}

impl EffectExecutor for ManifestTopCardOfLibraryEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let library_owner =
            crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
        let Some(&card_id) = game
            .player(library_owner)
            .and_then(|player| player.library.last())
        else {
            return Ok(EffectOutcome::count(0));
        };

        manifest_card(game, ctx, card_id, ctx.controller)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BolsterEffect {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulateEffect {
    pub count: Value,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    pub has_haste: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_end_of_combat: bool,
}

impl PopulateEffect {
    pub fn new(count: impl Into<Value>) -> Self {
        Self {
            count: count.into(),
            enters_tapped: false,
            enters_attacking: false,
            has_haste: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
        }
    }

    pub fn enters_tapped(mut self, value: bool) -> Self {
        self.enters_tapped = value;
        self
    }

    pub fn attacking(mut self, value: bool) -> Self {
        self.enters_attacking = value;
        self
    }

    pub fn haste(mut self, value: bool) -> Self {
        self.has_haste = value;
        self
    }

    pub fn sacrifice_at_next_end_step(mut self, value: bool) -> Self {
        self.sacrifice_at_next_end_step = value;
        self
    }

    pub fn exile_at_next_end_step(mut self, value: bool) -> Self {
        self.exile_at_next_end_step = value;
        self
    }

    pub fn exile_at_end_of_combat(mut self, value: bool) -> Self {
        self.exile_at_end_of_combat = value;
        self
    }

    pub fn sacrifice_at_end_of_combat(mut self, value: bool) -> Self {
        self.sacrifice_at_end_of_combat = value;
        self
    }
}

impl EffectExecutor for PopulateEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let mut created_ids = Vec::new();
        let mut events = Vec::new();

        for _ in 0..count {
            let candidates = game
                .battlefield
                .iter()
                .copied()
                .filter(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == ctx.controller
                            && obj.kind == ObjectKind::Token
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .collect::<Vec<_>>();

            if candidates.is_empty() {
                events.push(TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Populate,
                        ctx.controller,
                        ctx.source,
                        1,
                    ),
                    ctx.provenance,
                ));
                continue;
            }

            let chosen = if candidates.len() == 1 {
                candidates[0]
            } else {
                let spec = ChooseObjectsSpec::new(
                    ctx.source,
                    "Choose a creature token you control to populate",
                    candidates.clone(),
                    1,
                    Some(1),
                );
                let selection: Vec<ObjectId> = make_decision(
                    game,
                    ctx.decision_maker,
                    ctx.controller,
                    Some(ctx.source),
                    spec,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::with_objects(created_ids).with_events(events));
                }
                let normalized = normalize_object_selection(selection, &candidates, 1);
                normalized.first().copied().unwrap_or(candidates[0])
            };

            let outcome =
                crate::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(chosen))
                    .enters_tapped(self.enters_tapped)
                    .attacking(self.enters_attacking)
                    .haste(self.has_haste)
                    .sacrifice_at_next_end_step(self.sacrifice_at_next_end_step)
                    .exile_at_next_end_step(self.exile_at_next_end_step)
                    .exile_at_eoc(self.exile_at_end_of_combat)
                    .execute(game, ctx)?;
            if let OutcomeValue::Objects(ids) = outcome.value {
                created_ids.extend(ids);
            }
            events.extend(outcome.events);
            events.push(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(KeywordActionKind::Populate, ctx.controller, ctx.source, 1),
                ctx.provenance,
            ));
        }

        Ok(EffectOutcome::with_objects(created_ids).with_events(events))
    }
}

impl BolsterEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl EffectExecutor for BolsterEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == ctx.controller
                        && game.object_has_card_type(id, crate::types::CardType::Creature)
                })
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let least_toughness = candidates
            .iter()
            .filter_map(|&id| {
                game.calculated_toughness(id)
                    .or_else(|| game.object(id).and_then(|obj| obj.toughness()))
            })
            .min()
            .unwrap_or(0);
        candidates.retain(|&id| {
            game.calculated_toughness(id)
                .or_else(|| game.object(id).and_then(|obj| obj.toughness()))
                == Some(least_toughness)
        });
        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen = if candidates.len() == 1 {
            candidates[0]
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose a creature with the least toughness you control for bolster",
                candidates.clone(),
                1,
                Some(1),
            );
            let selection: Vec<ObjectId> = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let normalized = normalize_object_selection(selection, &candidates, 1);
            normalized.first().copied().unwrap_or(candidates[0])
        };

        let outcome = crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            self.amount,
            ChooseSpec::SpecificObject(chosen),
        )
        .execute(game, ctx)?;

        Ok(outcome.with_event(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Bolster, ctx.controller, ctx.source, 1),
            ctx.provenance,
        )))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CipherEffect;

impl CipherEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for CipherEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source_obj) = game.object(ctx.source).cloned() else {
            return Ok(EffectOutcome::target_invalid());
        };
        if source_obj.zone != Zone::Stack || source_obj.card.is_none() {
            return Ok(EffectOutcome::resolved());
        }

        let candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == ctx.controller
                        && game.object_has_card_type(id, crate::types::CardType::Creature)
                })
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let choice_ctx = crate::decisions::context::BooleanContext::new(
            ctx.controller,
            Some(ctx.source),
            format!(
                "Exile {} encoded on a creature you control?",
                source_obj.name
            ),
        );
        if !ctx.decision_maker.decide_boolean(game, &choice_ctx) {
            return Ok(EffectOutcome::declined());
        }

        let chosen_creature = if candidates.len() == 1 {
            candidates[0]
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose a creature you control to encode",
                candidates.clone(),
                1,
                Some(1),
            );
            let selection: Vec<ObjectId> = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            let normalized = normalize_object_selection(selection, &candidates, 1);
            let Some(chosen) = normalized.first().copied() else {
                return Ok(EffectOutcome::declined());
            };
            chosen
        };

        let exiled_id = match apply_zone_change(
            game,
            ctx.source,
            source_obj.zone,
            Zone::Exile,
            ctx.cause.clone(),
            &mut *ctx.decision_maker,
        ) {
            EventOutcome::Proceed(result) => {
                let Some(new_id) = result.new_object_id else {
                    return Ok(EffectOutcome::resolved());
                };
                if result.final_zone != Zone::Exile {
                    return Ok(EffectOutcome::resolved());
                }
                new_id
            }
            EventOutcome::Prevented => return Ok(EffectOutcome::prevented()),
            EventOutcome::Replaced => return Ok(EffectOutcome::replaced()),
            EventOutcome::NotApplicable => return Ok(EffectOutcome::target_invalid()),
        };

        let Some(exiled_stable_id) = game.object(exiled_id).map(|obj| obj.stable_id) else {
            return Ok(EffectOutcome::target_invalid());
        };

        game.imprint_card(chosen_creature, exiled_id);
        let trigger_text = "Whenever this creature deals combat damage to a player, its controller may cast a copy of the encoded card without paying its mana cost.";
        let ability = crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_deals_combat_damage_to_player(),
            vec![crate::effect::Effect::cast_encoded_card_copy(
                exiled_stable_id,
            )],
        )
        .with_text(trigger_text);
        if let Some(creature) = game.object_mut(chosen_creature) {
            creature.abilities.push(ability);
        }

        Ok(
            EffectOutcome::with_objects(vec![exiled_id, chosen_creature])
                .with_execution_fact(ExecutionFact::ChosenObjects(vec![chosen_creature]))
                .with_execution_fact(ExecutionFact::AffectedObjects(vec![exiled_id])),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastEncodedCardCopyEffect {
    pub encoded_card: StableId,
}

impl CastEncodedCardCopyEffect {
    pub fn new(encoded_card: StableId) -> Self {
        Self { encoded_card }
    }
}

impl EffectExecutor for CastEncodedCardCopyEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(encoded_id) = game.find_object_by_stable_id(self.encoded_card) else {
            return Ok(EffectOutcome::target_invalid());
        };
        let Some(encoded_obj) = game.object(encoded_id).cloned() else {
            return Ok(EffectOutcome::target_invalid());
        };
        if encoded_obj.zone != Zone::Exile {
            return Ok(EffectOutcome::target_invalid());
        }

        let choice_ctx = crate::decisions::context::BooleanContext::new(
            ctx.controller,
            Some(ctx.source),
            format!(
                "Cast a copy of {} without paying its mana cost?",
                encoded_obj.name
            ),
        );
        if !ctx.decision_maker.decide_boolean(game, &choice_ctx) {
            return Ok(EffectOutcome::declined());
        }

        let snapshot = ObjectSnapshot::from_object(&encoded_obj, game);
        let prior = ctx.clear_object_tag("cipher_encoded");
        ctx.set_tagged_objects("cipher_encoded", vec![snapshot]);
        let result = CastTaggedEffect::new("cipher_encoded")
            .as_copy()
            .without_paying_mana_cost()
            .execute(game, ctx);
        if let Some(previous) = prior {
            ctx.set_tagged_objects("cipher_encoded", previous);
        } else {
            ctx.clear_object_tag("cipher_encoded");
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DevourEffect {
    pub multiplier: u32,
}

impl DevourEffect {
    pub fn new(multiplier: u32) -> Self {
        Self { multiplier }
    }
}

impl EffectExecutor for DevourEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if !game
            .object(ctx.source)
            .is_some_and(|obj| obj.zone == Zone::Battlefield)
        {
            return Ok(EffectOutcome::resolved());
        }

        let candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| id != ctx.source)
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == ctx.controller
                        && game.object_has_card_type(id, crate::types::CardType::Creature)
                        && game.can_be_sacrificed(id)
                })
            })
            .collect::<Vec<_>>();

        let chosen = if candidates.is_empty() {
            Vec::new()
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose any number of other creatures you control to sacrifice for devour",
                candidates.clone(),
                0,
                Some(candidates.len()),
            );
            let selection: Vec<ObjectId> = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            selection
                .into_iter()
                .filter(|id| candidates.contains(id))
                .fold(Vec::new(), |mut chosen, id| {
                    if !chosen.contains(&id) {
                        chosen.push(id);
                    }
                    chosen
                })
        };

        let mut sacrificed_count: i32 = 0;
        let mut sacrifice_events = Vec::new();
        for id in chosen {
            let pre_snapshot = game
                .object(id)
                .map(|obj| ObjectSnapshot::from_object(obj, game));
            let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);

            match apply_zone_change(
                game,
                id,
                Zone::Battlefield,
                Zone::Graveyard,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) {
                EventOutcome::Prevented | EventOutcome::NotApplicable => {}
                EventOutcome::Proceed(result) => {
                    sacrificed_count += 1;
                    if result.final_zone == Zone::Graveyard {
                        sacrifice_events.push(TriggerEvent::new_with_provenance(
                            SacrificeEvent::new(id, Some(ctx.source))
                                .with_snapshot(pre_snapshot, sacrificing_player),
                            ctx.provenance,
                        ));
                    }
                }
                EventOutcome::Replaced => {
                    sacrificed_count += 1;
                }
            }
        }

        if sacrificed_count == 0 {
            return Ok(EffectOutcome::count(0).with_events(sacrifice_events));
        }

        let mut counters = crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            sacrificed_count.saturating_mul(self.multiplier as i32),
            ChooseSpec::Source,
        )
        .execute(game, ctx)?;
        counters.events.extend(sacrifice_events);
        Ok(counters)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SupportEffect {
    pub amount: u32,
    pub target: ChooseSpec,
}

impl SupportEffect {
    pub fn new(amount: u32) -> Self {
        Self {
            amount,
            target: ChooseSpec::target(ChooseSpec::Object(
                crate::target::ObjectFilter::creature().other(),
            )),
        }
    }
}

impl EffectExecutor for SupportEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut outcome = crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            1,
            self.target.clone(),
        )
        .with_target_count(ChoiceCount::up_to(self.amount as usize))
        .execute(game, ctx)?;
        outcome.events.push(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(
                KeywordActionKind::Support,
                ctx.controller,
                ctx.source,
                self.amount,
            ),
            ctx.provenance,
        ));
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        Some(ChoiceCount::up_to(self.amount as usize))
    }

    fn target_description(&self) -> &'static str {
        "target creature to support"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptEffect {
    pub amount: u32,
}

impl AdaptEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl EffectExecutor for AdaptEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::resolved())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterAbilityEffect;

impl CounterAbilityEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for CounterAbilityEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardDefinitionBuilder;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::events::{EventKind, KeywordActionEvent};
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::static_abilities::StaticAbilityId;
    use crate::types::{CardType, Subtype};
    use std::collections::VecDeque;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        controller: PlayerId,
        card_id: u32,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_creature_token(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        power: i32,
        toughness: i32,
        subtype: Subtype,
    ) -> ObjectId {
        let token = CardDefinitionBuilder::new(CardId::new(), name)
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let source = game.new_object_id();
        crate::effects::CreateTokenEffect::one(token)
            .execute(game, &mut ExecutionContext::new_default(source, controller))
            .expect("token creation should succeed")
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("token creation should produce one token")
    }

    fn create_library_card(
        game: &mut GameState,
        owner: PlayerId,
        card_id: u32,
        name: &str,
        card_types: Vec<CardType>,
        mana_cost: Option<crate::mana::ManaCost>,
        power: Option<i32>,
        toughness: Option<i32>,
    ) -> ObjectId {
        let mut builder = CardBuilder::new(CardId::from_raw(card_id), name).card_types(card_types);
        if let Some(cost) = mana_cost {
            builder = builder.mana_cost(cost);
        }
        if let (Some(power), Some(toughness)) = (power, toughness) {
            builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
        }
        let card = builder.build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    struct SelectIdsDecisionMaker {
        choices: VecDeque<Vec<ObjectId>>,
    }

    impl DecisionMaker for SelectIdsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.choices
                .pop_front()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }
    }

    struct PromptingDecisionMaker;

    impl DecisionMaker for PromptingDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            Vec::new()
        }

        fn awaiting_choice(&self) -> bool {
            true
        }
    }

    #[test]
    fn populate_copies_the_chosen_creature_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _soldier = create_creature_token(&mut game, alice, "Soldier", 1, 1, Subtype::Soldier);
        let rhino = create_creature_token(&mut game, alice, "Rhino", 4, 4, Subtype::Rhino);

        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![rhino]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = PopulateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("populate should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        assert_eq!(ids.len(), 1);
        let copy = game.object(ids[0]).expect("created token should exist");
        assert_eq!(copy.kind, ObjectKind::Token);
        assert_eq!(copy.name, "Rhino");
        assert_eq!(game.calculated_power(ids[0]), Some(4));
        assert_eq!(game.calculated_toughness(ids[0]), Some(4));
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Populate);
    }

    #[test]
    fn manifest_top_card_of_your_library_enters_face_down_under_your_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _card = create_library_card(
            &mut game,
            alice,
            200,
            "Manifest Test Creature",
            vec![CardType::Creature],
            Some(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Green,
            ])),
            Some(3),
            Some(3),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ManifestTopCardOfLibraryEffect::new(PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("manifest should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("manifest should return the manifested object id");
        };
        let manifested_id = *ids.first().expect("manifest should create one permanent");
        let manifested = game
            .object(manifested_id)
            .expect("manifested permanent should exist");

        assert_eq!(manifested.controller, alice);
        assert!(game.is_face_down(manifested_id));
        assert!(game.is_manifested(manifested_id));
        assert_eq!(game.calculated_power(manifested_id), Some(2));
        assert_eq!(game.calculated_toughness(manifested_id), Some(2));
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Manifest);
    }

    #[test]
    fn manifest_top_card_of_that_players_library_uses_effect_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let _card = create_library_card(
            &mut game,
            bob,
            201,
            "Stolen Manifest Card",
            vec![CardType::Creature],
            Some(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Blue,
            ])),
            Some(4),
            Some(4),
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::executor::ResolvedTarget::Player(bob)]);

        let outcome =
            ManifestTopCardOfLibraryEffect::new(PlayerFilter::TargetPlayerOrControllerOfTarget)
                .execute(&mut game, &mut ctx)
                .expect("manifest from that player's library should execute");

        let manifested_id = outcome
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("manifest should create one permanent");
        let manifested = game
            .object(manifested_id)
            .expect("manifested permanent should exist");
        assert_eq!(manifested.owner, bob);
        assert_eq!(manifested.controller, alice);
        assert!(game.is_face_down(manifested_id));
    }

    #[test]
    fn populate_multiple_times_reprompts_and_emits_per_iteration_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let soldier = create_creature_token(&mut game, alice, "Soldier", 1, 1, Subtype::Soldier);
        let rhino = create_creature_token(&mut game, alice, "Rhino", 4, 4, Subtype::Rhino);

        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![soldier], vec![rhino]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = PopulateEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("populate twice should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        assert_eq!(ids.len(), 2);
        let created_names = ids
            .iter()
            .filter_map(|id| game.object(*id))
            .map(|obj| obj.name.clone())
            .collect::<Vec<_>>();
        assert!(created_names.contains(&"Soldier".to_string()));
        assert!(created_names.contains(&"Rhino".to_string()));
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| event.kind() == EventKind::KeywordAction)
                .count(),
            2
        );
    }

    #[test]
    fn populate_with_no_creature_tokens_creates_nothing_but_still_performs_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = PopulateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("populate with no tokens should resolve");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        assert!(ids.is_empty());
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].kind(), EventKind::KeywordAction);
    }

    #[test]
    fn populate_applies_collapsed_token_copy_modifiers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, alice, 30, "Populate Source", 2, 2);
        let rhino = create_creature_token(&mut game, alice, "Rhino", 4, 4, Subtype::Rhino);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![rhino]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = PopulateEffect::new(1)
            .enters_tapped(true)
            .attacking(true)
            .haste(true)
            .sacrifice_at_next_end_step(true)
            .execute(&mut game, &mut ctx)
            .expect("populate with modifiers should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        let token_id = *ids.first().expect("populate should create one token");
        assert!(
            game.is_tapped(token_id),
            "populated token should enter tapped"
        );
        assert!(
            game.object_has_static_ability_id(token_id, StaticAbilityId::Haste),
            "populated token should gain haste"
        );
        let combat = game.combat.as_ref().expect("combat should still be active");
        let token_attacker = combat
            .attackers
            .iter()
            .find(|info| info.creature == token_id)
            .expect("populated token should enter attacking");
        assert_eq!(token_attacker.target, AttackTarget::Player(bob));
        assert_eq!(game.delayed_triggers.len(), 1);
        assert_eq!(game.delayed_triggers[0].target_objects, vec![token_id]);
    }

    #[test]
    fn support_exposes_up_to_n_other_target_creatures() {
        let effect = SupportEffect::new(3);
        let target = effect
            .get_target_spec()
            .expect("support should expose target metadata");

        assert!(target.is_target(), "support should target creatures");
        assert_eq!(effect.get_target_count(), Some(ChoiceCount::up_to(3)));

        let ChooseSpec::Target(inner) = target else {
            panic!("support should use a targeted ChooseSpec");
        };
        let ChooseSpec::Object(filter) = inner.as_ref() else {
            panic!("support target should resolve to an object filter");
        };
        assert!(
            filter.other,
            "support on permanents must use other creatures"
        );
        assert!(
            filter.card_types.contains(&CardType::Creature),
            "support should only target creatures"
        );
    }

    #[test]
    fn support_puts_one_counter_on_each_chosen_creature_and_emits_keyword_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 40, "Support Source", 2, 2);
        let first = create_creature(&mut game, alice, 41, "First Ally", 2, 2);
        let second = create_creature(&mut game, alice, 42, "Second Ally", 2, 2);
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            crate::executor::ResolvedTarget::Object(first),
            crate::executor::ResolvedTarget::Object(second),
        ]);

        let outcome = SupportEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("support should execute");

        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action payload");
        assert_eq!(keyword.action, KeywordActionKind::Support);
        assert_eq!(keyword.amount, 2);
    }

    #[test]
    fn support_on_spell_source_can_target_fewer_than_n_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_creature(&mut game, alice, 43, "Spell Support Target", 2, 2);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(target)]);

        SupportEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("support from a spell source should execute");

        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 1);
    }

    #[test]
    fn support_with_zero_targets_still_resolves_and_emits_keyword_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = SupportEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("support with zero chosen targets should resolve");

        assert_eq!(outcome.events.len(), 1);
        let keyword = outcome.events[0]
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action payload");
        assert_eq!(keyword.action, KeywordActionKind::Support);
        assert_eq!(keyword.amount, 2);
    }

    #[test]
    fn bolster_chooses_among_least_toughness_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 1, "First", 1, 1);
        let second = create_creature(&mut game, alice, 2, "Second", 1, 1);
        let _largest = create_creature(&mut game, alice, 3, "Largest", 4, 4);
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![second]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = BolsterEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("execute bolster");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 2);
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Bolster);
    }

    #[test]
    fn bolster_does_nothing_without_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = BolsterEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("bolster without creatures should resolve");

        assert!(!outcome.something_happened());
        assert!(
            outcome.events.is_empty(),
            "bolster should not emit events when no creature can be chosen"
        );
    }

    #[test]
    fn bolster_pauses_for_tied_creature_choice_instead_of_defaulting() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 1, "First", 1, 1);
        let second = create_creature(&mut game, alice, 2, "Second", 1, 1);
        let mut dm = PromptingDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = BolsterEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("bolster should wait for a choice");

        assert!(ctx.decision_maker.awaiting_choice());
        assert!(!outcome.something_happened());
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 0);
        assert!(
            outcome.events.is_empty(),
            "no bolster event should fire before a choice is made"
        );
    }

    #[test]
    fn devour_sacrifices_exactly_the_chosen_creatures_and_emits_sacrifice_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 10, "Devourer", 2, 2);
        let first = create_creature(&mut game, alice, 11, "First Food", 1, 1);
        let second = create_creature(&mut game, alice, 12, "Second Food", 1, 1);
        let keep = create_creature(&mut game, alice, 13, "Keep", 3, 3);
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![second]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = DevourEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("execute devour");

        assert!(game.battlefield.contains(&source));
        assert!(game.battlefield.contains(&first));
        assert!(!game.battlefield.contains(&second));
        assert!(game.battlefield.contains(&keep));
        assert_eq!(game.players[0].graveyard.len(), 1);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
        assert!(
            outcome
                .events_of_type::<crate::events::permanents::SacrificeEvent>()
                .count()
                == 1,
            "expected devour to emit one sacrifice event"
        );
    }

    #[test]
    fn backup_puts_counter_on_target_and_grants_following_ability_to_another_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 20, "Backup Source", 2, 2);
        let target = create_creature(&mut game, alice, 21, "Backup Target", 1, 1);
        let granted = Ability::static_ability(StaticAbility::flying()).with_text("Flying");
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(target)]);

        let outcome = BackupEffect::new(1, vec![granted])
            .execute(&mut game, &mut ctx)
            .expect("execute backup");

        assert!(outcome.something_happened());
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 1);
        assert!(
            game.object_has_static_ability_id(target, StaticAbilityId::Flying),
            "backup target should gain the granted ability until end of turn"
        );
    }
}
