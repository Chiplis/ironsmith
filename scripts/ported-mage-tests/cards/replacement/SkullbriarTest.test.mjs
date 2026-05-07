import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/SkullbriarTest.java",
  "tests": [
    {
      "name": "testSkullbriarCloudshift",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Skullbriar, the Walking Grave",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Battlegrowth",
          "target": "Skullbriar, the Walking Grave"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cloudshift",
          "target": "Skullbriar, the Walking Grave"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Skullbriar, the Walking Grave",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testHumilityAndSkullbriarCloudshift",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Skullbriar, the Walking Grave",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Battlegrowth",
          "target": "Skullbriar, the Walking Grave"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cloudshift",
          "target": "Skullbriar, the Walking Grave"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Skullbriar, the Walking Grave",
          "counter": "P1P1",
          "count": 0
        }
      ]
    }
  ]
});
