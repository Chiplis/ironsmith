use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct PutStickerEffect {
    pub target: ChooseSpec,
    pub action: KeywordActionKind,
    /// "an ability sticker with ticket cost X or less".
    pub max_ticket_cost: Option<crate::effect::Value>,
    /// "without paying that sticker's ticket cost".
    pub without_paying_ticket_cost: bool,
}

impl PutStickerEffect {
    pub fn new(target: ChooseSpec, action: KeywordActionKind) -> Self {
        Self {
            target,
            action,
            max_ticket_cost: None,
            without_paying_ticket_cost: false,
        }
    }

    pub fn with_ticket_terms(
        mut self,
        max_ticket_cost: Option<crate::effect::Value>,
        without_paying_ticket_cost: bool,
    ) -> Self {
        self.max_ticket_cost = max_ticket_cost;
        self.without_paying_ticket_cost = without_paying_ticket_cost;
        self
    }
}

/// Minimal sticker-sheet model: choose one of the player's accessible ability
/// stickers whose ticket cost fits the cap, pay its ticket cost in ticket
/// counters unless waived, and put it on the object. `None` means no sheet
/// sticker was placed (the caller falls back to an untyped sticker marker,
/// or fails when the sheets have stickers but none can be afforded).
fn place_sheet_ability_sticker(
    effect: &PutStickerEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object_id: crate::ids::ObjectId,
) -> Result<Option<crate::effect::EffectOutcome>, ExecutionError> {
    use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_option};
    let sheets = game.available_ability_stickers(ctx.controller);
    if sheets.is_empty() {
        // CR 123.3: with no accessible ability sticker, none can be placed.
        return Ok(Some(EffectOutcome::impossible()));
    }
    let cap = match &effect.max_ticket_cost {
        // Pin Collection's enters trigger reads the X paid to cast it (the
        // trigger inherits the permanent's cast X). Put onto the battlefield
        // without being cast, X is 0 (CR 107.3m) rather than unresolvable.
        Some(crate::effect::Value::X) if ctx.x_value.is_none() => Some(0),
        Some(value) => Some(crate::effects::helpers::resolve_nonnegative_u32(game, value, ctx)?),
        None => None,
    };
    let ticket = crate::object::CounterType::Named("ticket".into());
    let tickets = game
        .player(ctx.controller)
        .map_or(0, |player| player.counter_count(ticket.clone()));
    let options = sheets
        .iter()
        .filter(|entry| cap.is_none_or(|cap| entry.ticket_cost <= cap))
        .filter(|entry| effect.without_paying_ticket_cost || entry.ticket_cost <= tickets)
        .map(|entry| (entry.id, format!("{} ({} tickets)", entry.text, entry.ticket_cost)))
        .collect::<Vec<_>>();
    if options.is_empty() {
        return Ok(Some(EffectOutcome::impossible()));
    }
    let Some(sticker_id) = select_typed_option(
        game,
        ctx,
        ctx.controller,
        "Choose an ability sticker",
        &options,
        SingleChoicePolicy::First,
    )?
    else {
        return Ok(Some(EffectOutcome::impossible()));
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(Some(EffectOutcome::count(0)));
    }
    let cost = sheets
        .iter()
        .find(|entry| entry.id == sticker_id)
        .map_or(0, |entry| entry.ticket_cost);
    if !effect.without_paying_ticket_cost && cost > 0 {
        if !pay_ticket_cost(game, ctx, cost)? { return Ok(Some(EffectOutcome::impossible())); }
        if ctx.decision_maker.awaiting_choice() { return Ok(Some(EffectOutcome::count(0))); }
    }
    if game.apply_ability_sticker(object_id, sticker_id).is_none() {
        return Ok(Some(EffectOutcome::impossible()));
    }
    Ok(Some(EffectOutcome::with_objects(vec![object_id])))
}

/// A selected, affordable sticker starts one counter-payment instruction.
/// CR 118.11: its nominal quantity acknowledges payment even when a
/// replacement changes the number removed or replaces removal altogether.
fn pay_ticket_cost(game: &mut GameState, ctx: &mut ExecutionContext, cost: u32) -> Result<bool, ExecutionError> {
    let amount = i32::try_from(cost).map_err(|_| ExecutionError::UnresolvableValue("sticker ticket cost exceeds supported range".into()))?;
    let payment = crate::effects::RemoveCountersEffect::new(
        crate::object::CounterType::Named("ticket".into()), amount,
        ChooseSpec::SpecificPlayer(ctx.controller),
    ).execute_child(game, ctx)?;
    if ctx.decision_maker.awaiting_choice() { return Ok(true); }
    Ok(payment.status != crate::effect::OutcomeStatus::Impossible
        && payment.requested_amount() == Some(u64::from(cost)))
}

