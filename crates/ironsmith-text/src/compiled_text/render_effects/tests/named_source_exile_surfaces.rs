use super::*;

fn render_named_creature(name: &str, text: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("named-source activated text should compile");
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn activated_self_exile_uses_the_complete_authored_short_name() {
    for (name, text) in [
        (
            "Clive, Ifrit's Dominant",
            "{4}{R}{R}, {T}: Exile Clive, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.",
        ),
        (
            "Terra, Magical Adept",
            "Trance — {4}{R}{G}, {T}: Exile Terra, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.",
        ),
    ] {
        assert_eq!(render_named_creature(name, text), text);
    }
}

#[test]
fn activated_named_transform_keeps_authored_return_pronoun_and_possessive() {
    let text = "{T}: Draw a card, then discard a card. If there are five or more cards in your graveyard, exile Jace, then return him to the battlefield transformed under his owner's control.";
    assert_eq!(render_named_creature("Jace, Vryn's Prodigy", text), text);
}

#[test]
fn ordinary_this_creature_exile_is_not_rewritten_as_a_name() {
    let text = "{1}, {T}: Exile this creature, then return it to the battlefield under its owner's control.";
    let rendered = render_named_creature("Source Map Probe", text);

    assert!(rendered.contains("Exile this,"), "{rendered}");
    assert!(!rendered.contains("Exile Source Map Probe"), "{rendered}");
}

#[test]
fn triggered_conditional_self_exile_keeps_the_authored_short_name() {
    let text = "Deathtouch\nWhenever Grist or another creature you control enters, if it entered from your graveyard or you cast it from your graveyard, you may pay {G}. If you do, exile Grist, then return it to the battlefield transformed under its owner's control.";

    assert_eq!(render_named_creature("Grist, Voracious Larva", text), text);
}

#[test]
fn multiword_named_transform_and_chosen_complement_keep_the_authored_source() {
    let text = "My First Friend — When Zenos yae Galvus enters, choose a creature an opponent controls. Until end of turn, creatures other than Zenos yae Galvus and the chosen creature get -2/-2.\nWhen the chosen creature leaves the battlefield, transform Zenos yae Galvus.";

    assert_eq!(render_named_creature("Zenos yae Galvus", text), text);
}

#[test]
fn named_equipment_unattach_then_transform_keeps_both_authored_references() {
    let text = "Equipped creature gets +1/+0.\nWhen equipped creature deals combat damage to a player, unattach Elbrus, then transform it.\nEquip {1}";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Elbrus, the Binding Blade")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .parse_text(text)
            .expect("named Equipment transform text should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn labeled_reflexive_trigger_keeps_named_source_counter_target() {
    let text = "Nitro-9 — Whenever Ace attacks, you may sacrifice an artifact. When you do, put a +1/+1 counter on Ace, then it fights up to one target creature defending player controls.\nDoctor's companion";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ace, Fearless Rebel")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .parse_text(text)
            .expect("labeled named-source reflexive trigger should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );

    let ordinary = "Whenever this creature attacks, put a +1/+1 counter on this creature.";
    assert_eq!(
        render_named_creature("Ace, Fearless Rebel", ordinary),
        ordinary
    );
}

#[test]
fn named_source_copy_keeps_the_complete_copy_exception_bundle() {
    let text = "Vigilance\nAt the beginning of your first main phase, until your next turn, Absorbing Man becomes a copy of up to one target artifact, non-Aura enchantment, or land, except his name is Absorbing Man, he's a legendary 4/4 Human Villain creature in addition to his other types, and he has vigilance.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Absorbing Man")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("named copy-exception trigger should compile");

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("becomes a copy of up to one target artifact"),
        "{rendered}"
    );
    assert!(
        rendered.contains("except his name is Absorbing Man"),
        "{rendered}"
    );
    assert!(
        rendered.contains("legendary 4/4 human villain creature"),
        "{rendered}"
    );
    assert!(rendered.contains("and he has vigilance"), "{rendered}");
    assert!(!rendered.contains("choose up to one target"), "{rendered}");
    assert!(
        !rendered.contains("and this creature gains vigilance"),
        "{rendered}"
    );

    let ordinary = "Until your next turn, this creature becomes a copy of target creature and gains vigilance.";
    let ordinary_rendered = render_named_creature("Absorbing Man", ordinary);
    assert!(
        !ordinary_rendered.contains("except his name is"),
        "{ordinary_rendered}"
    );
}
