import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/CumulativeUpkeepTest.java",
  "tests": [
    {
      "name": "basicTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phobian Phantasm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Phobian Phantasm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Phobian Phantasm",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Age counters\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Phobian Phantasm\", CounterType.AGE, 1)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Phobian Phantasm",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Age counters\", 5, PhaseStep.PRECOMBAT_MAIN, playerA, \"Phobian Phantasm\", CounterType.AGE, 2)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Phobian Phantasm",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        }
      ]
    },
    {
      "name": "controlChangeTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Kor Celebrant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Illusions of Grandeur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Puca's Mischief",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Illusions of Grandeur"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Kor Celebrant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Puca's Mischief"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cumulative upkeep"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Illusions of Grandeur"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kor Celebrant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Age counters\", 5, PhaseStep.PRECOMBAT_MAIN, playerB, \"Illusions of Grandeur\", CounterType.AGE, 2)"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 40
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kor Celebrant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Illusions of Grandeur",
          "count": 1
        }
      ]
    }
  ]
});
