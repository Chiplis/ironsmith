import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/activated/LicidAbilityTest.java",
  "tests": [
    {
      "name": "BasicUsageTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enraging Licid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R},",
          "target": "Pillarfield Ox"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Pillarfield Ox",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Enraging Licid",
          "ability": "ColoredManaSymbol.R",
          "expected": false
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Enraging Licid\", CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Enraging Licid\", CardType.CREATURE, false)"
        }
      ]
    },
    {
      "name": "SpecialActionTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enraging Licid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R},",
          "target": "Pillarfield Ox"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: End"
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
          "op": "unsupported",
          "source": "assertActionsCount(playerA, 0)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Pillarfield Ox",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Enraging Licid",
          "ability": "ColoredManaSymbol.R",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Enraging Licid\", CardType.ENCHANTMENT, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Enraging Licid\", CardType.CREATURE, true)"
        }
      ],
      "skip": "upstream @Ignore: Test player can't activate special actions yet"
    },
    {
      "name": "EnchantedCreatureDiesTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enraging Licid",
          "count": 1
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
          "player": 0,
          "name": "Mountain",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R},",
          "target": "Pillarfield Ox"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Doom Blade",
          "target": "Pillarfield Ox"
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
          "name": "Enraging Licid",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enraging Licid",
          "count": 1
        }
      ]
    }
  ]
});
