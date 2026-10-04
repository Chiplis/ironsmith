//! Totals over completed, replacement-adjusted assignments of one action.
use crate::events::{DamageEvent, DamageTarget};
use crate::ids::ObjectId;
use crate::triggers::TriggerEvent;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
struct DamageAmounts {
    combat: u128,
    noncombat: u128,
}
impl DamageAmounts {
    fn add(&mut self, amount: u32, combat: bool) {
        let total = if combat {
            &mut self.combat
        } else {
            &mut self.noncombat
        };
        *total += u128::from(amount);
    }
    fn amount(&self, combat: Option<bool>) -> u128 {
        match combat {
            Some(true) => self.combat,
            Some(false) => self.noncombat,
            None => self.combat + self.noncombat,
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct ReceivedDamageAmounts {
    recipient: DamageAmounts,
    source_recipient: DamageAmounts,
}
impl ReceivedDamageAmounts {
    pub(super) fn amount(&self, combat: Option<bool>, single_source: bool) -> u128 {
        if single_source {
            &self.source_recipient
        } else {
            &self.recipient
        }
        .amount(combat)
    }
}
/// Only completed-event queue owners invoke this with all assignments from
/// one simultaneous operation. Separate instructions never share a sum.
/// Already completed receipts retain their own original occurrence. A broader
/// outer simultaneous scope can publish several independently completed damage
/// instructions together; it cannot merge their immutable source/recipient totals.
pub(crate) fn bind_received_damage_amounts(events: &mut [TriggerEvent]) {
    let mut recipients: HashMap<DamageTarget, DamageAmounts> = HashMap::new();
    let mut source_recipients: HashMap<(ObjectId, DamageTarget), DamageAmounts> = HashMap::new();
    for event in events.iter() {
        let Some(damage) = event
            .downcast::<DamageEvent>()
            .filter(|damage| damage.received_amounts.is_none())
        else {
            continue;
        };
        recipients
            .entry(damage.target)
            .or_default()
            .add(damage.amount, damage.is_combat);
        source_recipients
            .entry((damage.source, damage.target))
            .or_default()
            .add(damage.amount, damage.is_combat);
    }
    for event in events {
        let Some(damage) = event
            .downcast::<DamageEvent>()
            .filter(|damage| damage.received_amounts.is_none())
        else {
            continue;
        };
        let mut damage = damage.clone();
        damage.received_amounts = Some(ReceivedDamageAmounts {
            recipient: recipients[&damage.target].clone(),
            source_recipient: source_recipients[&(damage.source, damage.target)].clone(),
        });
        *event = event.with_inner_event(damage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cause::EventCause;
    fn event(source: u64, target: DamageTarget, amount: u32, combat: bool) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                ObjectId::from_raw(source),
                target,
                amount,
                combat,
                EventCause::effect(),
            ),
            Default::default(),
        )
    }
    #[test]
    fn batch_totals_distinguish_each_recipient_source_and_combat_class() {
        let a = DamageTarget::Player(crate::PlayerId(0));
        let b = DamageTarget::Player(crate::PlayerId(1));
        let mut events = vec![
            event(1, a, 1, false),
            event(2, a, 2, false),
            event(1, a, 4, true),
            event(1, b, 7, false),
        ];
        bind_received_damage_amounts(&mut events);
        let first = events[0].downcast::<DamageEvent>().unwrap();
        assert_eq!(first.received_amount(None, false), 7);
        assert_eq!(first.received_amount(Some(false), false), 3);
        assert_eq!(first.received_amount(None, true), 5);
        assert_eq!(first.received_amount(Some(false), true), 1);
        assert_eq!(
            events[1]
                .downcast::<DamageEvent>()
                .unwrap()
                .received_amount(None, true),
            2
        );
        assert_eq!(
            events[3]
                .downcast::<DamageEvent>()
                .unwrap()
                .received_amount(None, false),
            7
        );
        assert_eq!(
            first.amount, 1,
            "each contribution retains its own amount for grouped body totals"
        );
    }
    #[test]
    fn threshold_evidence_retains_wide_totals_without_saturating() {
        let player = DamageTarget::Player(crate::PlayerId(0));
        let mut events = vec![
            event(1, player, u32::MAX, false),
            event(2, player, u32::MAX, false),
        ];
        bind_received_damage_amounts(&mut events);
        assert_eq!(
            events[0]
                .downcast::<DamageEvent>()
                .unwrap()
                .received_amount(None, false),
            u128::from(u32::MAX) * 2
        );
    }
    #[test]
    fn independent_assignments_and_modified_prospective_events_cannot_reuse_totals() {
        let a = DamageTarget::Player(crate::PlayerId(0));
        let mut batch = vec![event(1, a, 1, false), event(1, a, 2, false)];
        bind_received_damage_amounts(&mut batch);
        let first = batch[0].downcast::<DamageEvent>().unwrap();
        assert_eq!(first.received_amount(None, true), 3);
        assert_eq!(first.with_amount(1).received_amount(None, true), 1);
        assert_eq!(
            first
                .with_target(DamageTarget::Player(crate::PlayerId(1)))
                .received_amount(None, true),
            1
        );
        assert_eq!(first.prevented().received_amount(None, false), 0);
        assert_eq!(
            event(1, a, 2, false)
                .downcast::<DamageEvent>()
                .unwrap()
                .received_amount(None, true),
            2
        );
    }
    #[test]
    fn a_later_publication_scope_cannot_merge_two_already_completed_occurrences() {
        let recipient = DamageTarget::Player(crate::PlayerId(0));
        let mut first = vec![event(1, recipient, 2, false), event(1, recipient, 3, false)];
        let mut second = vec![event(1, recipient, 4, false)];
        bind_received_damage_amounts(&mut first);
        bind_received_damage_amounts(&mut second);
        first.extend(second);
        bind_received_damage_amounts(&mut first);
        let totals = first
            .iter()
            .map(|event| {
                event
                    .downcast::<DamageEvent>()
                    .unwrap()
                    .completed_source_recipient_amount(None)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            totals,
            vec![5, 5, 4],
            "one outer queue boundary is not one original damage occurrence"
        );
    }
}
