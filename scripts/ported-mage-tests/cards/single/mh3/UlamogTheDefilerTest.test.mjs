import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/UlamogTheDefilerTest.java",
  "tests": [
    {
      "name": "test_OnlyLandsExiled",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ulamog, the Defiler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Taiga",
          "count": 11
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ulamog, the Defiler"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Ulamog, the Defiler",
          "defender": 1
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
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 6
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 13
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Mountain",
          "count": 0
        }
      ]
    },
    {
      "name": "test_Annihilator2_FromFreshlyExiled",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ulamog, the Defiler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Taiga",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ulamog, the Defiler"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Ulamog, the Defiler",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Mountain"
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
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 7
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 11
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Mountain",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Annihilator2_FromOldExiled",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ulamog, the Defiler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Taiga",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ulamog, the Defiler"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Ulamog, the Defiler",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Mountain"
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
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 6
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 11
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Mountain",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Annihilator4_Change_FromCounterLater",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ulamog, the Defiler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Luminarch Aspirant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Taiga",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ulamog, the Defiler"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ulamog, the Defiler"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Ulamog, the Defiler",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ulamog, the Defiler"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Mountain"
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
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 7
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 9
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Mountain",
          "count": 4
        }
      ]
    }
  ]
});
