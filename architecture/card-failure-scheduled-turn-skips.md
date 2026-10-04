# Scheduled turn skips (source-only)

Status: UNVALIDATED. No compiler probe, build, test, or corpus replay was run.

## Exact frozen cohort

Avizoa; Blinding Angel; Brine Elemental; Dovin, Architect of Law; Eater of Days;
Revenant Patriarch; Shisato, Whispering Hunter; Stonehorn Dignitary; and Yosei,
the Morning Star. Their complete frozen metadata/text is in
`fixtures/scheduled_turn_skips.json.fixture`. Draw-replacement cards are not
included in this cohort.

## Source implementation

- Complete schedule phrases retain their subject, unit and cardinal count in
  `SkipScheduled`. The shared core payload is materialized through both artifact
  decoder paths, lowered to the native effect, and rendered without inventing
  a this-turn limit. Unknown suffixes are rejected.
- Next combat phases use independently consumable, persistent counters. They
  are distinct from all combats of a next turn and this-turn-only skips. Two
  effects skip two occurrences, including added combat phases; beginning-of-
  combat triggers and phase counters are omitted for a skipped phase.
- Counted turn skips reuse the existing actual/extra-turn scheduler. Skipping
  an untap step omits all its actions, including phasing, untapping and events.
- Continuous-control eligibility begins once per actual turn, even if untap is
  skipped. Added untap steps, off-turn untaps, resumed choices and skipped turns
  do not cure newly acquired permanents. Phased-out permanents are included.
- Grand Melee carries outstanding future skips between lanes as shared game
  facts; current-turn restrictions/control markers remain lane-local. Saved
  checkpoints retain future turn/step/combat counts and each lane's boundary.
  Older checkpoints retain their authoritative sickness flags; missing new
  schedule facts are not guessed. Malformed new schedule facts reject restore.

Rules reviewed against the official comprehensive rules, especially 302.6,
500.11 and 614.10/614.10a:
https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf

## Authored, unrun regressions

`scheduled_turn_skips.rs` covers direct/restored exact definitions; independent
combat skips and added combats; Eater's two turns and extra turns; skipped untap
phasing/control/events; Brine's opponent scope; combat damage actor capture;
Avizoa's full effect body; Dovin's selected opponent; Yosei's player/permanent
target relation; Revenant's actual white payment; legacy schedule parity;
this-turn expiry; and cross-lane skip consumption without stale resurrection.
Grammar tests cover count/scope and complete-clause rejection. WASM tests cover
schedule wire state, malformed state and retained Grand Melee lanes.

Deferred commands (do not run until the campaign source-coverage gate):
- `cargo test -p ironsmith-compiler-runtime --test scheduled_turn_skips`
- `cargo test -p ironsmith-compiler-grammar scheduled_skip`
- `cargo test -p ironsmith-web-session scheduled_skip_transport`
- `cargo test -p ironsmith-web-session grand_melee_marker_lanes_sync_checkpoint_round_trip`

These are source proposals, not verified compile or gameplay recoveries.
