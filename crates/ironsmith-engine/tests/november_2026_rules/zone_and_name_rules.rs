use super::{A, can_cast, card, game, put, resolve_spell};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::{GameState, ManaSymbol, ObjectId, PlayerId, Target, Zone};

const B: PlayerId = PlayerId(1);

fn attachment(g: &mut GameState, name: &str, owner: PlayerId, host: ObjectId) {
    let id = put(g, name, owner, Zone::Battlefield);
    assert!(g.attach_object_to_target(id, ironsmith::object::AttachmentTarget::Object(host)));
}

#[test]
fn simultaneous_destruction_uses_the_same_pre_event_board_for_each_member() {
    for protector_first in [true, false] {
        let mut g = game();
        if protector_first {
            put(&mut g, "sephara-sky-s-blade", A, Zone::Battlefield);
        }
        let owl = put(&mut g, "sage-owl", A, Zone::Battlefield);
        if !protector_first {
            put(&mut g, "sephara-sky-s-blade", A, Zone::Battlefield);
        }
        resolve_spell(
            &mut g,
            "wrath-of-god",
            vec![],
            &mut SelectFirstDecisionMaker,
        );
        assert_eq!(g.battlefield.as_slice(), &[owl]);
    }
}

#[test]
fn march_of_souls_counts_completed_destructions() {
    for vanilla in [false, true] {
        let mut g = game();
        let myr = put(&mut g, "darksteel-myr", A, Zone::Battlefield);
        if vanilla {
            put(&mut g, "grizzly-bears", A, Zone::Battlefield);
        }
        resolve_spell(
            &mut g,
            "march-of-souls",
            vec![],
            &mut SelectFirstDecisionMaker,
        );
        assert!(g.battlefield.contains(&myr));
        assert_eq!(
            g.battlefield
                .iter()
                .filter(|&&id| g.object(id).unwrap().kind == ironsmith::ObjectKind::Token)
                .count(),
            usize::from(vanilla)
        );
    }
}

#[test]
fn hallowed_moonlight_checks_the_permanent_as_it_would_enter() {
    for (modifier, subject, spell, expected) in [
        (
            Some("march-of-the-machines"),
            "sol-ring",
            "refurbish",
            Zone::Exile,
        ),
        (None, "sol-ring", "refurbish", Zone::Battlefield),
        (None, "heliod-god-of-the-sun", "zombify", Zone::Battlefield),
        (None, "grizzly-bears", "zombify", Zone::Exile),
    ] {
        let mut g = game();
        if let Some(name) = modifier {
            put(&mut g, name, A, Zone::Battlefield);
        }
        put(&mut g, "opt", A, Zone::Library);
        resolve_spell(
            &mut g,
            "hallowed-moonlight",
            vec![],
            &mut SelectFirstDecisionMaker,
        );
        let id = put(&mut g, subject, A, Zone::Graveyard);
        let key = g.object(id).unwrap().stable_id;
        resolve_spell(
            &mut g,
            spell,
            vec![Target::Object(id)],
            &mut SelectFirstDecisionMaker,
        );
        assert_eq!(
            g.object(g.find_object_by_stable_id(key).unwrap())
                .unwrap()
                .zone,
            expected
        );
    }
}

#[test]
fn mirror_box_excludes_the_recipient() {
    let mut g = game();
    let first = put(&mut g, "savannah-lions", A, Zone::Battlefield);
    put(&mut g, "mirror-box", A, Zone::Battlefield);
    assert_eq!(g.current_power(first), Some(2));
    put(&mut g, "savannah-lions", A, Zone::Battlefield);
    assert_eq!(g.current_power(first), Some(3));
}

#[test]
fn removing_or_replacing_a_granted_name_affects_only_the_appropriate_references() {
    for (spell, witness, expect_others, expect_target) in [
        ("echoing-truth", false, false, false),
        ("echoing-truth", true, true, false),
        ("sever-the-bloodline", false, false, false),
        ("sever-the-bloodline", true, true, false),
        ("winnow", false, true, false),
        ("winnow", true, true, true),
    ] {
        let mut g = game();
        let bear = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        let lions = put(&mut g, "savannah-lions", B, Zone::Battlefield);
        attachment(&mut g, "spy-kit", B, bear);
        if witness {
            attachment(&mut g, "witness-protection", A, bear);
        }
        resolve_spell(
            &mut g,
            spell,
            vec![Target::Object(bear)],
            &mut SelectFirstDecisionMaker,
        );
        assert_eq!(
            (
                g.battlefield.contains(&lions),
                g.battlefield.contains(&bear)
            ),
            (expect_others, expect_target),
            "{spell} with later renaming = {witness}"
        );
    }
}

