import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/znr/MoraugFuryOfAkoumTest.java",
  "tests": [
    {
      "name": "testPrecombatLandfall",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Moraug, Fury of Akoum",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 100
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
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
          "op": "unsupported",
          "source": "assertTapped(lion, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(bear, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(corpse, true)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 91
        }
      ]
    },
    {
      "name": "testPostcombatLandfall",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Moraug, Fury of Akoum",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 100
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
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
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 79
        }
      ]
    },
    {
      "name": "testDoubleLandfall",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Moraug, Fury of Akoum",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Azusa, Lost but Seeking",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 100
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
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
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 64
        }
      ]
    },
    {
      "name": "testDoubleMoraug",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror Gallery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Moraug, Fury of Akoum",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 1,
          "life": 100
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
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
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 46
        }
      ]
    }
  ]
});
