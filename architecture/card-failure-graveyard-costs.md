# Graveyard additional-cost permissions

## Evidence and scope

The completed `e8740178` corpus baseline contains 483 entries in the exact
unsupported-line-family diagnostic group. Removing parenthesized reminder text
before looking for `may play` / `may cast` yields 77 entries, not a single parser
root cause. A disjoint triage of their first failed lines is:

| Shared surface family | Entries |
| --- | ---: |
| Self-graveyard casting with costs | 13 |
| Once-per-turn permissions | 14 |
| Flash timing grants and riders | 14 |
| Conditional/restricted self-graveyard casts | 6 |
| Top-library permissions | 6 |
| Source-linked exile permissions | 7 |
| Resolution/trigger permission programs | 7 |
| Other typed-zone permissions | 6 |
| Additional land plays | 4 |

These groups are triage buckets, not promises that a single implementation fixes
all members. For example, the self-cost family separates into eight additional
costs and five alternative costs. Seven of those eight additional-cost cards
use the reusable bounded shape implemented here. Quilled Greatwurm's removal
of counters across multiple permanents remains outside this change.

## Reusable correction

The existing graveyard permission recognizer already isolates an additional-cost
clause, and the runtime already derives an alternative casting method from the
cast card's actual printed mana cost. The missing layer was typed cost
composition: the leaf accepted one sacrifice or typed exile cost only.

The recognizer now accepts:

- discarding a positive fixed number of cards;
- paying a positive fixed life amount, optionally followed by `and` and another
  recognized additional cost;
- exiling another typed card or a fixed number of other cards from your graveyard.

The semantic layer preserves a vector of mandatory costs instead of selecting
one leaf. Exile filters explicitly retain the payer's ownership and `other`
identity. Permission source identity, zone, timing, lifetime, and once-per-turn
limits stay with the existing grant representation. No new casting engine
primitive, card-name dispatch, permissive fallback, or validator exception is
introduced.

Cost rendering uses typed life/discard/exile metadata so the additional-cost
wording and `another` / `other` restriction survive compilation and artifact
materialization. The same implementation is used for source-only and
once-during-your-turn graveyard grants.

## Baseline targets

The fixture `fixtures/graveyard_additional_costs.json.fixture` freezes the actual
complete Oracle text and type/mana data of these eight baseline failures:

- Alien Symbiosis
- Demonic Embrace
- Dragon Man, Reformed Robot
- Helbrute
- Me, the Immortal
- Rona, Sheoldred's Faithful
- Wickerfolk Indomitable
- Kotis, Sibsig Champion

The first seven use the self permission; Kotis uses the existing restricted
once-per-turn permission with the same additional-cost reader. These are targets
until the authoritative coordinator replay confirms each full-card result.

## Verification

Run without custom global Rust flags:

```sh
cargo test -p ironsmith-compiler-grammar permission_facts::graveyard_source
cargo test -p ironsmith-compiler-runtime --test graveyard_additional_costs
```

The normal integration target compiles both directly and through a serialized,
validated artifact. Live-game cases check printed mana plus exact discard count,
life plus discard, life plus either sacrifice type, source/zone/player identity,
normal timing, static duration, no unprinted exile rider, own-graveyard exile,
source-excluding exile, and once-per-turn use/reset/source departure. Grammar
negative cases retain unsupported qualifications instead of dropping them.

Rust builds are deliberately delegated to the coordinator's serialized build
lane. Local checks are rustfmt, fixture JSON parsing, and `git diff --check`.
