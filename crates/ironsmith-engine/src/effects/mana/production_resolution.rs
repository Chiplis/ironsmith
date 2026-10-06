//! Read-only resolution of the shared mana instruction vocabulary.
//!
//! Choices are data here. The caller must select an admissible output and run
//! the pending event through replacement processing before crediting mana.
use crate::color::Color;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::mana::ManaSymbol;
use crate::mana_payment::program::ManaProduction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedManaOutput {
    Exact(Vec<ManaSymbol>),
    Choice {
        available: Vec<ManaSymbol>,
        count: u32,
        same_type: bool,
        distinct: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedManaProduction {
    pub player: PlayerId,
    pub output: ResolvedManaOutput,
}

impl ResolvedManaOutput {
    /// Validate a concrete instruction-level choice before replacement effects.
    pub(crate) fn accepts(&self, selected: &[ManaSymbol]) -> bool {
        match self {
            Self::Exact(symbols) => symbols == selected,
            Self::Choice {
                available,
                count,
                same_type,
                distinct,
            } => {
                selected.len() == *count as usize
                    && selected.iter().all(|symbol| available.contains(symbol))
                    && (!same_type || selected.windows(2).all(|pair| pair[0] == pair[1]))
                    && (!distinct
                        || selected
                            .iter()
                            .enumerate()
                            .all(|(index, symbol)| !selected[..index].contains(symbol)))
            }
        }
    }

    /// Enumerate exact choices. `None` means the caller's exploration budget was
    /// insufficient, never that the production is impossible. Keep ordered
    /// outputs: an opaque event consumer is not proof that permutations commute.
    pub(crate) fn alternatives(&self, limit: usize) -> Option<Vec<Vec<ManaSymbol>>> {
        if limit == 0 {
            return None;
        }
        match self {
            Self::Exact(symbols) => Some(vec![symbols.clone()]),
            Self::Choice {
                available,
                count,
                same_type,
                distinct,
            } => {
                if *count == 0 {
                    return Some(vec![vec![]]);
                }
                let mut unique = Vec::new();
                for symbol in available {
                    if !unique.contains(symbol) {
                        unique.push(*symbol);
                    }
                }
                if *distinct && (*count as usize > unique.len() || (*same_type && *count > 1)) {
                    return Some(vec![]);
                }
                if *same_type {
                    if unique.len() > limit {
                        return None;
                    }
                    return Some(
                        unique
                            .into_iter()
                            .map(|symbol| vec![symbol; *count as usize])
                            .collect(),
                    );
                }
                if unique.is_empty() {
                    return Some(vec![]);
                }
                // Reject excessive breadth before allocating partial branches.
                let mut combinations = 1usize;
                for index in 0..*count as usize {
                    combinations = combinations.checked_mul(if *distinct {
                        unique.len() - index
                    } else {
                        unique.len()
                    })?;
                    if combinations > limit {
                        return None;
                    }
                }
                let mut outputs = vec![vec![]];
                for _ in 0..*count {
                    let mut next = Vec::new();
                    for prefix in outputs {
                        for symbol in &unique {
                            if *distinct && prefix.contains(symbol) {
                                continue;
                            }
                            let mut selected = prefix.clone();
                            selected.push(*symbol);
                            next.push(selected);
                        }
                    }
                    outputs = next;
                }
                Some(outputs)
            }
        }
    }
}

impl ManaProduction<'_> {
    /// Execute a production known by its native executor to contain no choice.
    pub(crate) fn resolve_exact(
        self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<(PlayerId, Vec<ManaSymbol>), ExecutionError> {
        let resolved = self.resolve(game, ctx)?;
        match resolved.output {
            ResolvedManaOutput::Exact(symbols) => Ok((resolved.player, symbols)),
            ResolvedManaOutput::Choice { .. } => Err(ExecutionError::InternalError(
                "choice-bearing mana instruction used as exact production".into(),
            )),
        }
    }

    /// Resolve against the state at this instruction, never a stale pre-payment
    /// snapshot. This method does not resolve choices, emit events or mutate the
    /// game. Errors remain errors rather than becoming zero-mana estimates.
    pub(crate) fn resolve(
        self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<ResolvedManaProduction, ExecutionError> {
        use ResolvedManaOutput::{Choice, Exact};
        let amount = |value| resolve_value(game, value, ctx).map(|n| n.max(0) as u32);
        let recipient = match self {
            Self::Fixed { player, .. }
            | Self::Repeated { player, .. }
            | Self::ChooseColors { player, .. }
            | Self::ChosenColor { player, .. }
            | Self::CommanderIdentity { player, .. }
            | Self::ColorsAmong { player, .. }
            | Self::LandProducedTypes { player, .. }
            | Self::NotedType { player, .. }
            | Self::DoublePool { player } => resolve_player_filter(game, player, ctx)?,
            Self::ImprintedColors => ctx.controller,
        };
        let output = match self {
            Self::Fixed { symbols, .. } => Exact(symbols.to_vec()),
            Self::Repeated {
                symbols,
                amount: value,
                ..
            } => Exact(symbols.repeat(amount(value)? as usize)),
            Self::ChooseColors {
                amount: value,
                available,
                same_color,
                distinct,
                ..
            } => Choice {
                available: available
                    .iter()
                    .copied()
                    .map(ManaSymbol::from_color)
                    .collect(),
                count: amount(value)?,
                same_type: same_color,
                distinct,
            },
            Self::ChosenColor {
                amount: value,
                fixed_option,
                ..
            } => {
                let count = amount(value)?;
                let chosen = game.chosen_color(ctx.source).unwrap_or(Color::Green);
                match fixed_option {
                    Some(fixed) if fixed != chosen => Choice {
                        available: vec![
                            ManaSymbol::from_color(fixed),
                            ManaSymbol::from_color(chosen),
                        ],
                        count,
                        same_type: true,
                        distinct: false,
                    },
                    _ => Exact(vec![ManaSymbol::from_color(chosen); count as usize]),
                }
            }
            Self::CommanderIdentity { amount: value, .. } => {
                let count = amount(value)?;
                let identity = game.get_commander_color_identity(recipient);
                if identity.is_empty() {
                    Exact(vec![ManaSymbol::Colorless; count as usize])
                } else {
                    Choice {
                        available: Color::ALL
                            .into_iter()
                            .filter(|color| identity.contains(*color))
                            .map(ManaSymbol::from_color)
                            .collect(),
                        count,
                        same_type: true,
                        distinct: false,
                    }
                }
            }
            Self::ColorsAmong {
                filter, choose_one, ..
            } => {
                let available = super::add_mana_of_colors_among::colors_among_for_execution(
                    game, filter, ctx, recipient,
                )?;
                if choose_one && !available.is_empty() {
                    Choice {
                        available,
                        count: 1,
                        same_type: false,
                        distinct: false,
                    }
                } else {
                    Exact(available)
                }
            }
            Self::ImprintedColors => {
                let available: Vec<_> =
                    super::add_mana_of_imprinted_colors::linked_exiled_card_colors(
                        game, ctx.source,
                    )
                    .into_iter()
                    .map(ManaSymbol::from_color)
                    .collect();
                if available.is_empty() {
                    Exact(vec![])
                } else {
                    Choice {
                        available,
                        count: 1,
                        same_type: true,
                        distinct: false,
                    }
                }
            }
            Self::LandProducedTypes {
                amount: value,
                filter,
                allow_colorless,
                same_type,
                source,
                ..
            } => {
                use super::add_mana_of_land_produced_types::{
                    collect_available_mana_symbols, collect_triggering_event_mana_symbols,
                    is_allowed_symbol,
                };
                let count = amount(value)?;
                let available = match source {
                    super::ManaTypeSource::MatchingLandsCouldProduce => {
                        collect_available_mana_symbols(game, ctx, filter)
                    }
                    super::ManaTypeSource::TriggeringEventProduced => {
                        collect_triggering_event_mana_symbols(game, ctx, filter)?
                    }
                }
                .into_iter()
                .filter(|symbol| is_allowed_symbol(*symbol, allow_colorless))
                .collect::<Vec<_>>();
                if available.is_empty() {
                    Exact(vec![])
                } else {
                    Choice {
                        available,
                        count,
                        same_type,
                        distinct: false,
                    }
                }
            }
            Self::NotedType { amount: value, .. } => {
                let count = amount(value)?;
                Exact(
                    game.noted_mana_type_for_source(ctx.source)
                        .map_or_else(Vec::new, |symbol| vec![symbol; count as usize]),
                )
            }
            Self::DoublePool { .. } => {
                let player = game
                    .player(recipient)
                    .ok_or(ExecutionError::PlayerNotFound(recipient))?;
                Exact(crate::mana_payment::program::pool_symbols(
                    &player.mana_pool,
                ))
            }
        };
        let output = match output {
            Choice { count: 0, .. } => Exact(vec![]),
            other => other,
        };
        Ok(ResolvedManaProduction {
            player: recipient,
            output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Effect, Value};

    #[test]
    fn mixed_single_and_distinct_choices_have_different_legal_sets() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, player);
        for (effect, count) in [
            (Effect::add_mana_of_any_color(2), 25),
            (Effect::add_mana_of_any_one_color(2), 5),
            (Effect::add_mana_of_different_colors(2), 20),
        ] {
            let resolved = effect
                .mana_production()
                .unwrap()
                .resolve(&game, &ctx)
                .unwrap();
            let choices = resolved.output.alternatives(25).unwrap();
            assert_eq!(choices.len(), count);
            assert!(choices.iter().all(|choice| resolved.output.accepts(choice)));
            assert!(!resolved.output.accepts(&[ManaSymbol::Colorless; 2]));
            assert!(!resolved.output.accepts(&[ManaSymbol::Green]));
            assert!(resolved.output.alternatives(count - 1).is_none());
        }
        assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn unresolved_values_fail_and_x_and_recipients_use_execution_context() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let effect =
            Effect::add_colorless_mana_player(Value::X, crate::target::PlayerFilter::Specific(bob));
        let unresolved = ExecutionContext::new_default(source, alice);
        assert!(
            effect
                .mana_production()
                .unwrap()
                .resolve(&game, &unresolved)
                .is_err()
        );
        let resolved_ctx = ExecutionContext::new_default(source, alice).with_x(3);
        let resolved = effect
            .mana_production()
            .unwrap()
            .resolve(&game, &resolved_ctx)
            .unwrap();
        assert_eq!(resolved.player, bob);
        assert_eq!(
            resolved.output,
            ResolvedManaOutput::Exact(vec![ManaSymbol::Colorless; 3])
        );
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn pool_doubling_reads_current_pool_without_copying_spending_restrictions() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let effect = Effect::double_mana_pool_player(crate::target::PlayerFilter::You);
        let ctx = ExecutionContext::new_default(source, alice);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 2);
        let first = effect
            .mana_production()
            .unwrap()
            .resolve(&game, &ctx)
            .unwrap();
        assert_eq!(
            first.output,
            ResolvedManaOutput::Exact(vec![ManaSymbol::Green; 2])
        );
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        let second = effect
            .mana_production()
            .unwrap()
            .resolve(&game, &ctx)
            .unwrap();
        assert_eq!(
            second.output,
            ResolvedManaOutput::Exact(vec![ManaSymbol::Blue, ManaSymbol::Green, ManaSymbol::Green])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 3);
    }

    #[test]
    fn impossible_distinct_choices_are_different_from_exhausted_budget() {
        let impossible = ResolvedManaOutput::Choice {
            available: vec![ManaSymbol::Green],
            count: 2,
            same_type: false,
            distinct: true,
        };
        assert_eq!(impossible.alternatives(10), Some(vec![]));
        let possible = ResolvedManaOutput::Choice {
            available: vec![ManaSymbol::Green, ManaSymbol::Blue],
            count: 2,
            same_type: false,
            distinct: false,
        };
        assert!(possible.alternatives(3).is_none());
        assert_eq!(possible.alternatives(4).unwrap().len(), 4);
    }
}
