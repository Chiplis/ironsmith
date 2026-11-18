import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/EndOfTurnMultiOpponentsTest.java",
  "tests": [
    {
      "name": "test_EndOfTurnMulti",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
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
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 6,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 8,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 9,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 10,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 11,
          "player": "playerC",
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 12,
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
          "player": "playerD",
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
          "player": "playerC",
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
          "player": "playerB",
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
          "turn": 5,
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
          "turn": 6,
          "player": "playerD",
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
          "turn": 7,
          "player": "playerC",
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
          "turn": 8,
          "player": "playerB",
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
          "turn": 9,
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
          "turn": 10,
          "player": "playerD",
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
          "turn": 11,
          "player": "playerC",
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
          "turn": 12,
          "player": "playerB",
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
          "turn": 12,
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
      "name": "test_UntilYourNextTurnMulti",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
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
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 6,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 8,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 9,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 10,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 11,
          "player": "playerC",
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 12,
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
          "player": "playerD",
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
          "player": "playerC",
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
          "player": "playerB",
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
          "turn": 5,
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
          "turn": 6,
          "player": "playerD",
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
          "turn": 7,
          "player": "playerC",
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
          "turn": 8,
          "player": "playerB",
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
          "turn": 9,
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
          "turn": 10,
          "player": "playerD",
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
          "turn": 11,
          "player": "playerC",
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
          "turn": 12,
          "player": "playerB",
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
          "turn": 12,
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
      "name": "test_UntilEndOfYourNextTurnMulti",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
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
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 6,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 8,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 9,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 10,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 11,
          "player": "playerC",
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 12,
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
          "player": "playerD",
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
          "player": "playerC",
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
          "player": "playerB",
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
          "turn": 5,
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
          "turn": 6,
          "player": "playerD",
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
          "turn": 7,
          "player": "playerC",
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
          "turn": 8,
          "player": "playerB",
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
          "turn": 9,
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
          "turn": 10,
          "player": "playerD",
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
          "turn": 11,
          "player": "playerC",
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
          "turn": 12,
          "player": "playerB",
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
          "turn": 12,
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
      "name": "test_UntilYourNextTurnMulti_Leaved",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerC",
          "name": "Eye of Doom",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Forest",
          "count": 4
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
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPlayerState",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "player": "playerA",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_TURN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPlayerState",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_TURN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerC",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": false
        },
        {
          "op": "concede",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "Balduvian Bears",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPlayerState",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerC",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerC",
          "name": "Eye of Doom"
        },
        {
          "op": "addTarget",
          "player": 2,
          "target": "Balduvian Bears"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Balduvian Bears"
        },
        {
          "op": "addTarget",
          "player": 3,
          "target": "Balduvian Bears"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "player": "playerC",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_TURN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPlayerState",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "player": "playerB",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3,
          "seatIndependent": true,
          "phase": "END_TURN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "attack",
          "turn": 5,
          "player": "playerD",
          "attacker": "Balduvian Bears",
          "defender": 2,
          "expectedLegal": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPlayerState",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "targetPlayer": 0,
          "property": "hasLeftGame",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true,
          "phase": "END_TURN"
        },
        {
          "op": "setStopAt",
          "turn": 5,
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
  ],
  "turnOrder": [
    0,
    3,
    2,
    1
  ]
});
