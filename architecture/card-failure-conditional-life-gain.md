# Conditional life-gain self-replacements

UNVALIDATED source work. No builds, compilation, tests or corpus replay were run.
Five exact baseline failures are proposed: Dega Sanctuary, Feed the Clan, Life
Goes On, Mine Worker and Rest for the Weary. The measured campaign remains 40
recovered and 3,193 unresolved unique cards. The complete frozen inputs are in
`fixtures/conditional_life_gain.json.fixture`, using corpus SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.

A named two-sentence grammar owns both the default fixed gain and its complete
conditional replacement. It accepts leading `if` and trailing `instead if`,
retains the full typed condition and builds `EffectAst::SelfReplacement` with
mutually exclusive true/default arms. Document grouping keeps the replacement
sentence with its owner across Oracle lines and ability-word labels. The general
life-gain leaf still rejects an isolated `instead`; nothing globally discards
that semantic marker.

The reader accepts the same player, or the reference `that player` to the
original declared target. It does not declare a second target, change the
recipient or accept a truncated multi-action body. Existing resolution-program
lowering preserves a real self-replacement segment and evaluates its predicate
when the ability resolves. External life-gain replacements then modify only the
chosen gain event. Dega Sanctuary's outer triggered intervening-if remains
separate from the inner choice of amount. Named-creature conditions retain their
conjunctive filters; landfall is relative to the caster, not the life recipient.

Two grammar scenarios and six direct/restored-artifact runtime scenarios are
authored and unrun. They cover complete cards, death after casting, power changing
before resolution, one shared target player, caster-versus-target landfall,
both independently controlled Worker names, actual upkeep generation and stack
resolution, and removing a red permanent after Dega's trigger reaches the stack.
No compile or gameplay recovery is claimed until deferred validation.
