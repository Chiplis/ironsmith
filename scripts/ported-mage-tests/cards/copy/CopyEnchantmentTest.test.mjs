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
          "name": "Copy Enchantment",
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
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Inferno Fist",
          "target": "Geist of the Moors"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Copy Enchantment"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Inferno Fist"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Inferno Fist",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Copy Enchantment",
          "count": 0
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Geist of the Moors",
          "power": 5,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 2
        }
      ]
    }
  ]
});
