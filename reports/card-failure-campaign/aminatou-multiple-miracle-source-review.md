# Aminatou: independent Miracle instances

Base: `d1737c6598429f9c7999f6737380ab18e6b1bde8`. This is a separate correction from the held linked-exile cohort. No builds, tests, compiler probes, formatters or corpus execution were performed.

The previous draw menu allowed only one Miracle instance to be accepted. The pinned September 25, 2026 Comprehensive Rules, 113.2c, 701.20c and 702.94a, support independent optional reveals for multiple instances: an already revealed card can be revealed again. Rules 603.11, 607.2h and 607.5 keep each reveal and casting instruction linked. This conclusion is an inference from those primary rules; there is no special one-instance exception in 702.94. [Official rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf).

## Correction

- The original first-draw window captures the same exact eligible instances and now accepts a distinct subset. Decline, duplicate indices, an out-of-range index and mixing decline with acceptance are validated. Pending selection restores the original draw through its existing owner.
- `MiracleDrawDecision::RevealedMany` is additive. The existing singleton decision and legacy intrinsic `None` notices remain intact. One actual public reveal event is emitted per accepted instance.
- Trigger discovery creates one trigger per accepted proof. Each trigger receives a narrowed single-instance draw receipt, preserving that instance's price, grant identity, actual drawn incarnation and original drawer. It still passes the existing suppression and additional-trigger owners. The casting effect rejects an unbound aggregate receipt rather than selecting an arbitrary price.
- Countering or declining one trigger cannot erase the others. Copied triggers retain the exact one-instance receipt while the copy controller owns the casting choice and payment. Successful casting moves the card, so the other receipts cannot cast its later incarnation.

## Reveal lifetime

The existing wasm `stack_revealed_view` derives semantic inspection from hidden-zone source snapshots on stack entries. Each accepted or copied Miracle trigger keeps its own snapshot, so inspection persists while any such trigger remains. A bounded Miracle guard now requires the original exact card to remain in its original owner's hand; a stale trigger cannot reveal a new incarnation. No separate reveal journal or lifetime counter was introduced. `publicly_revealed_hidden_cards` remains authenticated public knowledge and is not erased when a trigger ends.

## Authored evidence, unrun

The complete frozen Aminatou body is independently compiled through the direct and artifact helpers in the scenarios. New cases accept two/all of an intrinsic plus two distinct grants; counter or decline the first trigger and cast using another price; copy an accepted trigger under another controller and counter its original; replay a pending subset; reject malformed subsets with full draw/prior-life-gain rollback; and use both privately known and placeholder peers with one authenticated original opening before a source-removing draw addition. Existing singleton, face/X/hybrid, first-draw, stale-card and payment replay scenarios remain.

A wasm source-view scenario removes accepted/copy receipts one by one, checks visibility until the last one leaves, and rejects a later hand incarnation. This is source evidence only and awaits independent review.
