import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/BiovisionaryTest.java",
  "tests": [
    {
      "name": "testFourWin",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Biovisionary",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Biovisionary",
          "count": 4,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "count": 4,
          "source": "Biovisionary",
          "abilityKind": "Triggered"
        },
        {
          "op": "assertPermanentCount",
          "name": "Biovisionary",
          "count": 4,
          "seatIndependent": true,
          "player": 0,
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "WON"
        },
        {
          "op": "assertResult",
          "player": 1,
          "result": "LOST"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testFourWithThreeClonesWin",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Biovisionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Clone",
          "count": 3
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Biovisionary",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Clone"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "text": "Cast Clone"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Biovisionary"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Biovisionary",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Clone"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "text": "Cast Clone"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Biovisionary"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Biovisionary",
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Clone"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "text": "Cast Clone"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Biovisionary"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Biovisionary",
          "count": 4,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "count": 4,
          "source": "Biovisionary",
          "abilityKind": "Triggered"
        },
        {
          "op": "assertPermanentCount",
          "name": "Biovisionary",
          "count": 4,
          "seatIndependent": true,
          "player": 0,
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "WON"
        },
        {
          "op": "assertResult",
          "player": 1,
          "result": "LOST"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
