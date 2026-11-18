import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/WallOfDustNextTurnTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wall of Dust",
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
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "all attacks",
          "typeLine": "",
          "custom": true,
          "fixtureSource": true,
          "oracleText": "Creatures your opponents control attack each combat if able."
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": 20,
          "seatIndependent": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Wall of Dust",
          "attacker": "Balduvian Bears"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": 18,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": 18,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": 16,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": 16,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "assertLife",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": 12,
          "seatIndependent": true
        },
        {
          "op": "setStopAt",
          "turn": 6,
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
