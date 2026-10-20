//! Negative/regression guard for the static rules p03 made whole-line
//! (filtered ETB tapped/untapped/counter replacements, retrace grants,
//! skip-upkeep, skip-untap). Lines with other heads must keep their previous
//! readings and must not become ambiguous. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

#[test]
fn turn_gated_static_rules_use_the_authored_time_window() {
    let grand_abolisher = compile_to_runtime_definition(
        "Grand Abolisher",
        "Mana cost: {W}{W}\nType: Creature — Human Cleric\nPower/Toughness: 2/2\nDuring your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments.",
        false,
    )
    .unwrap();
    let text = ironsmith_text::compiled_text_lines(&grand_abolisher).join("\n");
    assert!(
        text.contains("During your turn, your opponents can't cast spells or activate abilities"),
        "{text}"
    );
    assert!(!text.contains("timing restriction:"), "{text}");
}

#[test]
fn shared_type_unless_blocking_is_static_and_keeps_the_full_condition() {
    let definition = compile(
        "This creature can't be blocked unless defending player controls three or more creatures that share a creature type.",
    );
    assert!(definition.spell_effect.is_none(), "{definition:#?}");
    assert!(static_ids(&definition).contains(&StaticAbilityId::RuleRestriction));
    let abilities = format!("{:#?}", definition.abilities);
    assert!(abilities.contains("BeBlocked"), "{abilities}");
    assert!(abilities.contains("GreatestSharedCreatureTypeCount"), "{abilities}");
    assert!(abilities.contains("Defending"), "{abilities}");
    assert!(abilities.contains("Not("), "{abilities}");

    let text = ironsmith_text::compiled_text_lines(&definition).join("\n");
    assert!(
        text.contains(
            "can't be blocked unless the defending player controls three or more creatures that share a creature type"
        ),
        "{text}"
    );
}

fn compile(text: &str) -> CardDefinition {
    let source = format!("Mana cost: {{2}}\nType: Creature — Bear\nPower/Toughness: 2/2\n{text}");
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition("Whole-line regression", &source, false)
    });
    assert!(!loss.is_lossy(), "{text}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{text}: {error}"));
    compile_to_artifact("Whole-line regression", &source, false)
        .unwrap_or_else(|error| panic!("{text} (artifact): {error}"));
    direct
}

fn static_ids(definition: &CardDefinition) -> Vec<StaticAbilityId> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(ability.id()),
            _ => None,
        })
        .collect()
}

#[test]
fn other_heads_keep_their_readings() {
    for (text, forbidden) in [
        (
            "As long as you control a Swamp, this creature gets +1/+1.",
            &[
                StaticAbilityId::EnterTappedForFilter,
                StaticAbilityId::EnterUntappedForFilter,
                StaticAbilityId::EnterWithCountersForFilter,
            ][..],
        ),
        ("Each creature you control gets +1/+1.", &[StaticAbilityId::EnterTappedForFilter][..]),
        ("This creature enters tapped.", &[StaticAbilityId::EnterTappedForFilter][..]),
        (
            "Creatures your opponents control enter tapped.",
            &[StaticAbilityId::EntersTapped][..],
        ),
        (
            "Instant and sorcery cards in your graveyard have retrace.",
            &[StaticAbilityId::PlayersSkipUpkeep][..],
        ),
        ("Players skip their upkeep steps.", &[StaticAbilityId::PlayersSkipUntapStep][..]),
        ("Skip your draw step.", &[StaticAbilityId::PlayersSkipUntapStep][..]),
    ] {
        let ids = static_ids(&compile(text));
        assert!(!ids.is_empty(), "{text}");
        for id in forbidden {
            assert!(!ids.contains(id), "{text} gained {id:?}: {ids:?}");
        }
    }
}

#[test]
fn opponents_enter_tapped_keeps_filter_reading() {
    let ids = static_ids(&compile("Creatures your opponents control enter tapped."));
    assert_eq!(ids, vec![StaticAbilityId::EnterTappedForFilter]);
}

#[test]
fn resolution_pronoun_enters_tapped_is_not_a_static() {
    let text = "Mana cost: {2}{G}\nType: Sorcery\nPut up to two land cards from your hand onto the battlefield. They enter tapped.";
    let definition = compile_to_runtime_definition("Pronoun probe", text, false)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(static_ids(&definition).is_empty(), "'They enter tapped' must stay a resolution rider");
}
