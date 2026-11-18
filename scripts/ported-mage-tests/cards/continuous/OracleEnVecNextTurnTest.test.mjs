import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/OracleEnVecNextTurnTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Oracle en-Vec",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Angelic Wall",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target opponent",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Angelic Wall"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": "20",
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": "20 - 2",
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_TURN",
          "source": "Oracle en-Vec",
          "onTop": true,
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Angelic Wall",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": "20 - 2",
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": "20 - 2",
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
