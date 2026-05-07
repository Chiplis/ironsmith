import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/ManaReflectionTest.java",
  "tests": [
    {
      "name": "generatesCorrectManaFromMarwyn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Reflection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Marwyn, the Nurturer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.UPKEEP, playerA, \"Marwyn, the Nurturer\", CounterType.P1P1, 2)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Marwyn, the Nurturer",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.GREEN, 6)"
        }
      ]
    },
    {
      "name": "generatesCorrectManaFromGemstoneCaverns",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Reflection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gemstone Caverns",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.UPKEEP, playerA, \"Gemstone Caverns\", CounterType.LUCK, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
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
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.GREEN, 2)"
        }
      ]
    },
    {
      "name": "generatesCorrectManaFromLlanowarElves",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Reflection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add"
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
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.GREEN, 2)"
        }
      ]
    },
    {
      "name": "ManaReflectionWithGoblinClearcutterTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Reflection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Clearcutter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}{R}{R}{R}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}{R}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "ManaReflectionWithHavenwoodBattlegroundTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Reflection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Havenwood Battleground",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}, Sacrifice"
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
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.GREEN, 4)"
        }
      ]
    }
  ]
});
