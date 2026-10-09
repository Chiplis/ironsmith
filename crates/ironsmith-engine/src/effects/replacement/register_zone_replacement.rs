use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutionCategory, EffectExecutor, ReplacementApplyMode};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// Registers a concrete zone-change replacement effect for the currently resolved object(s).
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterZoneReplacementEffect {
    pub target: ChooseSpec,
    pub from_zone: Option<Zone>,
    pub to_zone: Option<Zone>,
    pub replacement_zone: Zone,
    pub library_placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
    pub mode: ReplacementApplyMode,
    pub optional: bool,
    pub choice_description: Option<String>,
    pub counters: Vec<(CounterType, u32)>,
    pub linked_exile_follow_up: Option<ironsmith_core::LinkedExileFollowUp>,
}

impl RegisterZoneReplacementEffect {
    pub fn new(
        target: ChooseSpec,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            library_placement: None,
            mode,
            optional: false,
            choice_description: None,
            counters: Vec::new(),
            linked_exile_follow_up: None,
        }
    }

    pub fn optional(mut self, description: impl Into<String>) -> Self {
        self.optional = true;
        self.choice_description = Some(description.into());
        self
    }

    pub fn with_counters(mut self, counters: Vec<(CounterType, u32)>) -> Self {
        self.counters = counters;
        self
    }

    pub fn with_library_placement(
        mut self,
        placement: ironsmith_core::ZoneReplacementLibraryPlacement,
    ) -> Self {
        self.library_placement = Some(placement);
        self
    }

    pub fn with_linked_exile_follow_up(
        mut self,
        follow_up: ironsmith_core::LinkedExileFollowUp,
    ) -> Self {
        self.linked_exile_follow_up = Some(follow_up);
        self
    }

    pub fn resolve_replacements(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Vec<ReplacementEffect>, ExecutionError> {
        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if object_ids.is_empty() {
            return Err(ExecutionError::InvalidTarget);
        }

        Ok(object_ids
            .into_iter()
            .map(|object_id| {
                // "You may cast that card this turn. If that spell would be
                // put into a graveyard, ..." (Quintorius): the replacement is
                // created for a card that is not yet a spell. Casting it makes
                // a new object (CR 400.7); `execute` registers it as followed so
                // the manager rebinds it to that spell (and ends it otherwise).
                let follows_onto_stack = self.from_zone == Some(Zone::Stack)
                    && game
                        .object(object_id)
                        .is_some_and(|object| object.zone != Zone::Stack);
                let replacement = match self.library_placement {
                    Some(placement)
                        if follows_onto_stack
                            && self.replacement_zone == Zone::Library
                            && !self.optional
                            && self.counters.is_empty()
                            && self.linked_exile_follow_up.is_none() =>
                    {
                        ReplacementAction::Instead(vec![crate::effect::Effect::new(
                            super::MoveReplacedObjectToLibraryEffect::new(placement),
                        )])
                    }
                    _ => zone_replacement_action(
                        object_id,
                        self.to_zone,
                        self.replacement_zone,
                        self.library_placement,
                        self.optional,
                        self.choice_description.clone(),
                        self.counters.clone(),
                        self.linked_exile_follow_up,
                    ),
                };
                // A followed card's matcher is rebound to its spell when it is
                // cast (ReplacementEffectManager::rebind_followed_object).
                let matcher = crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(object_id),
                    self.from_zone,
                    self.to_zone,
                );
                ReplacementEffect::with_matcher(ctx.source, ctx.controller, matcher, replacement)
            })
            .collect())
    }
}

