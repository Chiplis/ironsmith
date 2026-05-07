import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/HeroicTest.java",
  "tests": [
    {
      "name": "testHeroicWithModal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Favored Hoplite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lagonna-Band Trailblazer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dromoka's Command",
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
          "name": "Plains",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dromoka's Command",
          "target": "mode=3Lagonna-Band Trailblazer^mode=4Favored Hoplite^Silvercoat Lion"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "3"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "4"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dromoka's Command",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Favored Hoplite",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lagonna-Band Trailblazer",
          "power": 2,
          "toughness": 6
        }
      ]
    }
  ]
});
