//! Compact mana instructions shared by effect executors and payment planning.
//!
//! An instruction denotes one production event, not an aggregate pool delta.
//! Keeping this boundary is essential for replacements and triggered abilities.
use crate::color::Color;
use crate::effect::Value;
use crate::mana::ManaSymbol;
use crate::target::PlayerFilter;

/// Exact semantics of a pure mana-producing effect. Values and player filters
/// remain expressions; consumers must not mistake an unresolved expression for
/// zero production. `None` from the executor means full execution is required.
#[derive(Debug, Clone, Copy)]
pub enum ManaProduction<'a> {
    Fixed {
        symbols: &'a [ManaSymbol],
        player: &'a PlayerFilter,
    },
    Repeated {
        symbols: &'a [ManaSymbol],
        amount: &'a Value,
        player: &'a PlayerFilter,
    },
    ChooseColors {
        amount: &'a Value,
        available: &'a [Color],
        same_color: bool,
        distinct: bool,
        player: &'a PlayerFilter,
    },
    ChosenColor {
        amount: &'a Value,
        fixed_option: Option<Color>,
        player: &'a PlayerFilter,
    },
    CommanderIdentity {
        amount: &'a Value,
        player: &'a PlayerFilter,
    },
    ColorsAmong {
        filter: &'a crate::target::ObjectFilter,
        choose_one: bool,
        player: &'a PlayerFilter,
    },
    ImprintedColors,
    LandProducedTypes {
        amount: &'a Value,
        player: &'a PlayerFilter,
        filter: &'a crate::target::ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
        source: crate::effects::ManaTypeSource,
    },
    NotedType {
        amount: &'a Value,
        player: &'a PlayerFilter,
    },
    DoublePool {
        player: &'a PlayerFilter,
    },
}

pub(crate) struct ProductionEvent {
    pub symbols: Vec<ManaSymbol>,
    pub needs_choice: bool,
}

