# Reused token prototype references

UNVALIDATED source proposal for eight frozen Oracle identities. No build, test,
compiler probe, formatter, or corpus execution has run. The only executed code
check is `git diff --check`. These are authored scenarios and source traces,
not measured recoveries or passing semantic results. No campaign matrix or
published count is changed by this branch.

`fixtures/reused_token_prototypes.json.fixture` retains the exact complete
frozen bodies, print IDs, Oracle IDs, type lines, mana costs, and available
power/toughness from `cards-20261003.json.xz`. All eight baseline diagnostics
are `unsupported token 'of those'`, including the oracle-only fallback.

## Shared grammar, binding, and native owner

The create-clause registry recognizes the exact lexical `N of those tokens`
head, an optional authored tapped-and-attacking override, and the existing typed
where-X binding grammar. It rejects malformed punctuation, singular noun,
missing count, unknown entry modifiers, and unconsumed tails. It does not splice
text or select implementations by card name.

`TokenDefinitionSpec::PrototypeReference(PreviousDefinition)` is a compiler
reference to an authored blueprint. The preparation walk resolves it across the
complete ability, independently of runtime object references and executed
branches. Ordinary conditional arms and die-table rows follow lexical order;
a self-replacement's original program is visited before its replacement even
though executable branch storage has the opposite order. Another ability cannot
supply the blueprint. The resolved action retains the full definition, named
builtin, intrinsic/granted abilities, dynamic P/T, entry modifiers and existing
cleanup fields; the referencing instruction supplies its count and actor.

Lowering continues through the ordinary complete-token owner,
`CreateTokenEffect<CardDefinition>`. Existing core payload mapping, the permanent
effect decoder's nested card graph, and artifact materialization retain that
complete executable definition. The compiler refuses an unbound prototype;
there is no runtime lookup of a token that happened to be created earlier.

## Complete frozen body traces

- Adipose Offspring (`1e21e57e-3bc9-41a8-9746-57b583a5ad63`): Emerge alternative
  payment remains executable. Its ETB self-replacement chooses one 2/2 white
  Alien normally and the sacrificed creature's actual payment-time toughness
  when Emerge was paid. The live ranked where-X sacrificed-noun reader now
  emits AdditionalCostObject rather than the ambient triggering-object `it`.
  The paid-Emerge branch binds that reference to a reserved cast receipt.
- Andúril, Flame of the West (`e6521404-8474-4727-b2d0-537d15d8a63f`): the
  Equipment's +3/+1 static bonus and Equip {2} remain. The actual equipped
  attacker determines the legendary condition. Both branches create two tapped
  white flying Spirits; the legendary branch additionally asks the existing
  ordinary combat-entry owner for a lawful destination. It retains choices of
  either opponent, their planeswalkers and protected battles, without importing
  Myriad-specific exclusions. `can_enter_attacking` leaves the tokens tapped,
  flying and controlled by the Equipment's controller when that controller is
  not an attacking player, including a foreign-controlled wielder's attack.
- Brood Birthing (`e7d1c762-69d0-401d-8035-6e9744baf9d8`): the control predicate
  chooses three Spawn or one from the unexecuted true-arm blueprint. The
  complete 0/1 colorless Eldrazi Spawn definition owns its sacrifice-for-{C}
  ability. The ordinary authored-grant owner retains the complete quoted rule
  exactly once, and prototype reuse preserves it on both branches.
- From Under the Floorboards (`a1fc4269-f3e4-4a59-849d-aa1727eef23a`): native
  discard/madness exile, linked cast permission and alternative payment remain.
  The coordinated self-replacement replaces both creation and life gain;
  native paid X supplies both amounts. Zombies inherit tapped entry. Normal
  casting still produces three Zombies and three life, including an unrelated X.
- Safana, Calimport Cutthroat (`afc1c443-93e3-4ef4-a404-1d9fc13e0b02`): Menace
  and Choose a Background remain. Its own-end-step initiative intervening
  predicate must pass both when triggering and resolving. The completed-dungeon
  predicate replaces one canonical Treasure with three. The builtin mana
  ability is retained.
- Swarming Goblins (`6a038dec-2d9c-4422-a0f3-94bf70e55217`): the ETB native d20
  receipt selects exactly one result row. All rows use the first row's 1/1 red
  Goblin blueprint, including rolls for which that row never executes.
