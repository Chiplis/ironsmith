//! Card definition for Mind Bend.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::continuous::{EffectSourceType, EffectTarget, Modification};
use crate::decisions::ask_choose_one;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;

/// Mind Bend {U}
/// Instant
/// Change the text of target spell or permanent by replacing all instances of one color word
/// with another or one basic land type with another. (This effect lasts indefinitely.)
pub fn mind_bend() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mind Bend")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .from_text_with_metadata("Change the text of target spell or permanent by replacing all instances of one color word with another or one basic land type with another. (This effect lasts indefinitely.)")
            target: ChooseSpec::target_permanent(),
        })])
        .build()
}

#[derive(Debug, Clone, PartialEq)]
struct MindBendEffect {
    target: ChooseSpec,
}

impl EffectExecutor for MindBendEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = ctx
            .targets
            .first()
            .and_then(|t| {
                if let ResolvedTarget::Object(id) = t {
                    Some(*id)
                } else {
                    None
                }
            })
            .ok_or(ExecutionError::InvalidTarget)?;

        let target = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;
        if target.zone != Zone::Battlefield && target.zone != Zone::Stack {
            return Err(ExecutionError::InvalidTarget);
        }

        let categories = [("Color word".to_string(), true), ("Basic land type".to_string(), false)];
        let choose_colors = ask_choose_one(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            ctx.source,
            &categories,
        );

        let options = if choose_colors {
            vec![
                "White".to_string(),
                "Blue".to_string(),
                "Black".to_string(),
                "Red".to_string(),
                "Green".to_string(),
            ]
        } else {
            vec![
                "Plains".to_string(),
                "Island".to_string(),
                "Swamp".to_string(),
                "Mountain".to_string(),
                "Forest".to_string(),
            ]
        };

        let from = ask_choose_one(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            ctx.source,
            &options
                .iter()
                .enumerate()
                .map(|(i, v)| (v.clone(), i))
                .collect::<Vec<_>>(),
        );

        let mut to_options = options.clone();
        if from < to_options.len() {
            to_options.remove(from);
        }
        let to = ask_choose_one(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            ctx.source,
            &to_options
                .iter()
                .enumerate()
                .map(|(i, v)| (v.clone(), i))
                .collect::<Vec<_>>(),
        );

        let to_word = to_options
            .get(to)
            .cloned()
            .unwrap_or_else(|| "Island".to_string());
        let from_word = options
            .get(from)
            .cloned()
            .unwrap_or_else(|| "Swamp".to_string());

        let effect = ApplyContinuousEffect::new(
            EffectTarget::AllPermanents,
            Modification::ChangeText {
                from: from_word,
                to: to_word,
            },
            crate::effect::Until::Forever,
        )
        .with_source_type(EffectSourceType::Resolution {
            locked_targets: vec![target_id],
        });

        let _ = execute_effect(game, &Effect::new(effect), ctx)?;

        Ok(EffectOutcome::count(1))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target spell or permanent"
    }
}
