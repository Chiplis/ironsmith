import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/rtr/VraskaTheUnseenTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vraska the Unseen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1:"
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": "5 + 1",
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": "Vraska the Unseen",
          "expectedLegal": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "source": "Vraska the Unseen",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": 4,
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 2,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Vraska the Unseen",
          "count": 1,
          "player": 0,
          "seatIndependent": true,
          "turn": 2,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": "5 + 1 - 2",
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": "5 + 1 - 2",
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": "Vraska the Unseen",
          "expectedLegal": true
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": "5 + 1 - 2 * 2",
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
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
      "name": "test_OnlyCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vraska the Unseen",
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
          "player": 1,
          "name": "Cinder Pyromancer",
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
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": "5 + 1",
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Cinder Pyromancer",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": "Vraska the Unseen",
          "expectedLegal": true
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "source": "Vraska the Unseen",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": 4,
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 2,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Vraska the Unseen",
          "count": 1,
          "player": 0,
          "seatIndependent": true,
          "turn": 2,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": "5 + 1 - 2",
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: {this} deals",
          "target": "Vraska the Unseen"
        },
        {
          "op": "assertCounterCount",
          "name": "Vraska the Unseen",
          "count": "5 + 1 - 2 - 1",
          "player": 0,
          "counter": "LOYALTY",
          "seatIndependent": true,
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "END_TURN",
          "player": 1,
          "name": "Cinder Pyromancer",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
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
