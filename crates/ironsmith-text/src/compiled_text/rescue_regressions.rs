//! Renderer normalization must not manufacture rules absent from its input.
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::game_state::{StackEntry, Target, TargetAssignment};
use crate::object::CounterType;
use crate::{CardDefinitionBuilder, CardId, GameState, PlayerId, Zone};

fn parse(name: &str, text: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .parse_text(text)
        .unwrap()
}

#[test]
fn rescues_do_not_supply_missing_semantics() {
    let cases = [
        (
            "Choose two target creatures controlled by the same player. Exile that creature and put two +1/+1 counters on any other target.",
            "any other target",
            "on the other",
        ),
        (
            "Target player chooses three cards and puts those cards on top of their library in any order.",
            "three cards",
            "from their hand",
        ),
        (
            "Each opponent chooses a creature card, then put it onto the battlefield under your control.",
            "creature card",
            "their graveyard",
        ),
        (
            "If it's a permanent, exile it.",
            "permanent",
            "would leave the battlefield",
        ),
        (
            "{T}: You can't be targeted until your next turn. Prevent all damage that would be dealt to you until your next turn.",
            "damage",
            "protection from everything",
        ),
        (
            "When this artifact enters, if you cast it, you can't be targeted until your next turn. Prevent all damage that would be dealt to you until your next turn.",
            "damage",
            "protection from everything",
        ),
        (
            "You can't have life total changed until your next turn, you gain shroud until your next turn, and prevent all damage that would be dealt to you until your next turn.",
            "shroud",
            "protection from everything",
        ),
        (
            "Whenever this creature or least two other creatures attack, this creature gets +2/+2 until end of turn.",
            "or least",
            "and at least",
        ),
        (
            "Whenever another Cat you control attacks, you may pay {1}{G}{W}. If you do, it gains trample. This creature gets +X/+X until end of turn, where X is this creature's power.",
            "this creature's power",
            "where X is its power",
        ),
        (
            "{T}: Choose target creature an opponent controls and creature. Flip a coin. If you win the flip, destroy that creature. If you lose the flip, destroy the creature your opponent chose.",
            "Flip a coin",
            "that opponent chooses target creature",
        ),
        (
            "Return target permanent spell to its owner's hand, Jeskai Revelation deals 4 damage to any target, create two 1/1 white Monk creature tokens with prowess, draw two cards, then gain 4 life.",
            "permanent spell",
            "spell or permanent",
        ),
    ];
    for (input, kept, forbidden) in cases {
        let output = normalize_ast_surface_lines(vec![input.into()])
            .into_iter()
            .map(normalize_scored_compiled_line)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(output.contains(kept), "lost {kept}: {output}");
        assert!(
            !output.contains(forbidden),
            "invented {forbidden}: {output}"
        );
    }
    let output = normalize_ast_surface_lines(vec![
        "Enchanted creature has \"{T}: This creature deals 1 damage to target creature.\"".into(),
        "Enchanted creature has \"{T}: This creature deals 2 damage to target creature.\"".into(),
    ])
    .join("\n");
    assert!(!output.to_lowercase().contains("wizard"), "{output}");
}

