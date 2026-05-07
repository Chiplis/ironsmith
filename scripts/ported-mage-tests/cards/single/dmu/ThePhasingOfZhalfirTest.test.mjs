import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dmu/ThePhasingOfZhalfirTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Phasing of Zhalfir",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Phasing of Zhalfir"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 1)"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ornithopter"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "The Phasing of Zhalfir",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Phyrexian Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        }
      ]
    }
  ]
});
