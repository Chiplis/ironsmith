import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/EndOfTurnOneOpponentTest.java",
  "tests": [
    {
      "name": "test_EndOfTurnSingle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "boost1",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Creatures get +1/+1 until end of turn."
        },
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
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "boost1",
          "ability": "Creatures get +1/+1",
          "resolve": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 2,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 3,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 4,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_UntilYourNextTurnSingle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "boost1",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Creatures get +1/+1 until your next turn."
        },
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
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "boost1",
          "ability": "Creatures get +1/+1",
          "resolve": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 2,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 3,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 4,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_UntilEndOfYourNextTurnSingle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "boost1",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "oracleText": "{0}: Creatures get +1/+1 until the end of your next turn."
        },
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
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "boost1",
          "ability": "Creatures get +1/+1",
          "resolve": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 2,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 3,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "assertPowerToughnessEachPhase",
          "turn": 4,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phases": [
            "PRECOMBAT_MAIN",
            "BEGIN_COMBAT",
            "DECLARE_ATTACKERS",
            "DECLARE_BLOCKERS",
            "COMBAT_DAMAGE",
            "END_COMBAT",
            "POSTCOMBAT_MAIN",
            "END_TURN"
          ]
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
