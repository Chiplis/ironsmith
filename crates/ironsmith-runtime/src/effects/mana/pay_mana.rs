//! Pay mana effect implementation.

use crate::decision::DecisionMaker;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::special_actions::{SpecialAction, can_perform, perform};
use crate::target::ChooseSpec;

/// Effect that asks a player to pay a mana cost.
///
/// Returns `Count(1)` when paid, `Impossible` when the player can't pay.
#[derive(Debug, Clone, PartialEq)]
pub struct PayManaEffect {
    /// Mana cost to pay.
    pub cost: ManaCost,
    /// Which player pays it.
    pub player: ChooseSpec,
}

impl PayManaEffect {
    /// Create a new pay-mana effect.
    pub fn new(cost: ManaCost, player: ChooseSpec) -> Self {
        Self { cost, player }
    }

    fn try_pay_interactively(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        player_id: PlayerId,
    ) -> bool {
        const MAX_PAYMENT_STEPS: usize = 32;

        for _ in 0..MAX_PAYMENT_STEPS {
            let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
                player_id,
                Some(ctx.source),
                &self.cost,
                crate::costs::PaymentReason::Effect,
            );
            let can_pay_now = game.can_pay_mana_cost_with_reason(
                player_id,
                Some(ctx.source),
                &adjusted_cost,
                0,
                crate::costs::PaymentReason::Effect,
            );
            let mana_abilities =
                get_available_mana_abilities(game, player_id, &mut ctx.decision_maker);

            if !can_pay_now && mana_abilities.is_empty() {
                return false;
            }

            let mut choices = Vec::new();
            let mut options = Vec::new();

            if can_pay_now {
                choices.push(PayManaChoice::PayNow);
                options.push(SelectableOption::new(choices.len() - 1, "Pay mana cost"));
            }

            for (permanent_id, ability_index, description) in mana_abilities {
                choices.push(PayManaChoice::ActivateManaAbility {
                    permanent_id,
                    ability_index,
                });
                options.push(SelectableOption::new(
                    choices.len() - 1,
                    format!(
                        "Tap {}: {}",
                        describe_permanent(game, permanent_id),
                        description
                    ),
                ));
            }

            if choices.is_empty() {
                return false;
            }

            let source_name = game
                .object(ctx.source)
                .map(|obj| obj.name.clone())
                .unwrap_or_else(|| "effect".to_string());
            let decision_ctx =
                SelectOptionsContext::mana_payment(player_id, ctx.source, source_name, options);
            let selected = ctx.decision_maker.decide_options(game, &decision_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return false;
            }
            let Some(selected_idx) = selected.first().copied() else {
                return false;
            };
            let Some(choice) = choices.get(selected_idx).copied() else {
                return false;
            };

            match choice {
                PayManaChoice::PayNow => {
                    return game.try_pay_mana_cost_with_reason(
                        player_id,
                        Some(ctx.source),
                        &adjusted_cost,
                        0,
                        crate::costs::PaymentReason::Effect,
                    );
                }
                PayManaChoice::ActivateManaAbility {
                    permanent_id,
                    ability_index,
                } => {
                    let action = SpecialAction::ActivateManaAbility {
                        permanent_id,
                        ability_index,
                    };

                    if perform(action, game, player_id, &mut ctx.decision_maker).is_err() {
                        return false;
                    }
                }
            }
        }

        let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
            player_id,
            Some(ctx.source),
            &self.cost,
            crate::costs::PaymentReason::Effect,
        );
        game.try_pay_mana_cost_with_reason(
            player_id,
            Some(ctx.source),
            &adjusted_cost,
            0,
            crate::costs::PaymentReason::Effect,
        )
    }
}

