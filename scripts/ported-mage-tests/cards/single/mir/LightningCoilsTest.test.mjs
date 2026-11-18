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
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Bottle Gnomes",
          "ability": "Sacrifice"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Lightning Coils",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Bottle Gnomes",
          "ability": "Sacrifice"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Lightning Coils",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Bottle Gnomes",
          "ability": "Sacrifice"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Lightning Coils",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Bottle Gnomes",
          "ability": "Sacrifice"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Lightning Coils",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Bottle Gnomes",
          "ability": "Sacrifice"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Lightning Coils",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 5
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Bottle Gnomes",
          "ability": "Sacrifice"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Lightning Coils",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Lightning Coils",
          "counter": "CHARGE",
          "count": 6
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
          "op": "assertStackObject",
          "turn": 3,
          "phase": "UPKEEP",
          "source": "Lightning Coils",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "UPKEEP"
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