impl ManaProduction<'_> {
    /// Expressions whose reads cannot change during a tap-only mana closure.
    /// Triggering-event types are read from the captured rewritten event, not
    /// guessed from what the activated source could have produced.
    pub(crate) fn stable_for_trigger(self) -> bool {
        use crate::effect::Value;
        let player = match self {
            Self::Fixed { player, .. } => player,
            Self::Repeated { amount: Value::Fixed(_), player, .. }
            | Self::ChooseColors { amount: Value::Fixed(_), player, .. }
            | Self::ChosenColor { amount: Value::Fixed(_), player, .. }
            | Self::CommanderIdentity { amount: Value::Fixed(_), player, .. }
            | Self::NotedType { amount: Value::Fixed(_), player, .. } => player,
            Self::LandProducedTypes {
                amount: Value::Fixed(_), player, filter,
                source: crate::effects::ManaTypeSource::TriggeringEventProduced, ..
            } if !crate::game_state::GameState::filter_reads_tapped_state_or_activation_history(filter, true) => player,
            Self::ImprintedColors => return true,
            _ => return false,
        };
        matches!(player, PlayerFilter::You | PlayerFilter::Specific(_))
    }

    pub(crate) fn stable_resolved(self, game: &crate::game_state::GameState,
        source: crate::ids::ObjectId, controller: crate::ids::PlayerId)
        -> Option<crate::effects::mana::production_resolution::ResolvedManaProduction> {
        let amount = match self {
            Self::Fixed { .. } | Self::ImprintedColors => None,
            Self::Repeated { amount, .. } | Self::ChooseColors { amount, .. }
            | Self::ChosenColor { amount, .. } | Self::CommanderIdentity { amount, .. }
            | Self::NotedType { amount, .. } => Some(amount),
            _ => return None,
        };
        if amount.is_some_and(|value| fixed_amount(value).is_none()) { return None; }
        self.resolve(game, &crate::effects::ExecutionContext::new_default(source, controller)).ok()
    }

    /// Projection for a tap-only source. These contextual operands cannot
    /// change merely because another source was tapped or credited mana. Other
    /// expressions require the sequential evaluator's resource/dependency state.
    pub(crate) fn stable_event(
        self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        color: Option<&[Color]>,
    ) -> Option<ProductionEvent> {
        if let Some(event) = self.fixed_event(color) {
            return Some(event);
        }
        let amount = match self {
            Self::ChosenColor { amount, .. }
            | Self::CommanderIdentity { amount, .. }
            | Self::NotedType { amount, .. } => Some(amount),
            Self::ImprintedColors => None,
            _ => return None,
        };
        // A stable color source does not make its quantity expression stable.
        if amount.is_some_and(|value| fixed_amount(value).is_none()) {
            return None;
        }
        let ctx = crate::effects::ExecutionContext::new_default(source, controller);
        let resolved = self.resolve(game, &ctx).ok()?;
        if resolved.player != controller {
            return None;
        }
        use crate::effects::mana::production_resolution::ResolvedManaOutput;
        match resolved.output {
            ResolvedManaOutput::Exact(symbols) => Some(ProductionEvent {
                symbols,
                needs_choice: false,
            }),
            ResolvedManaOutput::Choice {
                available,
                count,
                distinct,
                ..
            } => {
                if distinct && count > 1 {
                    return None;
                }
                let symbol = match color {
                    Some([color]) => {
                        let symbol = ManaSymbol::from_color(*color);
                        if !available.contains(&symbol) {
                            return None;
                        }
                        symbol
                    }
                    None => *available.first()?,
                    _ => return None,
                };
                Some(ProductionEvent {
                    symbols: vec![symbol; count as usize],
                    needs_choice: color.is_none(),
                })
            }
        }
    }

    /// Evaluate a fixed production for the current single-color activation
    /// selection. Broader choices remain explicit in the instruction and must
    /// never be silently collapsed into this restricted selection interface.
    pub(crate) fn fixed_event(self, color: Option<&[Color]>) -> Option<ProductionEvent> {
        let (symbols, needs_choice) = match self {
            Self::Fixed { symbols, player } => {
                if *player != PlayerFilter::You {
                    return None;
                }
                (symbols.to_vec(), false)
            }
            Self::Repeated {
                symbols,
                amount,
                player,
            } => {
                if *player != PlayerFilter::You {
                    return None;
                }
                let count = fixed_amount(amount)?;
                let capacity = symbols.len().checked_mul(count)?;
                let mut output = Vec::new();
                output.try_reserve(capacity).ok()?;
                for _ in 0..count {
                    output.extend_from_slice(symbols);
                }
                (output, false)
            }
            Self::ChooseColors {
                amount,
                available,
                same_color: _,
                distinct,
                player,
            } => {
                if *player != PlayerFilter::You {
                    return None;
                }
                let count = fixed_amount(amount)?;
                if count == 0 {
                    return Some(ProductionEvent {
                        symbols: vec![],
                        needs_choice: false,
                    });
                }
                if distinct && count > 1 {
                    return None;
                }
                let selected = match color {
                    Some([color]) if available.contains(color) => *color,
                    None => *available.first()?,
                    _ => return None,
                };
                let mut output = Vec::new();
                output.try_reserve(count).ok()?;
                output.resize(count, ManaSymbol::from_color(selected));
                (output, color.is_none())
            }
            // These expressions require an evaluation context. They are
            // described exactly, but are not context-free fixed productions.
            _ => return None,
        };
        Some(ProductionEvent {
            symbols,
            needs_choice,
        })
    }
}

