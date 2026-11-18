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
          "op": "addCard",
          "player": 0,
          "name": "Lore Counter Fixture",
          "zone": "BATTLEFIELD",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Put three lore counters on target Saga you control."
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hidetsugu Consumes All",
          "counter": "LORE",
          "count": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Hidetsugu Consumes All",
          "abilityKind": "Triggered",
          "count": 1,
          "text": "Exile all graveyards"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Lore Counter Fixture",
          "ability": "Put three lore counters",
          "target": "Hidetsugu Consumes All"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Lore Counter Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Hidetsugu Consumes All",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Hidetsugu Consumes All",
          "abilityKind": "Triggered",
          "count": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Hidetsugu Consumes All",
          "abilityKind": "Triggered",
          "count": 1,
          "text": "return it to the battlefield transformed",
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hidetsugu Consumes All",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "addObjectCounters",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "power": 5,
          "toughness": 5,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "ability": "Double strike",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "ability": "Lifelink",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Vessel of the All-Consuming",
          "defender": 1
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 1,
          "life": 15,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "life": 25,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "count": 1,
          "source": "Vessel of the All-Consuming",
          "abilityKind": "Triggered"
        },
        {
          "op": "assertResult",
          "player": 1,
          "result": "ONGOING"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 0,
          "name": "Vessel of the All-Consuming",
          "power": 6,
          "toughness": 6,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 1,
          "life": 9,
          "seatIndependent": true
        },
        {
          "op": "assertLife",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "life": 31,
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 2,
          "source": "Vessel of the All-Consuming",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "resolutions": 2
        },
        {
          "op": "assertResult",
          "player": 1,
          "result": "LOST"
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "WON"
        }
      ]
    }
  ]
});
