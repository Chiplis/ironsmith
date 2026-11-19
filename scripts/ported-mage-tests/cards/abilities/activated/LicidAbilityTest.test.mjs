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
          "source": "Enraging Licid",
          "ability": "{R},",
          "target": "Pillarfield Ox"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Enraging Licid",
          "target": "Pillarfield Ox",
          "expected": true
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Pillarfield Ox",
          "ability": "Haste",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Enraging Licid",
          "ability": "{R}: End"
        },
        {
          "op": "assertNoAction",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "kind": "special_action",
          "label": "End",
          "source": "Enraging Licid"
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
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Pillarfield Ox",
          "ability": "Haste",
          "count": 0
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Enraging Licid",
          "cardType": "CREATURE",
          "extra": "true"
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Enraging Licid",
          "cardType": "ENCHANTMENT",
          "extra": "false"
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Enraging Licid",
          "kind": "activated",
          "textPattern": "becomes an Aura",
          "count": 1
        }
      ]
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
