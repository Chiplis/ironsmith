import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m13/OmniscienceTest.java",
  "tests": [
    {
      "name": "testNoCastFromOpponentHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Omniscience",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sen Triplets",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "assertStackObject",
          "source": "Sen Triplets",
          "count": 1,
          "turn": 2,
          "phase": "UPKEEP",
          "abilityKind": "Triggered",
          "targetCount": 1,
          "completeTargets": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "assertCastMethods",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "owner": 0,
          "name": "Balduvian Bears",
          "zone": "hand",
          "methods": []
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 0
        }
      ]
    },
    {
      "name": "testYesCastFromOpponentHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Omniscience",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sen Triplets",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "assertStackObject",
          "source": "Sen Triplets",
          "count": 1,
          "turn": 2,
          "phase": "UPKEEP",
          "abilityKind": "Triggered",
          "targetCount": 1,
          "completeTargets": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "assertCastMethods",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "owner": 0,
          "name": "Balduvian Bears",
          "zone": "hand",
          "methods": [
            "play_from"
          ]
        },
        {
          "op": "castSpell",
          "name": "Balduvian Bears",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "permissionSource": "Sen Triplets"
        },
        {
          "op": "assertStackObject",
          "source": "Balduvian Bears",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "controller": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Forest",
          "count": 2,
          "tapped": true
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        }
      ]
    }
  ]
});
