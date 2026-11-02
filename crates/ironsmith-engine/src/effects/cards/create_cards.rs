//! Generate real cards from an explicitly admitted catalog, without moving a
//! pre-existing object or invoking token-creation replacements.
use crate::cards::CardDefinition;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter, resolve_value};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::{
    TraitEventProcessingState, TraitEventResult,
    process_trait_event_with_execution_context_and_state,
};
use crate::events::{CreateCardsEvent, Event};
use crate::filter::ObjectFilterExt as _;
use crate::{game_state::GameState, object::Object, snapshot::ObjectSnapshot, zone::Zone};
use ironsmith_core::{CardCreationLibraryPosition, CardCreationSelection};

pub type CreateCardsEffect = ironsmith_core::CreateCardsEffect<CardDefinition>;

pub(crate) trait CardCreationDefinitionGraphExt {
    fn admitted_definitions(&self)->Result<Vec<&CardDefinition>,ExecutionError>;
    fn close_generated_definition(&self,definition:CardDefinition)->Result<CardDefinition,ExecutionError>;
}
impl CardCreationDefinitionGraphExt for CreateCardsEffect {
    fn admitted_definitions(&self) -> Result<Vec<&CardDefinition>, ExecutionError> {
        let Some(scope)=self.definition_scope else {
            if !self.definition_refs.is_empty() {
                return Err(ExecutionError::IncompleteEvidence("real-card references require an explicit scope".into()));
            }
            for definition in &self.definitions {
                super::definition_scope::validate_native_definition(definition)
                    .map_err(ExecutionError::IncompleteEvidence)?;
            }
            return Ok(self.definitions.iter().collect());
        };
        super::definition_scope::validate_native_scope_nodes(scope,&self.definitions)
            .map_err(ExecutionError::IncompleteEvidence)?;
        if self.definition_refs.iter().collect::<std::collections::HashSet<_>>().len()!=self.definition_refs.len(){
            return Err(ExecutionError::IncompleteEvidence("duplicate identity in real-card reference subset".into()));
        }
        self.definition_refs.iter().map(|id| {
            self.definitions.iter().find(|node|node.card.id==*id)
                .ok_or_else(||ExecutionError::IncompleteEvidence("real-card reference escapes its catalog".into()))
        }).collect()
    }

    fn close_generated_definition(&self, definition:CardDefinition)->Result<CardDefinition,ExecutionError>{
        let Some(scope)=self.definition_scope else{
            super::definition_scope::validate_native_definition(&definition)
                .map_err(ExecutionError::IncompleteEvidence)?;
            return Ok(definition);
        };
        super::definition_scope::bind_card_creation_scope_runtime(definition,scope,&self.definitions)
            .map_err(ExecutionError::IncompleteEvidence)
    }
}

