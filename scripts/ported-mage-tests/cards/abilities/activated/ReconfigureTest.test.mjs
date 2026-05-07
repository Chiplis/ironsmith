import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/activated/ReconfigureTest.java",
  "tests": [
    {
      "name": "testAttach",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronzeplate Boar",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Reconfigure",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "unsupported",
          "source": "assertType(boar, CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(boar, SubType.EQUIPMENT)"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, boar, lion, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 5,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "testAttachDetach",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronzeplate Boar",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Reconfigure",
          "target": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{5}:"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "unsupported",
          "source": "assertType(boar, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(boar, SubType.EQUIPMENT)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "Trample",
          "expected": false
        }
      ]
    },
    {
      "name": "testSigardasAid",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sigarda's Aid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bronzeplate Boar",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bronzeplate Boar"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
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
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertType(boar, CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(boar, SubType.EQUIPMENT)"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, boar, lion, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 5,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "testPuresteelPaladin",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bronzeplate Boar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Puresteel Paladin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "unsupported",
          "source": "assertType(boar, CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(boar, SubType.EQUIPMENT)"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, boar, lion, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 5,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "Trample",
          "expected": true
        }
      ]
    }
  ]
});
