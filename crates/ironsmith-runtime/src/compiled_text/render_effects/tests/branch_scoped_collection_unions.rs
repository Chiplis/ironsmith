use super::*;

fn plural_host(mut filter: ObjectFilter) -> ObjectFilter {
    filter.set_plural_object_noun_surface(true);
    filter
}

fn aura_attached_to(host: ObjectFilter, other: bool) -> ObjectFilter {
    let mut aura = ObjectFilter::default().with_subtype(Subtype::Aura);
    aura.other = other;
    aura.attached_to_object = Some(Box::new(host));
    aura
}

fn branch_scoped_collection_union(owner: Option<PlayerFilter>, other: bool) -> ObjectFilter {
    let mut enchantment = ObjectFilter::default().with_type(CardType::Enchantment);
    enchantment.controller = Some(PlayerFilter::You);
    enchantment.other = other;

    let controlled_permanent =
        plural_host(ObjectFilter::permanent().controlled_by(PlayerFilter::You));
    let mut opposing_attacker = ObjectFilter::creature().controlled_by(PlayerFilter::Opponent);
    opposing_attacker.attacking = true;
    let opposing_attacker = plural_host(opposing_attacker);

    let mut union = ObjectFilter::default();
    union.zone = Some(Zone::Battlefield);
    union.owner = owner;
    union.any_of = vec![
        enchantment,
        aura_attached_to(controlled_permanent, other),
        aura_attached_to(opposing_attacker, other),
    ];
    union.set_conjunctive_set_surface(true);
    union
}

#[test]
fn all_conjunctive_union_repeats_quantifier_and_materializes_common_owner_scope() {
    let union = branch_scoped_collection_union(Some(PlayerFilter::You), false);

    assert_eq!(
        describe_choose_spec(&ChooseSpec::All(union)),
        "all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control"
    );
}

#[test]
fn all_conjunctive_union_keeps_other_and_each_attachment_controller_local() {
    let union = branch_scoped_collection_union(None, true);

    assert_eq!(
        describe_choose_spec(&ChooseSpec::All(union)),
        "all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control"
    );
}

#[test]
fn destination_first_return_uses_the_structured_collection_surface() {
    let mut union = branch_scoped_collection_union(Some(PlayerFilter::You), false);
    union.set_return_destination_first_surface(true);
    let effect = Effect::new(
        crate::effects::ReturnToHandEffect::all(union)
            .with_destination_player_surface(PlayerFilter::You),
    );

    assert_eq!(
        describe_effect(&effect),
        "Return to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control"
    );
}

#[test]
fn destroy_uses_the_same_union_renderer_without_destination_surface() {
    let effect = Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::All(
        branch_scoped_collection_union(None, true),
    )));

    assert_eq!(
        describe_effect(&effect),
        "Destroy all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control"
    );
}