impl EffectExecutor for CreateCardsEffect {
    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        match &self.selection {
            CardCreationSelection::Duplicate(target) => Some(target),
            _ => None,
        }
    }
    fn visit_card_definitions(&self, visitor: &mut dyn FnMut(&CardDefinition)) {
        for definition in &self.definitions {
            visitor(definition);
        }
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if let Some(scope)=self.definition_scope && self.definitions.is_empty() {
            let catalog=ctx.card_definition_scopes.iter().rev().find(|catalog|catalog.scope==scope)
                .ok_or_else(||ExecutionError::IncompleteEvidence("real-card reference has no lexical catalog".into()))?;
            let mut bound=self.clone();bound.definitions=catalog.definitions.clone();
            return bound.execute_with_outputs(game,ctx);
        }
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // Share the existing generated-object work budget and recursion
                // guard. This is resource accounting, never token game semantics.
                let (_, meter) = game.begin_token_resource_scope();
                let _guard =
                    crate::effects::tokens::resources::TokenInstructionGuard::enter(meter)?;
                if self.zone == Zone::Stack {
                    return Err(ExecutionError::InternalError("real-card generation onto the stack requires an explicit spell creation program".into()));
                }
                let replacement_definitions =
                    crate::effects::cards::capture_replacement_definitions(game)?;
                let player = resolve_player_filter(game, &self.player, ctx)?;
                if game.player(player).is_none() {
                    return Err(ExecutionError::PlayerNotFound(player));
                }
                let count = usize::try_from(resolve_value(game, &self.count, ctx)?.max(0))
                    .map_err(|_| {
                        ExecutionError::InternalError(
                            "card generation count exceeds address space".into(),
                        )
                    })?;
                let (duplicate,retained_duplicate) = match &self.selection {
                    CardCreationSelection::Duplicate(target) => {
                        let ids=resolve_objects_for_effect(game,ctx,target)?;
                        if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
                        match ids.first().and_then(|id|game.object(*id)) {
                            Some(object)=>{
                                let mut snapshot=ObjectSnapshot::from_object(object,game);
                                if let Some(name)=game.current_name(object.id){snapshot.name=name;}
                                // Exact retained identity, not a name search or a
                                // reconstruction from current copiable values.
                                let retained=if object.kind==crate::object::ObjectKind::Card {
                                    object.card.and_then(|id|game.retained_card_definition(id))
                                        .filter(|definition|!definition.card.is_token && definition_matches_current_names(definition,&snapshot)).cloned()
                                }else{None};
                                (Some(snapshot),retained)
                            },None=>(None,None),
                        }
                    },_=>(None,None),
                };
                let chosen_name = match &self.selection {
                    CardCreationSelection::ChosenName { tag } => {
                        let names = ctx.get_tagged_all(tag.as_str()).ok_or_else(|| {
                            ExecutionError::IncompleteEvidence(
                                "card generation has no local chosen-name receipt".into(),
                            )
                        })?;
                        let [chosen] = names.as_slice() else {
                            return Err(ExecutionError::IncompleteEvidence(
                                "card generation requires one exact chosen-name receipt".into(),
                            ));
                        };
                        if chosen.name.trim().is_empty() {
                            return Err(ExecutionError::IncompleteEvidence(
                                "card generation chosen-name receipt is empty".into(),
                            ));
                        }
                        Some(chosen.name.clone())
                    }
                    _ => None,
                };
                let filter_ctx = ctx.filter_context(game);
                let mut candidates = Vec::new();
                let mut names = std::collections::BTreeSet::new();
                for definition in self.admitted_definitions()? {
                    if !names.insert(definition.card.name.clone()) {
                        continue;
                    }
                    let matches = match &self.selection {
                        CardCreationSelection::Named(name) => {
                            crate::filter::names_match(&definition.card.name, name)
                        }
                        CardCreationSelection::ChosenName { .. } => {
                            chosen_name.as_ref().is_some_and(|name| {
                                crate::filter::names_match(&definition.card.name, name)
                            })
                        }
                        CardCreationSelection::Duplicate(_) => duplicate.as_ref().is_some_and(|object|definition_matches_current_names(definition,object)),
                        CardCreationSelection::RandomMatching { filter, .. } => {
                            let mut admitted = (*definition).clone();
                            admitted.card.is_token = false;
                            let candidate = Object::from_card_definition(
                                crate::ids::ObjectId::from_raw(0),
                                &admitted,
                                player,
                                self.zone,
                            );
                            filter.matches_snapshot(
                                &ObjectSnapshot::from_object(&candidate, game),
                                &filter_ctx,
                                game,
                            )
                        }
                    };
                    if matches {
                        candidates.push(definition);
                    }
                }
                if let Some(definition)=retained_duplicate.as_ref(){
                    candidates.clear();candidates.push(definition);
                }
                if candidates.is_empty() && count>0 && matches!(self.selection,CardCreationSelection::Duplicate(_))
                    && duplicate.as_ref().is_some_and(|object|!object.face_down && !object.name.trim().is_empty()) {
                    return Err(ExecutionError::IncompleteEvidence("duplicate current identity has no admitted real-card definition".into()));
                }
                if candidates.is_empty() && chosen_name.is_some() {
                    return Err(ExecutionError::IncompleteEvidence(format!(
                        "chosen name {} has no supplied real-card definition",
                        chosen_name.as_deref().unwrap_or_default()
                    )));
                }
                // Nameless objects and absent targets provide no real-card name.
                if candidates.is_empty() || count == 0 {
                    return Ok(CompletedEffectOutputs::aggregate_only(creation_receipt(
                        player,
                        Vec::new(),
                        Vec::new(),
                    )));
                }
                let random = matches!(self.selection, CardCreationSelection::RandomMatching { .. });
                let independent = matches!(
                    self.selection,
                    CardCreationSelection::RandomMatching {
                        independent: true,
                        ..
                    }
                );
                if random {
                    game.shuffle_slice(&mut candidates);
                }
                let iterations = if independent { count } else { 1 };
                game.reserve_token_repetition_work(iterations)?;
                let amount = if independent {
                    1
                } else {
                    u32::try_from(count).map_err(|_| {
                        ExecutionError::InternalError(
                            "card generation exceeds event quantity".into(),
                        )
                    })?
                };
                let mut outputs = CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved());
                let mut children = Vec::new();
                let mut created = Vec::new();
                let mut snapshots = Vec::new();
                for _ in 0..iterations {
                    if random && independent {
                        game.shuffle_slice(&mut candidates);
                    }
                    let selected_definition=if retained_duplicate.is_some(){
                        let definition=(*candidates[0]).clone();
                        super::definition_scope::validate_native_definition(&definition)
                            .map_err(ExecutionError::IncompleteEvidence)?;
                        definition
                    }else{self.close_generated_definition((*candidates[0]).clone())?};
                    let proposal = CreateCardsEvent {
                        operation_label: self.operation_label.clone(),
                        selection_finalized: false,
                        replacement_source: None,
                        card_name: candidates[0].card.name.clone(),
                        amount,
                        player,
                        source: ctx.source,
                        zone: self.zone,
                    };
                    let child = execute_creation_batch(
                        self,
                        game,
                        ctx,
                        proposal,
                        &selected_definition,
                        &replacement_definitions,
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    let original = child.outcome.instruction_result();
                    created.extend_from_slice(original.output_objects());
                    snapshots.extend(
                        original
                            .affected_object_memory()
                            .unwrap_or_default()
                            .iter()
                            .cloned(),
                    );
                    children.push(child.outcome.clone());
                    outputs.retain_owned_child(child);
                }
                let original = creation_receipt(player, created, snapshots);
                Ok(
                    outputs.project_aggregate(EffectOutcome::aggregate_replacement_outcomes(
                        original, children,
                    )),
                )
            },
        )
    }
}

