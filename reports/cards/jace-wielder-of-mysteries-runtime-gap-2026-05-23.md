# Jace, Wielder of Mysteries - reusable runtime/parser gap

## Card
- Name: `Jace, Wielder of Mysteries`
- Oracle line that fails strict parsing:
  - `If you would draw a card while your library has no cards in it, you win the game instead.`

## Current failure
- `compile_oracle_text` fails on the static line with:
  - `could not find verb in effect clause (clause: 'you win the game instead', ...)`

## Investigation summary
- The sentence is currently routed through generic effect-sentence conditional parsing.
- Parser/lowering does not model this line as a true replacement ability with:
  - draw-event matching (`would draw a card`), and
  - an additional state guard (`while your library has no cards in it`).
- Forcing it through generic conditional parsing causes strict marker failures (`instead` marker dropped) and does not give a structurally correct replacement model.

## Confirmed reusable gap
- Missing reusable support for replacement clauses that combine:
  - an event matcher (`would draw`) and
  - a state predicate (`library has no cards`)
  in one structural replacement ability.

## Needed follow-up capability
- Add reusable parser/lowering/runtime support for draw replacement with attached state conditions (library-empty guard), so this pattern lowers as a replacement static ability rather than a plain conditional effect sentence.
- This should be generic enough to cover other cards with the same alternate draw-loss replacement mechanic.
