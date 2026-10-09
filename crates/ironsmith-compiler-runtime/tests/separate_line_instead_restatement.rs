//! A separate line "<Ability word> — If <condition>, <restated action>
//! instead." modifies the preceding spell statement (a self-replacement,
//! CR 614.1a): one damage instruction whose amount depends on the condition.
//! Frozen complete bodies; source-authored and UNRUN.
use ironsmith::effect::Value;
use ironsmith::effects::DealDamageEffect;

#[path = "cf8_p08/support.rs"]
mod support;

const BODIES: &[(&str, &str, i32, i32)] = &[
    ("Slaying Fire", "Mana cost: {2}{R}\nType: Instant\nSlaying Fire deals 3 damage to any target.\nAdamant — If at least three red mana was spent to cast this spell, it deals 4 damage instead.", 3, 4),
    ("Summary Judgment", "Mana cost: {1}{W}\nType: Instant\nSummary Judgment deals 3 damage to target tapped creature.\nAddendum — If you cast this spell during your main phase, it deals 5 damage instead.", 3, 5),
    ("Fiery Impulse", "Mana cost: {R}\nType: Instant\nFiery Impulse deals 2 damage to target creature.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, Fiery Impulse deals 3 damage instead.", 2, 3),
];

#[test]
fn the_instead_line_is_one_conditional_amount_for_the_same_target() {
    for (name, body, base, boosted) in BODIES {
        for definition in support::definitions(name, body) {
            let damage = support::find_all::<DealDamageEffect>(&definition);
            let mut amounts: Vec<_> = damage.iter().map(|damage| damage.amount.clone()).collect();
            amounts.sort_by_key(|amount| format!("{amount:?}"));
            assert_eq!(damage.len(), 2, "{name}: exactly the default and the replacement arm");
            assert!(amounts.contains(&Value::Fixed(*base)) && amounts.contains(&Value::Fixed(*boosted)), "{name}: {amounts:?}");
            assert_eq!(damage[0].target, damage[1].target, "{name}: both arms hit the one declared target");
            assert_eq!(definition.spell_effect.as_ref().unwrap().all_effects().len(), 1, "{name}: one top-level instruction");
            let text = support::rendered(&definition);
            assert!(text.contains("instead"), "{name}: {text}");
        }
    }
}

#[test]
fn an_instead_line_restating_a_different_action_is_not_joined() {
    // The statement draws; the "instead" line restates a damage action it
    // never performed, so it must not become a self-replacement of the draw.
    for body in [
        "Mana cost: {U}\nType: Instant\nDraw a card.\nIf you control a Wizard, this spell deals 2 damage instead.",
        "Mana cost: {R}\nType: Instant\nThis spell deals 2 damage to any target. Draw a card.\nIf you control a Wizard, it deals 3 damage instead.",
    ] {
        let compiled = ironsmith_compiler_runtime::compile_to_runtime_definition("Unrelated instead", body, false);
        if let Ok(definition) = compiled {
            let amounts: Vec<_> = support::find_all::<DealDamageEffect>(&definition)
                .into_iter()
                .map(|damage| damage.amount)
                .collect();
            assert!(!amounts.contains(&Value::Fixed(3)), "{body}: joined an unrelated restatement");
            assert!(amounts.len() <= 1, "{body}: {amounts:?}");
        }
    }
}

#[test]
fn an_ordinary_replacement_line_is_not_joined_as_a_restatement() {
    let body = "Mana cost: {R}\nType: Instant\nThis spell deals 2 damage to any target.\nIf a source would deal damage to you, prevent that damage instead.";
    if let Ok(definition) = ironsmith_compiler_runtime::compile_to_runtime_definition("Replacement neighbor", body, false) {
        assert_eq!(support::find_all::<DealDamageEffect>(&definition).len(), 1);
    }
}
