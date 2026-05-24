Card: Kharn the Betrayer
Date: 2026-05-24

Current strict parse blocker:

- `Sigil of Corruption - When you lose control of this creature, draw two cards.`

What was fixed in this pass:

- Added reusable parser support for static combat requirements:
  - `attacks or blocks each combat if able`
  - `blocks each combat if able`
- Verified this parses on representative cards (for example Iron Golem).

Why Kharn still cannot reach strict parse in this pass:

- The remaining line is a control-loss trigger (`When you lose control of ...`).
- The current trigger parser/runtime model does not expose a reusable trigger family for
  source control-loss/control-change events that this clause can lower into.
- This is broader than a card-local parser tweak and needs reusable trigger modeling
  (AST + trigger matching/runtime wiring) to support this and similar cards.

Recommended follow-up capability:

- Add reusable trigger support for "you lose control of <object>" / control-change
  event triggers, then lower named and tagged source forms into that shared model.

Verification command used:

- `cargo run -p ironsmith-tools --bin compile_oracle_text -- --name "Khârn the Betrayer" --compare-text`
