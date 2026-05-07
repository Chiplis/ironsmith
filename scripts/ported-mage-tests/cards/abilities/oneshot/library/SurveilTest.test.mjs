import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/library/SurveilTest.java",
  "tests": [
    {
      "name": "Surveil2_YardYard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curate",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curate"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Alaborn Trooper^Barbtooth Wurm"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Alaborn Trooper",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Barbtooth Wurm",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Canopy Gorger",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardD)"
        }
      ]
    },
    {
      "name": "Surveil2_YardTop",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curate",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curate"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Alaborn Trooper",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Barbtooth Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardC, cardD)"
        }
      ]
    },
    {
      "name": "Surveil2_TopYard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curate",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curate"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Barbtooth Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardC, cardD)"
        }
      ]
    },
    {
      "name": "Surveil2_TopTop",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curate",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curate"
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
      "name": "Surveil2_TopTop_otherorder",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curate",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curate"
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
    },
    {
      "name": "SurveilX_one_Yard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Desmond Miles",
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Desmond Miles",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Alaborn Trooper"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
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
      "name": "SurveilX_two_Yard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Desmond Miles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Battlegrowth"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Desmond Miles"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Desmond Miles",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Alaborn Trooper^Barbtooth Wurm"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Alaborn Trooper",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Barbtooth Wurm",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "unsupported",
          "source": "assertLibrary(playerA, cardC, cardD)"
        }
      ]
    }
  ]
});
