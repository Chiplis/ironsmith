import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/ForgeAnewTest.java",
  "tests": [
    {
      "name": "testForgeAnew",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Forge Anew",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bronze Sword",
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
          "name": "Forge Anew"
        },
        {
          "op": "assertStackObject",
          "source": "Forge Anew",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bronze Sword"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "name": "Bronze Sword",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertGraveyardCount",
          "name": "Bronze Sword",
          "count": 0,
          "player": 0
        },
        {
          "op": "activateAbility",
          "player": 0,
          "source": "Bronze Sword",
          "ability": "Equip {0}",
          "turn": 1,
          "phase": "END_COMBAT",
          "target": "Silvercoat Lion"
        },
        {
          "op": "assertStackObject",
          "source": "Bronze Sword",
          "count": 1,
          "turn": 1,
          "phase": "END_COMBAT",
          "abilityKind": "Activated",
          "objectTarget": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Bronze Sword",
          "target": "Silvercoat Lion",
          "targetPlayer": 0,
          "expected": true
        }
      ]
    }
  ]
});