pub(crate) fn zone_replacement_action(
    object_id: crate::ids::ObjectId,
    original_zone: Option<Zone>,
    replacement_zone: Zone,
    library_placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
    optional: bool,
    choice_description: Option<String>,
    counters: Vec<(CounterType, u32)>,
    linked_exile_follow_up: Option<ironsmith_core::LinkedExileFollowUp>,
) -> ReplacementAction {
    if optional {
        let mut destinations = Vec::new();
        if let Some(zone) = original_zone {
            destinations.push(zone);
        }
        if !destinations.contains(&replacement_zone) {
            destinations.push(replacement_zone);
        }
        return ReplacementAction::InteractiveChooseDestination {
            destinations,
            description: choice_description.unwrap_or_else(|| "Choose a destination".to_string()),
        };
    }

    if let Some(ironsmith_core::LinkedExileFollowUp::GainSuspendIfMissing) = linked_exile_follow_up
    {
        // "exile that card with three time counters on it instead ... Then if
        // the exiled card doesn't have suspend, it gains suspend" (Gandalf of
        // the Secret Fire): the grant reaches the new exiled object only once
        // the replacement has moved it (CR 400.7, 614.1a, 702.62a).
        debug_assert_eq!(replacement_zone, Zone::Exile);
        let effects = vec![gain_suspend_if_missing_follow_up()];
        if counters.is_empty() {
            return ReplacementAction::ExileWithSourceLinkThen(effects);
        }
        return ReplacementAction::ExileWithSourceLinkCountersThen { counters, effects };
    }

    if !counters.is_empty() {
        return ReplacementAction::MoveToZoneWithCounters {
            zone: replacement_zone,
            counters,
        };
    }

    if let Some(ironsmith_core::LinkedExileFollowUp::ReturnToHandAtNextEndStep) =
        linked_exile_follow_up
    {
        debug_assert_eq!(replacement_zone, Zone::Exile);
        debug_assert!(library_placement.is_none());
        let tag = crate::tag::TagKey::from(crate::tag::ZONE_REPLACEMENT_OBJECT_TAG);
        let filter = ObjectFilter::tagged(tag.clone()).in_zone(Zone::Exile);
        let return_to_hand =
            crate::effect::Effect::new(crate::effects::ReturnToHandEffect::all(filter.clone()));
        let schedule = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
            crate::triggers::Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![return_to_hand],
            true,
            tag,
            PlayerFilter::You,
        )
        .with_target_filter(filter);
        return ReplacementAction::ExileWithSourceLinkThen(vec![crate::effect::Effect::new(
            schedule,
        )]);
    }

    if let Some(ironsmith_core::LinkedExileFollowUp::BecomePlotted) = linked_exile_follow_up {
        debug_assert_eq!(replacement_zone, Zone::Exile);
        let tag = crate::tag::TagKey::from(crate::tag::ZONE_REPLACEMENT_OBJECT_TAG);
        let plot = crate::effect::Effect::new(crate::effects::BecomePlottedEffect::new(
            ChooseSpec::All(ObjectFilter::tagged(tag).in_zone(Zone::Exile)),
        ));
        return ReplacementAction::ExileWithSourceLinkThen(vec![plot]);
    }

    if replacement_zone == Zone::Library
        && let Some(placement) = library_placement
    {
        let target = ChooseSpec::SpecificObject(object_id);
        let move_effect = match placement {
            ironsmith_core::ZoneReplacementLibraryPlacement::Top => {
                crate::effect::Effect::move_to_zone(target, Zone::Library, true)
            }
            ironsmith_core::ZoneReplacementLibraryPlacement::Bottom => {
                crate::effect::Effect::move_to_zone(target, Zone::Library, false)
            }
            ironsmith_core::ZoneReplacementLibraryPlacement::TopOrBottom => {
                crate::effect::Effect::new(
                    crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(target)
                        .with_chooser(PlayerFilter::You),
                )
            }
        };
        return ReplacementAction::Instead(vec![move_effect]);
    }

    ReplacementAction::ChangeDestination(replacement_zone)
}

