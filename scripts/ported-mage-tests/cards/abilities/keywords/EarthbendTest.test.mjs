import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/EarthbendTest.java",
  "tests": [
    {
      "name": "testMultipleEarthbend",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
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
          "name": "Wastes",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dai Li Agents",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dai Li Agents"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Swamp"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Forest"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Swamp\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Forest\", CardType.CREATURE, true)"
        }
      ]
    }
  ]
});
