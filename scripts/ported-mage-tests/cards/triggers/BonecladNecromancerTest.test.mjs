import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/BonecladNecromancerTest.java",
  "tests": [
    {
      "name": "testBonecladNecromancerCanExileCreaturesFromOwnGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Boneclad Necromancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Raptor Hatchling",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Boneclad Necromancer"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raptor Hatchling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
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
          "name": "Boneclad Necromancer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Raptor Hatchling",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Raptor Hatchling",
          "count": 0
        }
      ]
    },
    {
      "name": "testBonecladNecromancerCanExileCreaturesFromOtherGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Boneclad Necromancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Raptor Hatchling",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Boneclad Necromancer"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raptor Hatchling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
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
          "name": "Boneclad Necromancer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Raptor Hatchling",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Raptor Hatchling",
          "count": 0
        }
      ]
    },
    {
      "name": "testBonecladNecromancerCantExileNonCreatures",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Boneclad Necromancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Feral Invocation",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Boneclad Necromancer"
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
          "name": "Boneclad Necromancer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Feral Invocation",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Feral Invocation",
          "count": 1
        }
      ]
    }
  ]
});
