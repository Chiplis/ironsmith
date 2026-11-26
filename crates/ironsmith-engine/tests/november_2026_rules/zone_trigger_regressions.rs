use super::{A, game};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::{
    CardType, CounterType, GameState, ObjectId, PlayerId, StackEntry, Subtype, Target, Zone,
};

fn place(g: &mut GameState, name: &str, zone: Zone) -> ObjectId {
    let block = match name {
        "Pull from the Deep" => {
            "Mana cost: {2}{U}{U}\nType: Sorcery\nReturn up to one target instant card and up to one target sorcery card from your graveyard to your hand. Exile Pull from the Deep."
        }
        "Lightning Bolt" => {
            "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target."
        }
        "Divination" => "Mana cost: {2}{U}\nType: Sorcery\nDraw two cards.",
        "Thwart the Grave" => {
            "Mana cost: {4}{B}{B}\nType: Sorcery\nThis spell costs {1} less to cast for each creature in your party. (Your party consists of up to one each of Cleric, Rogue, Warrior, and Wizard.)\nReturn target creature card and up to one target Cleric, Rogue, Warrior, or Wizard creature card from your graveyard to the battlefield."
        }
        "Fugitive Wizard" => "Mana cost: {U}\nType: Creature — Human Wizard\nPower/Toughness: 1/1",
        "Dance of the Manse" => {
            "Mana cost: {X}{W}{U}\nType: Sorcery\nReturn up to X target artifact and/or non-Aura enchantment cards each with mana value X or less from your graveyard to the battlefield. If X is 6 or more, those permanents are 4/4 creatures in addition to their other types."
        }
        "Containment Priest" => {
            "Mana cost: {1}{W}\nType: Creature — Human Cleric\nPower/Toughness: 2/2\nFlash\nIf a nontoken creature would enter and it wasn't cast, exile it instead."
        }
        "Grafdigger's Cage" => {
            "Mana cost: {1}\nType: Artifact\nCreature cards in graveyards and libraries can't enter the battlefield.\nPlayers can't cast spells from graveyards or libraries."
        }
        "Sol Ring" => "Mana cost: {1}\nType: Artifact\n{T}: Add {C}{C}.",
        "Storm of Souls" => {
            "Mana cost: {4}{W}{W}\nType: Sorcery\nReturn all creature cards from your graveyard to the battlefield. Each of them is a 1/1 Spirit with flying in addition to its other types. Exile Storm of Souls."
        }
        "Colossal Dreadmaw" => {
            "Mana cost: {4}{G}{G}\nType: Creature — Dinosaur\nPower/Toughness: 6/6\nTrample (This creature can deal excess combat damage to the player or planeswalker it's attacking.)"
        }
        "Mentor of the Meek" => {
            "Mana cost: {2}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/2\nWhenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card."
        }
        "Elemental Bond" => {
            "Mana cost: {2}{G}\nType: Enchantment\nWhenever a creature you control with power 3 or greater enters, draw a card."
        }
        "Master Biomancer" => {
            "Mana cost: {2}{G}{U}\nType: Creature — Elf Wizard\nPower/Toughness: 2/4\nEach other creature you control enters with a number of additional +1/+1 counters on it equal to this creature's power and as a Mutant in addition to its other types."
        }
        "Glorious Anthem" => {
            "Mana cost: {1}{W}{W}\nType: Enchantment\nCreatures you control get +1/+1."
        }
        "Opalescence" => {
            "Mana cost: {2}{W}{W}\nType: Enchantment\nEach other non-Aura enchantment is a creature in addition to its other types and has base power and base toughness each equal to its mana value."
        }
        "Show and Tell" => {
            "Mana cost: {2}{U}\nType: Sorcery\nEach player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield."
        }
        "Bonesplitter" => {
            "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature gets +2/+0.\nEquip {1}"
        }
        "Leonin Shikari" => {
            "Mana cost: {1}{W}\nType: Creature — Cat Soldier\nPower/Toughness: 2/2\nYou may activate equip abilities any time you could cast an instant."
        }
        "Elvish Refueler" => {
            "Mana cost: {2}{G}\nType: Creature — Elf Druid\nPower/Toughness: 2/3\nDuring your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated.\nExhaust — {1}{G}: Put a +1/+1 counter on this creature. (Activate each exhaust ability only once.)"
        }
        "Camera Launcher" => {
            "Mana cost: {3}\nType: Artifact Creature — Construct\nPower/Toughness: 2/2\nExhaust — {3}: Put a +1/+1 counter on this creature. Create a 1/1 colorless Thopter artifact creature token with flying. (Activate each exhaust ability only once.)"
        }
        "Rusted Relic" => {
            "Mana cost: {4}\nType: Artifact\nMetalcraft — This artifact is a 5/5 Golem artifact creature as long as you control three or more artifacts."
        }
        "March of the Machines" => {
            "Mana cost: {3}{U}\nType: Enchantment\nEach noncreature artifact is an artifact creature with power and toughness each equal to its mana value. (Equipment that's a creature can't equip a creature.)"
        }
        "Abuelo's Awakening" => {
            "Mana cost: {X}{3}{W}\nType: Sorcery\nReturn target artifact or non-Aura enchantment card from your graveyard to the battlefield with X additional +1/+1 counters on it. It's a 1/1 Spirit creature with flying in addition to its other types."
        }
        "Afterlife from the Loam" => {
            "Mana cost: {5}{B}{B}{B}\nType: Sorcery\nDelve (Each card you exile from your graveyard while casting this spell pays for {1}.)\nFor each player, choose up to one target creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're Zombies in addition to their other types."
        }
        "Call a Surprise Witness" => {
            "Mana cost: {1}{W}\nType: Sorcery\nReturn target creature card with mana value 3 or less from your graveyard to the battlefield. Put a flying counter on it. It's a Spirit in addition to its other types."
        }
        "Ever After" => {
            "Mana cost: {4}{B}{B}\nType: Sorcery\nReturn up to two target creature cards from your graveyard to the battlefield. Each of those creatures is a black Zombie in addition to its other colors and types. Put Ever After on the bottom of its owner's library."
        }
        "Ghouls' Night Out" => {
            "Mana cost: {3}{B}{B}\nType: Sorcery\nFor each player, choose a creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're black Zombies in addition to their other colors and types and they gain decayed. (A creature with decayed can't block. When it attacks, sacrifice it at end of combat.)"
        }
        "Relive the Past" => {
            "Mana cost: {5}{G}{W}\nType: Sorcery\nReturn up to one target artifact card, up to one target land card, and up to one target non-Aura enchantment card from your graveyard to the battlefield. They are 5/5 Elemental creatures in addition to their other types."
        }
        _ => panic!("unknown fixture {name}"),
    };
    let definition =
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, block, false).unwrap();
    g.create_object_from_definition(&definition, A, zone)
}

