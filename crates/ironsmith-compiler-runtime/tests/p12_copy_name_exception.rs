//! CR 707.9b token-copy name/supertype exceptions are kept, never dropped.
//! Collateral of "copy ..., except its name is X" silently losing the name.
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::effects::CreateTokenCopyEffect;
use ironsmith::{Subtype, Supertype};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const MISHRA: &str = "Mana cost: {2}{U}{B}{R}\nType: Legendary Creature — Human Artificer\nPower/Toughness: 5/4\nAt the beginning of combat on your turn, create a token that's a copy of target noncreature artifact you control, except its name is Mishra's Warform and it's a 4/4 Construct artifact creature in addition to its other types. It gains haste until end of turn. Sacrifice it at the beginning of the next end step.";
const ELEVENTH_HOUR: &str = "Mana cost: {3}{U}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — Search your library for a Doctor card, reveal it, put it into your hand, then shuffle.\nII — Create a Food token and a 1/1 white Human creature token with \"Doctor spells you cast cost {1} less to cast.\"\nIII — Create a token that's a copy of target creature, except it's a legendary Alien named Prisoner Zero.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn copy_effect(definition: &CardDefinition) -> CreateTokenCopyEffect {
    fn collect(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| collect(child, out));
    }
    let mut all = Vec::new();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() {
                collect(effect, &mut all);
            }
        }
    }
    all.iter()
        .find_map(|effect| effect.downcast_ref::<CreateTokenCopyEffect>().cloned())
        .expect("typed token copy")
}

#[test]
fn mishra_warform_keeps_its_name_and_construct_exception() {
    for definition in definitions("Mishra, Eminent One", MISHRA) {
        let copy = copy_effect(&definition);
        assert_eq!(copy.set_name.as_deref(), Some("Mishra's Warform"));
        assert_eq!(copy.set_base_power_toughness, Some((4, 4)));
        assert!(copy.added_subtypes.contains(&Subtype::Construct));
        assert!(!copy.added_subtypes.iter().any(|subtype| format!("{subtype:?}").contains("Warform")));
    }
}

#[test]
fn eleventh_hour_prisoner_zero_is_a_legendary_alien_named_copy() {
    for definition in definitions("The Eleventh Hour", ELEVENTH_HOUR) {
        let copy = copy_effect(&definition);
        assert_eq!(copy.set_name.as_deref(), Some("Prisoner Zero"));
        assert!(copy.added_supertypes.contains(&Supertype::Legendary));
        assert_eq!(copy.set_subtypes.as_deref(), Some(&[Subtype::Alien][..]));
    }
}
