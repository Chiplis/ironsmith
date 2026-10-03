# Scalar and counter quantities

Status: **UNVALIDATED implementation-first source proposal**. Four proposed full
cards, with exact frozen inputs and diagnostics in `scalar_counter_quantities`:

- Nissa, Ascended Animist: the source planeswalker's current loyalty
- Rootwire Amalgam: three times the source creature's power
- Razorfield Ripper: its controller's energy after the trigger's energy gain
- Vault 12: The Necropolis: all players' rad counters, summed at resolution

The fixture also retains Toph, Greatest Earthbender as **pending**. Earthbend's
current action stores a fixed `u32` counter amount. Recognizing a spent-mana
pronoun cannot implement dynamic Earthbend; no Toph coverage is claimed here.

## Shared typed repair

Integer scalar words compose the existing `Value::Scaled` with an arbitrary
recognized numeric term. The old `Value::XTimes` representation is retained for
an X operand. Source loyalty uses the existing source-scoped `CountersOn` with a
loyalty counter kind and the authored source surface. The existing source/LKI
reader and the exact pending-stack identity correction own departure behavior.

Energy and player-wide counter totals use existing `PlayerCounters` values.
The grammar preserves counter kind and controller/player scope; a rad counter on
a permanent and another player-counter kind do not contribute to Vault 12.
The renderer now describes `Any` and `Opponent` counter values as player-set
sums rather than incorrectly describing a single player's amount. Existing
single-player wording remains unchanged.

No serialized variant, evaluator, or keyword marker was added. These expression
changes are used by the ordinary token, pump, and Saga/loyalty programs.

## Authored verification, not execution results

- Strict metadata compilation and loss checks for four full cards; the Nissa
  input explicitly includes its printed loyalty metadata
- Artifact JSON and materialization on every full card
- Source loyalty, independent scalar/source-power binding, legacy XTimes
  preservation, energy-symbol handling, player-wide counter domain negatives
- Nissa's real +1 activation, changed loyalty before resolution, and first
  departure LKI surviving a different-loyalty blink incarnation
- Rootwire's real normal/prototype casting and paid sacrifice activation,
  calculated pre-sacrifice power after a pump, triple-sized token, later blink
  rejection, and temporary haste expiration
- Razorfield's self attack, energy spending before resolution, both actual
  reconfigure cost alternatives, attached attack identity, and a different
  Equipment controller receiving/reading its own energy
- Vault 12's three chapter thresholds using public lore-counter actions; changed
  opponent rad count while chapter II waits; exclusion of permanent rad and
  player energy counters; Zombie/Mutant token types and chapter III counters
- Structural rendering of the rad total as the sum among players

Normal tools/runtime integration targets are `scalar_counter_quantities`.
No build, compiler, CLI replay, or test was run. All four coverage entries remain
proposed until the deferred aggregate validation phase.