#[test]
fn legend_rule_choice_removes_a_permanent_with_an_overlapping_name() {
    for keep_equipped in [false, true] {
        let mut g = game();
        put(&mut g, "leyline-of-singularity", B, Zone::Battlefield);
        let bear = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
        let lions = put(&mut g, "savannah-lions", A, Zone::Battlefield);
        attachment(&mut g, "spy-kit", A, bear);
        let specs = ironsmith::rules::state_based::get_legend_rule_specs(&g);
        assert_eq!(specs.len(), 1);
        assert_eq!(&specs[0].1.legends, &[bear, lions]);
        let keep = if keep_equipped { bear } else { lions };
        ironsmith::rules::state_based::apply_legend_rule_choice(&mut g, keep).unwrap();
        assert_eq!(
            (
                g.battlefield.contains(&bear),
                g.battlefield.contains(&lions)
            ),
            (keep_equipped, !keep_equipped)
        );
    }
}

#[test]
fn differently_named_lands_must_have_disjoint_names() {
    let mut g = game();
    put(&mut g, "living-plane", A, Zone::Battlefield);
    let forest = put(&mut g, "forest", A, Zone::Battlefield);
    put(&mut g, "dryad-arbor", A, Zone::Battlefield);
    let amalgam = put(&mut g, "awakened-amalgam", A, Zone::Battlefield);
    assert_eq!(g.current_power(amalgam), Some(2));
    attachment(&mut g, "spy-kit", A, forest);
    assert_eq!(g.current_power(amalgam), Some(1));
}

#[test]
fn graveyard_shift_counts_both_halves_of_a_split_card_before_casting() {
    for (fourth, expected) in [("grizzly-bears", true), ("hill-giant", false)] {
        let mut g = game();
        for route in ["ornithopter", "savannah-lions", "centaur-courser", fourth] {
            put(&mut g, route, A, Zone::Graveyard);
        }
        let mut left = card("grizzly-bears");
        let mut right = card("grizzly-bears");
        left.card.name = "First half".into();
        right.card.name = "Second half".into();
        left.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
        right.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
        left.card.other_face = Some(right.card.id);
        right.card.other_face = Some(left.card.id);
        g.register_linked_face_definition(&left);
        g.register_linked_face_definition(&right);
        g.create_object_from_definition(&left, A, Zone::Graveyard);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 5);
        g.turn.active_player = B;
        let spell = put(&mut g, "graveyard-shift", A, Zone::Hand);
        assert_eq!(can_cast(&g, spell), expected);
    }
}

#[test]
fn eradicate_searches_using_the_departing_creatures_names_in_all_three_zones() {
    use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
    struct TakeAll;
    impl ironsmith::DecisionMaker for TakeAll {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            true
        }
        fn decide_objects(
            &mut self,
            _: &GameState,
            choice: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            choice
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .take(choice.max.unwrap_or(usize::MAX))
                .map(|candidate| candidate.id)
                .collect()
        }
    }

    for witness in [false, true] {
        let mut g = game();
        let bear = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        attachment(&mut g, "spy-kit", B, bear);
        if witness {
            attachment(&mut g, "witness-protection", A, bear);
        }
        let candidates = [Zone::Hand, Zone::Graveyard, Zone::Library].map(|zone| {
            let lion = put(&mut g, "savannah-lions", B, zone);
            (g.object(lion).unwrap().stable_id, zone)
        });
        resolve_spell(
            &mut g,
            "eradicate",
            vec![Target::Object(bear)],
            &mut TakeAll,
        );
        for (key, original) in candidates {
            let id = g.find_object_by_stable_id(key).unwrap();
            assert_eq!(
                g.object(id).unwrap().zone,
                if witness { original } else { Zone::Exile }
            );
        }
    }
}

