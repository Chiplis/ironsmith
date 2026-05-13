//! Stored spell-copy effect used by Epic delayed triggers.

use crate::ability::{Ability, AbilityKind};
use crate::effect::EffectOutcome;
use crate::effects::stack::copy_spell::create_stack_copy_from_object;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::spells::SpellCopiedEvent;
use crate::game_state::{GameState, StackEntry};
use crate::object::Object;
use crate::static_abilities::StaticAbilityId;
use crate::triggers::TriggerEvent;

/// Copies the spell characteristics captured when an Epic spell resolved.
///
/// Epic copies are created from the original resolving spell as it existed on
/// the stack, not from the card after it moved to another zone. The copy is
/// made except for Epic, so resolving the upkeep copy must not install another
/// repeating delayed trigger.
#[derive(Debug, Clone)]
pub(crate) struct EpicSpellCopyEffect {
    spell: Object,
    entry: StackEntry,
}

impl EpicSpellCopyEffect {
    pub(crate) fn new(spell: &Object, entry: &StackEntry) -> Self {
        let mut spell = spell.clone();
        remove_epic_ability(&mut spell);
        Self {
            spell,
            entry: entry.clone(),
        }
    }
}

impl EffectExecutor for EpicSpellCopyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let copy_id = create_stack_copy_from_object(
            game,
            &self.spell,
            self.entry.object_id,
            &self.entry,
            ctx.controller,
            &[],
            remove_epic_ability,
            None,
        )?;

        game.queue_trigger_event(
            ctx.provenance,
            TriggerEvent::new_with_provenance(
                SpellCopiedEvent::new(copy_id, ctx.controller),
                ctx.provenance,
            ),
        );

        Ok(EffectOutcome::with_objects(vec![copy_id]))
    }
}

fn remove_epic_ability(object: &mut Object) {
    object.abilities.retain(|ability| !is_epic_ability(ability));
    object.compiled_card_text = object
        .compiled_card_text
        .lines()
        .filter(|line| {
            !line
                .trim()
                .trim_end_matches('.')
                .eq_ignore_ascii_case("epic")
        })
        .collect::<Vec<_>>()
        .join("\n");
}

fn is_epic_ability(ability: &Ability) -> bool {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    static_ability.id() == StaticAbilityId::KeywordMarker
        && static_ability
            .display()
            .trim()
            .trim_end_matches('.')
            .eq_ignore_ascii_case("epic")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::effect::{Effect, OutcomeStatus};
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn epic_spell_copy_strips_epic_from_created_copy() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(91_100), "Epic Probe")
            .card_types(vec![CardType::Sorcery])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Stack);
        {
            let object = game.object_mut(source).expect("source object");
            object.abilities.push(
                Ability::static_ability(StaticAbility::keyword_marker("Epic"))
                    .in_zones(vec![Zone::Stack]),
            );
            object.compiled_card_text = "Draw a card.\nEpic.".to_string();
            object.spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::draw(1),
            ]));
        }

        let entry = StackEntry::new(source, alice);
        let source_object = game.object(source).expect("source object").clone();
        let effect = EpicSpellCopyEffect::new(&source_object, &entry);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = effect.execute(&mut game, &mut ctx).expect("copy resolves");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        let copy_id = outcome.value.objects().expect("copy id")[0];
        let copy = game.object(copy_id).expect("copy object");
        assert!(
            copy.abilities
                .iter()
                .all(|ability| !is_epic_ability(ability)),
            "Epic ability should not be copied to the upkeep copy"
        );
        assert!(
            !copy
                .compiled_card_text
                .to_ascii_lowercase()
                .contains("epic")
        );
    }
}
