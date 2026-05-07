import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mir/LightningCoilsTest.java",
  "tests": [
    {
      "name": "sacrificeSixCreaturesProducesSixElementals",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lightning Coils",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bottle Gnomes",
          "count": 6
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Bottle Gnomes",
          "count": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lightning Coils",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elemental Token",
          "count": 6
        }
      ]
    },
    {
      "name": "sacrificeSixCreaturesProducesSixElementalsExiledAtEnd",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lightning Coils",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bottle Gnomes",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grand Melee",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grand Melee"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "CLEANUP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grand Melee",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Bottle Gnomes",
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lightning Coils",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elemental Token",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - (tokenCount * 3)"
        }
      ]
    }
  ]
});
