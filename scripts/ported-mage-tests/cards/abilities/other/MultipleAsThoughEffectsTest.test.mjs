import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/MultipleAsThoughEffectsTest.java",
  "tests": [
    {
      "name": "testChoosingAlternateCastingMethod",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vivien, Monsters' Advocate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bolas's Citadel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Abzan Beastmaster@second",
          "zone": "LIBRARY",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Abzan Beastmaster@first",
          "zone": "LIBRARY",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Grizzly Bears@drawn",
          "zone": "LIBRARY",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Vivien, Monsters' Advocate",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Vivien, Monsters' Advocate",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vivien, Monsters' Advocate",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "@drawn",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Abzan Beastmaster",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "permissionSource": "Vivien, Monsters' Advocate",
          "objectAlias": "@first"
        },
        {
          "op": "assertStackObject",
          "source": "Abzan Beastmaster",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@first"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Forest",
          "count": 3,
          "tapped": true
        },
        {
          "op": "castSpell",
          "name": "Abzan Beastmaster",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "permissionSource": "Bolas's Citadel",
          "objectAlias": "@second"
        },
        {
          "op": "assertStackObject",
          "source": "Abzan Beastmaster",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@second"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Forest",
          "count": 3,
          "tapped": true
        }
      ]
    },
    {
      "name": "testRisenExecutioner",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Risen Executioner",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gisa and Geralf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Risen Executioner"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gisa"
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swamp",
          "tapped": true,
          "count": 4,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Risen Executioner"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Risen Executioner",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 9
        }
      ]
    }
  ]
});
