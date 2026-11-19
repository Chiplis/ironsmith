import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/SeraphAndSengirVampireTest.java",
  "tests": [
    {
      "name": "testBothDieButTriggersStillFire",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Seraph",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sengir Vampire",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Seraph",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Sengir Vampire",
          "attacker": "Seraph"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "source": "Seraph",
          "count": 1,
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "source": "Sengir Vampire",
          "count": 1,
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Seraph",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Sengir Vampire",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sengir Vampire",
          "count": 1
        }
      ]
    }
  ]
});