fn definition_matches_current_names(definition:&CardDefinition,object:&ObjectSnapshot)->bool{
    if object.face_down || object.name.trim().is_empty(){return false;}
    [Some(definition.card.name.as_str()),definition.card.other_face_name.as_deref()].into_iter().flatten().any(|candidate|{
        [Some(object.name.as_str()),object.split_other_half_name()].into_iter().flatten().any(|name|crate::filter::names_match(candidate,name))
    })
}

fn execute_creation_batch(
    effect: &CreateCardsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    proposal: CreateCardsEvent,
    selected_definition:&CardDefinition,
    captured: &crate::effects::cards::CapturedReplacementDefinitions,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let event = Event::new_with_provenance(proposal, ctx.provenance);
    let processed = process_trait_event_with_execution_context_and_state(
        game,
        event,
        ctx,
        &mut TraitEventProcessingState::default(),
    )?;
    crate::effects::replacement::execute_event_expansion_with_outputs(
        game,
        ctx,
        processed,
        |game, ctx, result| match result {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                let event = event
                    .inner()
                    .as_any()
                    .downcast_ref::<CreateCardsEvent>()
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "card creation proposal changed operation type".into(),
                        )
                    })?;
                let source_definition = event
                    .replacement_source
                    .map(|source| captured.resolve(source, &event.card_name))
                    .transpose()?;
                let definition=source_definition.as_deref().or_else(||{
                    crate::filter::names_match(&selected_definition.card.name,&event.card_name).then_some(selected_definition)
                }).ok_or_else(||ExecutionError::IncompleteEvidence(format!("card creation changed to an unproven identity named {}",event.card_name)))?;
                let mut definition=definition.clone();
                definition.card.is_token = false;
                game.reserve_token_creation(u128::from(event.amount))?;
                if event.zone == Zone::Battlefield {
                    let mut creation = event.clone();
                    creation.selection_finalized = true;
                    return crate::effects::cards::execute_real_card_battlefield_creation(
                        game,
                        ctx,
                        &definition,
                        &creation,
                    );
                }
                let mut ids = Vec::new();
                ids.try_reserve(event.amount as usize).map_err(|_| {
                    ExecutionError::InternalError("cannot reserve generated-card batch".into())
                })?;
                for _ in 0..event.amount {
                    game.commit_token_resource_slot()?;
                    let id =
                        game.create_object_from_definition(&definition, event.player, event.zone);
                    if event.zone == Zone::Library {
                        match effect.library_position {
                            CardCreationLibraryPosition::Top => {
                                game.move_library_card_to_top(
                                    event.player,
                                    id,
                                    "create real card at top of library",
                                );
                            }
                            CardCreationLibraryPosition::Bottom => {
                                game.move_library_card_to_bottom(
                                    event.player,
                                    id,
                                    "create real card at bottom of library",
                                );
                            }
                        }
                    }
                    ids.push(id);
                }
                game.mark_hidden_cards_publicly_revealed(&ids);
                let objects = ids
                    .iter()
                    .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
                    .collect();
                Ok(CompletedEffectOutputs::aggregate_only(creation_receipt(
                    event.player,
                    ids,
                    objects,
                )))
            }
            TraitEventResult::Prevented => Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::prevented(),
            )),
            TraitEventResult::Replaced {
                effects,
                source,
                controller,
                context,
                ..
            } => {
                let child = crate::effects::replacement::execute_replacement_payload_with_outputs(
                    game,
                    ctx,
                    &effects,
                    source,
                    controller,
                    &context,
                    None,
                    None,
                    Vec::new(),
                )?;
                let receipt = EffectOutcome::aggregate_replacement_outcomes(
                    EffectOutcome::replaced(),
                    [child.outcome.clone()],
                );
                Ok(child.project_aggregate(receipt))
            }
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                if ctx.decision_maker.awaiting_choice() {
                    Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ))
                } else {
                    Err(ExecutionError::InternalError(
                        "card creation suspended without a decision".into(),
                    ))
                }
            }
            TraitEventResult::Expanded { .. } => unreachable!("expansion owner removes wrappers"),
        },
        |_game, _context, original| {
            let objects = original
                .instruction_result()
                .affected_object_memory()
                .unwrap_or_default()
                .to_vec();
            Ok(crate::effects::replacement::ReplacementProgramBindings {
                targets: None,
                object_tags: vec![("it".into(), objects.clone()), ("__it__".into(), objects)],
            })
        },
    )
}

