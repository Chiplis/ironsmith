//! "Choose a creature type ... becomes that type" effect.

use crate::continuous::Modification;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::Subtype;

/// Effect: target object(s) become a chosen creature type.
#[derive(Debug, Clone, PartialEq)]
pub struct BecomeCreatureTypeChoiceEffect {
    pub target: ChooseSpec,
    pub until: Until,
    pub chooser: PlayerFilter,
    pub excluded_subtypes: Vec<Subtype>,
}

impl BecomeCreatureTypeChoiceEffect {
    pub fn new(target: ChooseSpec, until: Until, excluded_subtypes: Vec<Subtype>) -> Self {
        Self {
            target,
            until,
            chooser: PlayerFilter::You,
            excluded_subtypes,
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    fn creature_type_options(&self) -> Vec<Subtype> {
        Self::all_creature_types()
            .iter()
            .copied()
            .filter(|subtype| !self.excluded_subtypes.contains(subtype))
            .collect()
    }

    fn all_creature_types() -> &'static [Subtype] {
        &[
            Subtype::Advisor,
            Subtype::Ally,
            Subtype::Alien,
            Subtype::Angel,
            Subtype::Ape,
            Subtype::Army,
            Subtype::Archer,
            Subtype::Artificer,
            Subtype::Assassin,
            Subtype::Astartes,
            Subtype::Avatar,
            Subtype::Barbarian,
            Subtype::Bard,
            Subtype::Bear,
            Subtype::Beast,
            Subtype::Berserker,
            Subtype::Bird,
            Subtype::Boar,
            Subtype::Cat,
            Subtype::Centaur,
            Subtype::Citizen,
            Subtype::Coward,
            Subtype::Changeling,
            Subtype::Cleric,
            Subtype::Construct,
            Subtype::Crab,
            Subtype::Crocodile,
            Subtype::Detective,
            Subtype::Demon,
            Subtype::Devil,
            Subtype::Dinosaur,
            Subtype::Djinn,
            Subtype::Efreet,
            Subtype::Dog,
            Subtype::Drone,
            Subtype::Dragon,
            Subtype::Drake,
            Subtype::Druid,
            Subtype::Dwarf,
            Subtype::Elder,
            Subtype::Eldrazi,
            Subtype::Spawn,
            Subtype::Scion,
            Subtype::Elemental,
            Subtype::Elephant,
            Subtype::Elf,
            Subtype::Faerie,
            Subtype::Fish,
            Subtype::Fox,
            Subtype::Frog,
            Subtype::Fungus,
            Subtype::Gargoyle,
            Subtype::Giant,
            Subtype::Gnome,
            Subtype::Glimmer,
            Subtype::Goat,
            Subtype::Goblin,
            Subtype::God,
            Subtype::Golem,
            Subtype::Gorgon,
            Subtype::Gremlin,
            Subtype::Germ,
            Subtype::Griffin,
            Subtype::Hag,
            Subtype::Halfling,
            Subtype::Harpy,
            Subtype::Hippo,
            Subtype::Horror,
            Subtype::Homunculus,
            Subtype::Horse,
            Subtype::Hound,
            Subtype::Human,
            Subtype::Hydra,
            Subtype::Illusion,
            Subtype::Imp,
            Subtype::Insect,
            Subtype::Inkling,
            Subtype::Jellyfish,
            Subtype::Kavu,
            Subtype::Kirin,
            Subtype::Kithkin,
            Subtype::Knight,
            Subtype::Kobold,
            Subtype::Kor,
            Subtype::Kraken,
            Subtype::Leviathan,
            Subtype::Lizard,
            Subtype::Manticore,
            Subtype::Mercenary,
            Subtype::Merfolk,
            Subtype::Minion,
            Subtype::Minotaur,
            Subtype::Mole,
            Subtype::Monk,
            Subtype::Monkey,
            Subtype::Moonfolk,
            Subtype::Mount,
            Subtype::Mouse,
            Subtype::Mutant,
            Subtype::Myr,
            Subtype::Naga,
            Subtype::Necron,
            Subtype::Nightmare,
            Subtype::Ninja,
            Subtype::Noble,
            Subtype::Octopus,
            Subtype::Ogre,
            Subtype::Ooze,
            Subtype::Orc,
            Subtype::Otter,
            Subtype::Ox,
            Subtype::Oyster,
            Subtype::Peasant,
            Subtype::Pegasus,
            Subtype::Phyrexian,
            Subtype::Phoenix,
            Subtype::Pincher,
            Subtype::Pilot,
            Subtype::Pirate,
            Subtype::Plant,
            Subtype::Praetor,
            Subtype::Raccoon,
            Subtype::Rabbit,
            Subtype::Rat,
            Subtype::Reflection,
            Subtype::Rebel,
            Subtype::Rhino,
            Subtype::Rogue,
            Subtype::Robot,
            Subtype::Salamander,
            Subtype::Saproling,
            Subtype::Samurai,
            Subtype::Satyr,
            Subtype::Scarecrow,
            Subtype::Scout,
            Subtype::Servo,
            Subtype::Serpent,
            Subtype::Shade,
            Subtype::Shaman,
            Subtype::Shapeshifter,
            Subtype::Shark,
            Subtype::Sheep,
            Subtype::Skeleton,
            Subtype::Slith,
            Subtype::Sliver,
            Subtype::Slug,
            Subtype::Snake,
            Subtype::Soldier,
            Subtype::Sorcerer,
            Subtype::Sphinx,
            Subtype::Specter,
            Subtype::Spider,
            Subtype::Spike,
            Subtype::Splinter,
            Subtype::Spirit,
            Subtype::Sponge,
            Subtype::Squid,
            Subtype::Squirrel,
            Subtype::Starfish,
            Subtype::Surrakar,
            Subtype::Thopter,
            Subtype::Thrull,
            Subtype::Tiefling,
            Subtype::Tentacle,
            Subtype::Toy,
            Subtype::Treefolk,
            Subtype::Triskelavite,
            Subtype::Trilobite,
            Subtype::Troll,
            Subtype::Turtle,
            Subtype::Unicorn,
            Subtype::Vampire,
            Subtype::Vedalken,
            Subtype::Viashino,
            Subtype::Villain,
            Subtype::Wall,
            Subtype::Warlock,
            Subtype::Warrior,
            Subtype::Weird,
            Subtype::Werewolf,
            Subtype::Whale,
            Subtype::Wizard,
            Subtype::Wolf,
            Subtype::Wolverine,
            Subtype::Wombat,
            Subtype::Worm,
            Subtype::Wraith,
            Subtype::Wurm,
            Subtype::Yeti,
            Subtype::Zombie,
            Subtype::Zubera,
        ]
    }
}

impl EffectExecutor for BecomeCreatureTypeChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let subtype_options = self.creature_type_options();
        if subtype_options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let options: Vec<SelectableOption> = subtype_options
            .iter()
            .enumerate()
            .map(|(idx, subtype)| SelectableOption::new(idx, format!("{subtype:?}")))
            .collect();
        let choice_ctx = SelectOptionsContext::new(
            chooser,
            Some(ctx.source),
            "Choose a creature type",
            options,
            1,
            1,
        );
        let chosen = ctx
            .decision_maker
            .decide_options(game, &choice_ctx)
            .into_iter()
            .next()
            .unwrap_or(0);
        let chosen_subtype = subtype_options
            .get(chosen)
            .copied()
            .unwrap_or(subtype_options[0]);

        let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
            self.target.clone(),
            Modification::RemoveAllCreatureTypes,
            self.until.clone(),
        );
        apply = apply.with_additional_modification(Modification::AddSubtypes(vec![chosen_subtype]));
        apply.execute(game, ctx)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::definitions::grizzly_bears;
    use crate::decision::DecisionMaker;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    struct ChooseZombieDm;
    impl DecisionMaker for ChooseZombieDm {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("zombie"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    #[test]
    fn become_creature_type_choice_replaces_creature_subtype() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_def = grizzly_bears();
        let creature_id =
            game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);

        let source = game.new_object_id();
        let mut dm = ChooseZombieDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = BecomeCreatureTypeChoiceEffect::new(
            ChooseSpec::SpecificObject(creature_id),
            Until::EndOfTurn,
            vec![],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("become-creature-type-choice should execute");

        let subtypes = game.calculated_subtypes(creature_id);
        assert!(
            subtypes.contains(&Subtype::Zombie),
            "expected target creature to have Zombie subtype, got {subtypes:?}"
        );
        assert!(
            !subtypes.contains(&Subtype::Bear),
            "expected original Bear subtype to be replaced, got {subtypes:?}"
        );
    }
}
