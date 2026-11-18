import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/fin/EdgarKingOfFigaroTest.java",
  "tests": [
    {
      "name": "testTraprunnerThenSwindler",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Edgar, King of Figaro",
          "count": 1
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
          "name": "Tavern Swindler",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1,
          "coinFaces": [
            "tails",
            "tails",
            "tails"
          ]
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}",
          "coinFaces": [
            "tails"
          ],
          "source": "Tavern Swindler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 3
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 13
        }
      ]
    },
    {
      "name": "testSwindlerThenTraprunner",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Edgar, King of Figaro",
          "count": 1
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
          "name": "Tavern Swindler",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}",
          "coinFaces": [
            "tails"
          ],
          "source": "Tavern Swindler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Heads"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Traprunner",
          "defender": 1,
          "coinFaces": [
            "tails",
            "tails",
            "tails"
          ]
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testTwoHeadedGiant",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Edgar, King of Figaro",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Two-Headed Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Two-Headed Giant",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(giant, true)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Two-Headed Giant",
          "ability": "DoubleStrike",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 12
        }
      ]
    }
  ]
});