// Bind the compiler's actual announcement slots, including empty choices and
// repeated object IDs in separate slots. Resolve through the normal stack.
fn resolve(g: &mut GameState, name: &str, x: u32, slots: &[&[ObjectId]]) {
    let spell = place(g, name, Zone::Stack);
    g.object_mut(spell).unwrap().x_value = Some(x);
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        g,
        g.object(spell).unwrap().spell_effect.as_deref().unwrap(),
        A,
        Some(spell),
        None,
    );
    let mut entry = StackEntry::new(spell, A);
    entry.x_value = Some(x);
    assert_eq!(slots.len(), requirements.len(), "{name}");
    for (ids, requirement) in slots.iter().zip(requirements) {
        let start = entry.targets.len();
        for id in *ids {
            let target = Target::Object(*id);
            assert!(
                requirement.legal_targets.contains(&target),
                "{name} {target:?}"
            );
            entry.targets.push(target);
        }
        entry
            .target_assignments
            .push(ironsmith::game_state::TargetAssignment {
                spec: requirement.spec,
                range: start..entry.targets.len(),
            });
    }
    g.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(g, &mut SelectFirstDecisionMaker).unwrap();
}

#[test]
fn return_clauses_resolve_each_announced_slot_including_an_empty_one() {
    for return_sorcery in [false, true] {
        let mut g = game();
        let bolt = place(&mut g, "Lightning Bolt", Zone::Graveyard);
        let divination = place(&mut g, "Divination", Zone::Graveyard);
        let bolt_key = g.object(bolt).unwrap().stable_id;
        let divination_key = g.object(divination).unwrap().stable_id;
        let sorceries = [divination];
        resolve(
            &mut g,
            "Pull from the Deep",
            0,
            &[&[bolt], if return_sorcery { &sorceries } else { &[] }],
        );
        assert_eq!(
            g.object(g.find_object_by_stable_id(bolt_key).unwrap())
                .unwrap()
                .zone,
            Zone::Hand
        );
        let expected = if return_sorcery {
            Zone::Hand
        } else {
            Zone::Graveyard
        };
        assert_eq!(
            g.object(g.find_object_by_stable_id(divination_key).unwrap())
                .unwrap()
                .zone,
            expected
        );
    }
}

