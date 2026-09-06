use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ForEachEffectAst;
use super::*;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;

const LIM_DUL_EFFECT: &str =
    "For each player, this enchantment deals 1 damage to that player unless they pay {B} or {3}.";

#[test]
fn trailing_they_pay_keeps_the_each_player_ast_and_payer_reference() {
    let tokens = lex_line(LIM_DUL_EFFECT, 0).expect("Lim-Dûl effect should lex");
    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("Lim-Dûl effect sentence should parse");

    let [
        EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
            effects,
            player,
            cost,
            ..
        }),
    ] = parsed.as_slice()
    else {
        panic!("expected a typed trailing unless-payment wrapper, got {parsed:#?}");
    };
    assert_eq!(*player, crate::cards::builders::PlayerAst::That);
    assert!(
        matches!(
            cost.kind(),
            ironsmith_core::TotalCostKind::OneOf(branches) if branches.len() == 2
        ),
        "expected the two authored alternative payments, got {cost:#?}"
    );
    assert!(
        matches!(effects.as_slice(), [EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { .. })]),
        "expected the consequence to retain its each-player loop, got {effects:#?}"
    );
}

#[test]
fn lowering_moves_they_pay_inside_the_each_player_loop() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Lim-Dûl's Hex Variant")
        .card_types(vec![CardType::Enchantment]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        format!("At the beginning of your upkeep, {LIM_DUL_EFFECT}"),
        false,
    )?;

    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!(
            "expected a triggered ability, got {:#?}",
            definition.abilities
        );
    };
    let [outer] = triggered.effects.segments[0].default_effects.as_slice() else {
        panic!(
            "expected one outer per-player effect, got {:#?}",
            triggered.effects
        );
    };
    let for_players = outer
        .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
        .expect("payment must be nested under the each-player runtime loop");
    assert_eq!(for_players.filter, crate::target::PlayerFilter::Any);

    let [per_player] = for_players.effects.as_slice() else {
        panic!("expected one per-player payment branch, got {for_players:#?}");
    };
    let unless_pays = per_player
        .downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
        .expect("each iterated player should receive their own payment choice");
    assert_eq!(
        unless_pays.player,
        crate::target::PlayerFilter::IteratedPlayer
    );
    assert!(
        matches!(
            unless_pays.cost.kind(),
            ironsmith_core::TotalCostKind::OneOf(branches) if branches.len() == 2
        ),
        "expected two alternative costs, got {:#?}",
        unless_pays.cost
    );

    let [damage_effect] = unless_pays.effects.as_slice() else {
        panic!("expected one damage consequence, got {unless_pays:#?}");
    };
    let damage = damage_effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("nonpayment should deal damage");
    assert_eq!(
        damage.target,
        crate::target::ChooseSpec::Player(crate::target::PlayerFilter::IteratedPlayer)
    );

    Ok(())
}

#[test]
fn imperative_consequence_keeps_the_iterated_players_sacrifice_payment() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Quantified Unless Probe")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Whenever this creature attacks, for each opponent, you create a 2/2 black Zombie creature token unless that player sacrifices a creature of their choice."
            .to_string(),
        false,
    )?;

    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected a triggered ability");
    };
    let [outer] = triggered.effects.segments[0].default_effects.as_slice() else {
        panic!("expected one quantified effect: {:#?}", triggered.effects);
    };
    let for_players = outer
        .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
        .expect("the consequence should remain quantified by opponent");
    let [per_player] = for_players.effects.as_slice() else {
        panic!("expected one per-player payment branch: {for_players:#?}");
    };
    let unless_pays = per_player
        .downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
        .expect("the sacrifice must remain an unless payment");
    assert_eq!(
        unless_pays.player,
        crate::target::PlayerFilter::IteratedPlayer
    );
    assert!(format!("{:#?}", unless_pays.cost).contains("Sacrifice"));

    Ok(())
}