#[test]
fn rescues_cannibalize_declares_only_the_pair_and_resolves_to_its_complement() {
    for (name, amount, word) in [("Cannibalize", 2, "two"), ("Partition fixture", 3, "three")] {
        let def = parse(
            name,
            &format!(
                "Type: Sorcery\nChoose two target creatures controlled by the same player. Exile one of those creatures and put {word} +1/+1 counters on the other."
            ),
        );
        let text = compiled_text_lines(&def).join("\n");
        assert!(
            text.contains(&format!("put {word} +1/+1 counters on the other")),
            "{text}"
        );
        let bear = parse("Bear", "Type: Creature — Bear\nPower/Toughness: 2/2");
        for reverse in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let a = game.create_object_from_definition(&bear, PlayerId(1), Zone::Battlefield);
            let b = game.create_object_from_definition(&bear, PlayerId(1), Zone::Battlefield);
            let decoy = game.create_object_from_definition(&bear, PlayerId(0), Zone::Battlefield);
            let source = game.create_object_from_definition(&def, PlayerId(0), Zone::Stack);
            let reqs = crate::game_loop::extract_target_requirements_from_program_with_modes(
                &game,
                def.spell_effect.as_ref().unwrap(),
                PlayerId(0),
                Some(source),
                None,
            );
            assert_eq!(reqs.len(), 1, "must not announce a third target: {reqs:?}");
            assert_eq!((reqs[0].min_targets, reqs[0].max_targets), (2, Some(2)));
            let ids = if reverse { [b, a] } else { [a, b] };
            let mut entry = StackEntry::new(source, PlayerId(0))
                .with_targets(ids.into_iter().map(Target::Object).collect());
            entry.target_assignments = vec![TargetAssignment {
                spec: reqs[0].spec.clone(),
                range: 0..2,
            }];
            game.push_to_stack(entry);
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
                .unwrap();
            let survivors: Vec<_> = [a, b]
                .into_iter()
                .filter(|id| game.battlefield.contains(id))
                .collect();
            assert_eq!(survivors.len(), 1);
            assert_eq!(game.exile.len(), 1);
            assert_eq!(
                game.counter_count(survivors[0], CounterType::PlusOnePlusOne),
                amount
            );
            assert_eq!(game.counter_count(decoy, CounterType::PlusOnePlusOne), 0);
        }
    }
}

#[test]
fn rescues_cannibalize_does_not_reuse_an_illegal_target() {
    let def = parse(
        "Cannibalize",
        "Type: Sorcery\nChoose two target creatures controlled by the same player. Exile one of those creatures and put two +1/+1 counters on the other.",
    );
    let bear = parse("Bear", "Type: Creature — Bear\nPower/Toughness: 2/2");
    for invalid in [1, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let a = game.create_object_from_definition(&bear, PlayerId(1), Zone::Battlefield);
        let b = game.create_object_from_definition(&bear, PlayerId(1), Zone::Battlefield);
        let source = game.create_object_from_definition(&def, PlayerId(0), Zone::Stack);
        let reqs = crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game,
            def.spell_effect.as_ref().unwrap(),
            PlayerId(0),
            Some(source),
            None,
        );
        let mut entry = StackEntry::new(source, PlayerId(0))
            .with_targets(vec![Target::Object(a), Target::Object(b)]);
        entry.target_assignments = vec![TargetAssignment {
            spec: reqs[0].spec.clone(),
            range: 0..2,
        }];
        game.push_to_stack(entry);
        let moved: Vec<_> = [a, b]
            .into_iter()
            .take(invalid)
            .map(|id| game.move_object_by_effect(id, Zone::Graveyard).unwrap())
            .collect();
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(game.exile.len(), 2 - invalid);
        for id in moved {
            assert_eq!(game.object(id).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 0);
        }
    }
}

#[test]
fn rescues_attached_grant_retains_its_typed_condition_and_recipient() {
    for subtype in ["Wizard", "Goblin"] {
        let def = parse(
            "Lavamancer's Skill",
            &format!(
                "Type: Enchantment — Aura\nEnchant creature\nEnchanted creature has \"{{T}}: This creature deals 1 damage to target creature.\"\nAs long as enchanted creature is a {subtype}, it has \"{{T}}: This creature deals 2 damage to target creature.\""
            ),
        );
        for lines in [debug_compiled_lines(&def), compiled_text_lines(&def)] {
            let text = lines.join("\n").to_lowercase();
            assert!(
                text.contains(&format!(
                    "as long as enchanted creature is a {}",
                    subtype.to_lowercase()
                )),
                "{text}"
            );
            assert!(
                text.contains("enchanted creature has \"{t}: this creature deals 2"),
                "{text}"
            );
            assert!(!text.contains("this source has"), "{text}");
        }
    }
}

#[test]
fn rescues_choice_zones_come_from_the_model() {
    for (name, input, zone) in [
        (
            "Stunted Growth",
            "Type: Sorcery\nTarget player chooses three cards from their hand and puts them on top of their library in any order.",
            "hand",
        ),
        (
            "Dredge the Mire",
            "Type: Sorcery\nEach opponent chooses a creature card in their graveyard. Put those cards onto the battlefield under your control.",
            "graveyard",
        ),
    ] {
        let def = parse(name, input);
        for lines in [debug_compiled_lines(&def), compiled_text_lines(&def)] {
            assert!(lines.join("\n").contains(zone), "{name}: {lines:?}");
        }
    }
}
