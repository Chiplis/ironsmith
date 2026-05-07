import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planeswalker/GideonTest.java",
  "tests": [
    {
      "name": "testGideonBattleForged",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kytheon, Hero of Akros",
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
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Kytheon, Hero of Akros",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "+1: Until your next turn, target creature gains indestructible. Untap that creature.",
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gideon, Battle-Forged",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Gideon, Battle-Forged",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Silvercoat Lion",
          "ability": "Indestructible",
          "expected": false
        }
      ]
    },
    {
      "name": "testGideonChampionOfJusticeSecondAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gideon, Champion of Justice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gideon, Champion of Justice"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Until end of turn"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Put a loyalty counter on",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0: Until end of turn"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gideon, Champion of Justice",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gideon, Champion of Justice",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Gideon, Champion of Justice",
          "counter": "LOYALTY",
          "count": 7
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Gideon, Champion of Justice",
          "power": 7,
          "toughness": 7
        }
      ]
    },
    {
      "name": "testGideonBattleForgedSacrifice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kytheon, Hero of Akros",
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
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Stitcher's Graft",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Kytheon, Hero of Akros",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Gideon, Battle-Forged",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "0: Until "
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip {2}",
          "target": "Gideon, Battle-Forged"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Gideon, Battle-Forged",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 7
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gideon, Battle-Forged",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kytheon, Hero of Akros",
          "count": 1
        }
      ]
    },
    {
      "name": "testGideonJuraNoPrevention",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon Jura",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Leyline of Punishment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0:"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Gideon Jura"
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
          "op": "unsupported",
          "source": "assertType(\"Gideon Jura\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Gideon Jura\", 3)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Gideon Jura",
          "counter": "LOYALTY",
          "count": 3
        }
      ]
    },
    {
      "name": "testGideonJuraNoPreventionCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon Jura",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Leyline of Punishment",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "0:"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Gideon Jura",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Grizzly Bears",
          "attacker": "Gideon Jura"
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
          "op": "unsupported",
          "source": "assertType(\"Gideon Jura\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Gideon Jura\", 2)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Gideon Jura",
          "counter": "LOYALTY",
          "count": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    }
  ]
});
