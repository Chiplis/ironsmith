//! Blacksmith's Skill card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::{Condition, Effect, EffectId, EffectPredicate, Until};
use crate::effects::GrantAbilitiesTargetEffect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::CardType;

/// Blacksmith's Skill - {W}
/// Instant
/// Target permanent gains hexproof and indestructible until end of turn.
/// If it's an artifact creature, it gets +2/+2 until end of turn.
pub fn blacksmiths_skill() -> CardDefinition {
    let grant = Effect::with_id(
        0,
        Effect::new(GrantAbilitiesTargetEffect::new(
            ChooseSpec::permanent(),
            vec![StaticAbility::hexproof(), StaticAbility::indestructible()],
            Until::EndOfTurn,
        ))
        .tag("target"),
    );
    let pump = Effect::if_then(
        EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::conditional(
            Condition::TaggedObjectMatches(
                "target".into(),
                ObjectFilter::artifact().with_all_type(CardType::Creature),
            ),
            vec![Effect::pump(
                2,
                2,
                ChooseSpec::Tagged("target".into()),
                Until::EndOfTurn,
            )],
            vec![],
        )],
    );

    CardDefinitionBuilder::new(CardId::new(), "Blacksmith's Skill")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![grant, pump])
        .oracle_text(
            "Target permanent gains hexproof and indestructible until end of turn. \
             If it's an artifact creature, it gets +2/+2 until end of turn.",
        )
        .build()
}
