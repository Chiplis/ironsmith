import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/ErsatzGnomesTest.java",
  "tests": [
    {
      "name": "testColorlessSpellCreatesColorlessPermanent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ersatz Gnomes",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Silvercoat Lion",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Target spell",
          "target": "Silvercoat Lion",
          "source": "Ersatz Gnomes"
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Ersatz Gnomes\", true)"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Silvercoat Lion",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "colorCount",
              "expected": 0
            }
          ]
        }
      ]
    },
    {
      "name": "testColorlessSpellCreatesColorlessPermanentUntilItBattlefieldLeft",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ersatz Gnomes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Silvercoat Lion",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Target spell",
          "target": "Silvercoat Lion",
          "source": "Ersatz Gnomes"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 1,
          "name": "Unsummon",
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Ersatz Gnomes\", true)"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Silvercoat Lion",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "colorCount",
              "expected": 1
            },
            {
              "property": "color",
              "expected": true,
              "value": "white"
            }
          ]
        }
      ]
    },
    {
      "name": "testChangeColorOfBestowSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hopeful Eidolon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ersatz Gnomes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hopeful Eidolon using bestow",
          "target": "Silvercoat Lion",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Hopeful Eidolon",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Target spell",
          "target": "Hopeful Eidolon",
          "source": "Ersatz Gnomes"
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
          "name": "Hopeful Eidolon",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hopeful Eidolon",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Ersatz Gnomes\", true)"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Hopeful Eidolon",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "colorCount",
              "expected": 0
            }
          ]
        }
      ]
    },
    {
      "name": "testChangeColorOfBestowSpellUnsummon",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hopeful Eidolon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ersatz Gnomes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hopeful Eidolon",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Hopeful Eidolon",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Target spell",
          "target": "Hopeful Eidolon",
          "source": "Ersatz Gnomes"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 1,
          "name": "Unsummon",
          "target": "Hopeful Eidolon"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Hopeful Eidolon"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Hopeful Eidolon",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hopeful Eidolon",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Ersatz Gnomes\", true)"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Hopeful Eidolon",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "colorCount",
              "expected": 1
            },
            {
              "property": "color",
              "expected": true,
              "value": "white"
            }
          ]
        }
      ]
    }
  ]
});
