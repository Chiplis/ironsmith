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
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Hapatra, Vizier of Poisons",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Grizzly Bears",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Grizzly Bears",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Snake Token",
          "ability": "Deathtouch",
          "expected": true
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
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
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
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wall of Omens",
          "power": -1,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Wall of Omens",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Snake Token",
          "ability": "Deathtouch",
          "expected": true
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Put"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Devoted Druid",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Devoted Druid",
          "power": -1,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Snake Token",
          "ability": "Deathtouch",
          "expected": true
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