impl EffectExecutor for PayManaEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        if self.try_pay_interactively(game, ctx, player_id) {
            Ok(EffectOutcome::count(1))
        } else {
            Ok(EffectOutcome::impossible())
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.player.is_target() {
            Some(&self.player)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "player to pay mana"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PayManaChoice {
    PayNow,
    ActivateManaAbility {
        permanent_id: ObjectId,
        ability_index: usize,
    },
}

fn get_available_mana_abilities(
    game: &GameState,
    player: PlayerId,
    decision_maker: &mut &mut dyn DecisionMaker,
) -> Vec<(ObjectId, usize, String)> {
    let mut abilities = Vec::new();

    for &permanent_id in &game.battlefield {
        let Some(permanent) = game.object(permanent_id) else {
            continue;
        };

        if permanent.controller != player {
            continue;
        }

        for (ability_index, ability) in permanent.abilities.iter().enumerate() {
            if !ability.is_mana_ability() {
                continue;
            }

            let action = SpecialAction::ActivateManaAbility {
                permanent_id,
                ability_index,
            };
            if can_perform(&action, game, player, decision_maker).is_err() {
                continue;
            }

            abilities.push((
                permanent_id,
                ability_index,
                describe_mana_ability(&ability.kind),
            ));
        }
    }

    abilities
}

fn describe_mana_ability(kind: &crate::ability::AbilityKind) -> String {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    if let AbilityKind::Activated(mana_ability) = kind
        && mana_ability.is_mana_ability()
    {
        let produced: Vec<&str> = mana_ability
            .mana_symbols()
            .iter()
            .map(|symbol| match symbol {
                ManaSymbol::White => "{W}",
                ManaSymbol::Blue => "{U}",
                ManaSymbol::Black => "{B}",
                ManaSymbol::Red => "{R}",
                ManaSymbol::Green => "{G}",
                ManaSymbol::Colorless => "{C}",
                _ => "mana",
            })
            .collect();
        if produced.is_empty() {
            "Add mana".to_string()
        } else {
            format!("Add {}", produced.join(""))
        }
    } else {
        "Add mana".to_string()
    }
}

fn describe_permanent(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|obj| obj.name.clone())
        .unwrap_or_else(|| "Unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::static_abilities::StaticAbility;
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_payment_replacement_permanent(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        ability: StaticAbility,
    ) {
        let source = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source, controller, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("static-ability source should exist")
            .abilities
            .push(Ability::static_ability(ability));
    }

    #[derive(Default)]
    struct ActivateThenPayDecisionMaker {
        mana_payment_prompts: usize,
    }

    impl DecisionMaker for ActivateThenPayDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description.starts_with("Pay mana for") {
                self.mana_payment_prompts += 1;

                // First prompt: activate a mana ability if available.
                if self.mana_payment_prompts == 1
                    && let Some(activation) = ctx
                        .options
                        .iter()
                        .find(|opt| opt.legal && opt.description != "Pay mana cost")
                {
                    return vec![activation.index];
                }

                if let Some(pay) = ctx
                    .options
                    .iter()
                    .find(|opt| opt.legal && opt.description == "Pay mana cost")
                {
                    return vec![pay.index];
                }
            }

            ctx.options
                .iter()
                .filter(|opt| opt.legal)
                .map(|opt| opt.index)
                .take(ctx.min)
                .collect()
        }
    }

    #[test]
    fn pay_mana_effect_activates_mana_ability_then_pays() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let mountain_def = crate::cards::definitions::basic_mountain();
        let mountain_id =
            game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);

        let mut dm = ActivateThenPayDecisionMaker::default();
        let mut ctx =
            ExecutionContext::new_default(mountain_id, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Red]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(dm.mana_payment_prompts, 2);
        assert!(game.is_tapped(mountain_id));
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            0
        );
    }

    #[test]
    fn pay_mana_effect_is_impossible_without_mana_sources() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Red]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Impossible);
    }

    #[test]
    fn pay_mana_effect_can_use_krrik_life_for_black() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Krrik Effect Helper",
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        );

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }

    #[test]
    fn pay_mana_effect_still_can_use_krrik_under_yasharn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Krrik Effect Helper",
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        );
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Yasharn Effect Helper",
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
        );

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }
}
