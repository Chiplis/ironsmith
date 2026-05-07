import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/library/ScryTest.java",
  "tests": [
    {
      "name": "Scry2_BottomBottom",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Preordain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Preordain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Alaborn Trooper^Barbtooth Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Barbtooth Wurm"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Canopy Gorger",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardD, cardB, cardA)"
        }
      ]
    },
    {
      "name": "Scry2_BottomBottom_otherorder",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Preordain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Preordain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Alaborn Trooper^Barbtooth Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alaborn Trooper"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Canopy Gorger",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardD, cardA, cardB)"
        }
      ]
    },
    {
      "name": "Scry2_BottomTop",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Preordain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Preordain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Alaborn Trooper"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Barbtooth Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardC, cardD, cardA)"
        }
      ]
    },
    {
      "name": "Scry2_TopBottom",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Preordain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Preordain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Barbtooth Wurm"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Alaborn Trooper",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardC, cardD, cardB)"
        }
      ]
    },
    {
      "name": "Scry2_TopTop",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Preordain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Preordain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Barbtooth Wurm"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Alaborn Trooper",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardB, cardC, cardD)"
        }
      ]
    },
    {
      "name": "Scry2_TopTop_otherorder",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Preordain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Preordain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alaborn Trooper"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Barbtooth Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardA, cardC, cardD)"
        }
      ]
    }
  ]
});
