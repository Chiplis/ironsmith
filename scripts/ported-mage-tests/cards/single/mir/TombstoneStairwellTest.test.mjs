import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mir/TombstoneStairwellTest.java",
  "tests": [
    {
      "name": "test_LeavesTheBattlefield",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerA)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tombstone Stairwell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 3
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cumulative upkeep"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"turn 1 - pays done\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Swamp\", true, 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tombspawn",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Tombspawn",
          "count": 3
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Tombspawn",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Tombspawn",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tombspawn",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Tombspawn",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "target destroy",
          "target": "Tombstone Stairwell"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Tombspawn",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Tombspawn",
          "count": 0
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
