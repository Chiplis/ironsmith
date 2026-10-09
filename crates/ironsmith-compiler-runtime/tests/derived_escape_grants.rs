//! cf8/p07: "<card> gains escape until end of turn. The escape cost is equal
//! to its mana cost plus exile N other cards from your graveyard." is a
//! derived escape grant (CR 702.138a). Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::alternative_cast::AlternativeCastingMethod;
use ironsmith::effects::{EffectContext as ExecutionContext, EffectExecutor, GrantEffect, ResolvedTarget};
use ironsmith::grant::{DerivedAlternativeCast, GrantDuration, Grantable};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const FIXTURE: &str = include_str!("../../../fixtures/derived_escape_grants.json.fixture");
const A: PlayerId = PlayerId(0);

#[test]
fn confession_dial_grants_escape_with_three_other_graveyard_cards() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Confession Dial");
    assert_eq!(row["oracle_id"], "6316158c-2f66-4dc4-b78c-c6ff7c39bb77");
    for definition in support::definitions(row) {
        let grants: Vec<GrantEffect> = support::activated(&definition)
            .into_iter()
            .flat_map(|activated| support::find::<GrantEffect>(&support::activated_effects(activated)))
            .collect();
        assert_eq!(grants.len(), 1);
        let grant = &grants[0];
        assert!(matches!(
            grant.grantable,
            Grantable::DerivedAlternativeCast(DerivedAlternativeCast::EscapeFromCardManaCost {
                exile_count: 3
            })
        ));
        assert_eq!(grant.duration, GrantDuration::UntilEndOfTurn);
        assert!(grant.target.is_target());

        // The granted card can be cast with escape from the graveyard this turn.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let legend = compile_to_runtime_definition(
            "Legendary Probe",
            "Mana cost: {2}{B}\nType: Legendary Creature — Human\nPower/Toughness: 3/3",
            false,
        )
        .unwrap();
        let card = game.create_object_from_definition(&legend, A, Zone::Graveyard);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, A);
        ctx.targets = vec![ResolvedTarget::Object(card)];
        grant.execute(&mut game, &mut ctx).unwrap();
        let casts = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, card, Zone::Graveyard, A);
        assert!(casts.iter().any(|cast| matches!(
            cast.method,
            AlternativeCastingMethod::Escape { exile_count: 3, .. }
        )));
    }
}
