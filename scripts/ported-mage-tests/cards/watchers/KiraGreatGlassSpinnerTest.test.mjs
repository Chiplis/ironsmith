import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/KiraGreatGlassSpinnerTest.java",
  "tests": [
    {
      "name": "counterFirst",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: {this} deals 3 damage",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "counter": "LOYALTY",
          "count": 9
        }
      ]
    },
    {
      "name": "counterFirstResolveSecond",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
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
          "zone": "HAND",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: {this} deals 3 damage",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unsummon",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "counter": "LOYALTY",
          "count": 9
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        }
      ]
    },
    {
      "name": "counterFirstThisTurn_counterFirstOnNextTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: {this} deals 3 damage to",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: {this} deals 3 damage to",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "counter": "LOYALTY",
          "count": 11
        }
      ]
    },
    {
      "name": "testKiraGreatGlassSpinnerFirstSpellTurn",
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
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        }
      ]
    },
    {
      "name": "testKiraGreatGlassSpinnerRightAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Steadfast Guard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angelic Benediction",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Steadfast Guard",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "unsupported",
          "source": "assertTapped(kira, false)"
        }
      ]
    },
    {
      "name": "testKiraGreatGlassSpinnerRightAbilityOtherOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Steadfast Guard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angelic Benediction",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Steadfast Guard",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "exalted"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "unsupported",
          "source": "assertTapped(kira, false)"
        }
      ]
    },
    {
      "name": "counterFirstBlinkCounterNext",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: {this} deals 3 damage",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Cloudshift",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Shock",
          "target": "Kira, Great Glass-Spinner"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "counter": "LOYALTY",
          "count": 9
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        }
      ]
    },
    {
      "name": "counterCorrectlyMultipleOnStack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dragonscale Boon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Battlegrowth",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dragonscale Boon",
          "target": "Kira, Great Glass-Spinner"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Battlegrowth",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dragonscale Boon",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kira, Great Glass-Spinner",
          "power": 4,
          "toughness": 4
        }
      ]
    }
  ]
});
