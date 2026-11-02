use ironsmith::ability::AbilityKind;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_core::custom_subtypes::CustomSubtypeDefinition;
use ironsmith_core::SubtypeFamily;
use ironsmith_compiler_runtime::compile_to_runtime_definition;

fn main() {
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(run).unwrap().join().unwrap();
}

fn run() {
    ironsmith_compiler::with_custom_subtypes(&[CustomSubtypeDefinition {
        singular: "Factory".into(), plural: "Factories".into(), family: SubtypeFamily::Land,
    }], || {
        let compile = |name: &str, text: &str| compile_to_runtime_definition(name, text, false).unwrap();
        let probe = compile("Factory mana probe", "Type: Land\n{T}: Add one mana of any color that a Factory you control could produce.");
        println!("Revised wording compiled: {}", ironsmith_text::canonical_compiled_lines(&probe).join("\n"));
        let original = compile_to_runtime_definition("Original wording", "Type: Land\n{T}: Add one mana of any color among colors that Factories you control could produce.", false);
        println!("Original wording: {:?}", original.as_ref().map(|d| ironsmith_text::canonical_compiled_lines(d)));
        let blue = compile("Blue Factory", "Type: Land — Factory\n{T}: Add {U}.");
        let red = compile("Red Factory", "Type: Land — Factory\n{T}: Add {R}.");
        let green = compile("Non-Factory", "Type: Land\n{T}: Add {G}.");
        let colorless = compile("Colorless Factory", "Type: Land — Factory\n{T}: Add {C}.");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let alice = PlayerId(0);
        let source = game.create_object_from_definition(&probe, alice, Zone::Battlefield);
        let blue_id = game.create_object_from_definition(&blue, alice, Zone::Battlefield);
        game.create_object_from_definition(&red, PlayerId(1), Zone::Battlefield);
        game.create_object_from_definition(&green, alice, Zone::Battlefield);
        game.create_object_from_definition(&colorless, alice, Zone::Battlefield);
        let AbilityKind::Activated(ability) = &probe.abilities[0].kind else { panic!("expected activation") };
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in ability.effects.all_effects() {
            effect.0.execute(&mut game, &mut ctx).unwrap();
        }
        println!("Actual mana pool: {:?}", game.player(alice).unwrap().mana_pool);
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
        println!("PASS: resolving the compiled effect added exactly one blue mana.");
        game.battlefield.retain(|id| *id != blue_id);
        game.player_mut(alice).unwrap().mana_pool = Default::default();
        for effect in ability.effects.all_effects() {
            effect.0.execute(&mut game, &mut ctx).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        println!("PASS: opponent's Factory, non-Factory land, and colorless Factory contribute no eligible colored mana.");
    }).unwrap();
}
