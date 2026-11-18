import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/exile/ExileAndReturnTest.java",
  "tests": [
    {
      "name": "testExileAndReturn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tawnos's Coffin"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}, {T}",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Tawnos's Coffin\", false)"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testExileAndReturnWithCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tawnos's Coffin"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Battlegrowth",
          "target": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{3}, {T}",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Tawnos's Coffin\", false)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testExileAndReturnWithCountersAndAuras",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bramble Elemental@subject",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Blanchwood Armor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Frog Tongue",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Tawnos's Coffin",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Tawnos's Coffin",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Battlegrowth",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "target": "@subject"
        },
        {
          "op": "assertStackObject",
          "source": "Battlegrowth",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Bramble Elemental"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Blanchwood Armor",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "target": "@subject"
        },
        {
          "op": "assertStackObject",
          "source": "Blanchwood Armor",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Bramble Elemental"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Frog Tongue",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "target": "@subject"
        },
        {
          "op": "assertStackObject",
          "source": "Frog Tongue",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Bramble Elemental"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "@subject",
          "count": 1,
          "counter": "+1/+1"
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "\"Blanchwood Armor\"",
          "target": "@subject",
          "expected": true
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "\"Frog Tongue\"",
          "target": "@subject",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Saproling Token",
          "count": 4
        },
        {
          "op": "activateAbility",
          "source": "Tawnos's Coffin",
          "ability": "Exile target creature",
          "player": 0,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "target": "@subject"
        },
        {
          "op": "assertStackObject",
          "source": "Tawnos's Coffin",
          "count": 1,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Bramble Elemental"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "@subject",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@subject"
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Tawnos's Coffin",
          "tapped": true
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Blanchwood Armor",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Frog Tongue",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "kind": "boolean",
          "player": 0,
          "description": "untap Tawnos's Coffin during your untap step",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 1,
          "description": "Attach Aura to",
          "choices": [
            "Bramble Elemental"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 1,
          "description": "Attach Aura to",
          "choices": [
            "Bramble Elemental"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Tawnos's Coffin",
          "tapped": false
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@subject"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "@subject",
          "count": 0
        },
        {
          "op": "assertTapped",
          "player": 1,
          "name": "@subject",
          "tapped": true
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "@subject",
          "count": 1,
          "counter": "+1/+1"
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "\"Blanchwood Armor\"",
          "target": "@subject",
          "expected": true
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "\"Frog Tongue\"",
          "target": "@subject",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bramble Elemental",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Blanchwood Armor",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Blanchwood Armor",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Frog Tongue",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Frog Tongue",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Saproling Token",
          "count": 8
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Bramble Elemental",
          "power": 10,
          "toughness": 10
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Bramble Elemental",
          "ability": "Reach",
          "expected": true
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 3
        }
      ]
    },
    {
      "name": "testExileAndReturnIfTawnosLeftBattlefield",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion@subject",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Tawnos's Coffin",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Tawnos's Coffin",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Tawnos's Coffin",
          "ability": "Exile target creature",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "@subject"
        },
        {
          "op": "assertStackObject",
          "source": "Tawnos's Coffin",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "@subject",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@subject"
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Tawnos's Coffin",
          "tapped": true
        },
        {
          "op": "castSpell",
          "name": "Disenchant",
          "player": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "Tawnos's Coffin"
        },
        {
          "op": "assertStackObject",
          "source": "Disenchant",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "objectTarget": "Tawnos's Coffin"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Tawnos's Coffin",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Tawnos's Coffin",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@subject"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "@subject",
          "count": 0
        },
        {
          "op": "assertTapped",
          "player": 1,
          "name": "@subject",
          "tapped": true
        }
      ]
    }
  ]
});
