import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/InvestigateTest.java",
  "tests": [
    {
      "name": "testInvestigatingBothPlayers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Wernog, Rider's Chaplain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fateful Absence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Erdwal Illuminator",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Erdwal Illuminator",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Wernog, Rider's Chaplain"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you investigate for the "
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"lost 1 life\", 1, PhaseStep.BEGIN_COMBAT, playerB, 19)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Clue Token",
          "count": 3
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "label": "{2}, Sacrifice ",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Fateful Absence",
          "target": "Wernog, Rider's Chaplain"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Fateful Absence",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Wernog, Rider's Chaplain",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Clue Token",
          "count": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Clue Token",
          "count": 2
        }
      ]
    },
    {
      "name": "testBriarbridgePatrol",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Briarbridge Patrol",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Briarbridge Patrol",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Pillarfield Ox",
          "attacker": "Briarbridge Patrol"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Briarbridge Patrol",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Pillarfield Ox",
          "attacker": "Briarbridge Patrol"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{2},"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
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
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Briarbridge Patrol",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Clue Token",
          "count": 1
        }
      ]
    }
  ]
});
