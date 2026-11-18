import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mir/TombstoneStairwellTest.java",
  "tests": [
    {
      "name": "test_LeavesTheBattlefield",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Stairwell Removal Fixture",
          "count": 1,
          "fixtureSource": true,
          "manaCost": "{0}",
          "typeLine": "Instant",
          "oracleText": "Destroy target enchantment."
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
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swamp",
          "tapped": true,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tombspawn",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Tombspawn",
          "count": 3,
          "seatIndependent": true
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
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Tombspawn",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tombspawn",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Tombspawn",
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Stairwell Removal Fixture",
          "target": "Tombstone Stairwell"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "source": "Stairwell Removal Fixture",
          "onTop": true,
          "count": 1,
          "objectTarget": "Tombstone Stairwell"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Tombstone Stairwell",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "source": "Tombstone Stairwell",
          "onTop": true,
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Tombspawn",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Tombspawn",
          "count": 0,
          "seatIndependent": true
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
