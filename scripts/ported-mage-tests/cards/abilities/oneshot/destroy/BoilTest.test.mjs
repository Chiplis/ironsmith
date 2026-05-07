import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/destroy/BoilTest.java",
  "tests": [
    {
      "name": "testDestroy",
      "operations": [
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
          "player": 0,
          "name": "Breeding Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Boil",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Breeding Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Boil"
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
          "name": "Island",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Breeding Pool",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Island",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Breeding Pool",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Mountain",
          "count": 2
        }
      ]
    }
  ]
});
