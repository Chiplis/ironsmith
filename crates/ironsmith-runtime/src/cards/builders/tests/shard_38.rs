use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;
use crate::GameState;

#[test]
pub(super) fn next61_strict_parse_regressions_keep_structured_references() {
    for name in [
        "Golgothian Sylex",
        "Oblivion's Hunger",
        "Revelation of Power",
        "Wild Mongrel",
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        let compiled = compiled_text_lines(&definition)
            .join("\n")
            .to_ascii_lowercase();

        match name {
            "Golgothian Sylex" => {
                assert!(debug.contains("ForEachObject"), "{name}: {debug}");
                assert!(debug.contains("SacrificeTargetEffect"), "{name}: {debug}");
                assert!(
                    debug.contains("name_originally_printed_in_set")
                        && debug.contains("Antiquities"),
                    "{name}: {debug}"
                );
                assert!(
                    compiled.contains(
                        "each nontoken permanent with a name originally printed in the antiquities expansion is sacrificed by its controller"
                    ),
                    "{name}: {compiled}"
                );
            }
            "Oblivion's Hunger" => {
                assert!(debug.contains("TaggedObjectMatches"), "{name}: {debug}");
                assert!(debug.contains("PlusOnePlusOne"), "{name}: {debug}");
                assert!(
                    compiled.contains(
                        "target creature you control gains indestructible until end of turn. draw a card if that creature has a +1/+1 counter on it"
                    ),
                    "{name}: {compiled}"
                );
            }
            "Revelation of Power" => {
                assert!(debug.contains("TaggedObjectMatches"), "{name}: {debug}");
                assert!(debug.contains("with_counter: Some"), "{name}: {debug}");
                assert!(
                    compiled.contains(
                        "target creature gets +2/+2 until end of turn. if it has a counter on it, it also gains flying and lifelink until end of turn"
                    ),
                    "{name}: {compiled}"
                );
            }
            "Wild Mongrel" => {
                assert!(debug.contains("BecomeColorChoiceEffect"), "{name}: {debug}");
                assert!(
                    !compiled.contains("tagged '") && !compiled.contains("tagged object"),
                    "{name}: {compiled}"
                );
                assert!(
                    compiled.contains(
                        "this creature gets +1/+1 and becomes the color of your choice until end of turn"
                    ),
                    "{name}: {compiled}"
                );
            }
            _ => unreachable!(),
        }
    }
}

#[test]
pub(super) fn sacrificed_this_way_predicates_survive_source_effects() {
    for name in ["Boneyard Desecrator", "Thallid Omnivore", "Warren Weirding"] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        let compiled = compiled_text_lines(&definition)
            .join("\n")
            .to_ascii_lowercase();

        if name == "Warren Weirding" {
            assert!(debug.contains("sacrificed_0"), "{name}: {debug}");
        } else {
            assert!(debug.contains("sacrifice_cost_0"), "{name}: {debug}");
        }
        assert!(debug.contains("TaggedObjectMatches"), "{name}: {debug}");
        match name {
            "Boneyard Desecrator" => assert!(
                compiled.contains("if an outlaw was sacrificed this way, create a treasure token"),
                "{name}: {compiled}"
            ),
            "Thallid Omnivore" => assert!(
                compiled.contains("if a saproling was sacrificed this way, you gain 2 life"),
                "{name}: {compiled}"
            ),
            "Warren Weirding" => {
                assert!(
                    compiled.contains(
                        "if a goblin is sacrificed this way, that player creates two 1/1 black goblin rogue creature tokens"
                    ),
                    "{name}: {compiled}"
                );
                assert!(
                    compiled.contains("and those tokens gain haste until end of turn"),
                    "{name}: {compiled}"
                );
                assert!(
                    !compiled.contains("tagged '") && !compiled.contains("tagged object"),
                    "{name}: {compiled}"
                );
            }
            _ => unreachable!(),
        }
    }
}
#[test]
pub(super) fn graven_dominator_preserves_haunt_linkage_and_other_creature_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Graven Dominator")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nHaunt\nWhen this creature enters or the creature it haunts dies, each other creature has base power and toughness 1/1 until end of turn.",
        )
        .expect("Graven Dominator should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(rendered.lines().any(|line| line == "Haunt"), "{rendered}");
    assert!(
        rendered.contains(
            "When this creature enters or the creature it haunts dies, each other creature has base power and toughness 1/1 until end of turn"
        ),
        "{rendered}"
    );
    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Triggered(triggered)
                if triggered.effects.segments.iter().any(|segment| segment
                    .default_effects
                    .iter()
                    .any(|effect| effect
                        .downcast_ref::<crate::effects::HauntExileEffect>()
                        .is_some()))
        )),
        "{def:#?}"
    );
}

#[test]
pub(super) fn living_inferno_reuses_every_distributed_damage_target_as_a_source() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Living Inferno")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: This creature deals damage equal to its power divided as you choose among any number of target creatures. Each of those creatures deals damage equal to its power to this creature.",
        )
        .expect("Living Inferno should parse");
    let debug = format!("{def:#?}");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");

    assert!(debug.contains("DealDistributedDamageEffect"), "{debug}");
    assert!(debug.contains("ForEachObject"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(
        rendered.contains(
            "deals damage equal to its power divided as you choose among any number of target creatures"
        ),
        "{rendered}"
    );
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("for each creature dealt damage this way")
            && rendered_lower.contains("deals damage equal to its power to this creature"),
        "{rendered}"
    );
}

#[test]
pub(super) fn lichs_mirror_parses_as_generic_loss_replacement() {
    let def = parse_oracle_card_definition("Lich's Mirror");
    let debug = format!("{def:#?}");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");

    assert!(debug.contains("LoseGameReplacement"), "{debug}");
    assert!(
        debug.contains("include_owned_permanents: true"),
        "the replacement must move owned permanents as well as hand and graveyard: {debug}"
    );
    assert!(
        debug.contains("DrawCardsEffect") && debug.contains("SetLifeTotalEffect"),
        "{debug}"
    );
    assert!(
        rendered.contains("If you would lose the game")
            && rendered.contains("all permanents you own")
            && rendered.contains("draw seven cards")
            && rendered.contains("life total becomes 20"),
        "{rendered}"
    );
}

#[test]
pub(super) fn lichs_mirror_replaces_an_sba_loss_with_its_full_effect_sequence() {
    let mirror = parse_oracle_card_definition("Lich's Mirror");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let mirror_id = game.create_object_from_definition(&mirror, alice, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Mirror Filler")
        .card_types(vec![CardType::Artifact])
        .build();
    for _ in 0..7 {
        game.create_object_from_definition(&filler, alice, Zone::Hand);
    }
    game.create_object_from_definition(&filler, alice, Zone::Graveyard);
    game.player_mut(alice).expect("alice").life = 0;

    assert!(crate::rules::apply_state_based_actions(&mut game));

    let player = game.player(alice).expect("alice");
    assert!(player.is_in_game());
    assert_eq!(player.life, 20);
    assert_eq!(player.hand.len(), 7);
    assert_eq!(player.library.len(), 2);
    assert!(
        game.object(mirror_id).is_none(),
        "the battlefield incarnation of Lich's Mirror must have changed zones"
    );
    assert!(
        player
            .library
            .iter()
            .chain(player.hand.iter())
            .any(|object_id| game
                .object(*object_id)
                .is_some_and(|object| object.name == "Lich's Mirror")),
        "Lich's Mirror itself must be shuffled before drawing seven"
    );
    assert!(
        !crate::rules::apply_state_based_actions(&mut game),
        "the post-replacement SBA recheck must find no remaining loss condition"
    );
}
