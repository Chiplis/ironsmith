import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/akh/HapatraVizierOfPoisonsTest.java",
  "tests": [
    {
      "name": "hapatraCombatDamageToPlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hapatra, Vizier of Poisons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Hapatra, Vizier of Poisons",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "assertStackObject",
          "source": "Hapatra, Vizier of Poisons",
          "count": 1,
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "completeTargets": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Put a -1/-1 counter on target creature",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1,
          "counter": "-1/-1"
        },
        {
          "op": "assertStackObject",
          "source": "Hapatra, Vizier of Poisons",
          "count": 1,
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Grizzly Bears",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Snake Token",
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
            },
            {
              "property": "keyword",
              "value": "deathtouch",
              "expected": true
            }
          ]
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Snake Token",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 5,
          "player": 1,
          "blocker": "Hill Giant",
          "attacker": "Snake Token"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        }
      ]
    },
    {
      "name": "infectDamageTriggersHapatra",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hapatra, Vizier of Poisons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blight Mamba",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Omens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Blight Mamba",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Omens",
          "attacker": "Blight Mamba"
        },
        {
          "op": "assertStackObject",
          "source": "Hapatra, Vizier of Poisons",
          "count": 1,
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "count": 0,
          "counter": "POISON"
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Wall of Omens",
          "count": 1,
          "counter": "-1/-1"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wall of Omens",
          "power": -1,
          "toughness": 3
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Snake Token",
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
            },
            {
              "property": "keyword",
              "value": "deathtouch",
              "expected": true
            }
          ]
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Snake Token",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 5,
          "player": 1,
          "blocker": "Hill Giant",
          "attacker": "Snake Token"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        }
      ]
    },
    {
      "name": "devotedDruidTriggersHapatra",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hapatra, Vizier of Poisons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Devoted Druid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Devoted Druid",
          "ability": "Untap this creature"
        },
        {
          "op": "assertStackObject",
          "source": "Hapatra, Vizier of Poisons",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Devoted Druid",
          "count": 1,
          "counter": "-1/-1"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Devoted Druid",
          "power": -1,
          "toughness": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Snake Token",
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
            },
            {
              "property": "keyword",
              "value": "deathtouch",
              "expected": true
            }
          ]
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Snake Token",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 5,
          "player": 1,
          "blocker": "Hill Giant",
          "attacker": "Snake Token"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        }
      ]
    },
    {
      "name": "testTokensWithInfectTriggerHapatra",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sprout",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Triumph of the Hordes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hapatra, Vizier of Poisons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sprout"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Triumph of the Hordes"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Saproling Token",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Kraken Hatchling",
          "attacker": "Saproling Token"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Kraken Hatchling",
          "power": -2,
          "toughness": 2
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Kraken Hatchling",
          "counter": "M1M1",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 1
        }
      ]
    }
  ]
});
