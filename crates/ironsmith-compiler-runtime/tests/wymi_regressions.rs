use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::DecisionMaker;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}

struct Choose(usize);
impl DecisionMaker for Choose {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.options.iter().any(|option| option.description == "3/3") {
            vec![self.0]
        } else {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .take(ctx.min)
                .map(|option| option.index)
                .collect()
        }
    }
}

fn primal_clay_choice_types_are_copiable_and_survive_artifact_roundtrip_inner() {
    for definition in definitions(
        "Primal Clay",
        "Mana cost: {4}\nType: Artifact Creature — Shapeshifter\nPower/Toughness: */*\nAs this creature enters, it becomes your choice of a 3/3 artifact creature, a 2/2 artifact creature with flying, or a 1/6 Wall artifact creature with defender in addition to its other types.",
    ) {
        for (index, power, toughness) in [(0, 3, 3), (1, 2, 2), (2, 1, 6)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            let id = game.create_object_from_definition(&definition, player, Zone::Hand);
            let receipt = game
                .move_object_with_etb_processing_with_dm(id, Zone::Battlefield, &mut Choose(index))
                .unwrap();
            assert!(!receipt.pending);
            assert!(receipt.programs.is_empty());
            let entry = receipt.original.into_result().unwrap();
            let object = game.object(entry.new_id).unwrap();
            assert_eq!(object.power(), Some(power));
            assert_eq!(object.toughness(), Some(toughness));
            assert!(object.card_types.contains(&CardType::Artifact));
            assert!(object.card_types.contains(&CardType::Creature));
            assert_eq!(object.subtypes.contains(&Subtype::Wall), index == 2);
            let copied =
                ironsmith::snapshot::ObjectSnapshot::from_object(object, &game).copiable_values;
            assert_eq!(copied.subtypes.contains(&Subtype::Wall), index == 2);
            assert_eq!(
                game.object_has_ability(
                    entry.new_id,
                    &ironsmith::static_abilities::StaticAbility::flying()
                ),
                index == 1
            );
            assert_eq!(
                game.object_has_ability(
                    entry.new_id,
                    &ironsmith::static_abilities::StaticAbility::defender()
                ),
                index == 2
            );
        }
    }
}

fn compound_entry_copy_preserves_name_legendary_and_return_ability_inner() {
    for definition in definitions(
        "Sakashima the Impostor",
        "Mana cost: {2}{U}{U}\nType: Legendary Creature — Human Rogue\nPower/Toughness: 3/1\nYou may have Sakashima the Impostor enter as a copy of any creature on the battlefield, except its name is Sakashima the Impostor, it's legendary in addition to its other types, and it has \"{2}{U}{U}: Return Sakashima the Impostor to its owner's hand at the beginning of the next end step.\"",
    ) {
        let spec = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => ability.enter_as_copy_as_enters(),
                _ => None,
            })
            .expect("entry copy ability");
        assert_eq!(
            spec.name_override.as_deref(),
            Some("Sakashima the Impostor")
        );
        assert!(
            spec.added_supertypes
                .contains(&ironsmith::types::Supertype::Legendary)
        );
        assert_eq!(spec.added_abilities.len(), 1);
        assert!(matches!(
            spec.added_abilities[0].kind,
            AbilityKind::Activated(_)
        ));
    }
}

fn noted_name_copy_uses_printed_card_even_when_associated_object_is_face_down_inner() {
    use ironsmith::effects::{CastTaggedEffect, EffectContext, EffectExecutor};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::tag::TagKey;
    let text = "Mana cost: {2}\nType: Creature\nPower/Toughness: 3/3\nWhenever this creature deals combat damage to a player, you may copy the card with the noted name. You may cast the copy without paying its mana cost.";
    fn find_copy(effect: &ironsmith::effect::Effect) -> Option<CastTaggedEffect> {
        if let Some(copy) = effect.downcast_ref::<CastTaggedEffect>() {
            return Some(copy.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_copy(child);
            }
        });
        found
    }
    for definition in definitions("Noted-name trigger", text) {
        let copy = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(trigger) => trigger
                    .effects
                    .all_effects()
                    .into_iter()
                    .find_map(find_copy),
                _ => None,
            })
            .expect("cast a named definition");
        assert!(copy.copy_named_definition);
        let printed = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Remembered creature",
            "Mana cost: {5}\nType: Creature — Bird\nPower/Toughness: 4/5\nFlying",
            false,
        )
        .unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(0);
        game.register_linked_face_definition(&printed);
        let id = game.create_object_from_definition(&printed, player, Zone::Battlefield);
        let note = ObjectSnapshot::from_object(game.object(id).unwrap(), &game);
        assert!(game.object_mut(id).unwrap().apply_face_down_cast_overlay());
        assert_eq!(game.object(id).unwrap().power(), Some(2));
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("__noted_card_name__"), vec![note]);
        let mut ctx = EffectContext::new(id, player, &mut dm).with_tagged_objects(tags);
        copy.execute(&mut game, &mut ctx).unwrap();
        let spell = game
            .object(game.stack.last().expect("cast copy").object_id)
            .unwrap();
        assert_eq!(spell.name, "Remembered creature");
        assert_eq!(spell.power(), Some(4));
        assert_eq!(spell.toughness(), Some(5));
    }
}

