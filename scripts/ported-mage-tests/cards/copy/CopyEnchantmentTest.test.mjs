import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/CopyEnchantmentTest.java",
  "tests": [
    {
      "name": "copyNonAuraEnchantment",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Copy Enchantment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Furnace Whelp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Crucible of Fire",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Copy Enchantment"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Crucible of Fire"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Copy Enchantment",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Crucible of Fire",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Furnace Whelp",
          "power": 5,
          "toughness": 5
        }
      ]
    },
    {
      "name": "copyAuraEnchantment",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Inferno Fist",
          "count": 1
        },
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
          "name": "Geist of the Moors",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Copy Enchantment@copy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion@lion1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion@lion2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion@lion3",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Inferno Fist",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Geist of the Moors"
        },
        {
          "op": "assertStackObject",
          "source": "Inferno Fist",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Geist of the Moors"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Inferno Fist\"",
          "target": "Geist of the Moors",
          "expected": true
        },
        {
          "op": "castSpell",
          "name": "Copy Enchantment",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@copy"
        },
        {
          "op": "assertStackObject",
          "source": "Copy Enchantment",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Inferno Fist"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 1,
          "description": "Attach Aura to",
          "choices": [
            "Silvercoat Lion"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "\"@copy\"",
          "target": "@lion1",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Geist of the Moors",
          "power": 5,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "@lion1",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "@lion2",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Inferno Fist",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Inferno Fist",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Inferno Fist",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "@copy",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