fn fixed_amount(value: &Value) -> Option<usize> {
    match value {
        Value::Fixed(amount) => Some((*amount).max(0) as usize),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;
    use crate::ids::PlayerId;

    #[test]
    fn fixed_instructions_match_executed_events_including_empty_production() {
        for effect in [
            Effect::add_mana(vec![ManaSymbol::Green, ManaSymbol::Blue]),
            Effect::add_mana(vec![]),
            Effect::add_colorless_mana(3),
            Effect::add_colorless_mana(-2),
            Effect::add_mana_of_any_color(0),
            Effect::add_mana_of_any_one_color(0),
        ] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let player = PlayerId::from_index(0);
            let source = game.new_object_id();
            let projected = effect.mana_production().unwrap().fixed_event(None).unwrap();
            let mut ctx = ExecutionContext::new_default(source, player);
            let outcome = effect.0.execute(&mut game, &mut ctx).unwrap();
            let actual = outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::ManaAddedEvent>())
                .flat_map(|event| event.mana.iter().copied())
                .collect::<Vec<_>>();
            assert_eq!(projected.symbols, actual);
            assert!(!projected.needs_choice);
            assert_eq!(
                game.player(player).unwrap().mana_pool.total() as usize,
                actual.len()
            );
        }
    }

    #[test]
    fn unresolved_choices_values_and_side_effects_are_not_fixed_productions() {
        assert!(Effect::gain_life(1).mana_production().is_none());
        assert!(
            Effect::add_colorless_mana(Value::X)
                .mana_production()
                .unwrap()
                .fixed_event(None)
                .is_none()
        );
        assert!(
            Effect::add_mana_of_different_colors(2)
                .mana_production()
                .unwrap()
                .fixed_event(Some(&[Color::Blue]))
                .is_none()
        );
        let event = Effect::add_mana_of_any_one_color(2)
            .mana_production()
            .unwrap()
            .fixed_event(None)
            .unwrap();
        assert!(event.needs_choice);
        assert_eq!(event.symbols, vec![ManaSymbol::White; 2]);
        let event = Effect::add_mana_of_any_one_color(2)
            .mana_production()
            .unwrap()
            .fixed_event(Some(&[Color::Blue]))
            .unwrap();
        assert!(!event.needs_choice);
        assert_eq!(event.symbols, vec![ManaSymbol::Blue; 2]);
    }

    #[test]
    fn replacement_instructions_preserve_amount_semantics_and_order() {
        use crate::replacement::{EventModification, ReplacementAction};
        let original = [ManaSymbol::Green, ManaSymbol::Red];
        let recolor: ReplacementAction = ReplacementAction::ReplaceMana(vec![ManaSymbol::Blue]);
        let exact: ReplacementAction = ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]);
        let triple: ReplacementAction = ReplacementAction::Modify(EventModification::Multiply(3));
        let zero: ReplacementAction = ReplacementAction::Modify(EventModification::Multiply(0));
        assert_eq!(
            recolor.mana_transformation().unwrap().apply(&original),
            vec![ManaSymbol::Blue; 2]
        );
        assert_eq!(
            exact.mana_transformation().unwrap().apply(&original),
            vec![ManaSymbol::Blue]
        );
        assert!(
            zero.mana_transformation()
                .unwrap()
                .apply(&original)
                .is_empty()
        );
        let exact_then_triple = triple
            .mana_transformation()
            .unwrap()
            .apply(&exact.mana_transformation().unwrap().apply(&original));
        let triple_then_exact = exact
            .mana_transformation()
            .unwrap()
            .apply(&triple.mana_transformation().unwrap().apply(&original));
        assert_eq!(exact_then_triple, vec![ManaSymbol::Blue; 3]);
        assert_eq!(triple_then_exact, vec![ManaSymbol::Blue]);
        // Generic Double currently covers other event families, not mana.
        // A name alone must never be used as a production-semantics hint.
        let unrelated: ReplacementAction = ReplacementAction::Double;
        assert!(unrelated.mana_transformation().is_none());
    }
}

/// Canonical symbol order shared by context-dependent pool production.
pub(crate) fn pool_symbols(pool: &crate::player::ManaPool) -> Vec<ManaSymbol> {
    super::sources::pool_units(pool)
}