#[test]
fn primal_clay_choice_types_are_copiable_and_survive_artifact_roundtrip() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(primal_clay_choice_types_are_copiable_and_survive_artifact_roundtrip_inner)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn compound_entry_copy_preserves_name_legendary_and_return_ability() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(compound_entry_copy_preserves_name_legendary_and_return_ability_inner)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn noted_name_copy_uses_printed_card_even_when_associated_object_is_face_down() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(noted_name_copy_uses_printed_card_even_when_associated_object_is_face_down_inner)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn creation_time_pt_is_copiable_on_direct_and_artifact_paths() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            use ironsmith::effects::EffectContext;
            for definition in definitions(
                "Variable token",
                "Type: Sorcery\nCreate an X/X green Beast creature token.",
            ) {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let player = PlayerId::from_index(0);
                let source = game.create_object_from_definition(&definition, player, Zone::Stack);
                let mut ctx = EffectContext::new_default(source, player);
                ctx.x_value = Some(5);
                for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
                    ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                let token = *game.battlefield.last().unwrap();
                let values = game.calculated_copiable_values(token).unwrap();
                assert_eq!((values.power, values.toughness), (Some(5), Some(5)));
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn copy_envelope_retains_flip_half_after_donor_leaves_and_codec_roundtrip() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            use ironsmith::card::LinkedFaceLayout;
            use ironsmith::continuous::EffectTarget;
            use ironsmith::effects::continuous::{ApplyContinuousEffect, RuntimeModification};
            use ironsmith::effects::{EffectContext, EffectExecutor};
            use ironsmith::target::ChooseSpec;
            let mut front = definitions(
                "Normal half",
                "Type: Creature — Human\nPower/Toughness: 1/2\nFlying",
            )[0]
            .clone();
            let mut back = definitions(
                "Flipped half",
                "Type: Legendary Creature — Spirit\nPower/Toughness: 4/5\nVigilance",
            )[0]
            .clone();
            front.card.linked_face_layout = LinkedFaceLayout::Flip;
            front.card.other_face = Some(back.card.id);
            front.card.other_face_name = Some(back.card.name.clone());
            back.card.linked_face_layout = LinkedFaceLayout::Flip;
            back.card.other_face = Some(front.card.id);
            back.card.other_face_name = Some(front.card.name.clone());
            let recipient =
                definitions("Recipient", "Type: Creature\nPower/Toughness: 0/1")[0].clone();
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let donor = game.create_object_from_definition(&front, player, Zone::Battlefield);
            let host = game.create_object_from_definition(&recipient, player, Zone::Battlefield);
            ApplyContinuousEffect::new_runtime(
                EffectTarget::Specific(host),
                RuntimeModification::CopyOf {
                    source: ChooseSpec::SpecificObject(donor),
                    preserve_source_abilities: false,
                    name_override: None,
                    name_override_surface: None,
                    add_supertypes: vec![],
                    copy_exception_surface: None,
                },
                ironsmith::effect::Until::Forever,
            )
            .execute(&mut game, &mut EffectContext::new_default(host, player))
            .unwrap();
            game.move_object_by_game_rule(donor, Zone::Graveyard);
            let values = game.calculated_copiable_values(host).unwrap();
            assert_eq!(
                values.other_copy_face.as_ref().unwrap().name,
                "Flipped half"
            );
            let wire =
                ironsmith_runtime_catalog::artifact_materializer::encode_runtime_copy_values(
                    values,
                )
                .unwrap();
            let wire = serde_json::from_str(&serde_json::to_string(&wire).unwrap()).unwrap();
            let restored =
                ironsmith_runtime_catalog::artifact_materializer::restore_runtime_copy_values(wire)
                    .unwrap();
            assert_eq!(restored.other_copy_face.as_ref().unwrap().power, Some(4));
            assert!(game.flip_permanent(host));
            assert_eq!(
                game.current_characteristics(host).unwrap().name.as_ref(),
                "Flipped half"
            );
            assert_eq!(
                game.calculated_copiable_values(host).unwrap().name,
                "Normal half"
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn leave_battlefield_exile_is_an_active_replacement_on_both_compiler_paths() {
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(|| {
        use ironsmith::effects::{EffectExecutor, EffectContext, ReturnToHandEffect};
        use ironsmith::target::ChooseSpec;
        for definition in definitions("Exile on leaving", "Type: Creature\nPower/Toughness: 3/3\nIf this creature would leave the battlefield, exile it instead.") {
            assert!(definition.spell_effect.is_none());
            assert!(!definition.abilities.is_empty());
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            let id = game.create_object_from_definition(&definition, player, Zone::Battlefield);
            let stable_id = game.object(id).unwrap().stable_id;
            ReturnToHandEffect::with_spec(ChooseSpec::SpecificObject(id))
                .execute(&mut game, &mut EffectContext::new_default(id, player)).unwrap();
            assert!(game.exile.iter().any(|id| game.object(*id).is_some_and(|object| object.stable_id == stable_id)));
            assert!(game.player(player).unwrap().hand.is_empty());
        }
    }).unwrap().join().unwrap();
}

#[test]
fn embalmed_entry_copy_retains_exceptions_only_when_embalmed() {
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(|| {
        struct PickDonor(ironsmith::ObjectId);
        impl DecisionMaker for PickDonor {
            fn decide_options(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
                vec![ctx.options.iter().find(|option| option.object_id == Some(self.0)).expect("donor copy option").index]
            }
        }
        for definition in definitions("Vizier of Many Faces", "Mana cost: {2}{U}{U}\nType: Creature — Shapeshifter Cleric\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any creature on the battlefield, except if this creature was embalmed, the token has no mana cost, it's white, and it's a Zombie in addition to its other types.") {
            for embalmed in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let player = PlayerId::from_index(0);
                let donor = definitions("Donor", "Mana cost: {2}{R}\nType: Creature — Bird\nPower/Toughness: 4/5\nFlying")[0].clone();
                let donor_id = game.create_object_from_definition(&donor, player, Zone::Battlefield);
                let id = game.create_object_from_definition(&definition, player, Zone::Command);
                if embalmed {
                    game.object_mut(id).unwrap().kind = ironsmith::object::ObjectKind::Token;
                    game.object_mut(id).unwrap().token_creation_method = Some(ironsmith_core::effect::TokenCreationMethod::Embalm);
                }
                let receipt = game.move_object_with_etb_processing_with_dm(id, Zone::Battlefield, &mut PickDonor(donor_id)).unwrap();
                assert!(!receipt.pending);
                let copied = game.calculated_copiable_values(receipt.original.into_result().unwrap().new_id).unwrap();
                assert_eq!(copied.name, "Donor");
                assert_eq!((copied.power, copied.toughness), (Some(4), Some(5)));
                assert_eq!(copied.mana_cost.is_none(), embalmed);
                assert_eq!(copied.subtypes.contains(&Subtype::Zombie), embalmed);
                assert_eq!(copied.colors, if embalmed { ironsmith::color::ColorSet::WHITE } else { ironsmith::color::ColorSet::RED });
            }
        }
    }).unwrap().join().unwrap();
}

#[test]
fn split_card_exile_exposes_both_halves_trigger_occurrences() {
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(|| {
        let fronts = definitions("Left room", "Mana cost: {1}{R}\nType: Enchantment — Room\nWhenever a creature enters, you gain 1 life.");
        let backs = definitions("Right room", "Mana cost: {2}{U}\nType: Enchantment — Room\nWhenever a creature enters, you gain 1 life.");
        for (mut front, mut back) in fronts.into_iter().zip(backs) {
            front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
            back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
            front.card.other_face = Some(back.card.id);
            front.card.other_face_name = Some(back.card.name.clone());
            back.card.other_face = Some(front.card.id);
            back.card.other_face_name = Some(front.card.name.clone());
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let id = game.create_object_from_definition(&front, PlayerId::from_index(0), Zone::Exile);
            let current = game.current_characteristics(id).unwrap();
            assert_eq!(current.abilities.iter().filter(|a| matches!(a.kind, AbilityKind::Triggered(_))).count(), 2);
        }
    }).unwrap().join().unwrap();
}

#[test]
fn token_copy_exceptions_apply_to_retained_flip_half() {
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(|| {
        use ironsmith::effects::{CreateTokenCopyEffect, EffectExecutor, EffectContext};
        use ironsmith::target::ChooseSpec;
        let mut front = definitions("Normal half", "Mana cost: {1}{U}\nType: Creature — Human\nPower/Toughness: 1/2\nFlying")[0].clone();
        let mut back = definitions("Flipped half", "Mana cost: {3}{R}\nType: Legendary Creature — Spirit\nPower/Toughness: 4/5\nVigilance")[0].clone();
        front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Flip;
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.clone());
        back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Flip;
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.clone());
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(0);
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let donor = game.create_object_from_definition(&front, player, Zone::Battlefield);
        CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(donor)).without_mana_cost().set_base_power_toughness(7, 8)
            .execute(&mut game, &mut EffectContext::new_default(donor, player)).unwrap();
        let token = *game.battlefield.last().unwrap();
        assert_ne!(token, donor);
        let copy = game.calculated_copiable_values(token).unwrap();
        assert!(copy.mana_cost.is_none());
        assert_eq!((copy.power, copy.toughness), (Some(7), Some(8)));
        let other = copy.other_copy_face.unwrap();
        assert!(other.mana_cost.is_none());
        assert_eq!((other.power, other.toughness), (Some(7), Some(8)));
        game.move_object_by_game_rule(donor, Zone::Graveyard);
        assert!(game.flip_permanent(token));
        let current = game.current_characteristics(token).unwrap();
        assert_eq!(current.name.as_ref(), "Flipped half");
        assert_eq!((current.power, current.toughness), (Some(7), Some(8)));
    }).unwrap().join().unwrap();
}
