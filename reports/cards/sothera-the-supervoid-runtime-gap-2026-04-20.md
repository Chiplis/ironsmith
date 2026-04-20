# Sothera, the Supervoid Runtime Gap

Date: 2026-04-20

## Card

`Sothera, the Supervoid`

Oracle text:

```text
Whenever a creature you control dies, each opponent chooses a creature they control and exiles it.
At the beginning of your end step, if a player controls no creatures, sacrifice Sothera, then put a creature card exiled with it onto the battlefield under your control with two additional +1/+1 counters on it.
```

## Current compiled shape

The first triggered ability is partially represented:

- `ZoneChangeTrigger` for a creature you control dying.
- `ForPlayersEffect` over opponents.
- Each iterated opponent chooses one creature they control.
- The chosen creature is moved to exile.

The second triggered ability is not behaviorally faithful. The latest historical strict compile lowers it to:

- `BeginningOfEndStepTrigger { player: You }`.
- `TagTriggeringObjectEffect`.
- `PutCountersEffect` putting one `+1/+1` counter on the triggering object.

That misses the actual card behavior.

## Missing reusable support

This card needs reusable parser/lowering/runtime support for all of the following before the compiled text can honestly render the oracle behavior:

- Intervening-if trigger condition: `if a player controls no creatures`.
- Self-sacrifice from a noncreature enchantment source as part of a triggered ability resolution.
- A linked exile relation for "creature card exiled with it" that is created by the first trigger and consumed by the second.
- Choosing or otherwise selecting a creature card from the source-linked exile set.
- Moving that exiled card onto the battlefield under your control.
- Applying two additional `+1/+1` counters as an enter-with modification to the card moved from exile.

## Current blocker

The existing AST does not contain enough information to render Sothera's second ability without inventing behavior in compiled text. A renderer-only patch would have to fabricate the sacrifice, linked-exile selection, battlefield move, controller override, and two additional counters.

## Recommended follow-up

Implement this as a triggered-ability and zone-linking feature, not a card-specific renderer rescue:

1. Add a reusable "exiled with source" object reference/tag relation for objects moved to exile by a source ability.
2. Add parser/lowering support for `card exiled with it/this` references.
3. Add an enter-with-counters modifier for zone moves from linked exile.
4. Add parser/lowering support for `if a player controls no creatures` as an intervening-if condition.
5. Recompile Sothera and add gameplay tests covering:
   - first trigger exiles one creature per opponent,
   - second trigger does nothing while every player controls a creature,
   - second trigger sacrifices Sothera when any player controls no creatures,
   - one linked exiled creature card returns under your control with exactly two additional `+1/+1` counters.

## Status

Blocked for score-improver purposes until the reusable linked-exile and intervening-if support exists.
