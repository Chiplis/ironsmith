import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/soi/KindlyStrangerTest.java",
  "tests": [
    {
      "name": "transformDestroyCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kindly Stranger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bitterblossom",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "<i>Delirium"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hill Giant"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon-Possessed Witch",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kindly Stranger",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        }
      ]
    }
  ]
});
