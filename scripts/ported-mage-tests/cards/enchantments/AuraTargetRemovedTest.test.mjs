import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/AuraTargetRemovedTest.java",
  "tests": [
    {
      "name": "testOneAttackerDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Academy Ruins",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spreading Seas",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Field of Ruin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spreading Seas",
          "target": "Field of Ruin"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{2}, {T}",
          "target": "Academy Ruins"
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
          "player": 1,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Field of Ruin",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Spreading Seas",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Academy Ruins",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    }
  ]
});
