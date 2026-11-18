import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/EstridsInvocationTest.java",
  "tests": [
    {
      "name": "testCopyAura",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Estrid's Invocation@copy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Feather of Flight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
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
          "name": "Impact Tremors",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Feather of Flight",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "source": "Feather of Flight",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Feather of Flight\"",
          "target": "Balduvian Bears",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "castSpell",
          "name": "Estrid's Invocation",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@copy"
        },
        {
          "op": "assertStackObject",
          "source": "Estrid's Invocation",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Feather of Flight"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Attach Aura to",
          "choices": [
            "Bear Cub"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"@copy\"",
          "target": "Bear Cub",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bear Cub",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertStackObject",
          "source": "Feather of Flight",
          "count": 1,
          "turn": 5,
          "phase": "UPKEEP",
          "abilityKind": "Triggered"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Feather of Flight"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Attach Aura to",
          "choices": [
            "Grizzly Bears"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "UPKEEP"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"@copy\"",
          "target": "Grizzly Bears",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bear Cub",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
