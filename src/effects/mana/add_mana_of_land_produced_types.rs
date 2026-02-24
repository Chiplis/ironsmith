//! Add mana of any color/type that lands matching a filter could produce.

use super::choice_helpers::{choose_mana_symbols, credit_mana_symbols};
use crate::ability::{AbilityKind, ActivatedAbility};
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

        let chosen_symbols = choose_mana_symbols(
            game,
            ctx,
            player_id,
            amount,
            self.same_type,
            &available,
            available[0],
        );

        credit_mana_symbols(game, player_id, chosen_symbols);

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
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_mana_ability() {
                continue;
            }
            if !mana_ability_condition_met(game, perm, mana_ability) {
                continue;
            }

            for symbol in mana_ability.mana_symbols() {
                push_symbol_if_addable(&mut symbols, *symbol);
            }
            for effect in &mana_ability.effects {
                infer_symbols_from_mana_effect(
                    game,
                    perm.id,
                    perm.controller,
                    effect,
                    &mut symbols,
                );
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
    mana_ability: &ActivatedAbility,
) -> bool {
    mana_ability
        .activation_condition
        .as_ref()
        .is_none_or(|condition| {
            let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
                controller: source.controller,
                source: source.id,
                filter_source: Some(source.id),
                triggering_event: None,
                trigger_identity: None,
                ability_index: None,
                options: crate::condition_eval::ExternalEvaluationOptions {
                    // For mana-production inference we only care about what colors can be
                    // produced, not whether the ability is currently activatable by timing/limits.
                    ignore_timing: true,
                    ignore_activation_limits: true,
                },
            };
            crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
        })
}

fn infer_symbols_from_mana_effect(
    game: &GameState,
    source: crate::ids::ObjectId,
    land_controller: crate::ids::PlayerId,
    effect: &crate::effect::Effect,
    out: &mut Vec<ManaSymbol>,
) {
    if let Some(inferred) = effect.producible_mana_symbols(game, source, land_controller) {
        for symbol in inferred {
            push_symbol_if_addable(out, symbol);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cost::TotalCost;
    use crate::effect::EffectResult;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::object::Object;
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

    fn create_land_with_mana_effects(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        effects: Vec<crate::effect::Effect>,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build();
        let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
        if let Some(obj) = game.object_mut(id) {
            obj.abilities
                .push(Ability::mana_with_effects(TotalCost::free(), effects));
        }
        id
    }

    fn setup_commander(game: &mut GameState, player: PlayerId, colors: Vec<ManaSymbol>) {
        let commander_card = CardBuilder::new(CardId::new(), "Commander")
            .mana_cost(ManaCost::from_pips(
                colors.into_iter().map(|symbol| vec![symbol]).collect(),
            ))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();

        let id = game.new_object_id();
        let commander = Object::from_card(id, &commander_card, player, Zone::Command);
        game.add_object(commander);

        if let Some(state) = game.player_mut(player) {
            state.add_commander(id);
        }
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

    #[test]
    fn infers_symbols_from_effect_based_any_color_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_land_with_mana_effects(
            &mut game,
            alice,
            "Prismatic Test Land",
            vec![crate::effect::Effect::new(
                crate::effects::AddManaOfAnyColorEffect::you(1),
            )],
        );

        let effect = AddManaOfLandProducedTypesEffect::new(
            1,
            PlayerFilter::You,
            ObjectFilter::land().you_control(),
            false,
            false,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(result.result, EffectResult::Count(1));
        let pool = &game.player(alice).expect("alice exists").mana_pool;
        assert_eq!(
            pool.white, 1,
            "default choice should use first inferred symbol"
        );
        assert_eq!(pool.total(), 1);
    }

    #[test]
    fn infers_symbols_from_effect_based_commander_identity_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        setup_commander(&mut game, alice, vec![ManaSymbol::Black, ManaSymbol::Red]);
        let source = create_land_with_mana_effects(
            &mut game,
            alice,
            "Commander Test Land",
            vec![crate::effect::Effect::new(
                crate::effects::AddManaFromCommanderColorIdentityEffect::you(1),
            )],
        );

        let effect = AddManaOfLandProducedTypesEffect::new(
            1,
            PlayerFilter::You,
            ObjectFilter::land().you_control(),
            false,
            false,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(result.result, EffectResult::Count(1));
        let pool = &game.player(alice).expect("alice exists").mana_pool;
        assert_eq!(
            pool.black, 1,
            "default choice should use first inferred symbol"
        );
        assert_eq!(pool.total(), 1);
    }
}
