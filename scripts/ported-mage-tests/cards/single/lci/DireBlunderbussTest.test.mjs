import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/DireBlunderbussTest.java",
  "tests": [
    {
      "name": "DireBlunderbussTest",
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
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dire Blunderbuss",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears@bearsA",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears@bearsB",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tormod's Crypt",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip",
          "target": {
            "groups": [
              [
                "@bearsA"
              ]
            ]
          },
          "source": "Dire Blunderbuss"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Dire Blunderbuss",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "@bearsA"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "@bearsA",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "choices": [
            "Tormod's Crypt"
          ],
          "description": "Sacrifice exactly 1 artifact"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "@bearsB"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "@bearsA",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "@bearsA",
          "count": 1,
          "abilityKind": "Triggered",
          "completeTargets": true,
          "objectTarget": "@bearsB"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Tormod's Crypt",
          "count": 1
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
          "player": 1,
          "name": "@bearsB",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    }
  ]
});
