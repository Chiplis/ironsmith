import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/OneShotNonTargetTest.java",
  "tests": [
    {
      "name": "YorionChooseAfterTriggerTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Yorion, Sky Nomad",
          "count": 1
        },
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
          "name": "Resolute Reinforcements",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Yorion, Sky Nomad"
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
          "source": "Yorion, Sky Nomad",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Resolute Reinforcements"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Resolute Reinforcements",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Yorion, Sky Nomad",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose another nonland permanent you both own and control",
          "choices": [
            "Resolute Reinforcements"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Resolute Reinforcements",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Yorion, Sky Nomad",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Resolute Reinforcements",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 7
        }
      ]
    },
    {
      "name": "NonTargetAdjusterTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Temporal Firestorm",
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
          "count": 2
        },
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
          "name": "Squire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Python",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Temporal Firestorm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire^Python"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Squire",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Python",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Watchwolf",
          "count": 1
        }
      ]
    },
    {
      "name": "ModeSelectionTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "SOLDIER Military Program",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "SOLDIER Military Program",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "SOLDIER Military Program",
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
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Put a +1/+1 counter on each of up to two Soldiers you control."
          ]
        },
        {
          "op": "assertStackObject",
          "source": "SOLDIER Military Program",
          "count": 1,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Triggered",
          "targetCount": 0,
          "completeChoices": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Squire"
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 3
            }
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create a 1/1 white Soldier creature token."
          ]
        },
        {
          "op": "assertStackObject",
          "source": "SOLDIER Military Program",
          "count": 1,
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Triggered",
          "targetCount": 0,
          "completeChoices": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Squire"
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 3
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Soldier Token",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            }
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Put a +1/+1 counter on each of up to two Soldiers you control."
          ]
        },
        {
          "op": "assertStackObject",
          "source": "SOLDIER Military Program",
          "count": 1,
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Triggered",
          "targetCount": 0,
          "completeChoices": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Soldier Token"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Squire"
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 3
            },
            {
              "property": "toughness",
              "expected": 4
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Soldier Token",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            }
          ]
        }
      ]
    },
    {
      "name": "MuseVesselTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Muse Vessel@vessel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 30
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vizier of Tumbling Sands",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Squire@squire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alpha Myr@myr",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Void Snare",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Island@island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Corridor Monitor@monitor",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Muse Vessel",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@vessel"
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Target player exiles a card",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "targetCount": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Vizier of Tumbling Sands",
          "ability": "Untap another target permanent",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Muse Vessel"
        },
        {
          "op": "assertStackObject",
          "source": "Vizier of Tumbling Sands",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Muse Vessel",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "@vessel",
          "tapped": false
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Target player exiles a card",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "targetCount": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Corridor Monitor"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Corridor Monitor",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Vizier of Tumbling Sands",
          "ability": "Untap another target permanent",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Muse Vessel"
        },
        {
          "op": "assertStackObject",
          "source": "Vizier of Tumbling Sands",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Muse Vessel",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "@vessel",
          "tapped": false
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Target player exiles a card",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 0
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "targetCount": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Choose a card exiled with",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Choose a card exiled with",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Squire",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@squire"
        },
        {
          "op": "assertStackObject",
          "source": "Squire",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "playLand",
          "name": "@island",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@island"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@squire"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "castSpell",
          "name": "Void Snare",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "@vessel"
        },
        {
          "op": "assertStackObject",
          "source": "Void Snare",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "objectTarget": "Muse Vessel"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Void Snare",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Muse Vessel",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "objectAlias": "@vessel"
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Choose a card exiled with",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "@monitor",
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "owner": 1,
          "name": "Corridor Monitor",
          "objectAlias": "@monitor",
          "zone": "exile",
          "methods": []
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Target player exiles a card",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": 0
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "targetCount": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alpha Myr"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Alpha Myr",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Choose a card exiled with",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alpha Myr"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Alpha Myr",
          "objectAlias": "@myr",
          "zone": "exile",
          "methods": [
            "play_from"
          ]
        },
        {
          "op": "assertCastMethods",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alpha Myr",
          "objectAlias": "@myr",
          "zone": "exile",
          "methods": []
        },
        {
          "op": "activateAbility",
          "source": "Muse Vessel",
          "ability": "Choose a card exiled with",
          "player": 0,
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertStackObject",
          "source": "Muse Vessel",
          "count": 1,
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alpha Myr"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "castSpell",
          "name": "Alpha Myr",
          "player": 0,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "objectAlias": "@myr"
        },
        {
          "op": "assertStackObject",
          "source": "Alpha Myr",
          "count": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@myr"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "@monitor",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "@island",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@squire",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@myr",
          "count": 0
        }
      ]
    }
  ]
});
