# Event and prior-object quantities

Status: **UNVALIDATED implementation-first source proposal**. No build, compiler,
CLI replay, or test was executed. `event_object_quantities.json.fixture` contains
all eight exact frozen cards, complete metadata, and stack07 first diagnostics:

- Bioplasm
- Death's Presence
- Felisa, Fang of Silverquill
- Infernal Genesis
- Narci, Fable Singer
- Ovika, Enigma Goliath
- Prossh, Skyraider of Kher
- Pure Reflection

The proposed full-card count is eight, subject to deferred validation. These are
additional to the nine continuous-quantity cards and the four where-X members of
the seven-card excess-damage family; other where-X records remain open.

## Shared grammar and identity

A shared value-expression leaf accepts named characteristics already expressible
by `PowerOf`, `ToughnessOf`, `ManaValueOf`, `ManaSpentToCast`, and `CountersOn`.
It preserves source/exiled/event-object references, rather than guessing a fixed
amount, counting generic objects, or dropping metadata. The milled card's mana
value is an action-qualified `PendingPriorEffectMetric` over the mill's affected
object memory, not an arbitrary current object or the number milled.

"That spell" retains its noun-bearing reference surface. A subsequent destroy or
sacrifice result cannot become that spell: the existing prior-antecedent facility
now preserves that distinction, without overriding a newly named spell target.

Bioplasm additionally needs two independent local variables. A bounded complete
X/Y pump reader parses both quantity expressions and their signs, then supplies
those typed component values to the existing pump subject/duration reader. It
handles a leading condition and an explicit end-of-turn duration. It does not add
a persistent Y variable, synthesize a numeric placeholder, or accept an unparsed
continuation. Ordinary pump parsing remains unchanged.

## Runtime corrections

The separate prerequisite `fb04e6d1` fixes `latest_tagged_lki_snapshot`: search all
members of a simultaneous zone-change event and match the exact departed ObjectId
and zone. Matching merely the stable physical-card ID could use a later blink
incarnation's characteristics. The callers are numeric characteristics, printed
mana-symbol counts, and effect-source LKI. The object-selection/following policy
and the live-graveyard/history separation used by Second Sunrise are unchanged.
Explicit same-resolution movement links and updated tags retain their existing
permission to follow their destination objects.

A follow-up source audit found the same stable-card overreach while refreshing
pending stack entries at a departure. That writer now refreshes tagged and source
LKI only for their exact bound object identity (and matching original zone). The
pre-stack cost-tag linkage in `push_to_stack`, explicit movement permissions, and
same-resolution tag updates are unchanged. The separate `pending_lki_identity`
runtime target authors source/tagged blink-negative cases and the pre-stack
cost-link positive case, including a later move that must break that link.

A chapter-resolution event carries the Saga in its outer source snapshot. The
normal triggering-object prelude now accepts that fallback only when the snapshot
ObjectId equals the event's object identity, after the live object and inner
snapshot paths. This keeps Narci's Saga after its final-chapter sacrifice without
substituting a different damage source or following a later incarnation.

## Authored verification

Normal `ironsmith-tools` and `ironsmith-compiler-runtime` integration targets are
both named `event_object_quantities`.

- Strict lossless metadata compilation, direct materialization, and artifact JSON
  round trips for all eight frozen cards; typed characteristic/metric preservation
- Shared quantity grammar, independent signed X/Y components and strict duration
- Spell-reference binding after destruction/sacrifice and preservation of a new
  targeted spell rather than an older antecedent
- Death's Presence with two simultaneous deaths, public pump/counter effects,
  calculated departing power, and a later different-power blink incarnation
- Felisa dying simultaneously with a creature carrying multiple counter kinds;
  tapped flying 2/1 Inklings count those pre-death counters
- Infernal Genesis mills and creates for the correct upkeep player after its
  source leaves; the count reads the exact milled card's mana value
- Bioplasm distinguishes creature/noncreature exiles, independently uses 2/7
  rather than one shared X value, and expires its pump at cleanup
- Narci resolves using the original Saga after its departure/return/departure and
  its own source's departure; an unrelated outer event source is a negative case
- Ovika and Pure Reflection use an actually announced X on a spell subsequently
  countered; an intervening Reflection destruction cannot steal the reference
- Ovika uses only the selected split half, and Pure Reflection uses a face-down
  creature spell's zero mana value after it is countered
- Prossh's actual paid mana includes cost increases and survives being countered;
  a shared cast-quantity probe pays with Convoke and Phyrexian life and counts zero
  mana rather than printed cost, tapped creatures, or life
- The LKI prerequisite has direct unit contracts for resolution-time live power,
  exact departure LKI, non-primary batch members, and permitted own-resolution
  movement links followed by an explicit tag update

All listed tests are authored, not passing claims. No measured coverage increase
is claimed before the deferred build and aggregate replay.
