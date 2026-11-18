import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mrd/KrarksThumbTest.java",
  "tests": [
    {
      "name": "test_NoThumb",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Traprunner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Completed Flip Fixture",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "Whenever you win a coin flip, you gain 1 life.\nWhenever you lose a coin flip, you gain 1 life."
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1,
          "coinFaces": [
            "heads",
            "tails",
            "heads"
          ]
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "You gain 1 life"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "You gain 1 life"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    },
    {
      "name": "test_OneThumb",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Traprunner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Krark's Thumb",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Completed Flip Fixture",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "Whenever you win a coin flip, you gain 1 life.\nWhenever you lose a coin flip, you gain 1 life."
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1,
          "coinFaces": [
            "heads",
            "heads",
            "tails",
            "heads",
            "tails",
            "tails"
          ]
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Heads (win)",
            "Keep Heads (win)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Tails (loss)",
            "Keep Heads (win)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Tails (loss)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Tails"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "You gain 1 life"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "You gain 1 life"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 2
        }
      ]
    },
    {
      "name": "test_TwoThumb",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Traprunner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror Box",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Krark's Thumb",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Completed Flip Fixture",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "Whenever you win a coin flip, you gain 1 life.\nWhenever you lose a coin flip, you gain 1 life."
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1,
          "coinFaces": [
            "heads",
            "tails",
            "tails",
            "tails",
            "tails",
            "tails",
            "tails",
            "tails",
            "heads",
            "heads",
            "tails",
            "tails"
          ]
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Heads",
            "Tails"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Heads (win)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Tails (loss)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Tails (loss)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Tails (loss)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Heads (win)",
            "Keep Heads (win)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Tails (loss)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Heads (win)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Tails (loss)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_options",
          "source": "Goblin Traprunner",
          "description": "Choose one",
          "options": [
            "Keep Heads (win)",
            "Keep Tails (loss)"
          ],
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Tails"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Tails"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Tails"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Tails"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Tails"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Keep Heads"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "You gain 1 life"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "You gain 1 life"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 2
        }
      ]
    }
  ]
});
