import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ExertTest.java",
  "tests": [
    {
      "name": "exertGustWalker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gust Walker",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Gust Walker",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gust Walker",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Gust Walker",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "exertHoodedBrawler",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hooded Brawler",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Hooded Brawler",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Hooded Brawler",
          "power": 5,
          "toughness": 4
        }
      ]
    },
    {
      "name": "exertBitterbladeWarrior",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bitterblade Warrior",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bitterblade Warrior",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Bitterblade Warrior",
          "ability": "Deathtouch",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Bitterblade Warrior",
          "power": 3,
          "toughness": 2
        }
      ]
    },
    {
      "name": "exertRhetCropSpearmaster",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rhet-Crop Spearmaster",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rhet-Crop Spearmaster",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Rhet-Crop Spearmaster",
          "ability": "FirstStrike",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rhet-Crop Spearmaster",
          "power": 4,
          "toughness": 1
        }
      ]
    },
    {
      "name": "exertTahCropElite",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tah-Crop Elite",
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
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Tah-Crop Elite",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Tah-Crop Elite",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "exertEmberhornMinotaur",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Emberhorn Minotaur",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Emberhorn Minotaur",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Emberhorn Minotaur",
          "ability": "new MenaceAbility()",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Emberhorn Minotaur",
          "power": 5,
          "toughness": 4
        }
      ]
    },
    {
      "name": "exertNefcropEntangler",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nef-Crop Entangler",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Nef-Crop Entangler",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Nef-Crop Entangler",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "exertTrueheartTwins",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Trueheart Twins",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hyena Pack",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Dune Beetle",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Trueheart Twins",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Trueheart Twins",
          "power": 5,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Hyena Pack",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Dune Beetle",
          "power": 1,
          "toughness": 4
        }
      ]
    },
    {
      "name": "exertOtherCreatureTrueheartTwins",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gust Walker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Trueheart Twins",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Gust Walker",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Trueheart Twins",
          "power": 5,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Gust Walker",
          "power": 4,
          "toughness": 3
        }
      ]
    },
    {
      "name": "exertUsedDoesNotUntapNextUntapStep",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glory-Bound Initiate",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Glory-Bound Initiate",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "unsupported",
          "source": "assertTapped(gbInitiate, true)"
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
        }
      ]
    },
    {
      "name": "exertNotUsedRegularAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glory-Bound Initiate",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Glory-Bound Initiate",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "unsupported",
          "source": "assertTapped(gbInitiate, false)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "stolenExertCreatureShouldUntapDuringOwnersUntapStep",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Glory-Bound Initiate",
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
          "zone": "HAND",
          "player": 1,
          "name": "Act of Treason",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Glory-Bound Initiate"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Act of Treason",
          "target": "Glory-Bound Initiate"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Glory-Bound Initiate",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Act of Treason",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 24
        },
        {
          "op": "unsupported",
          "source": "assertTapped(gbInitiate, false)"
        }
      ]
    },
    {
      "name": "combatCelebrantExertedCannotExertAgainDuringNextCombatPhase",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Combat Celebrant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Combat Celebrant",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "unsupported",
          "source": "assertTapped(cCelebrant, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(memnite, false)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "unsupported",
          "source": "assertTapped(cCelebrant, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(memnite, true)"
        }
      ]
    },
    {
      "name": "combatCelebrantExertedCannotExertDuringNextCombatPhase_InteractionWithAlwaysWatching",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Always Watching",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Combat Celebrant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Combat Celebrant",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 13
        },
        {
          "op": "unsupported",
          "source": "assertTapped(cCelebrant, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(memnite, false)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Combat Celebrant",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(cCelebrant, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(memnite, false)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 6
        }
      ]
    },
    {
      "name": "combatCelebrantExertedCannotExertAgainDuringNextCombatPhase_InteractionWithArlinnKord",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Arlinn Kord",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Combat Celebrant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1:"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Combat Celebrant"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Combat Celebrant",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Combat Celebrant",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(cCelebrant, false)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Arlinn Kord",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 8
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Combat Celebrant",
          "power": 6,
          "toughness": 3
        }
      ]
    }
  ]
});
