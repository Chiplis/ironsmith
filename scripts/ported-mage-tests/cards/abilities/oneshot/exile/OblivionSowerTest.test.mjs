import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/exile/OblivionSowerTest.java",
  "tests": [
    {
      "name": "testPlayLandsFromExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Oblivion Sower",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Canopy Vista",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Oblivion Sower",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Oblivion Sower",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered",
          "targetCount": 1,
          "completeTargets": true
        },
        {
          "op": "setOptionChoices",
          "kind": "boolean",
          "player": 0,
          "description": "Put any number of land card in target opponent's exiles onto the battlefield under your control",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose land card in target opponent's exile",
          "choices": [
            "Canopy Vista",
            "Canopy Vista",
            "Canopy Vista"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Oblivion Sower",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Canopy Vista",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Canopy Vista",
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Canopy Vista",
          "count": 3,
          "tapped": false
        }
      ]
    }
  ]
});
