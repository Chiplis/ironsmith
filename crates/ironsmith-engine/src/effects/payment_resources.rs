//! Nominal resources declared by the prepared action owners.

use crate::game_state::GameState;
use crate::game_state::Target;
use crate::ids::PlayerId;
use crate::object::CounterType;

/// A payment's nominal inputs before replacements alter its physical actions.
/// These claims reserve no state and do not infer payment acknowledgement.
#[derive(Debug, Clone)]
pub enum PaymentResourceClaim {
    Life {
        player: PlayerId,
        amount: u32,
    },
    Counters {
        target: Target,
        counter_type: CounterType,
        count: u32,
    },
}

/// Independent prepared originals share one prepayment budget. Life delegates
/// to its existing team/rule owner; counter requests share each subject/type.
/// Mana funding, resource credits and result-dependent programs need their own
/// dependency contract before participating in this independent budget check.
pub(crate) fn can_pay_declared_resources(
    game: &GameState,
    claims: &[PaymentResourceClaim],
) -> bool {
    let mut life = Vec::new();
    let mut counters = std::collections::HashMap::<(Target, CounterType), u64>::new();
    for claim in claims {
        match claim {
            PaymentResourceClaim::Life { player, amount } => life.push((*player, *amount)),
            PaymentResourceClaim::Counters {
                target,
                counter_type,
                count,
            } => {
                let available = match target {
                    Target::Object(id) => {
                        if game.object(*id).is_none() || game.is_phased_out(*id) {
                            return false;
                        }
                        game.counter_count(*id, *counter_type)
                    }
                    Target::Player(id) => {
                        let Some(player) = game.player(*id) else {
                            return false;
                        };
                        player.counter_count(*counter_type)
                    }
                };
                let total = counters.entry((target.clone(), *counter_type)).or_default();
                let Some(requested) = total.checked_add(u64::from(*count)) else {
                    return false;
                };
                *total = requested;
                if requested > u64::from(available) {
                    return false;
                }
            }
        }
    }
    game.can_pay_life_simultaneously(&life)
}
