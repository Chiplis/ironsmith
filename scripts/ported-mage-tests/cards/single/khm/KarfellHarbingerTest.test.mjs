import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/khm/KarfellHarbingerTest.java",
  "tests": [
    {
      "name": "testForetellMana",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Karfell Harbinger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Augury Raven",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "player": 0,
          "source": "Karfell Harbinger",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "ability": "Add {U}"
        },
        {
          "op": "activateAbility",
          "source": null,
          "ability": "Foretell Augury Raven",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Augury Raven",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Augury Raven",
          "count": 1
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Karfell Harbinger",
          "tapped": true
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Island",
          "tapped": true
        }
      ]
    },
    {
      "name": "testSpellMana",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Karfell Harbinger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Obsessive Search",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Obsessive Search"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Obsessive Search",
          "count": 1
        }
      ]
    },
    {
      "name": "testOtherMana",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Karfell Harbinger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Flying Men",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Flying",
          "expected": false
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Flying Men",
          "count": 1
        }
      ]
    }
  ]
});
