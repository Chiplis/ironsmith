use crate::cost::{OptionalCost, OptionalCostsPaid};
use crate::decision::ManaPipPaymentAction;
use crate::game_state::Target;
use crate::mana::ManaSymbol;

use super::{
    CostPayment, CostSpec, CostStep, GameObjectId, GamePlayerId, ManaSymbolCode, ManaSymbolSpec,
    TargetSpec,
};

pub fn targets_from_game(targets: &[Target]) -> Vec<TargetSpec> {
    targets.iter().map(TargetSpec::from).collect()
}

pub fn mana_symbols_to_spec(symbols: &[ManaSymbol]) -> Vec<ManaSymbolSpec> {
    symbols.iter().map(ManaSymbolSpec::from).collect()
}

pub fn optional_costs_to_spec(optional: &OptionalCostsPaid) -> Vec<u32> {
    optional.costs.iter().map(|(_, times)| *times).collect()
}

pub fn optional_costs_from_choices(
    optional_costs: &[OptionalCost],
    choices: &[(usize, u32)],
) -> OptionalCostsPaid {
    let mut paid = OptionalCostsPaid::from_costs(optional_costs);
    for (index, times) in choices {
        paid.pay_times(*index, *times);
    }
    paid
}

pub fn cost_spec_from_steps(
    steps: Vec<CostStep>,
    optional: &OptionalCostsPaid,
    x_value: Option<u32>,
) -> CostSpec {
    CostSpec {
        payment_trace: steps,
        optional_costs: optional_costs_to_spec(optional),
        x_value,
    }
}

pub fn steps_from_pip_actions(actions: &[ManaPipPaymentAction]) -> Vec<CostStep> {
    let mut steps = Vec::new();

    for action in actions {
        match action {
            ManaPipPaymentAction::UseFromPool(symbol) => {
                steps.push(CostStep::Mana(ManaSymbolSpec::from(*symbol)))
            }
            ManaPipPaymentAction::PayLife(amount) => steps.push(CostStep::Mana(ManaSymbolSpec {
                symbol: ManaSymbolCode::Life,
                value: (*amount).min(u8::MAX as u32) as u8,
            })),
            ManaPipPaymentAction::ActivateManaAbility {
                source_id,
                ability_index,
            } => steps.push(CostStep::Payment(CostPayment::ActivateManaAbility {
                source: GameObjectId(source_id.0),
                ability_index: (*ability_index).min(u32::MAX as usize) as u32,
            })),
        }
    }

    steps
}

pub fn cost_spec_from_pip_actions(
    actions: &[ManaPipPaymentAction],
    optional: &OptionalCostsPaid,
    mut extra_steps: Vec<CostStep>,
    x_value: Option<u32>,
) -> CostSpec {
    let mut steps = steps_from_pip_actions(actions);
    steps.append(&mut extra_steps);
    cost_spec_from_steps(steps, optional, x_value)
}

impl From<Target> for TargetSpec {
    fn from(value: Target) -> Self {
        match value {
            Target::Object(id) => TargetSpec::Object(GameObjectId(id.0)),
            Target::Player(id) => TargetSpec::Player(GamePlayerId(id.0)),
        }
    }
}

impl From<&Target> for TargetSpec {
    fn from(value: &Target) -> Self {
        match value {
            Target::Object(id) => TargetSpec::Object(GameObjectId(id.0)),
            Target::Player(id) => TargetSpec::Player(GamePlayerId(id.0)),
        }
    }
}

