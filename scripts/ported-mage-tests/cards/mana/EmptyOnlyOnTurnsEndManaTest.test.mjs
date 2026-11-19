import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/EmptyOnlyOnTurnsEndManaTest.java",
  "tests": [
    {
      "name": "testDaxosOfMeletis",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shizuko, Caller of Autumn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Birds of Paradise",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Birds of Paradise",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "UPKEEP",
          "source": "Shizuko, Caller of Autumn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 3
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 3,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Birds of Paradise",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Birds of Paradise",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "UPKEEP",
          "source": "Shizuko, Caller of Autumn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "assertManaPool",
          "player": 1,
          "colors": "G",
          "amount": 3
        },
        {
          "op": "assertManaPool",
          "player": 1,
          "colors": "G",
          "amount": 3,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Birds of Paradise",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Birds of Paradise",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Birds of Paradise",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Birds of Paradise",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "player": 1,
          "colors": "G",
          "amount": 2
        },
        {
          "op": "assertManaPool",
          "player": 1,
          "colors": "WUBRGC",
          "amount": 2
        }
      ]
    }
  ]
});
