import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/akh/NewPerspectivesTest.java",
  "tests": [
    {
      "name": "newPerspectives_7Cards_FreeCycling",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reliquary Tower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "New Perspectives",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "New Perspectives"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "Cycling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "name": "New Perspectives",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 7
        }
      ]
    },
    {
      "name": "newPerspectives_LessThan7Cards_CyclingNotFree",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reliquary Tower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "New Perspectives",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "New Perspectives"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "label": "Cycling",
          "expected": false
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
          "name": "New Perspectives",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flame Jet",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 4
        }
      ]
    },
    {
      "name": "newPerspectives_PlayingSecondOneWithFewerThan7CardsOnCast",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reliquary Tower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "New Perspectives",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "New Perspectives"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "New Perspectives"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "Cycling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "New Perspectives",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 8
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        }
      ]
    },
    {
      "name": "newPerspectives_PlayingSecondOneWithMoreThan7CardsOnCast",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reliquary Tower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "New Perspectives",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "New Perspectives"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "New Perspectives"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "Cycling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "New Perspectives",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "New Perspectives",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flame Jet",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 11
        }
      ]
    }
  ]
});
