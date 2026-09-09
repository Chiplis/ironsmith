//! Mana payment cost implementation.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::mana::ManaCost;

/// Resolve Phyrexian alternatives without spending resources until every pip
/// has a legal choice. Each candidate is checked against the whole remaining
/// cost, so choosing life early cannot strand a later mandatory payment.
pub(crate) fn pay_mana_cost_with_choices(
    game: &mut GameState,
    payer: crate::ids::PlayerId,
    source: Option<crate::ids::ObjectId>,
    cost: &ManaCost,
    x_value: u32,
    reason: crate::costs::PaymentReason,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Result<(), CostPaymentError> {
    use crate::mana::ManaSymbol;
    if game.player(payer).is_none() {
        return Err(CostPaymentError::PlayerNotFound);
    }
    if !game.can_pay_mana_cost_with_reason(payer, source, cost, x_value, reason) {
        return Err(CostPaymentError::InsufficientMana);
    }
    let policy = game.mana_spend_policy(payer, source);
    let allow_black_life = game.player_can_pay_black_with_life_for_reason(payer, source, reason);
    // Expand optional black-mana substitutions once. After a player chooses
    // mana for a pip, the solver must not substitute life for that choice.
    let mut pips = GameState::expanded_payment_pips(cost, x_value, allow_black_life);
    for index in 0..pips.len() {
        if pips[index].len() < 2
            || !pips[index]
                .iter()
                .any(|symbol| matches!(symbol, ManaSymbol::Life(_)))
        {
            continue;
        }
        let feasible = pips[index]
            .iter()
            .copied()
            .filter(|symbol| {
                let mut candidate = pips.clone();
                candidate[index] = vec![*symbol];
                game.can_pay_mana_cost_with_payment_options(
                    payer,
                    source,
                    &ManaCost::from_pips(candidate),
                    x_value,
                    reason,
                    &policy,
                    true,
                    false,
                    false,
                )
            })
            .collect::<Vec<_>>();
        let chosen = match feasible.as_slice() {
            [] => return Err(CostPaymentError::InsufficientMana),
            [only] => *only,
            _ => {
                let options = feasible
                    .iter()
                    .enumerate()
                    .map(|(index, symbol)| {
                        let label = match symbol {
                            ManaSymbol::Life(amount) => format!("Pay {amount} life"),
                            symbol => format!(
                                "Pay {}",
                                format_mana_cost(&ManaCost::from_pips(vec![vec![*symbol]]))
                            ),
                        };
                        crate::decisions::context::SelectableOption::new(index, label)
                    })
                    .collect();
                let context = crate::decisions::context::SelectOptionsContext::new(
                    payer,
                    source,
                    "Choose mana or life payment",
                    options,
                    1,
                    1,
                );
                let selected = decision_maker.decide_options(game, &context);
                if decision_maker.awaiting_choice() {
                    return Err(CostPaymentError::InsufficientMana);
                }
                let [selected] = selected.as_slice() else {
                    return Err(CostPaymentError::Other(
                        "Invalid mana or life payment choice".into(),
                    ));
                };
                *feasible.get(*selected).ok_or_else(|| {
                    CostPaymentError::Other("Invalid mana or life payment choice".into())
                })?
            }
        };
        pips[index] = vec![chosen];
    }
    if game.try_pay_mana_cost_with_payment_options(
        payer,
        source,
        &ManaCost::from_pips(pips),
        x_value,
        reason,
        &policy,
        true,
        false,
        false,
    ) {
        Ok(())
    } else {
        Err(CostPaymentError::InsufficientMana)
    }
}

/// A mana payment cost (e.g., {2}{W}{W}).
///
/// This wraps the existing ManaCost type and provides CostPayer implementation.
/// Mana payment typically happens through the mana payment phase in the game loop,
/// so the `pay` method here defers to the mana pool's `try_pay` method.
#[derive(Debug, Clone, PartialEq)]
pub struct ManaPaymentCost {
    /// The mana cost to pay.
    pub cost: ManaCost,
}

impl ManaPaymentCost {
    /// Create a new mana payment cost.
    pub fn new(cost: ManaCost) -> Self {
        Self { cost }
    }

    /// Get the wrapped mana cost.
    pub fn mana_cost(&self) -> &ManaCost {
        &self.cost
    }
}

impl CostPayer for ManaPaymentCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let x_value = ctx.x_value.unwrap_or(0);
        if game.player(ctx.payer).is_none() {
            return Err(CostPaymentError::PlayerNotFound);
        }
        if !game.can_pay_mana_cost_with_reason(
            ctx.payer,
            Some(ctx.source),
            &self.cost,
            x_value,
            ctx.reason,
        ) {
            return Err(CostPaymentError::InsufficientMana);
        }

        Ok(())
    }

    fn can_potentially_pay(
        &self,
        game: &GameState,
        ctx: &CostContext,
    ) -> Result<(), CostPaymentError> {
        let x_value = ctx.x_value.unwrap_or(0);

        let view = crate::derived_view::DerivedGameView::new(game);
        if !view.can_potentially_pay_with_reason(
            ctx.payer,
            Some(ctx.source),
            &self.cost,
            x_value,
            ctx.reason,
        ) {
            return Err(CostPaymentError::InsufficientMana);
        }

        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        pay_mana_cost_with_choices(
            game,
            ctx.payer,
            Some(ctx.source),
            &self.cost,
            ctx.x_value.unwrap_or(0),
            ctx.reason,
            ctx.decision_maker,
        )?;

        Ok(CostPaymentResult::Paid)
    }

    fn display(&self) -> String {
        // Format the mana cost as a string
        format_mana_cost(&self.cost)
    }

    fn is_mana_cost(&self) -> bool {
        true
    }

    fn mana_cost(&self) -> Option<&crate::mana::ManaCost> {
        Some(&self.cost)
    }

    fn needs_player_choice(&self) -> bool {
        // Mana payment requires player to select which mana sources to tap
        true
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::ManaPayment {
            cost: self.cost.clone(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Format a ManaCost for display.
fn format_mana_cost(cost: &ManaCost) -> String {
    use crate::mana::ManaSymbol;

    let mut parts = Vec::new();

    for pip in cost.pips() {
        if pip.len() == 1 {
            // Single option pip
            match pip[0] {
                ManaSymbol::White => parts.push("{W}".to_string()),
                ManaSymbol::Blue => parts.push("{U}".to_string()),
                ManaSymbol::Black => parts.push("{B}".to_string()),
                ManaSymbol::Red => parts.push("{R}".to_string()),
                ManaSymbol::Green => parts.push("{G}".to_string()),
                ManaSymbol::Colorless => parts.push("{C}".to_string()),
                ManaSymbol::Generic(n) => parts.push(format!("{{{}}}", n)),
                ManaSymbol::Snow => parts.push("{S}".to_string()),
                ManaSymbol::Life(n) => parts.push(format!("{{{}/P}}", n)),
                ManaSymbol::X => parts.push("{X}".to_string()),
            }
        } else {
            // Hybrid/alternative pip - format as {W/U}, {2/W}, etc.
            let alts: Vec<String> = pip
                .iter()
                .map(|s| match s {
                    ManaSymbol::White => "W".to_string(),
                    ManaSymbol::Blue => "U".to_string(),
                    ManaSymbol::Black => "B".to_string(),
                    ManaSymbol::Red => "R".to_string(),
                    ManaSymbol::Green => "G".to_string(),
                    ManaSymbol::Colorless => "C".to_string(),
                    ManaSymbol::Generic(n) => format!("{}", n),
                    ManaSymbol::Snow => "S".to_string(),
                    ManaSymbol::Life(n) => format!("P{}", n),
                    ManaSymbol::X => "X".to_string(),
                })
                .collect();
            parts.push(format!("{{{}}}", alts.join("/")));
        }
    }

    if parts.is_empty() {
        "{0}".to_string()
    } else {
        parts.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::costs::PaymentReason;
    use crate::decision::DecisionMaker;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;

    #[derive(Default)]
    struct ChooseLife {
        prompts: usize,
    }
    impl DecisionMaker for ChooseLife {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.prompts += 1;
            vec![ctx.options.last().unwrap().index]
        }
    }

    struct StopAtChoice {
        calls: usize,
        waiting: bool,
        invalid: bool,
    }
    impl DecisionMaker for StopAtChoice {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.calls += 1;
            if self.calls == 1 {
                return vec![1];
            }
            if self.invalid {
                return vec![999];
            }
            self.waiting = true;
            vec![0]
        }
        fn awaiting_choice(&self) -> bool {
            self.waiting
        }
    }

    #[test]
    fn phyrexian_component_never_spends_before_all_choices_are_valid() {
        for invalid in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::White, 2);
            let source = game.new_object_id();
            let cost = ManaPaymentCost::new(ManaCost::from_pips(vec![
                vec![
                    ManaSymbol::White,
                    ManaSymbol::Life(2)
                ];
                2
            ]));
            let mut dm = StopAtChoice {
                calls: 0,
                waiting: false,
                invalid,
            };
            let mut ctx =
                CostContext::new(source, alice, &mut dm).with_reason(PaymentReason::Other);
            assert!(cost.pay(&mut game, &mut ctx).is_err());
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 2);
            assert_eq!(dm.calls, 2);
        }
    }

    #[test]
    fn phyrexian_component_respects_selected_mana_with_optional_black_life() {
        use crate::ability::Ability;
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::static_abilities::StaticAbility;
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        let card = CardBuilder::new(CardId::new(), "Life payment permission")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));
        game.update_cant_effects();
        struct ChooseMana(usize);
        impl DecisionMaker for ChooseMana {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                self.0 += 1;
                vec![ctx.options[0].index]
            }
        }
        let mut dm = ChooseMana(0);
        let cost = ManaPaymentCost::new(ManaCost::from_pips(vec![
            vec![
                ManaSymbol::Black,
                ManaSymbol::Life(2)
            ];
            2
        ]));
        let mut ctx = CostContext::new(source, alice, &mut dm).with_reason(PaymentReason::Other);
        cost.pay(&mut game, &mut ctx).unwrap();
        assert_eq!(
            dm.0, 1,
            "the second pip must use life after choosing the only black mana"
        );
        assert_eq!(game.player(alice).unwrap().life, 18);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn phyrexian_component_preserves_choices_and_remaining_pip_affordability() {
        for route in 0..4 {
            for (pip_count, starting_life, expected_life, expected_mana) in
                [(1, 20, 18, 1), (2, 3, 1, 0)]
            {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = PlayerId::from_index(0);
                game.player_mut(alice).unwrap().life = starting_life;
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::White, 1);
                let source = game.new_object_id();
                let cost = ManaCost::from_pips(vec![
                    vec![ManaSymbol::White, ManaSymbol::Life(2)];
                    pip_count
                ]);
                let mut dm = ChooseLife::default();
                if route > 0 {
                    let component = if route == 1 {
                        crate::costs::Cost::mana(cost)
                    } else {
                        crate::costs::Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
                            cost,
                            None,
                            None,
                            None,
                            ironsmith_core::DynamicManaDisplayHint::Default,
                        ))
                    };
                    let total = crate::cost::TotalCost::from_cost(component);
                    if route == 3 {
                        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
                        crate::special_actions::pay_total_cost_with_choice_in_context(
                            &mut game,
                            alice,
                            source,
                            &total,
                            PaymentReason::Other,
                            &mut ctx,
                        )
                        .unwrap();
                    } else {
                        crate::special_actions::pay_total_cost_with_choice(
                            &mut game,
                            alice,
                            source,
                            &total,
                            PaymentReason::Other,
                            &mut dm,
                        )
                        .unwrap();
                    }
                } else {
                    let mut ctx =
                        CostContext::new(source, alice, &mut dm).with_reason(PaymentReason::Other);
                    ManaPaymentCost::new(cost).pay(&mut game, &mut ctx).unwrap();
                }
                assert_eq!(dm.prompts, 1, "route={route}, pips={pip_count}");
                assert_eq!(game.player(alice).unwrap().life, expected_life);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), expected_mana);
            }
        }
    }
}
