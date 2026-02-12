//! Add mana of any color/type that lands matching a filter could produce.

use crate::ability::{AbilityKind, ManaAbility, ManaAbilityCondition};
use crate::color::Color;
use crate::decisions::ask_choose_one;
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::object::Object;
use crate::target::{ObjectFilter, PlayerFilter};

/// Effect that adds mana constrained to what matching lands could produce.
///
/// This models text like:
/// - "Add one mana of any color that a land an opponent controls could produce."
/// - "Add one mana of any type that a Gate you control could produce."
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfLandProducedTypesEffect {
    /// Number of mana to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
    /// Lands to inspect for producible mana.
    pub land_filter: ObjectFilter,
    /// Whether colorless mana is allowed ("any type" vs "any color").
    pub allow_colorless: bool,
    /// If true, all mana must be the same type.
    pub same_type: bool,
}

impl AddManaOfLandProducedTypesEffect {
    pub fn new(
        amount: impl Into<Value>,
        player: PlayerFilter,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            land_filter,
            allow_colorless,
            same_type,
        }
    }
}

impl EffectExecutor for AddManaOfLandProducedTypesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        if amount == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let available = collect_available_mana_symbols(game, ctx, &self.land_filter);
        let available = available
            .into_iter()
            .filter(|symbol| is_allowed_symbol(*symbol, self.allow_colorless))
            .collect::<Vec<_>>();
        if available.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let choices = available
            .iter()
            .map(|symbol| (mana_symbol_oracle(*symbol), *symbol))
            .collect::<Vec<_>>();

        let mut chosen_symbols = Vec::new();
        if self.same_type {
            let chosen = ask_choose_one(
                game,
                &mut ctx.decision_maker,
                player_id,
                ctx.source,
                &choices,
            );
            chosen_symbols.resize(amount as usize, chosen);
        } else {
            for _ in 0..amount {
                let chosen = ask_choose_one(
                    game,
                    &mut ctx.decision_maker,
                    player_id,
                    ctx.source,
                    &choices,
                );
                chosen_symbols.push(chosen);
            }
        }

        if let Some(player) = game.player_mut(player_id) {
            for symbol in chosen_symbols {
                player.mana_pool.add(symbol, 1);
            }
        }

        Ok(EffectOutcome::count(amount as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

fn collect_available_mana_symbols(
    game: &GameState,
    ctx: &ExecutionContext,
    land_filter: &ObjectFilter,
) -> Vec<ManaSymbol> {
    let mut symbols = Vec::new();
    let filter_ctx = ctx.filter_context(game);
    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        if !perm.is_land() || !land_filter.matches(perm, &filter_ctx, game) {
            continue;
        }

        for ability in &perm.abilities {
            let AbilityKind::Mana(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability_condition_met(game, perm, mana_ability) {
                continue;
            }

            for symbol in &mana_ability.mana {
                push_symbol_if_addable(&mut symbols, *symbol);
            }
            if let Some(effects) = &mana_ability.effects {
                for effect in effects {
                    infer_symbols_from_mana_effect(game, perm.controller, effect, &mut symbols);
                }
            }
        }
    }

    symbols.sort_by_key(|symbol| canonical_symbol_order(*symbol));
    symbols.dedup();
    symbols
}

fn mana_ability_condition_met(
    game: &GameState,
    source: &Object,
    mana_ability: &ManaAbility,
) -> bool {
    fn condition_met(
        game: &GameState,
        source: &Object,
        condition: &ManaAbilityCondition,
    ) -> bool {
        match condition {
            ManaAbilityCondition::ControlLandWithSubtype(required_subtypes) => {
                game.battlefield.iter().any(|&perm_id| {
                    let Some(perm) = game.object(perm_id) else {
                        return false;
                    };
                    perm.controller == source.controller
                        && perm.is_land()
                        && required_subtypes.iter().any(|st| perm.has_subtype(*st))
                })
            }
            // For mana-production inference we only care about what colors can be
            // produced, not whether the ability is currently activatable by timing.
            ManaAbilityCondition::Timing(_) => true,
            ManaAbilityCondition::All(conditions) => conditions
                .iter()
                .all(|inner| condition_met(game, source, inner)),
        }
    }

    mana_ability
        .activation_condition
        .as_ref()
        .is_none_or(|condition| condition_met(game, source, condition))
}

fn infer_symbols_from_mana_effect(
    game: &GameState,
    land_controller: crate::ids::PlayerId,
    effect: &crate::effect::Effect,
    out: &mut Vec<ManaSymbol>,
) {
    if let Some(add_mana) = effect.downcast_ref::<crate::effects::AddManaEffect>() {
        for symbol in &add_mana.mana {
            push_symbol_if_addable(out, *symbol);
        }
        return;
    }
    if let Some(add_scaled) = effect.downcast_ref::<crate::effects::AddScaledManaEffect>() {
        for symbol in &add_scaled.mana {
            push_symbol_if_addable(out, *symbol);
        }
        return;
    }
    if effect
        .downcast_ref::<crate::effects::AddColorlessManaEffect>()
        .is_some()
    {
        push_symbol_if_addable(out, ManaSymbol::Colorless);
        return;
    }
    if effect
        .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
        .is_some()
        || effect
            .downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>()
            .is_some()
        || effect
            .downcast_ref::<crate::effects::mana::AddManaOfImprintedColorsEffect>()
            .is_some()
    {
        push_all_colored_symbols(out);
        return;
    }
    if effect
        .downcast_ref::<crate::effects::AddManaFromCommanderColorIdentityEffect>()
        .is_some()
    {
        let identity = game.get_commander_color_identity(land_controller);
        if identity.is_empty() {
            push_symbol_if_addable(out, ManaSymbol::Colorless);
        } else {
            if identity.contains(Color::White) {
                push_symbol_if_addable(out, ManaSymbol::White);
            }
            if identity.contains(Color::Blue) {
                push_symbol_if_addable(out, ManaSymbol::Blue);
            }
            if identity.contains(Color::Black) {
                push_symbol_if_addable(out, ManaSymbol::Black);
            }
            if identity.contains(Color::Red) {
                push_symbol_if_addable(out, ManaSymbol::Red);
            }
            if identity.contains(Color::Green) {
                push_symbol_if_addable(out, ManaSymbol::Green);
            }
        }
    }
}

fn push_all_colored_symbols(out: &mut Vec<ManaSymbol>) {
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ] {
        push_symbol_if_addable(out, symbol);
    }
}

fn push_symbol_if_addable(out: &mut Vec<ManaSymbol>, symbol: ManaSymbol) {
    if matches!(
        symbol,
        ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless
    ) {
        out.push(symbol);
    }
}

fn is_allowed_symbol(symbol: ManaSymbol, allow_colorless: bool) -> bool {
    match symbol {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green => true,
        ManaSymbol::Colorless => allow_colorless,
        _ => false,
    }
}

fn canonical_symbol_order(symbol: ManaSymbol) -> usize {
    match symbol {
        ManaSymbol::White => 0,
        ManaSymbol::Blue => 1,
        ManaSymbol::Black => 2,
        ManaSymbol::Red => 3,
        ManaSymbol::Green => 4,
        ManaSymbol::Colorless => 5,
        _ => 100,
    }
}

fn mana_symbol_oracle(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "{W}".to_string(),
        ManaSymbol::Blue => "{U}".to_string(),
        ManaSymbol::Black => "{B}".to_string(),
        ManaSymbol::Red => "{R}".to_string(),
        ManaSymbol::Green => "{G}".to_string(),
        ManaSymbol::Colorless => "{C}".to_string(),
        _ => "{?}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::effect::EffectResult;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_land_with_mana(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        mana: Vec<ManaSymbol>,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build();
        let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
        if let Some(obj) = game.object_mut(id) {
            obj.abilities
                .push(Ability::mana(crate::cost::TotalCost::free(), mana));
        }
        id
    }

    #[test]
    fn adds_only_colors_matching_lands_could_produce() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        create_land_with_mana(&mut game, bob, "Mountain", vec![ManaSymbol::Red]);
        let source = create_land_with_mana(&mut game, alice, "Source", vec![ManaSymbol::Colorless]);

        let effect = AddManaOfLandProducedTypesEffect::new(
            1,
            PlayerFilter::You,
            ObjectFilter::land().opponent_controls(),
            false,
            false,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(result.result, EffectResult::Count(1));
        let pool = &game.player(alice).expect("alice exists").mana_pool;
        assert_eq!(pool.red, 1);
        assert_eq!(pool.total(), 1);
    }

    #[test]
    fn returns_zero_when_no_matching_lands_can_produce_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_land_with_mana(&mut game, alice, "Source", vec![ManaSymbol::Colorless]);

        let effect = AddManaOfLandProducedTypesEffect::new(
            1,
            PlayerFilter::You,
            ObjectFilter::land().opponent_controls(),
            false,
            false,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0,
            "no matching lands should produce no mana"
        );
    }

    #[test]
    fn any_type_clause_allows_colorless() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_land_with_mana(&mut game, alice, "Source", vec![ManaSymbol::Colorless]);
        create_land_with_mana(&mut game, alice, "Wastes", vec![ManaSymbol::Colorless]);

        let effect = AddManaOfLandProducedTypesEffect::new(
            1,
            PlayerFilter::You,
            ObjectFilter::land().you_control(),
            true,
            false,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(result.result, EffectResult::Count(1));
        let pool = &game.player(alice).expect("alice exists").mana_pool;
        assert_eq!(pool.colorless, 1);
    }
}
