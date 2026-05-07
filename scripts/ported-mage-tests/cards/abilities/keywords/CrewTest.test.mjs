import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/CrewTest.java",
  "tests": [
    {
      "name": "crewBasicTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cultivator's Caravan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Crew 3"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion^Silvercoat Lion"
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
          "op": "assertTappedCount",
          "name": "Silvercoat Lion",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cultivator's Caravan",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "unsupported",
          "source": "assertType(caravan, CardType.CREATURE, SubType.VEHICLE)"
        }
      ]
    },
    {
      "name": "testThatBouncingACrewedVehicleWillUncrewIt",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Smuggler's Copter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Speedway Fanatic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Evacuation",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Crew 1"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Speedway Fanatic"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Evacuation"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Smuggler's Copter"
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
          "source": "assertNotType(copter, CardType.CREATURE)"
        }
      ]
    },
    {
      "name": "testGiantOx",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Giant Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Colossal Plow",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Crew"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Giant Ox"
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
          "source": "assertTapped(ox, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(plow, CardType.CREATURE, true)"
        }
      ]
    },
    {
      "name": "testGrantedAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kotori, Pilot Prodigy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Irontread Crusher",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Crew 2"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Kotori, Pilot Prodigy"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Irontread Crusher"
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
          "op": "unsupported",
          "source": "assertTapped(kotori, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(crusher, CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(crusher, CardType.CREATURE, SubType.VEHICLE)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Irontread Crusher",
          "ability": "Lifelink",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Irontread Crusher",
          "ability": "Vigilance",
          "expected": true
        }
      ]
    },
    {
      "name": "testHotshotMechanic",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hotshot Mechanic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Aradara Express",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Crew 4"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hotshot Mechanic"
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
          "source": "assertTapped(mechanic, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(express, CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(express, CardType.CREATURE, SubType.VEHICLE)"
        }
      ]
    },
    {
      "name": "testHeartOfKiran",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jace Beleren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Heart of Kiran",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Crew 3"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Jace Beleren"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Jace Beleren",
          "counter": "LOYALTY",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertType(heart, CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(heart, CardType.CREATURE, SubType.VEHICLE)"
        }
      ]
    }
  ]
});
