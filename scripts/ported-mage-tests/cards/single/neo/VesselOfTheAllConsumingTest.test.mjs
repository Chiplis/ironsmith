import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/neo/VesselOfTheAllConsumingTest.java",
  "tests": [
    {
      "name": "doubleStrike",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hidetsugu Consumes All",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "True Conviction",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, hidetsugu, CounterType.LORE, 3)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, vessel, CounterType.P1P1, 2)"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Vessel of the All-Consuming"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Vessel of the All-Consuming",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"post damage\", 3, PhaseStep.COMBAT_DAMAGE, playerB, 20 - 5 - 6)"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"post damage\", 3, PhaseStep.COMBAT_DAMAGE, playerA, \"Whenever {this} deals damage to a player\", 1)"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertLostTheGame(playerB)"
        }
      ]
    }
  ]
});