impl From<ManaSymbol> for ManaSymbolSpec {
    fn from(value: ManaSymbol) -> Self {
        match value {
            ManaSymbol::White => ManaSymbolSpec {
                symbol: ManaSymbolCode::White,
                value: 0,
            },
            ManaSymbol::Blue => ManaSymbolSpec {
                symbol: ManaSymbolCode::Blue,
                value: 0,
            },
            ManaSymbol::Black => ManaSymbolSpec {
                symbol: ManaSymbolCode::Black,
                value: 0,
            },
            ManaSymbol::Red => ManaSymbolSpec {
                symbol: ManaSymbolCode::Red,
                value: 0,
            },
            ManaSymbol::Green => ManaSymbolSpec {
                symbol: ManaSymbolCode::Green,
                value: 0,
            },
            ManaSymbol::Colorless => ManaSymbolSpec {
                symbol: ManaSymbolCode::Colorless,
                value: 0,
            },
            ManaSymbol::Generic(n) => ManaSymbolSpec {
                symbol: ManaSymbolCode::Generic,
                value: n,
            },
            ManaSymbol::Snow => ManaSymbolSpec {
                symbol: ManaSymbolCode::Snow,
                value: 0,
            },
            ManaSymbol::Life(n) => ManaSymbolSpec {
                symbol: ManaSymbolCode::Life,
                value: n,
            },
            ManaSymbol::X => ManaSymbolSpec {
                symbol: ManaSymbolCode::X,
                value: 0,
            },
        }
    }
}

impl From<&ManaSymbol> for ManaSymbolSpec {
    fn from(value: &ManaSymbol) -> Self {
        (*value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn target_adapter_round_trip() {
        let obj_target = Target::Object(ObjectId::from_raw(10));
        let player_target = Target::Player(PlayerId::from_index(3));

        assert_eq!(
            TargetSpec::from(obj_target),
            TargetSpec::Object(GameObjectId(10))
        );
        assert_eq!(
            TargetSpec::from(player_target),
            TargetSpec::Player(GamePlayerId(3))
        );
    }

    #[test]
    fn mana_symbol_spec_mapping() {
        let spec = ManaSymbolSpec::from(ManaSymbol::Generic(5));
        assert_eq!(spec.symbol, ManaSymbolCode::Generic);
        assert_eq!(spec.value, 5);
    }

    #[test]
    fn cost_spec_builder() {
        let mut optional = OptionalCostsPaid::new(1);
        optional.pay(0);
        let steps = vec![
            CostStep::Mana(ManaSymbolSpec::from(ManaSymbol::Red)),
            CostStep::Payment(CostPayment::Life { amount: 2 }),
        ];
        let spec = cost_spec_from_steps(steps, &optional, Some(2));

        assert_eq!(spec.payment_trace.len(), 2);
        assert_eq!(spec.optional_costs.len(), 1);
        assert_eq!(spec.x_value, Some(2));
    }

    #[test]
    fn optional_costs_from_choices_builds_paid() {
        let optional_costs = vec![
            OptionalCost::kicker(crate::cost::TotalCost::free()),
            OptionalCost::multikicker(crate::cost::TotalCost::free()),
        ];
        let paid = optional_costs_from_choices(&optional_costs, &[(0, 1), (1, 2)]);
        assert!(paid.was_paid(0));
        assert_eq!(paid.times_paid(1), 2);
        let spec = optional_costs_to_spec(&paid);
        assert_eq!(spec, vec![1, 2]);
    }

    #[test]
    fn mana_pip_action_adapter() {
        let actions = vec![
            ManaPipPaymentAction::ActivateManaAbility {
                source_id: ObjectId::from_raw(5),
                ability_index: 1,
            },
            ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
            ManaPipPaymentAction::PayLife(2),
        ];

        let steps = steps_from_pip_actions(&actions);
        assert_eq!(steps.len(), 3);
        assert!(matches!(
            steps[0],
            CostStep::Payment(CostPayment::ActivateManaAbility { .. })
        ));
        assert!(matches!(steps[1], CostStep::Mana(_)));
        assert!(matches!(steps[2], CostStep::Mana(_)));
    }

    #[test]
    fn cost_steps_preserve_order() {
        let steps = vec![
            CostStep::Payment(CostPayment::Life { amount: 2 }),
            CostStep::Mana(ManaSymbolSpec::from(ManaSymbol::Blue)),
            CostStep::Payment(CostPayment::Discard {
                objects: vec![GameObjectId(77)],
            }),
        ];
        let spec = cost_spec_from_steps(steps.clone(), &OptionalCostsPaid::default(), None);
        assert_eq!(spec.payment_trace, steps);
    }
}
