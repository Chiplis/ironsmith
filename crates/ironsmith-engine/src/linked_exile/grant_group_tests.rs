use super::*;
use crate::continuous::{ContinuousEffect, ContinuousEffectManager, EffectTarget, Modification};
use crate::ids::PlayerId;

#[test]
fn shared_grant_links_keep_distinct_ability_occurrences_and_separate_later_grants() {
    let host = ObjectId::from_raw(1);
    let pair = LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition([83; 32]),
        pair: 0,
    };
    let mut manager = ContinuousEffectManager::new();
    let group = manager.next_effect_group_id();
    let definition = || {
        ContinuousEffect::new(
            host,
            PlayerId::from_index(0),
            EffectTarget::Source,
            Modification::RemoveAllAbilities,
        )
    };
    for _ in 0..2 {
        manager.add_effect(definition().with_group(group));
    }
    let separate_group = manager.next_effect_group_id();
    manager.add_effect(definition().with_group(separate_group));
    manager.add_effect(definition());
    let origins: Vec<_> = manager
        .effects()
        .iter()
        .map(|effect| AbilityOrigin::Effect {
            effect: AbilityEffectOrigin::from(effect),
            slot: 0,
        })
        .collect();
    assert_ne!(
        origins[0], origins[1],
        "mana recursion and other occurrence consumers stay distinct"
    );
    let owner =
        |origin: &AbilityOrigin| LinkedExileOwner::capture(host, Some(pair), Some(origin)).unwrap();
    assert_eq!(owner(&origins[0]), owner(&origins[1]));
    assert_ne!(owner(&origins[0]), owner(&origins[2]));
    assert_ne!(owner(&origins[0]), owner(&origins[3]));
    assert_ne!(
        owner(&origins[0]),
        LinkedExileOwner::capture(ObjectId::from_raw(2), Some(pair), Some(&origins[1])).unwrap()
    );
    let other_pair = LinkedExilePair { pair: 1, ..pair };
    assert_ne!(
        owner(&origins[0]),
        LinkedExileOwner::capture(host, Some(other_pair), Some(&origins[1])).unwrap()
    );
    let unregistered = AbilityOrigin::Effect {
        effect: AbilityEffectOrigin::from(&definition().with_group(group)),
        slot: 0,
    };
    assert_ne!(
        owner(&origins[0]),
        owner(&unregistered),
        "unregistered static descriptors need their generating occurrence"
    );
}
