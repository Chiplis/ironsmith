import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m3c/OmoQueenOfVesuvaTest.java",
  "tests": [
    {
      "name": "test_TronLand",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Urza's Tower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Omo, Queen of Vesuva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Abzan Banner",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Omo, Queen of Vesuva"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Forest"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Abzan Banner"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Abzan Banner",
          "count": 1
        }
      ]
    },
    {
      "name": "test_IsForest",
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
          "name": "Coiling Woodworm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Omo, Queen of Vesuva",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Omo, Queen of Vesuva"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Island"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Coiling Woodworm",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        }
      ]
    },
    {
      "name": "test_EveryCreatureSubType",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
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
          "name": "Coiling Woodworm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Imperious Perfect",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Omo, Queen of Vesuva",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Omo, Queen of Vesuva"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Island"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Coiling Woodworm"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Coiling Woodworm",
          "counter": "EVERYTHING",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Coiling Woodworm",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