fn creation_receipt(
    player: crate::ids::PlayerId,
    ids: Vec<crate::ids::ObjectId>,
    objects: Vec<ObjectSnapshot>,
) -> EffectOutcome {
    EffectOutcome::count(ids.len() as i64).with_execution_facts([
        ExecutionFact::ResultObjects(ids.clone()),
        ExecutionFact::AffectedObjects(ids),
        ExecutionFact::ResultObjectMemory(objects.clone()),
        ExecutionFact::AffectedObjectMemory(objects.clone()),
        ExecutionFact::PlayerAffectedObjectMemory(vec![(player, objects)]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cards::builders::CardDefinitionBuilder,
        ids::{CardId, PlayerId},
        object::ObjectKind,
        target::{ChooseSpec, ObjectFilter, PlayerFilter},
        types::CardType,
    };
    fn definition(name: &str, kind: CardType) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![kind])
            .build()
    }
    fn effect(
        definitions: Vec<CardDefinition>,
        name: &str,
        zone: Zone,
        count: i32,
    ) -> CreateCardsEffect {
        CreateCardsEffect {
            definitions,
            definition_scope: None,
            definition_refs: Vec::new(),
            operation_label: None,
            selection: CardCreationSelection::Named(name.into()),
            count: count.into(),
            player: PlayerFilter::You,
            zone,
            library_position: CardCreationLibraryPosition::Top,
        }
    }
    #[test]
    fn generated_cards_are_owned_by_actor_and_keep_complete_definition() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let owner = PlayerId::from_index(1);
        let controller = PlayerId::from_index(0);
        let target = CardDefinitionBuilder::new(CardId::new(), "Supplied real card")
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![crate::effect::Effect::gain_life(4)])
            .build();
        let mut action = effect(vec![target.clone()], "Supplied real card", Zone::Hand, 2);
        action.player = PlayerFilter::Specific(owner);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), controller);
        let outcome = action.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.as_count(), Some(2));
        assert_eq!(outcome.output_objects().len(), 2);
        assert_eq!(
            outcome
                .action_objects(ironsmith_core::PriorEffectAction::Affected, Some(owner))
                .len(),
            2
        );
        for id in outcome.output_objects() {
            let card = game.object(*id).unwrap();
            assert_eq!(card.kind, ObjectKind::Card);
            assert_eq!(card.owner, owner);
            assert_eq!(card.card, Some(target.card.id));
            assert!(card.spell_effect.is_some());
        }
        assert!(game.player(controller).unwrap().hand.is_empty());
    }
    #[test]
    fn checklist_presentation_does_not_turn_generated_cards_into_tokens() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let target = CardDefinitionBuilder::new(CardId::new(), "Checklist real card")
            .token()
            .card_types(vec![CardType::Sorcery])
            .build();
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), PlayerId::from_index(0));
        let out = effect(vec![target], "Checklist real card", Zone::Hand, 1)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.object(out.output_objects()[0]).unwrap().kind,
            ObjectKind::Card
        );
    }
    #[test]
    fn missing_identity_exports_known_empty_receipt_without_fallback() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), PlayerId::from_index(0));
        let out = effect(vec![], "Unsupplied identity", Zone::Hand, 2)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(out.as_count(), Some(0));
        assert_eq!(out.affected_object_memory(), Some([].as_slice()));
        assert_eq!(out.result_objects(), Some([].as_slice()));
    }
    #[test]
    fn random_batch_selects_one_matching_identity_for_all_copies() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let pool = vec![
            definition("First creature", CardType::Creature),
            definition("Second creature", CardType::Creature),
            definition("Excluded artifact", CardType::Artifact),
        ];
        let mut action = effect(pool, "unused", Zone::Hand, 8);
        action.selection = CardCreationSelection::RandomMatching {
            // This is an identity predicate over a supplied catalog, not
            // a battlefield-object selector (ObjectFilter::creature()).
            filter: ObjectFilter { card_types:vec![CardType::Creature],..Default::default() },
            independent: false,
        };
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), PlayerId::from_index(0));
        let out = action.execute(&mut game, &mut ctx).unwrap();
        let names: std::collections::BTreeSet<_> = out
            .output_objects()
            .iter()
            .map(|id| game.object(*id).unwrap().name.as_str())
            .collect();
        assert_eq!(names.len(), 1);
        assert!(!names.contains("Excluded artifact"));
        assert_eq!(out.as_count(), Some(8));
    }
    #[test]
    fn duplicate_missing_actual_card_identity_does_not_generate_type_named_token() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let token = CardDefinitionBuilder::new(CardId::new(), "Soldier")
            .token()
            .card_types(vec![CardType::Creature])
            .build();
        let id =
            game.create_object_from_definition(&token, PlayerId::from_index(0), Zone::Battlefield);
        let mut action = effect(
            vec![definition("Actual supplied card", CardType::Creature)],
            "unused",
            Zone::Hand,
            1,
        );
        action.selection = CardCreationSelection::Duplicate(ChooseSpec::SpecificObject(id));
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), PlayerId::from_index(0));
        assert!(matches!(action.execute(&mut game,&mut ctx),Err(ExecutionError::IncompleteEvidence(_))));
        assert!(game.player(PlayerId::from_index(0)).unwrap().hand.is_empty());
    }
    #[test]
    fn library_bottom_creation_preserves_existing_top() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let existing = game.create_object_from_definition(
            &definition("Existing top", CardType::Sorcery),
            player,
            Zone::Library,
        );
        let mut action = effect(
            vec![definition("Generated bottom", CardType::Sorcery)],
            "Generated bottom",
            Zone::Library,
            2,
        );
        action.library_position = CardCreationLibraryPosition::Bottom;
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), player);
        let out = action.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            game.player(player).unwrap().library.last().copied(),
            Some(existing)
        );
        assert_eq!(out.as_count(), Some(2));
    }
    #[test]
    fn restart_retains_generated_real_cards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), player);
        effect(
            vec![definition("Generated restart identity", CardType::Sorcery)],
            "Generated restart identity",
            Zone::Hand,
            1,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        game.restart_game(player, &[]);
        assert_eq!(
            game.objects_in_deterministic_order()
                .into_iter()
                .filter(|object| object.name.as_str() == "Generated restart identity")
                .count(),
            1
        );
    }
    #[test]
    fn definition_substitution_preserves_actor_count_destination_and_library_position() {
        use crate::static_abilities::{CardCreationSelectionReplacement,StaticAbility};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let replacement=CardDefinitionBuilder::new(CardId::new(),"Replacement identity")
            .card_types(vec![CardType::Sorcery]).with_spell_effect(vec![crate::Effect::gain_life(9)]).build();
        // The replacement provider, rather than the initiating creator, owns
        // the actual supplied definition. Prototype actor/count/zone are inert.
        let rule=CardCreationSelectionReplacement {
            operation_label:Some("recorded operation".into()),player:PlayerFilter::Any,
            replacement:crate::Effect::new(effect(vec![replacement.clone()],"Replacement identity",Zone::Hand,1)),
            display:"Use the provider's selected identity instead.".into(),
        };
        let source_definition=CardDefinitionBuilder::new(CardId::new(),"Creation rule source")
            .card_types(vec![CardType::Artifact]).with_ability(crate::Ability::static_ability(StaticAbility::new(rule))).build();
        let source=game.create_object_from_definition(&source_definition,a,Zone::Battlefield);
        let existing_top=game.create_object_from_definition(&definition("Existing actor library top",CardType::Sorcery),b,Zone::Library);
        let pool = vec![
            definition("Original identity", CardType::Sorcery),
            // Same printed name is deliberately not the provider's identity.
            definition("Replacement identity", CardType::Sorcery),
        ];
        let mut action = effect(pool, "Original identity", Zone::Library, 3);
        action.player = PlayerFilter::Specific(b);
        action.operation_label = Some("recorded operation".into());
        action.library_position = CardCreationLibraryPosition::Bottom;
        let mut ctx = ExecutionContext::new_default(source, a);
        let out = action.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(out.as_count(), Some(3));
        for id in out.output_objects() {
            let object = game.object(*id).unwrap();
            assert_eq!(object.name.as_str(), "Replacement identity");
            assert_eq!(object.owner, b);
            assert_eq!(object.zone, Zone::Library);
            assert_eq!(object.card,Some(replacement.card.id),"same-name creator catalog cannot replace the source-owned definition");
        }
        assert_eq!(
            out.action_objects(ironsmith_core::PriorEffectAction::Affected, Some(b))
                .len(),
            3
        );
        assert_eq!(game.player(b).unwrap().library.last().copied(),Some(existing_top),"substitution preserves original bottom placement");
        assert!(game.player(a).unwrap().hand.is_empty());
        assert!(game.player(a).unwrap().library.is_empty());
        assert!(
            !out.events
                .iter()
                .any(|event| event.downcast::<crate::events::ZoneChangeEvent>().is_some())
        );
    }
    #[test]
    fn typed_creation_replacement_does_not_rewrite_other_named_operations() {
        use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(
            &definition("Creation rule source", CardType::Artifact),
            a,
            Zone::Battlefield,
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                a,
                crate::events::WouldCreateCardsMatcher {
                    operation_label: Some("different operation".into()),
                    player: None,
                },
                ReplacementAction::Modify(EventModification::ReplaceCardDefinition(
                    "Replacement identity".into(),
                )),
            ),
        );
        let mut action = effect(
            vec![
                definition("Original identity", CardType::Sorcery),
                definition("Replacement identity", CardType::Sorcery),
            ],
            "Original identity",
            Zone::Hand,
            1,
        );
        action.operation_label = Some("this operation".into());
        let mut ctx = ExecutionContext::new_default(source, a);
        let out = action.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            game.object(out.output_objects()[0]).unwrap().name.as_str(),
            "Original identity"
        );
    }
    #[test]
    fn creation_receipt_drives_existing_named_completion_with_actual_created_cards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(
            &definition("Instruction source", CardType::Artifact),
            a,
            Zone::Battlefield,
        );
        let mut ctx = ExecutionContext::new_default(source, a);
        let created = effect(
            vec![definition("Public known identity", CardType::Sorcery)],
            "Public known identity",
            Zone::Hand,
            2,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        let ids = created.output_objects().to_vec();
        let id = ironsmith_core::EffectId(401);
        ctx.store_outcome(id, created);
        let completion = crate::effects::PerformNamedActionEffect {
            label: "sample operation".into(),
            performer: None,
            objects: None,
            prior_objects: Some((id, ironsmith_core::PriorEffectAction::Affected)),
        }
        .execute(&mut game, &mut ctx)
        .unwrap();
        let event = completion
            .events
            .iter()
            .find_map(|event| event.downcast::<crate::events::NamedActionEvent>())
            .expect("generic named completion");
        assert_eq!(event.amount, 2);
        assert_eq!(
            event
                .objects
                .iter()
                .map(|object| object.object_id)
                .collect::<Vec<_>>(),
            ids
        );
        assert!(
            event
                .objects
                .iter()
                .all(|object| object.name == "Public known identity")
        );
    }

    #[test]
    fn duplicate_back_face_name_selects_the_supplied_physical_front_card() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let front_id=CardId::new();let back_id=CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Supplied physical front")
            .card_types(vec![CardType::Creature]).other_face(back_id)
            .other_face_name("Supplied reverse identity")
            .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike).build();
        let back=CardDefinitionBuilder::new(back_id,"Supplied reverse identity")
            .card_types(vec![CardType::Creature]).other_face(front_id)
            .other_face_name("Supplied physical front")
            .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike).build();
        game.register_linked_face_definition(&front);game.register_linked_face_definition(&back);
        let original=game.create_object_from_definition(&front,a,Zone::Battlefield);
        assert!(game.transform_permanent(original).unwrap(),"the target is the reverse of this physical front, not an independent same-name card");
        assert_eq!(game.current_name(original).as_deref(),Some("Supplied reverse identity"));
        assert_eq!(game.object(original).unwrap().card,Some(front_id),"transforming does not replace the physical identity anchor");
        let mut action = effect(vec![front], "unused", Zone::Hand, 1);
        action.selection = CardCreationSelection::Duplicate(ChooseSpec::SpecificObject(original));
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), a);
        let out = action.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(out.as_count(), Some(1));
        assert_eq!(
            game.object(out.output_objects()[0]).unwrap().name.as_str(),
            "Supplied physical front"
        );
    }
    #[test]
    fn independent_random_generation_resamples_each_card_from_the_supplied_pool() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        game.set_random_seed(23);
        let mut action = effect(
            vec![
                definition("First admitted identity", CardType::Sorcery),
                definition("Second admitted identity", CardType::Sorcery),
            ],
            "unused",
            Zone::Hand,
            128,
        );
        action.selection = CardCreationSelection::RandomMatching {
            filter: ObjectFilter::default(),
            independent: true,
        };
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), PlayerId::from_index(0));
        let out = action.execute(&mut game, &mut ctx).unwrap();
        let names: std::collections::BTreeSet<_> = out
            .output_objects()
            .iter()
            .map(|id| game.object(*id).unwrap().name.as_str())
            .collect();
        assert_eq!(out.as_count(), Some(128));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn independently_supplied_replacement_definition_is_not_taken_from_creator_pool() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let supplied = CardDefinitionBuilder::new(CardId::new(), "Provider owned identity")
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![crate::effect::Effect::gain_life(9)])
            .build();
        let template = effect(
            vec![supplied.clone()],
            "Provider owned identity",
            Zone::Hand,
            1,
        );
        let rule = crate::static_abilities::misc::CardCreationSelectionReplacement {
            operation_label: Some("coherent operation".into()),
            player: PlayerFilter::Any,
            replacement: crate::effect::Effect::new(template),
            display: "selection-only supplied definition replacement".into(),
        };
        let source = CardDefinitionBuilder::new(CardId::new(), "Independent replacement provider")
            .card_types(vec![CardType::Artifact])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::new(rule),
            ))
            .build();
        game.create_object_from_definition(&source, b, Zone::Battlefield);
        let mut action = effect(
            vec![definition("Creator owned identity", CardType::Sorcery)],
            "Creator owned identity",
            Zone::Hand,
            2,
        );
        action.operation_label = Some("coherent operation".into());
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), a);
        let out = action.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(out.as_count(), Some(2));
        for id in out.output_objects() {
            let object = game.object(*id).unwrap();
            assert_eq!(object.card, Some(supplied.card.id));
            assert_eq!(object.owner, a);
            assert!(object.spell_effect.is_some());
        }
    }
    #[test]
    fn independent_replacement_sources_with_same_name_retain_their_own_definitions() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let mut expected = Vec::new();
        for index in 0..2 {
            let supplied = CardDefinitionBuilder::new(CardId::new(), "Shared replacement name")
                .card_types(vec![CardType::Sorcery])
                .with_spell_effect(vec![crate::effect::Effect::gain_life(index + 1)])
                .build();
            expected.push(supplied.card.id);
            let rule = crate::static_abilities::misc::CardCreationSelectionReplacement {
                operation_label: Some(format!("independent operation {index}")),
                player: PlayerFilter::Any,
                replacement: crate::effect::Effect::new(effect(
                    vec![supplied],
                    "Shared replacement name",
                    Zone::Hand,
                    1,
                )),
                display: "exact provider definition replacement".into(),
            };
            let source =
                CardDefinitionBuilder::new(CardId::new(), format!("Replacement provider {index}"))
                    .card_types(vec![CardType::Artifact])
                    .with_ability(crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::new(rule),
                    ))
                    .build();
            game.create_object_from_definition(&source, a, Zone::Battlefield);
        }
        for (index, card) in expected.into_iter().enumerate() {
            let mut action = effect(
                vec![definition("Original creator name", CardType::Sorcery)],
                "Original creator name",
                Zone::Hand,
                1,
            );
            action.operation_label = Some(format!("independent operation {index}"));
            let mut ctx = ExecutionContext::new_default(game.new_object_id(), a);
            let out = action.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(
                game.object(out.output_objects()[0]).unwrap().card,
                Some(card)
            );
        }
    }

    #[test]
    fn chosen_name_uses_each_exact_local_choice_and_allows_repeated_names() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(
            &definition("Name selection source", CardType::Artifact),
            a,
            Zone::Battlefield,
        );
        game.set_chosen_named_option(source, "Old unrelated name".into());
        let mut ctx = ExecutionContext::new_default(source, a);
        let mut create = effect(
            vec![
                definition("First chosen identity", CardType::Sorcery),
                definition("Second chosen identity", CardType::Sorcery),
                definition("Old unrelated name", CardType::Sorcery),
            ],
            "unused",
            Zone::Hand,
            1,
        );
        create.selection = CardCreationSelection::ChosenName {
            tag: "exact-local-name".into(),
        };
        let mut actual = Vec::new();
        for name in [
            "First chosen identity",
            "Second chosen identity",
            "First chosen identity",
        ] {
            let mut choice = crate::effects::ChooseCardNameEffect::new(
                PlayerFilter::You,
                None,
                "exact-local-name",
            );
            choice.options = vec![name.into()];
            choice.execute(&mut game, &mut ctx).unwrap();
            let out = create.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(out.as_count(), Some(1));
            actual.push(
                game.object(out.output_objects()[0])
                    .unwrap()
                    .name
                    .to_string(),
            );
        }
        assert_eq!(
            actual,
            vec![
                "First chosen identity",
                "Second chosen identity",
                "First chosen identity"
            ]
        );
    }
    #[test]
    fn missing_chosen_name_receipt_does_not_fall_back_to_persistent_source_names() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(
            &definition("Name selection source", CardType::Artifact),
            a,
            Zone::Battlefield,
        );
        game.set_chosen_named_option(source, "Previously selected identity".into());
        let mut create = effect(
            vec![definition(
                "Previously selected identity",
                CardType::Sorcery,
            )],
            "unused",
            Zone::Hand,
            1,
        );
        create.selection = CardCreationSelection::ChosenName {
            tag: "missing-current-choice".into(),
        };
        let mut ctx = ExecutionContext::new_default(source, a);
        assert!(matches!(
            create.execute(&mut game, &mut ctx),
            Err(ExecutionError::IncompleteEvidence(_))
        ));
        assert!(game.player(a).unwrap().hand.is_empty());
    }
    #[test]
    fn chosen_name_without_its_supplied_definition_fails_closed() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(
            &definition("Name selection source", CardType::Artifact),
            a,
            Zone::Battlefield,
        );
        let mut ctx = ExecutionContext::new_default(source, a);
        let mut choice =
            crate::effects::ChooseCardNameEffect::new(PlayerFilter::You, None, "exact-name");
        choice.options = vec!["Unsupplied chosen identity".into()];
        choice.execute(&mut game, &mut ctx).unwrap();
        let mut create = effect(
            vec![definition("Different supplied identity", CardType::Sorcery)],
            "unused",
            Zone::Hand,
            1,
        );
        create.selection = CardCreationSelection::ChosenName {
            tag: "exact-name".into(),
        };
        assert!(matches!(
            create.execute(&mut game, &mut ctx),
            Err(ExecutionError::IncompleteEvidence(_))
        ));
        assert!(game.player(a).unwrap().hand.is_empty());
    }
}

