import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/EmbalmTest.java",
  "tests": [
    {
      "name": "testCreatureWithEmbalmJustCastAndTarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Yoked Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "White Knight",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel of Sanctions"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Yoked Ox"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Yoked Ox",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "White Knight",
          "count": 1
        }
      ]
    },
    {
      "name": "testCreatureETBAfterEmbalm",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Yoked Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "White Knight",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel of Sanctions"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Yoked Ox"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Doom Blade"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Angel of Sanctions"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Embalm"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "White Knight"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Yoked Ox",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "White Knight",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 0
        }
      ]
    },
    {
      "name": "testCreatureExiledByEmbalmCreatureReturns",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Doom Blade",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Yoked Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "White Knight",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel of Sanctions"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Yoked Ox"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Doom Blade",
          "target": "Angel of Sanctions"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Embalm"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "White Knight"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Doom Blade",
          "target": "Angel of Sanctions"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "CLEANUP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Yoked Ox",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "White Knight",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angel of Sanctions",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Doom Blade",
          "count": 2
        }
      ]
    }
  ]
});
