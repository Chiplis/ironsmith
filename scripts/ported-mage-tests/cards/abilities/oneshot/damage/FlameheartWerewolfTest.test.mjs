import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/damage/FlameheartWerewolfTest.java",
  "tests": [
    {
      "name": "testBlockingKalitas",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flameheart Werewolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kalitas, Traitor of Ghet",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Kalitas, Traitor of Ghet",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Flameheart Werewolf",
          "attacker": "Kalitas, Traitor of Ghet"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 23
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Flameheart Werewolf",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Kessig Forgemaster",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kalitas, Traitor of Ghet",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kalitas, Traitor of Ghet",
          "count": 1
        }
      ]
    },
    {
      "name": "testBlockedByTwo22s",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flameheart Werewolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Falkenrath Reaver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wind Drake",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Flameheart Werewolf",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Falkenrath Reaver",
          "attacker": "Flameheart Werewolf"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wind Drake",
          "attacker": "Flameheart Werewolf"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kessig Forgemaster",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Flameheart Werewolf",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Falkenrath Reaver",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Falkenrath Reaver",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Wind Drake",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Wind Drake",
          "count": 1
        }
      ]
    },
    {
      "name": "testKessigForgemaster",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kessig Forgemaster",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wily Bandar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Stern Constable",
          "count": 1
        },
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
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Explosive Apparatus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Explosive Apparatus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Explosive Apparatus"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Explosive Apparatus"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Kessig Forgemaster",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wily Bandar",
          "attacker": "Kessig Forgemaster"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Stern Constable",
          "attacker": "Kessig Forgemaster"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Forgemaster",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Kessig Forgemaster",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Wily Bandar",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Wily Bandar",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Stern Constable",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Stern Constable",
          "count": 1
        }
      ]
    }
  ]
});