/// "If [the exiled card] doesn't have suspend, it gains suspend" applied to
/// the object a zone replacement just exiled (tagged under
/// [`crate::tag::ZONE_REPLACEMENT_OBJECT_TAG`]). The granted abilities are the
/// two suspend triggers (CR 702.62a), the same pair the compiler lowers for a
/// printed "it gains suspend" grant.
fn gain_suspend_if_missing_follow_up() -> crate::effect::Effect {
    let tag = crate::tag::TagKey::from(crate::tag::ZONE_REPLACEMENT_OBJECT_TAG);
    let has_suspend = ObjectFilter::default()
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend);
    let mut abilities = granted_suspend_abilities().into_iter();
    let (Some(upkeep), Some(last_counter)) = (abilities.next(), abilities.next()) else {
        unreachable!("suspend grants exactly two triggered abilities");
    };
    let grant = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Tagged(tag.clone()),
        crate::continuous::Modification::AddAbilityGeneric(upkeep),
        crate::effect::Until::Forever,
    )
    .with_additional_modification(crate::continuous::Modification::AddAbilityGeneric(
        last_counter,
    ));
    crate::effect::Effect::conditional(
        crate::effect::Condition::Not(Box::new(crate::effect::Condition::TaggedObjectMatches(
            tag,
            has_suspend,
        ))),
        vec![crate::effect::Effect::new(grant)],
        Vec::new(),
    )
}

/// The two exile-zone suspend triggers a card gains with "it gains suspend"
/// (CR 702.62a): remove a time counter each upkeep, and cast it without paying
/// its mana cost when the last is removed.
fn granted_suspend_abilities() -> Vec<crate::ability::Ability> {
    use crate::ability::{
        Ability, AbilityKind, PresentationKeyword, PresentationLabel, TriggeredAbility,
    };
    vec![
        Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::You),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::remove_counters(CounterType::Time, 1, ChooseSpec::Source),
                ]),
                choices: vec![],
                intervening_if: Some(crate::effect::Condition::SourceHasCounterAtLeast {
                    counter_type: CounterType::Time,
                    count: 1,
                    surface: crate::effect::SourceCounterThresholdSurface::SourceHas,
                }),
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Suspend)),
            }),
            functional_zones: vec![Zone::Exile],
        },
        Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: crate::triggers::Trigger::new(
                    crate::triggers::CounterRemovedFromTrigger::new(ObjectFilter::source())
                        .counter_type(CounterType::Time)
                        .last(),
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::may_single(crate::effect::Effect::new(
                        crate::effects::CastSourceEffect::new()
                            .without_paying_mana_cost()
                            .require_exile()
                            .cast_as_suspend(),
                    )),
                ]),
                choices: vec![],
                intervening_if: Some(crate::effect::Condition::SourceIsInZone(Zone::Exile)),
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Suspend)),
            }),
            functional_zones: vec![Zone::Exile],
        },
    ]
}

