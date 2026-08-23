use super::*;

fn attached_aura_filter(tag: TagKey) -> ObjectFilter {
    let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    filter.owner = Some(PlayerFilter::You);
    filter.subtypes.push(Subtype::Aura);
    filter.colors = Some(crate::color::ColorSet::WHITE);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
    });
    filter
}

fn attached_equipment_filter(tag: TagKey) -> ObjectFilter {
    let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    filter.subtypes.push(Subtype::Equipment);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
    });
    filter.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Creature,
    ));
    filter
}

#[test]
fn target_and_attached_objects_destroy_as_one_tag_linked_bundle() {
    let tag = TagKey::from("destroy_attachment_target");
    let target = Effect::new(crate::effects::TargetOnlyEffect::explicit(
        ChooseSpec::target_creature(),
    ))
    .tag_all(tag.clone());
    let attached = Effect::destroy_all(attached_equipment_filter(tag.clone()));
    let target_destroy = Effect::new(crate::effects::DestroyEffect::with_spec(
        ChooseSpec::Tagged(tag),
    ));

    assert_eq!(
        describe_effect_list(&[target, attached, target_destroy]),
        "Destroy target creature and all Equipment attached to that creature"
    );
}

#[test]
fn target_and_attached_destroy_bundle_rejects_a_different_final_object() {
    let tag = TagKey::from("destroy_attachment_target");
    let target = Effect::new(crate::effects::TargetOnlyEffect::explicit(
        ChooseSpec::target_creature(),
    ))
    .tag_all(tag.clone());
    let attached = Effect::destroy_all(attached_equipment_filter(tag));
    let unrelated_destroy = Effect::new(crate::effects::DestroyEffect::with_spec(
        ChooseSpec::Tagged(TagKey::from("other_target")),
    ));

    assert_ne!(
        describe_effect_list(&[target, attached, unrelated_destroy]),
        "Destroy target creature and all Equipment attached to that creature"
    );
}

#[test]
fn target_and_attached_objects_return_to_their_owners_as_one_typed_bundle() {
    let tag = TagKey::from("returned_target");
    let target_return = Effect::new(crate::effects::ReturnToHandEffect::target(
        ChooseSpec::creature(),
    ))
    .tag(tag.clone());
    let attached_return = Effect::new(crate::effects::ReturnToHandEffect::all(
        attached_aura_filter(tag),
    ));
    let effects = vec![target_return, attached_return];

    assert_eq!(
        describe_effect_list(&effects),
        "Return target creature and all white Auras you own attached to it to their owners' hands"
    );
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::SequenceEffect::coordinated(
            effects
        ))),
        "Return target creature and all white Auras you own attached to it to their owners' hands"
    );
}

#[test]
fn attached_return_bundle_rejects_an_unrelated_attachment_tag() {
    let target_return = Effect::new(crate::effects::ReturnToHandEffect::target(
        ChooseSpec::creature(),
    ))
    .tag(TagKey::from("returned_target"));
    let attached_return = Effect::new(crate::effects::ReturnToHandEffect::all(
        attached_aura_filter(TagKey::from("different_target")),
    ));

    assert_ne!(
        describe_effect_list(&[target_return, attached_return]),
        "Return target creature and all white Auras you own attached to it to their owners' hands"
    );
}

#[test]
fn enchanted_creature_and_its_attached_auras_exile_as_one_typed_bundle() {
    let enchanted = TagKey::from("enchanted");
    let mut auras = ObjectFilter::default();
    auras.subtypes.push(Subtype::Aura);
    auras.tagged_constraints.push(TaggedObjectConstraint {
        tag: enchanted.clone(),
        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
    });
    auras.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Creature,
    ));
    let effects = vec![
        Effect::new(crate::effects::TagAttachedToSourceEffect::new(
            enchanted.clone(),
        )),
        Effect::new(crate::effects::ExileEffect::all(auras)),
        Effect::move_to_zone(ChooseSpec::Tagged(enchanted), Zone::Exile, true),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile enchanted creature and all Auras attached to it"
    );
}

#[test]
fn enchanted_creature_exile_bundle_survives_a_trailing_delayed_effect() {
    let enchanted = TagKey::from("enchanted");
    let mut auras = ObjectFilter::default();
    auras.subtypes.push(Subtype::Aura);
    auras.tagged_constraints.push(TaggedObjectConstraint {
        tag: enchanted.clone(),
        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
    });
    auras.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Creature,
    ));
    let delayed_return = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::beginning_of_end_step(PlayerFilter::Any),
        vec![Effect::move_to_zone(
            ChooseSpec::Tagged(enchanted.clone()),
            Zone::Battlefield,
            true,
        )],
        true,
        Vec::new(),
        PlayerFilter::You,
    ));
    let effects = vec![
        Effect::new(crate::effects::TagAttachedToSourceEffect::new(
            enchanted.clone(),
        )),
        Effect::new(crate::effects::ExileEffect::all(auras)),
        Effect::move_to_zone(ChooseSpec::Tagged(enchanted), Zone::Exile, true),
        delayed_return,
    ];

    let rendered = describe_structural_multisentence_effect_list(&effects)
        .expect("the typed attachment bundle should remain visible before its delayed follow-up");
    assert!(
        rendered.starts_with("Exile enchanted creature and all Auras attached to it. "),
        "{rendered}"
    );
    assert!(!rendered.starts_with("Exile all Auras"), "{rendered}");
}

#[test]
fn return_all_to_battlefield_attachment_compacts_a_tagged_collection() {
    let moved = TagKey::from("returned_auras");
    let creature = TagKey::from("targeted_returned_creature");
    let mut auras = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    auras.subtypes.push(Subtype::Aura);
    let move_all = Effect::new(
        crate::effects::MoveToZoneEffect::new(ChooseSpec::All(auras), Zone::Battlefield, false)
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
            .under_owner_control(),
    )
    .tag_all(moved.clone());
    let attach = Effect::attach_objects(
        ChooseSpec::All(ObjectFilter::tagged(moved)),
        ChooseSpec::Tagged(creature),
    );

    let rendered = describe_effect_list(&[move_all, attach]);
    assert!(
        rendered.contains("to the battlefield attached to that creature"),
        "{rendered}"
    );
    assert!(!rendered.contains(". Attach "), "{rendered}");
}

#[test]
fn returned_collection_with_individual_destinations_keeps_the_plural_target() {
    let moved = TagKey::from("returned_auras");
    let mut auras = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    auras.subtypes.push(Subtype::Aura);
    let move_all = Effect::new(
        crate::effects::MoveToZoneEffect::new(ChooseSpec::All(auras), Zone::Battlefield, false)
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
            .under_owner_control(),
    )
    .tag_all(moved.clone());
    let destination = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    let attach = Effect::new(
        crate::effects::AttachObjectsEffect::new(
            ChooseSpec::All(ObjectFilter::tagged(moved)),
            ChooseSpec::Object(destination),
        )
        .with_individual_targets(),
    );

    assert_eq!(
        describe_effect_list(&[move_all, attach]),
        "Return all Aura cards from your graveyard to the battlefield attached to creatures you control"
    );
}
