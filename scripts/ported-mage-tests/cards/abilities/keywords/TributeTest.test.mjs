import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/TributeTest.java",
  "tests": [
    {
      "name": "testPharagaxGiant",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ink-Eyes, Servant of Oni",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Pharagax Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ink-Eyes, Servant of Oni",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Ink-Eyes, Servant of Oni\", true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Pharagax Giant",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Pharagax Giant",
          "power": 5,
          "toughness": 5
        }
      ]
    }
  ]
});