#[test]
fn returning_one_card_for_two_legal_target_clauses_finishes_the_spell() {
    let mut g = game();
    let wizard = place(&mut g, "Fugitive Wizard", Zone::Graveyard);
    let key = g.object(wizard).unwrap().stable_id;
    resolve(&mut g, "Thwart the Grave", 0, &[&[wizard], &[wizard]]);
    assert_eq!(
        g.object(g.find_object_by_stable_id(key).unwrap())
            .unwrap()
            .zone,
        Zone::Battlefield
    );
}

#[test]
fn conditional_animation_checks_the_incoming_creature_and_the_original_graveyard_card() {
    for (x, watcher, expected) in [
        (5, "Containment Priest", Zone::Battlefield),
        (6, "Containment Priest", Zone::Exile),
        (6, "Grafdigger's Cage", Zone::Battlefield),
    ] {
        let mut g = game();
        place(&mut g, watcher, Zone::Battlefield);
        let ring = place(&mut g, "Sol Ring", Zone::Graveyard);
        let key = g.object(ring).unwrap().stable_id;
        resolve(&mut g, "Dance of the Manse", x, &[&[ring]]);
        let id = g.find_object_by_stable_id(key).unwrap();
        assert_eq!(g.object(id).unwrap().zone, expected, "{watcher} X={x}");
        assert_eq!(
            g.current_has_card_type(id, CardType::Creature),
            x >= 6 && expected == Zone::Battlefield
        );
    }
}

#[test]
fn entry_power_determines_which_abilities_trigger() {
    let mut g = game();
    let mentor = place(&mut g, "Mentor of the Meek", Zone::Battlefield);
    let bond = place(&mut g, "Elemental Bond", Zone::Battlefield);
    place(&mut g, "Colossal Dreadmaw", Zone::Graveyard);
    g.take_pending_trigger_events();
    resolve(&mut g, "Storm of Souls", 0, &[]);
    let mut triggers = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut triggers).unwrap();
    assert_eq!(
        triggers
            .entries
            .iter()
            .filter(|e| e.source == mentor)
            .count(),
        1
    );
    assert_eq!(
        triggers.entries.iter().filter(|e| e.source == bond).count(),
        0
    );
}

#[test]
fn animated_anthem_enters_as_a_mutant_using_biomancers_pre_entry_power() {
    let mut g = game();
    place(&mut g, "Opalescence", Zone::Battlefield);
    let biomancer = place(&mut g, "Master Biomancer", Zone::Battlefield);
    let anthem = place(&mut g, "Glorious Anthem", Zone::Hand);
    let key = g.object(anthem).unwrap().stable_id;
    resolve(&mut g, "Show and Tell", 0, &[]);
    let id = g.find_object_by_stable_id(key).unwrap();
    assert_eq!(g.current_power(biomancer), Some(3));
    assert!(g.current_has_subtype(id, Subtype::Mutant));
    assert_eq!(g.counter_count(id, CounterType::PlusOnePlusOne), 2);
}

