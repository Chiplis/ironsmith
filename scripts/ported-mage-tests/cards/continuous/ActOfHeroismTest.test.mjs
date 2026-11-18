import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/ActOfHeroismTest.java",
  "tests": [
    {
      "name": "testCanBlockMultiple",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tasseled Dromedary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Act of Heroism",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Unwavering Initiate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sacred Cat",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Overwhelming Splendor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Act of Heroism",
          "target": "Tasseled Dromedary"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "UPKEEP",
          "source": "Act of Heroism",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Overwhelming Splendor",
          "target": 0
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Overwhelming Splendor",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tasseled Dromedary",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Unwavering Initiate",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Sacred Cat",
          "defender": 0
        },
        {
          "op": "assertBlockerLegality",
          "turn": 2,
          "player": 0,
          "attacker": "Unwavering Initiate",
          "blocker": "Tasseled Dromedary",
          "expected": true
        },
        {
          "op": "assertBlockerLegality",
          "turn": 2,
          "player": 0,
          "attacker": "Sacred Cat",
          "blocker": "Tasseled Dromedary",
          "expected": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Tasseled Dromedary",
          "attacker": "Unwavering Initiate"
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Tasseled Dromedary",
          "attacker": "Sacred Cat"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
