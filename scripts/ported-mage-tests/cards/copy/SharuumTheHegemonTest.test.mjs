import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/SharuumTheHegemonTest.java",
  "tests": [
    {
      "name": "testCloneTriggered",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sharuum the Hegemon@original",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Clone@copy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Artist",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Clone",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@copy"
        },
        {
          "op": "assertStackObject",
          "source": "Clone",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Sharuum the Hegemon"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose which Sharuum the Hegemon to keep (legend rule)",
          "choices": [
            {
              "alias": "@copy"
            }
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "trigger_order",
          "description": "Order triggered abilities. The leftmost item becomes the top of your stack.",
          "choices": [
            "Blood Artist\nWhenever this creature or another creature dies\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life",
            "Sharuum the Hegemon\nWhen Sharuum enters the battlefield\nWhen Sharuum enters, you may return target artifact card from your graveyard to the battlefield"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@original"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Blood Artist",
          "count": 1,
          "abilityKind": "Triggered",
          "onTop": true,
          "targetCount": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Sharuum the Hegemon",
          "count": 1,
          "abilityKind": "Triggered",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose which Sharuum the Hegemon to keep (legend rule)",
          "choices": [
            {
              "alias": "@copy"
            }
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "trigger_order",
          "description": "Order triggered abilities. The leftmost item becomes the top of your stack.",
          "choices": [
            "Blood Artist\nWhenever this creature or another creature dies\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life",
            "Sharuum the Hegemon\nWhen Sharuum enters the battlefield\nWhen Sharuum enters, you may return target artifact card from your graveyard to the battlefield"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@original"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Blood Artist",
          "count": 1,
          "abilityKind": "Triggered",
          "onTop": true,
          "targetCount": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Sharuum the Hegemon",
          "count": 1,
          "abilityKind": "Triggered",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose which Sharuum the Hegemon to keep (legend rule)",
          "choices": [
            {
              "alias": "@copy"
            }
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "trigger_order",
          "description": "Order triggered abilities. The leftmost item becomes the top of your stack.",
          "choices": [
            "Blood Artist\nWhenever this creature or another creature dies\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life",
            "Sharuum the Hegemon\nWhen Sharuum enters the battlefield\nWhen Sharuum enters, you may return target artifact card from your graveyard to the battlefield"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@original"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Blood Artist",
          "count": 1,
          "abilityKind": "Triggered",
          "onTop": true,
          "targetCount": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Sharuum the Hegemon",
          "count": 1,
          "abilityKind": "Triggered",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose which Sharuum the Hegemon to keep (legend rule)",
          "choices": [
            {
              "alias": "@copy"
            }
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "trigger_order",
          "description": "Order triggered abilities. The leftmost item becomes the top of your stack.",
          "choices": [
            "Blood Artist\nWhenever this creature or another creature dies\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life",
            "Sharuum the Hegemon\nWhen Sharuum enters the battlefield\nWhen Sharuum enters, you may return target artifact card from your graveyard to the battlefield"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@original"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Blood Artist",
          "count": 1,
          "abilityKind": "Triggered",
          "onTop": true,
          "targetCount": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Sharuum the Hegemon",
          "count": 1,
          "abilityKind": "Triggered",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "@copy",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@original",
          "count": 1
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
