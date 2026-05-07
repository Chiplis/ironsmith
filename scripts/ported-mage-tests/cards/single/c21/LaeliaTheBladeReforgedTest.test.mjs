import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c21/LaeliaTheBladeReforgedTest.java",
  "tests": [
    {
      "name": "controllerExilesOwnCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Laelia, the Blade Reforged",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cranial Extraction",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cranial Extraction",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Laelia, the Blade Reforged",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "opponentExilesControllersCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cranial Extraction",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Laelia, the Blade Reforged",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cranial Extraction",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
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
          "op": "assertCounterCount",
          "player": 1,
          "name": "Laelia, the Blade Reforged",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "controllerExilesOpponentsCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Laelia, the Blade Reforged",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cranial Extraction",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cranial Extraction",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Laelia, the Blade Reforged",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "opponentExilesOwnCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cranial Extraction",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Laelia, the Blade Reforged",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cranial Extraction",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
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
          "op": "assertCounterCount",
          "player": 1,
          "name": "Laelia, the Blade Reforged",
          "counter": "P1P1",
          "count": 0
        }
      ]
    }
  ]
});
