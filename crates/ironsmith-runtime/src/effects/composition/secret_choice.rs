use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter_to_list;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SecretChoiceResult {
    pub choices: Vec<(PlayerId, String)>,
    pub object_choices: Vec<(PlayerId, ObjectSnapshot)>,
}

impl SecretChoiceResult {
    pub fn choices_match(&self) -> bool {
        let Some((_, first)) = self.choices.first() else {
            return false;
        };
        self.choices.len() > 1
            && self
                .choices
                .iter()
                .all(|(_, choice)| choice.eq_ignore_ascii_case(first))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecretChoiceEffect {
    pub options: Vec<String>,
    pub participants: Vec<PlayerFilter>,
    pub participant_target: Option<ChooseSpec>,
    pub object_choice: Option<ironsmith_core::SecretObjectChoice>,
}

impl SecretChoiceEffect {
    pub fn new(options: Vec<String>, participants: Vec<PlayerFilter>) -> Self {
        let participant_target = participants.iter().find_map(|participant| {
            if let PlayerFilter::Target(inner) = participant {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            } else {
                None
            }
        });
        Self {
            options,
            participants,
            participant_target,
            object_choice: None,
        }
    }

    pub fn new_objects(
        participants: Vec<PlayerFilter>,
        object_choice: ironsmith_core::SecretObjectChoice,
    ) -> Self {
        let participant_target = participants.iter().find_map(|participant| {
            if let PlayerFilter::Target(inner) = participant {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            } else {
                None
            }
        });
        Self {
            options: Vec::new(),
            participants,
            participant_target,
            object_choice: Some(object_choice),
        }
    }

    fn participating_players(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);
        let mut players = Vec::new();
        for participant in &self.participants {
            for player in resolve_player_filter_to_list(game, participant, &filter_ctx, ctx)? {
                if game
                    .player(player)
                    .is_some_and(|player| player.is_in_game())
                    && !players.contains(&player)
                {
                    players.push(player);
                }
            }
        }
        Ok(players)
    }

    fn execute_object_choices(
        &self,
        choice: &ironsmith_core::SecretObjectChoice,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let players = self.participating_players(game, ctx)?;
        let mut selected = Vec::new();

        for player in players {
            let original_iterated_player =
                std::mem::replace(&mut ctx.iteration.iterated_player, Some(player));
            let filter_ctx = ctx.filter_context(game);
            let zone = choice.filter.zone.unwrap_or(Zone::Battlefield);
            let candidates = game
                .zone_ids(zone)
                .filter_map(|id| game.object(id).map(|object| (id, object)))
                .filter(|(_, object)| choice.filter.matches(object, &filter_ctx, game))
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            ctx.iteration.iterated_player = original_iterated_player;

            let minimum = choice.count.min.min(candidates.len());
            let maximum = choice
                .count
                .max
                .unwrap_or(candidates.len())
                .min(candidates.len());
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Secretly choose an object",
                candidates,
                minimum,
                Some(maximum),
            );
            let chosen = make_decision(
                game,
                &mut ctx.decision_maker,
                player,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            selected.extend(chosen.into_iter().map(|object_id| (player, object_id)));
        }

        let object_choices = selected
            .into_iter()
            .filter_map(|(player, object_id)| {
                game.object(object_id)
                    .map(|object| (player, ObjectSnapshot::from_object(object, game)))
            })
            .collect::<Vec<_>>();
        ctx.set_tagged_objects(
            choice.tag.clone(),
            object_choices
                .iter()
                .map(|(_, snapshot)| snapshot.clone())
                .collect(),
        );
        let count = object_choices.len();
        ctx.secret_choice_results.insert(
            ctx.source,
            SecretChoiceResult {
                choices: Vec::new(),
                object_choices,
            },
        );
        Ok(EffectOutcome::count(count as i32))
    }
}

impl EffectExecutor for SecretChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if let Some(object_choice) = &self.object_choice {
            return self.execute_object_choices(object_choice, game, ctx);
        }
        if self.options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        let players = self.participating_players(game, ctx)?;
        let display_options = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, option)| SelectableOption::new(idx, option.clone()))
            .collect::<Vec<_>>();
        let mut choices = Vec::new();
        for player in players {
            let choice_ctx = SelectOptionsContext::new(
                player,
                Some(ctx.source),
                "Secretly choose one",
                display_options.clone(),
                1,
                1,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if let Some(chosen) = selected
                .into_iter()
                .next()
                .filter(|idx| *idx < self.options.len())
            {
                choices.push((player, self.options[chosen].clone()));
            }
        }
        let count = choices.len();
        ctx.secret_choice_results.insert(
            ctx.source,
            SecretChoiceResult {
                choices,
                object_choices: Vec::new(),
            },
        );
        Ok(EffectOutcome::count(count as i32))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.participant_target.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::ChoiceCount;
    use crate::effects::ResolvedTarget;
    use crate::filter::ObjectFilter;
    use crate::ids::CardId;
    use crate::types::CardType;

    fn creature_card(id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    #[test]
    fn participant_relative_object_choices_are_correlated_and_tagged_after_collection() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let alice_creature = game.create_object_from_card(
            &creature_card(97_001, "Alice Creature"),
            alice,
            Zone::Battlefield,
        );
        let bob_creature = game.create_object_from_card(
            &creature_card(97_002, "Bob Creature"),
            bob,
            Zone::Battlefield,
        );
        let tag = crate::tag::TagKey::from("secretly_chosen");
        let object_choice = ironsmith_core::SecretObjectChoice {
            filter: ObjectFilter {
                zone: Some(Zone::Battlefield),
                controller: Some(PlayerFilter::IteratedPlayer),
                card_types: vec![CardType::Creature],
                ..ObjectFilter::default()
            },
            count: ChoiceCount::exactly(1),
            tag: tag.clone(),
            reveal_after_choice: true,
        };
        let effect = SecretChoiceEffect::new_objects(
            vec![PlayerFilter::You, PlayerFilter::target_opponent()],
            object_choice,
        );
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_decision_maker(&mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("secret object choices should resolve");

        assert_eq!(outcome.as_count(), Some(2));
        let result = ctx
            .secret_choice_results
            .get(&source)
            .expect("correlated result should be retained");
        assert_eq!(result.object_choices.len(), 2);
        assert_eq!(result.object_choices[0].0, alice);
        assert_eq!(result.object_choices[0].1.object_id, alice_creature);
        assert_eq!(result.object_choices[1].0, bob);
        assert_eq!(result.object_choices[1].1.object_id, bob_creature);
        let tagged = ctx
            .get_tagged_all(&tag)
            .expect("all collected choices should be revealed through one result tag");
        assert_eq!(
            tagged
                .iter()
                .map(|snapshot| snapshot.object_id)
                .collect::<Vec<_>>(),
            vec![alice_creature, bob_creature]
        );
    }
}