#[test]
fn a_shared_name_count_uses_granted_names_and_ignores_nameless_objects() {
    use ironsmith::{EffectContext, ObjectFilter, Value};
    let mut g = game();
    let lions = put(&mut g, "savannah-lions", A, Zone::Battlefield);
    let bear = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let value = Value::GreatestSharedNameCount(ObjectFilter::creature().you_control());
    let ctx = EffectContext::new_default(lions, A);
    let count = |game: &GameState| ironsmith::effects::resolve_value(game, &value, &ctx).unwrap();
    assert_eq!(count(&g), 1);
    attachment(&mut g, "spy-kit", A, bear);
    assert_eq!(count(&g), 2);
    g.set_face_down(lions);
    assert_eq!(count(&g), 1);
    let second = put(&mut g, "savannah-lions", A, Zone::Battlefield);
    g.set_face_down(second);
    assert_eq!(count(&g), 1);
}

#[test]
fn leyline_of_singularity_exempts_land_creatures_from_the_legend_rule() {
    let mut g = game();
    put(&mut g, "leyline-of-singularity", B, Zone::Battlefield);
    put(&mut g, "dryad-arbor", A, Zone::Battlefield);
    put(&mut g, "dryad-arbor", A, Zone::Battlefield);
    assert!(ironsmith::rules::state_based::get_legend_rule_specs(&g).is_empty());
}

#[test]
fn spy_kit_recognizes_oracle_creature_faces_even_when_the_card_is_absent() {
    use ironsmith::{EffectContext, ObjectFilter, Value};
    let mut g = game();
    let host = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    attachment(&mut g, "spy-kit", A, host);
    let ctx = EffectContext::new_default(host, A);
    for (name, expected) in [
        ("Howlpack Alpha", 1),
        ("Sephara, Sky's Blade", 0),
        ("Sol Ring", 0),
    ] {
        let mut filter = ObjectFilter::creature().you_control();
        filter.name = Some(name.into());
        assert_eq!(
            ironsmith::effects::resolve_value(&g, &Value::Count(filter), &ctx).unwrap(),
            expected,
            "{name}"
        );
    }
    let filter = ObjectFilter::creature().you_control();
    assert!(
        ironsmith::effects::resolve_value(&g, &Value::DistinctNames(filter.clone()), &ctx).unwrap()
            > 1_000
    );
    assert_eq!(
        ironsmith::effects::resolve_value(&g, &Value::differently_named_objects(filter), &ctx)
            .unwrap(),
        1
    );
}

#[test]
fn distinct_door_names_come_from_the_unlocked_halves_of_each_room() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, EffectContext, ObjectFilter, Subtype, Value};
    let mut g = game();
    let a = CardId::new();
    let b = CardId::new();
    let half = |id, name, other, other_name| {
        CardDefinitionBuilder::new(id, name)
            .card_types(vec![CardType::Enchantment, CardType::Creature])
            .subtypes(vec![Subtype::Room])
            .other_face(other)
            .other_face_name(other_name)
            .linked_face_layout(ironsmith::card::LinkedFaceLayout::Split)
            .build()
    };
    let left = half(a, "First door", b, "Second door");
    let right = half(b, "Second door", a, "First door");
    g.register_linked_face_definition(&left);
    g.register_linked_face_definition(&right);
    let first = g.create_object_from_definition(&left, A, Zone::Battlefield);
    let second = g.create_object_from_definition(&left, A, Zone::Battlefield);
    let mut filter = ObjectFilter::permanent().you_control();
    filter.subtypes.push(Subtype::Room);
    filter.distinct_names = true;
    let value = Value::UnlockedDoorsAmong(filter);
    let ctx = EffectContext::new_default(first, A);
    let count = |g: &GameState| ironsmith::effects::resolve_value(g, &value, &ctx).unwrap();
    assert_eq!(count(&g), 1);
    attachment(&mut g, "spy-kit", A, first);
    assert_eq!(count(&g), 1);
    g.object_mut(second)
        .unwrap()
        .apply_fused_split_spell_overlay(&right);
    g.mark_room_fully_unlocked(second);
    assert_eq!(count(&g), 2);
    g.mark_room_entered_with_no_unlocked_door(second);
    assert_eq!(count(&g), 1);
}