- The Final Days (`f2b88031-bfb7-46c4-abdd-3b6b5f5acfa4`): cast origin, rather
  than present zone or a matching alternative-cost name alone, selects the
  replacement. Its live creature-card graveyard count excludes other players
  and noncreature cards. Both branches inherit tapped Horror entry. Native
  Flashback payment and post-resolution exile remain in the complete card.
- Throne of Empires (`12afe8e6-eaf0-45c9-8086-4c658db7cb7e`): activation pays
  {1} and taps the Throne. The shared plural named-control predicate already
  builds two independently controller-scoped PlayerControls predicates joined
  by AND. LayeredSubject supplies current names. Two Crowns, an opponent's
  Scepter, and a nonartifact Scepter each fail; a controlled copy with the
  current Scepter name succeeds. The replacement makes five ordinary Soldiers.

## Emerge payment evidence and incomplete execution

The native casting owner identifies the paid preannounced Emerge resource
independently of the locked price reduction. The original sacrifice owner freezes
calculated pre-departure characteristics and appends an OriginalSacrificeObjects
receipt only after deferred replacement programs finish. Its vector records only
the original Proceed movement, including redirection, and is empty for a wholly
prevented/substituted action. Nested replacement-added sacrifices cannot fill it.
CostContext passes this typed receipt to the casting owner, which checks the
selected resource identity before publishing the reserved Emerge alias. The
price-locking owner marks the exact Emerge cost component; another later cost
that sacrifices the same selected object cannot overwrite an empty receipt. The
announcement's mana-value reduction is never used as Adipose's quantity.

The original completed-entry batch freezes the receipt for the exact stack-to-
battlefield incarnation. It also transfers it to the completed ZoneChangeEvent
by destination ID, because ordinary `this enters` matchers observe that normalized
event. Splitting a zone event filters receipts by destination. A matched trigger
imports only its own entrant's receipt, not ambient cast tags or another entering
creature's payment. Trigger and spell copies retain the existing copied-choice
receipt; an ordinary permanent/token copy does not acquire paid Emerge state.
Source departure and later incarnations do not overwrite the retained receipt.

The reserved source-cast stat reference reads immutable actual-sacrifice LKI.
A known empty original sacrifice receipt produces zero, even though Emerge is
paid and the selected creature may remain live with subsequently changed stats.
Missing, multiple, wrong-zone, noncreature or stat-incomplete receipts report
IncompleteEvidence and enter the existing incomplete-execution latch. Positive
and negated predicates cannot turn an unknown payment quantity into a successful
false/true branch, and enclosing mutations roll back.

This distinction follows CR 118.11 (a replaced cost can remain paid), 614.6 (a
replaced event does not occur), and 107.2 (the unprovided number is zero) in the
[September 25, 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf).
The [Doctor Who release notes](https://magic.wizards.com/en/news/feature/magic-the-gathering-doctor-who-release-notes)
also specify Adipose's last battlefield toughness for the actually sacrificed
creature. The unusual synthetic-prevention control is an inference from those
general rules, not a separate Adipose ruling.

The legacy snapshot helper itself is not a checked discovery boundary. Actual
sacrifice payment subsequently enters the checked effect/control-transition
boundary before mutation. A native regression is authored at that exact payment
owner for an out-of-range final characteristic; it asserts no sacrifice, published
payment receipt, or token result. No broad snapshot semantics were changed.

## Deferred evidence

The full-body runtime file independently compiles every exact card by the strict
runtime entry point and by artifact compilation/serialization/materialization,
with separate parse-loss capture, then uses native casts, Madness
discard/cast, activation payment, attack declaration, die rolling and trigger
stacking. It includes all d20 range edges, 0/nonzero Madness X, one-of-each-name
and controller/current-copy-name controls, actual Emerge mana reduction and
payment LKI, a post-announcement characteristic change, copied/departed-source
cases, prevented/substituted/redirected sacrifices and unrelated replacement
sacrifices, a prevented material's later live toughness change, malformed
required receipts, and unchanged ordinary Andúril combat destination choices.
Grammar/preparation scenarios include the exact frozen Adipose body and inspect
its executable count reference. Runtime blueprint independence is exercised
with the original token modified or exiled before the later creation.
The native cost-boundary scenario also suspends a replacement-added choice,
verifies no receipt or sacrifice is committed while pending, and replays once.
An independent later additional sacrifice of the same previously protected
material is a separate negative, beyond replacement-added sacrifice controls.

These checks are unrun. Any additional complete-body failure discovered during
review or later authorized execution must remain explicitly partial rather than
being omitted, shortened, or counted as closed.
