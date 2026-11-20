import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/AlteredEgoTest.java",
  "tests": [
    {
      "name": "testAddingTheCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Altered Ego@ego",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "castSpell",
          "name": "Altered Ego",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@ego"
        },
        {
          "op": "assertStackObject",
          "source": "Altered Ego",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "xValue": 3
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Silvercoat Lion"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@ego"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "@ego",
          "count": 3,
          "counter": "+1/+1"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 5,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testNoCreatureToCopyAvailable",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Altered Ego",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Altered Ego"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Altered Ego",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Altered Ego",
          "count": 1
        }
      ]
    },
    {
      "name": "copyXCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Tropical Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Endless One",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Altered Ego@ego",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "castSpell",
          "name": "Endless One",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Endless One",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "xValue": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Endless One",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Endless One",
          "count": 2,
          "counter": "+1/+1"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Endless One",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=3"
        },
        {
          "op": "castSpell",
          "name": "Altered Ego",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@ego"
        },
        {
          "op": "assertStackObject",
          "source": "Altered Ego",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "xValue": 3
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Endless One"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@ego"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Endless One",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "@ego",
          "count": 6,
          "counter": "+1/+1",
          "note": "Port correction (CR 107.3m): the copied 'enters with X +1/+1 counters' replacement uses the X chosen for the Altered Ego spell that became the permanent (3), plus Altered Ego's own X additional counters (3) = 6. XMage treats the copied X as 0 (3 counters)."
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Endless One",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Endless One",
          "count": 2,
          "counter": "+1/+1"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Endless One",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