fn place_sheet_pt_sticker(effect: &PutStickerEffect, game: &mut GameState,
    ctx: &mut ExecutionContext, object_id: crate::ids::ObjectId) -> Result<EffectOutcome, ExecutionError> {
    use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_option};
    let sheets = game.available_pt_stickers(ctx.controller);
    let tickets = game.player(ctx.controller).map_or(0, |player|
        player.counter_count(crate::object::CounterType::Named("ticket".into())));
    let cap = effect.max_ticket_cost.as_ref().map(|value|
        crate::effects::helpers::resolve_nonnegative_u32(game, value, ctx)).transpose()?;
    let options = sheets.iter().filter(|entry| cap.is_none_or(|cap| entry.1 <= cap))
        .filter(|entry| effect.without_paying_ticket_cost || entry.1 <= tickets)
        .map(|entry| (entry.0, format!("{}/{} ({} tickets)", entry.2, entry.3, entry.1))).collect::<Vec<_>>();
    let Some(id) = select_typed_option(game, ctx, ctx.controller, "Choose a power and toughness sticker", &options, SingleChoicePolicy::First)?
        else { return Ok(EffectOutcome::impossible()); };
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    let cost = sheets.iter().find(|entry| entry.0 == id).map_or(0, |entry| entry.1);
    if !effect.without_paying_ticket_cost && cost > 0 {
        if !pay_ticket_cost(game, ctx, cost)? { return Ok(EffectOutcome::impossible()); }
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    }
    Ok(if game.apply_pt_sticker(object_id, id).is_some() { EffectOutcome::with_objects(vec![object_id]) }
        else { EffectOutcome::impossible() })
}