#[test]
fn phased_out_grant_sources_stop_modifying_activation_rules() {
    let mut g = game();
    let shikari = place(&mut g, "Leonin Shikari", Zone::Battlefield);
    let equipment = place(&mut g, "Bonesplitter", Zone::Battlefield);
    let refueler = place(&mut g, "Elvish Refueler", Zone::Battlefield);
    let launcher = place(&mut g, "Camera Launcher", Zone::Battlefield);
    let exhaust = g
        .object(launcher)
        .unwrap()
        .abilities_vec()
        .iter()
        .position(
            |a| matches!(&a.kind, ironsmith::AbilityKind::Activated(a) if a.is_exhaust_ability()),
        )
        .unwrap();
    g.record_ability_activation(launcher, exhaust);
    g.next_turn();
    g.next_turn();
    assert!(g.exhaust_ability_activated(launcher, exhaust));
    g.turn.active_player = A;
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    for symbol in [ironsmith::ManaSymbol::Red, ironsmith::ManaSymbol::Colorless] {
        g.player_mut(A).unwrap().mana_pool.add(symbol, 10);
    }
    let can_activate = |g: &GameState, permanent| {
        ironsmith::compute_legal_actions(g, A).unwrap()
        .iter().any(|a| matches!(a, ironsmith::LegalAction::ActivateAbility { source, .. } if *source == permanent))
    };
    assert!(can_activate(&g, launcher));
    g.phase_out(refueler);
    assert!(!can_activate(&g, launcher));
    g.turn.active_player = PlayerId(1);
    assert!(can_activate(&g, equipment));
    g.phase_out(shikari);
    assert!(!can_activate(&g, equipment));
}

#[test]
fn adding_a_creature_subtype_waits_for_animation_in_layer_four() {
    let mut g = game();
    place(&mut g, "March of the Machines", Zone::Battlefield);
    place(&mut g, "Master Biomancer", Zone::Battlefield);
    let relic = place(&mut g, "Rusted Relic", Zone::Hand);
    let key = g.object(relic).unwrap().stable_id;
    resolve(&mut g, "Show and Tell", 0, &[]);
    let id = g.find_object_by_stable_id(key).unwrap();
    assert!(g.current_has_card_type(id, CardType::Creature));
    assert!(g.current_has_subtype(id, Subtype::Mutant));
    assert!(!g.current_has_subtype(id, Subtype::Golem));
}

#[test]
fn printed_arrival_descriptions_compile_into_every_corresponding_move() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::MoveToZoneEffect;
    type Entry = ironsmith_core::effect::BattlefieldEntryTypeChanges;
    fn collect(effect: &Effect, entries: &mut Vec<Entry>) {
        if let Some(movement) = effect.downcast_ref::<MoveToZoneEffect>()
            && movement.zone == Zone::Battlefield
        {
            entries.push(
                movement
                    .entry_type_changes
                    .clone()
                    .expect("entry description"),
            );
        }
        effect.visit_child_effects(&mut |child| collect(child, entries));
    }
    for (name, moves, subtype, pt, minimum_x) in [
        (
            "Abuelo's Awakening",
            1,
            Some(Subtype::Spirit),
            Some((1, 1)),
            None,
        ),
        (
            "Afterlife from the Loam",
            1,
            Some(Subtype::Zombie),
            None,
            None,
        ),
        (
            "Call a Surprise Witness",
            1,
            Some(Subtype::Spirit),
            None,
            None,
        ),
        ("Dance of the Manse", 1, None, Some((4, 4)), Some(6)),
        ("Ever After", 1, Some(Subtype::Zombie), None, None),
        ("Ghouls' Night Out", 1, Some(Subtype::Zombie), None, None),
        (
            "Relive the Past",
            3,
            Some(Subtype::Elemental),
            Some((5, 5)),
            None,
        ),
        (
            "Storm of Souls",
            1,
            Some(Subtype::Spirit),
            Some((1, 1)),
            None,
        ),
    ] {
        let mut g = game();
        let spell = place(&mut g, name, Zone::Hand);
        let mut entries = Vec::new();
        for effect in g
            .object(spell)
            .unwrap()
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            collect(effect, &mut entries);
        }
        assert_eq!(entries.len(), moves, "{name}");
        for entry in entries {
            assert!(
                subtype.is_none_or(|t| entry.added_subtypes.contains(&t)),
                "{name}"
            );
            assert_eq!(entry.base_power_toughness, pt, "{name}");
            assert_eq!(entry.minimum_x, minimum_x, "{name}");
        }
    }
}