impl EffectExecutor for RegisterZoneReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let replacements = match self.resolve_replacements(game, ctx) {
            Ok(replacements) => replacements,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };

        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        for (index, replacement) in replacements.into_iter().enumerate() {
            // "You may cast that card this turn. If that spell would be put
            // into a graveyard, ..." (Quintorius): the card is not yet a
            // spell. The replacement waits for that card: casting it rebinds
            // the replacement to the new spell object, any other zone change
            // ends it (CR 400.7), and if the card is still uncast at cleanup it
            // ends with the turn's other one-shots.
            if self.from_zone == Some(Zone::Stack)
                && let Some(&object_id) = object_ids.get(index)
                && game
                    .object(object_id)
                    .is_some_and(|object| object.zone != Zone::Stack)
            {
                game.effect_store.replacement_effects.add_followed_one_shot_effect(
                    replacement,
                    crate::replacement::FollowedReplacementObject {
                        object: object_id,
                        from_zone: self.from_zone,
                        to_zone: self.to_zone,
                        on_stack: false,
                    },
                );
                continue;
            }
            match self.mode {
                ReplacementApplyMode::OneShot => {
                    game.effect_store
                        .replacement_effects
                        .add_one_shot_effect(replacement);
                }
                ReplacementApplyMode::UntilEndOfTurn => {
                    game.effect_store
                        .replacement_effects
                        .add_until_end_of_turn_effect(replacement);
                }
                ReplacementApplyMode::UntilYourNextTurn => {
                    game.effect_store
                        .replacement_effects
                        .add_until_next_turn_effect(
                            replacement,
                            ctx.controller,
                            game.turn.turn_number,
                        );
                }
                ReplacementApplyMode::Resolution => {
                    game.effect_store
                        .replacement_effects
                        .add_resolution_effect(replacement);
                }
            }
        }

        Ok(EffectOutcome::with_objects(object_ids))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target for replacement"
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::OutcomeStatus;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, owner: PlayerId, zone: Zone) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Replacement Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn test_registered_zone_replacement_exiles_matching_death_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move effect to resolve on the creature"
        );

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after replacement");
        assert_eq!(
            game.object(exiled_id)
                .expect("exiled creature should exist")
                .zone,
            Zone::Exile
        );
    }

    #[test]
    fn test_registered_zone_replacement_moves_with_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        )
        .with_counters(vec![(CounterType::Time, 3)]);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move effect to resolve on the creature"
        );

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after replacement");
        assert_eq!(
            game.object(exiled_id)
                .expect("exiled creature should exist")
                .zone,
            Zone::Exile
        );
        assert_eq!(game.counter_count(exiled_id, CounterType::Time), 3);
    }

    /// Quintorius: a replacement made for an exiled card follows it onto the
    /// stack (a new object, CR 400.7) and puts the spell on the bottom of its
    /// owner's library instead of into the graveyard (CR 614.1a).
    #[test]
    fn test_stack_zone_replacement_follows_card_cast_from_exile_to_library_bottom() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let filler = create_creature(&mut game, alice, Zone::Library);
        let card = create_creature(&mut game, alice, Zone::Exile);
        let stable_id = game.object(card).expect("card should exist").stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(card),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Library,
            ReplacementApplyMode::OneShot,
        )
        .with_library_placement(ironsmith_core::ZoneReplacementLibraryPlacement::Bottom);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(card, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let spell = game
            .move_object(
                card,
                Zone::Stack,
                crate::events::cause::EventCause::from_game_rule(),
            )
            .expect("the card is cast");
        assert_ne!(spell, card, "the spell is a new object");
        let _ = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(spell),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");

        let moved = game
            .find_object_by_stable_id(stable_id)
            .expect("the card is still findable");
        assert_eq!(game.object(moved).unwrap().zone, Zone::Library);
        assert_eq!(
            game.player(alice).unwrap().library.first(),
            Some(&moved),
            "the spell went to the bottom, under {filler:?}"
        );
    }

    fn register_library_bottom_for_exiled_card(
        game: &mut GameState,
        alice: PlayerId,
        card: crate::ids::ObjectId,
    ) {
        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(card),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Library,
            ReplacementApplyMode::OneShot,
        )
        .with_library_placement(ironsmith_core::ZoneReplacementLibraryPlacement::Bottom);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(card, alice, &mut dm);
        let _ = execute_effect(game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");
    }

    /// A card that leaves exile other than by being cast is a new object the
    /// replacement no longer refers to (CR 400.7): the replacement ends, so a
    /// later cast of that card goes to the graveyard normally.
    #[test]
    fn test_followed_stack_replacement_ends_when_card_moves_elsewhere() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = create_creature(&mut game, alice, Zone::Exile);
        let stable_id = game.object(card).expect("card should exist").stable_id;
        let baseline = game.effect_store.replacement_effects.effects().len();
        register_library_bottom_for_exiled_card(&mut game, alice, card);
        assert_eq!(game.effect_store.replacement_effects.effects().len(), baseline + 1);

        let in_hand = game
            .move_object(card, Zone::Hand, crate::events::cause::EventCause::from_game_rule())
            .expect("card moves to hand");
        assert_eq!(
            game.effect_store.replacement_effects.effects().len(),
            baseline,
            "the replacement ended with the exiled object"
        );
        let spell = game
            .move_object(in_hand, Zone::Stack, crate::events::cause::EventCause::from_game_rule())
            .expect("the card is cast later");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(spell, alice, &mut dm);
        let _ = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(spell),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        let moved = game.find_object_by_stable_id(stable_id).expect("card exists");
        assert_eq!(game.object(moved).unwrap().zone, Zone::Graveyard);
    }

    /// The permission to cast the card is "this turn": an uncast card loses
    /// the replacement at cleanup, while a card already cast keeps it for its
    /// spell, which it then replaces exactly once.
    #[test]
    fn test_followed_stack_replacement_expires_uncast_and_applies_once_when_cast() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let uncast = create_creature(&mut game, alice, Zone::Exile);
        game.effect_store.replacement_effects.clear_one_shot_effects();
        let baseline = game.effect_store.replacement_effects.effects().len();
        register_library_bottom_for_exiled_card(&mut game, alice, uncast);
        game.effect_store.replacement_effects.clear_one_shot_effects();
        assert_eq!(
            game.effect_store.replacement_effects.effects().len(),
            baseline,
            "an uncast card's replacement ends at cleanup"
        );

        let card = create_creature(&mut game, alice, Zone::Exile);
        let stable_id = game.object(card).expect("card should exist").stable_id;
        register_library_bottom_for_exiled_card(&mut game, alice, card);
        let spell = game
            .move_object(card, Zone::Stack, crate::events::cause::EventCause::from_game_rule())
            .expect("the card is cast");
        game.effect_store.replacement_effects.clear_one_shot_effects();
        assert_eq!(
            game.effect_store.replacement_effects.effects().len(),
            baseline + 1,
            "the spell keeps its replacement across cleanup"
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(spell, alice, &mut dm);
        let _ = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(spell),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        let moved = game.find_object_by_stable_id(stable_id).expect("card exists");
        assert_eq!(game.object(moved).unwrap().zone, Zone::Library);
        assert_eq!(
            game.effect_store.replacement_effects.effects().len(),
            baseline,
            "the replacement applied once"
        );
    }

    #[test]
    fn followed_stack_replacement_survives_runtime_checkpoint_before_and_after_cast() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = create_creature(&mut game, alice, Zone::Exile);
        let stable_id = game.object(card).unwrap().stable_id;
        register_library_bottom_for_exiled_card(&mut game, alice, card);
        let checkpoint = game.clone();
        game.move_object(card, Zone::Hand, crate::events::cause::EventCause::from_game_rule()).unwrap();
        game = checkpoint;
        let spell = game.move_object(card, Zone::Stack, crate::events::cause::EventCause::from_game_rule()).unwrap();
        let checkpoint = game.clone();
        game.move_object(spell, Zone::Exile, crate::events::cause::EventCause::from_game_rule()).unwrap();
        game = checkpoint;
        game.effect_store.replacement_effects.clear_one_shot_effects();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(spell, alice, &mut dm);
        execute_effect(&mut game, &crate::effect::Effect::move_to_zone(
            ChooseSpec::SpecificObject(spell), Zone::Graveyard, false,
        ), &mut ctx).unwrap();
        let moved = game.find_object_by_stable_id(stable_id).unwrap();
        assert_eq!(game.object(moved).unwrap().zone, Zone::Library);
        assert!(game.effect_store.replacement_effects.effects().is_empty());
    }

    /// Gandalf of the Secret Fire: the replacement exiles with time counters
    /// and only then grants suspend to the exiled card (CR 400.7, 702.62a).
    #[test]
    fn test_registered_zone_replacement_grants_suspend_after_exiling_with_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        )
        .with_counters(vec![(CounterType::Time, 3)])
        .with_linked_exile_follow_up(ironsmith_core::LinkedExileFollowUp::GainSuspendIfMissing);
        assert!(matches!(
            zone_replacement_action(
                creature,
                Some(Zone::Graveyard),
                Zone::Exile,
                None,
                false,
                None,
                vec![(CounterType::Time, 3)],
                Some(ironsmith_core::LinkedExileFollowUp::GainSuspendIfMissing),
            ),
            ReplacementAction::ExileWithSourceLinkCountersThen { .. }
        ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");
        let _ = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after replacement");
        assert_eq!(game.object(exiled_id).unwrap().zone, Zone::Exile);
        assert_eq!(game.counter_count(exiled_id, CounterType::Time), 3);
        let chars = game
            .current_characteristics(exiled_id)
            .expect("exiled card has characteristics");
        let suspend_triggers = chars
            .abilities
            .iter()
            .filter(|ability| {
                matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Triggered(triggered)
                        if matches!(
                            triggered.presentation_label,
                            Some(crate::ability::PresentationLabel::Keyword(
                                crate::ability::PresentationKeyword::Suspend
                            ))
                        )
                )
            })
            .count();
        assert_eq!(suspend_triggers, 2, "the exiled card gains suspend");
    }

    #[test]
    fn test_registered_zone_replacement_does_not_apply_to_nonmatching_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ReplacementApplyMode::OneShot,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        let _ = execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("replacement registration should succeed");

        let move_outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Hand,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve");
        assert!(
            move_outcome.status != OutcomeStatus::TargetInvalid,
            "expected move-to-hand effect to resolve on the creature"
        );
        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should still be findable after moving to hand");
        assert_eq!(
            game.object(moved_id)
                .expect("moved creature should exist")
                .zone,
            Zone::Hand
        );
    }

    #[test]
    fn persistent_leave_battlefield_replacement_survives_cleanup_and_exiles_any_destination() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice, Zone::Battlefield);
        let stable_id = game
            .object(creature)
            .expect("creature should exist")
            .stable_id;

        let effect = RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(creature),
            Some(Zone::Battlefield),
            None,
            Zone::Exile,
            ReplacementApplyMode::Resolution,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(creature, alice, &mut dm);
        execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .expect("persistent replacement registration should succeed");

        crate::turn::execute_cleanup_step(&mut game);

        execute_effect(
            &mut game,
            &crate::effect::Effect::move_to_zone(
                ChooseSpec::SpecificObject(creature),
                Zone::Hand,
                false,
            ),
            &mut ctx,
        )
        .expect("move effect should resolve through the replacement");

        let exiled_id = game
            .find_object_by_stable_id(stable_id)
            .expect("creature should remain findable after the replacement");
        assert_eq!(
            game.object(exiled_id)
                .expect("replaced creature should exist")
                .zone,
            Zone::Exile,
            "leave-battlefield replacement must survive cleanup and replace a move to hand"
        );
    }

    #[test]
    fn library_destination_replacement_honors_top_and_bottom_placement() {
        for (placement, expect_top) in [
            (ironsmith_core::ZoneReplacementLibraryPlacement::Top, true),
            (
                ironsmith_core::ZoneReplacementLibraryPlacement::Bottom,
                false,
            ),
        ] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let _sentinel = create_creature(&mut game, alice, Zone::Library);
            let moving = create_creature(&mut game, alice, Zone::Graveyard);
            let stable_id = game.object(moving).expect("moving card").stable_id;

            let effect = RegisterZoneReplacementEffect::new(
                ChooseSpec::SpecificObject(moving),
                Some(Zone::Graveyard),
                Some(Zone::Hand),
                Zone::Library,
                ReplacementApplyMode::OneShot,
            )
            .with_library_placement(placement);
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(moving, alice, &mut dm);
            execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
                .expect("library replacement registration");
            execute_effect(
                &mut game,
                &crate::effect::Effect::move_to_zone(
                    ChooseSpec::SpecificObject(moving),
                    Zone::Hand,
                    false,
                ),
                &mut ctx,
            )
            .expect("move through library replacement");

            let moved = game
                .find_object_by_stable_id(stable_id)
                .expect("moved card remains findable");
            let library = &game.player(alice).expect("player").library;
            let expected = if expect_top {
                library.last()
            } else {
                library.first()
            };
            assert_eq!(expected, Some(&moved));
        }
    }
}
