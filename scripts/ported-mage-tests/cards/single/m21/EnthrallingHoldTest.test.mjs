import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m21/EnthrallingHoldTest.java",
  "tests": [
    {
      "name": "testTappedTarget_untapped_doesNotFizzle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Traxos, Scourge of Kroog",
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
          "name": "Enthralling Hold",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Twiddle",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enthralling Hold"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Traxos, Scourge of Kroog"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Twiddle"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Traxos, Scourge of Kroog"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Traxos, Scourge of Kroog",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Traxos, Scourge of Kroog",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enthralling Hold",
          "count": 1
        }
      ]
    },
    {
      "name": "testTappedTarget_becomesIllegal_fizzles",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Traxos, Scourge of Kroog",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
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
          "zone": "HAND",
          "player": 0,
          "name": "Enthralling Hold",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enthralling Hold",
          "target": "Traxos, Scourge of Kroog"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Doom Blade",
          "target": "Traxos, Scourge of Kroog"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Traxos, Scourge of Kroog",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Traxos, Scourge of Kroog",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Traxos, Scourge of Kroog",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enthralling Hold",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        }
      ]
    }
  ]
});