impl EffectExecutor for PutStickerEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            if self.action == KeywordActionKind::Sticker {
                use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_option};
                let mut options = Vec::new();
                let tickets = game.player(ctx.controller).map_or(0, |player|
                    player.counter_count(crate::object::CounterType::Named("ticket".into())));
                if !game.available_art_stickers(ctx.controller).is_empty() {
                    options.push((KeywordActionKind::ArtSticker, "Art sticker".to_string()));
                }
                if !game.available_name_stickers(ctx.controller).is_empty() {
                    options.push((KeywordActionKind::NameSticker, "Name sticker".to_string()));
                }
                if game.available_ability_stickers(ctx.controller).iter().any(|entry| self.without_paying_ticket_cost || entry.ticket_cost <= tickets) {
                    options.push((KeywordActionKind::AbilitySticker, "Ability sticker".to_string()));
                }
                if game.available_pt_stickers(ctx.controller).iter().any(|entry| self.without_paying_ticket_cost || entry.1 <= tickets) {
                    options.push((KeywordActionKind::PowerToughnessSticker, "Power and toughness sticker".to_string()));
                }
                let Some(action) = select_typed_option(game, ctx, ctx.controller, "Choose a sticker kind", &options, SingleChoicePolicy::First)?
                    else { return Ok(EffectOutcome::impossible()); };
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                let mut concrete = self.clone(); concrete.action = action;
                return concrete.execute_child(game, ctx);
            }
            let object_id = match resolve_single_object_from_spec(game, &self.target, ctx) {
                Ok(id) => id,
                Err(ExecutionError::InvalidTarget) if self.target.is_target() => {
                    return Ok(EffectOutcome::target_invalid());
                }
                Err(ExecutionError::InvalidTarget)
                    if self.action == KeywordActionKind::NameSticker =>
                {
                    return Ok(EffectOutcome::impossible());
                }
                Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::count(0)),
                Err(err) => return Err(err),
            };

            // CR 123.3b: a player can't put a sticker on an object they don't own.
            if game.object(object_id).is_some_and(|object| object.owner != ctx.controller) {
                return Ok(EffectOutcome::impossible());
            }
            let snapshot = game
                .object(object_id)
                .map(|object| ObjectSnapshot::from_object(object, game));
            let name_fact = if self.action == KeywordActionKind::NameSticker {
                use crate::effects::composition::scoped_choice::{
                    SingleChoicePolicy, select_typed_option,
                };
                let Some(object) = game.object(object_id) else {
                    return Ok(EffectOutcome::impossible());
                };
                if object.owner != ctx.controller || !object.zone.is_public() {
                    return Ok(EffectOutcome::impossible());
                }
                let controller = game.current_controller(object_id).unwrap_or(object.owner);
                let available = game.available_name_stickers(ctx.controller);
                if available.is_empty() {
                    return Ok(EffectOutcome::impossible());
                }
                let selected = select_typed_option(
                    game,
                    ctx,
                    ctx.controller,
                    "Choose a name sticker",
                    &available,
                    SingleChoicePolicy::First,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let Some(sticker_id) = selected else {
                    return Ok(EffectOutcome::impossible());
                };
                let name = game.current_name(object_id).unwrap_or_default();
                let word_count = name
                    .split_whitespace()
                    .filter(|word| !word.chars().all(|ch| ch == '_'))
                    .count();
                let positions = (0..=word_count)
                    .map(|i| {
                        (
                            i,
                            if i == 0 {
                                "At the beginning".to_string()
                            } else {
                                format!("After word {i}")
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                let selected = select_typed_option(
                    game,
                    ctx,
                    controller,
                    "Choose the name sticker's position",
                    &positions,
                    SingleChoicePolicy::First,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let Some(position) = selected else {
                    return Ok(EffectOutcome::impossible());
                };
                let Some(name) = game.apply_available_name_sticker(
                    ctx.controller,
                    object_id,
                    sticker_id,
                    position,
                ) else {
                    return Ok(EffectOutcome::impossible());
                };
                Some(crate::effect::ExecutionFact::AppliedNameSticker {
                    sticker_id: sticker_id,
                    name,
                })
            } else if self.action == KeywordActionKind::ArtSticker {
                use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_option};
                let options = game.available_art_stickers(ctx.controller).into_iter()
                    .map(|id| (id, format!("Art sticker {id}"))).collect::<Vec<_>>();
                let Some(id) = select_typed_option(game, ctx, ctx.controller, "Choose an art sticker", &options, SingleChoicePolicy::First)?
                    else { return Ok(EffectOutcome::impossible()); };
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                if game.apply_art_sticker(object_id, id).is_none() { return Ok(EffectOutcome::impossible()); }
                None
            } else if self.action == KeywordActionKind::PowerToughnessSticker {
                let placed = place_sheet_pt_sticker(self, game, ctx, object_id)?;
                if placed.objects().is_none() || ctx.decision_maker.awaiting_choice() { return Ok(placed); }
                None
            } else if self.action == KeywordActionKind::AbilitySticker
                && let Some(placed) = place_sheet_ability_sticker(self, game, ctx, object_id)?
            {
                if placed.objects().is_none() || ctx.decision_maker.awaiting_choice() {
                    return Ok(placed);
                }
                None
            } else {
                game.put_sticker_on_object(object_id, self.action);
                None
            };
            let mut outcome = EffectOutcome::with_objects(vec![object_id]);
            if let Some(fact) = name_fact {
                outcome = outcome.with_execution_fact(fact);
            }
            crate::effects::composition::complete_keyword_action_with_result(
                game,
                ctx,
                outcome,
                KeywordActionEvent::new(self.action, ctx.controller, ctx.source, 1)
                    .with_snapshot(snapshot),
            )
        })
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectExecutor;
    use crate::effects::ResolvedTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    #[test]
    fn put_sticker_effect_emits_sticker_action_for_chosen_object() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        game.add_accessible_art_sticker(alice);
        let source_id = game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(1), "Sticker Source")
                .card_types(vec![crate::types::CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let target_id = game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(2), "Sticker Target")
                .card_types(vec![crate::types::CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets.push(ResolvedTarget::Object(target_id));

        let effect = PutStickerEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact())),
            KeywordActionKind::Sticker,
        );
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("put sticker should resolve");

        assert_eq!(
            outcome.value.objects().expect("sticker target outcome"),
            &[target_id]
        );
        let action = outcome.events.iter()
            .find_map(|event| event.downcast::<crate::events::KeywordActionEvent>())
            .expect("placing the available art sticker emits its keyword action");
        assert_eq!(action.action, KeywordActionKind::ArtSticker);
        assert_eq!(action.player, alice);
        assert_eq!(action.amount, 1);
    }
}