#[cfg(test)]
mod native_creation_evidence_fixture_tests {
    use super::*;
    use crate::{CardDefinitionBuilder,CardId,CardType,PlayerId};
    #[test]
    fn raw_name_substitution_without_a_source_owned_definition_still_fails_closed() {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let player=PlayerId(0);
        let source=game.create_object_from_definition(&CardDefinitionBuilder::new(CardId::new(),"Raw replacement source").card_types(vec![CardType::Artifact]).build(),player,Zone::Battlefield);
        game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
            source,player,crate::events::WouldCreateCardsMatcher{operation_label:None,player:None},
            crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::ReplaceCardDefinition("Unproven identity".into()))));
        let action=CreateCardsEffect { definitions:vec![CardDefinitionBuilder::new(CardId::new(),"Original identity").build()],definition_scope:None,definition_refs:vec![],operation_label:None,
            selection:CardCreationSelection::Named("Original identity".into()),count:1.into(),player:ironsmith_core::PlayerFilter::You,zone:Zone::Hand,library_position:CardCreationLibraryPosition::Top };
        let result=action.execute(&mut game,&mut ExecutionContext::new_default(source,player));
        assert!(matches!(result,Err(ExecutionError::IncompleteEvidence(_))));
        assert!(game.player(player).unwrap().hand.is_empty());
    }
    #[test]
    fn an_independent_real_reverse_named_card_is_not_an_unrelated_supplied_front() {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let player=PlayerId(0);
        let reverse=CardDefinitionBuilder::new(CardId::new(),"Independent reverse identity").card_types(vec![CardType::Creature]).build();
        let target=game.create_object_from_definition(&reverse,player,Zone::Battlefield);
        let unrelated=CardDefinitionBuilder::new(CardId::new(),"Unrelated supplied front").card_types(vec![CardType::Creature])
            .other_face_name("Independent reverse identity").linked_face_layout(crate::card::LinkedFaceLayout::TransformLike).build();
        let action=CreateCardsEffect { definitions:vec![unrelated],definition_scope:None,definition_refs:vec![],operation_label:None,
            selection:CardCreationSelection::Duplicate(crate::target::ChooseSpec::SpecificObject(target)),count:1.into(),player:ironsmith_core::PlayerFilter::You,zone:Zone::Hand,library_position:CardCreationLibraryPosition::Top };
        let result=action.execute(&mut game,&mut ExecutionContext::new_default(target,player)).unwrap();
        let created=game.object(result.output_objects()[0]).unwrap();
        assert_eq!(created.card,Some(reverse.card.id));assert_eq!(created.name.as_str(),"Independent reverse identity");
    }
}
