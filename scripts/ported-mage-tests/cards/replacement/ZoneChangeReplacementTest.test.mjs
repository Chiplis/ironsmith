import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/ZoneChangeReplacementTest.java",
  "tests": [
    {
      "name": "testFromLibraryZoneChange",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Darksteel Colossus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tome Scour",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tome Scour",
          "target": 0
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
          "name": "Darksteel Colossus",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 5
        }
      ]
    },
    {
      "name": "testFromHandZoneChange",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Progenitus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Distress",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Distress",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Progenitus"
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
          "name": "Progenitus",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Progenitus",
          "count": 0
        }
      ]
    },
    {
      "name": "checkBridgeDoesWork",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Diabolic Edict",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bridge from Below",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Diabolic Edict",
          "target": 0
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testDoesntTriggerDiesTriggeredAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Progenitus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Diabolic Edict",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bridge from Below",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Diabolic Edict",
          "target": 0
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
          "name": "Progenitus",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Progenitus",
          "count": 1
        }
      ]
    },
    {
      "name": "testHumilityDeactivatesReplacementEffectAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Progenitus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Diabolic Edict",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Diabolic Edict",
          "target": 0
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
          "name": "Progenitus",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Progenitus",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 2
        }
      ]
    },
    {
      "name": "testHumilityAndKumano",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kumano's Pupils",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aggressive Urge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aggressive Urge",
          "target": "Kumano's Pupils"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Kumano's Pupils",
          "attacker": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Aggressive Urge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kumano's Pupils",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kumano's Pupils",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testCreatureGetsExiledByKumano",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kumano's Pupils",
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
          "zone": "HAND",
          "player": 0,
          "name": "Terminate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
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
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Kumano's Pupils",
          "attacker": "Pillarfield Ox"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Terminate",
          "target": "Pillarfield Ox"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Terminate",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 0
        }
      ]
    },
    {
      "name": "testPermanentNewInstanceAndKumano",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kumano's Pupils",
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
          "zone": "HAND",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Terminate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Pillarfield Ox",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Kumano's Pupils",
          "attacker": "Pillarfield Ox"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Unsummon",
          "target": "Pillarfield Ox"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Pillarfield Ox"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Terminate",
          "target": "Pillarfield Ox"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kumano's Pupils",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kumano's Pupils",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Terminate",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        }
      ]
    },
    {
      "name": "testCounterAndMoveToLibrary",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Legacy Weapon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mana Drain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Legacy Weapon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Legacy Weapon"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mana Drain",
          "target": "Legacy Weapon"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Legacy Weapon"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Legacy Weapon",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Legacy Weapon",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Legacy Weapon",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Mana Drain",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Legacy Weapon",
          "count": 1
        }
      ]
    },
    {
      "name": "testWhipOfErebos",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Whip of Erebos",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{2}{B}{B}, {T}: Return target creature",
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Terror",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testWhipOfErebosTransformPlaneswalker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Whip of Erebos",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Jace, Vryn's Prodigy",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{2}{B}{B}, {T}: Return target creature",
          "target": "Jace, Vryn's Prodigy"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "ability": "{T}:"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "+1: Up to one target creature gets -2/-0 until your next turn"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Jace, Vryn's Prodigy",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Jace, Telepath Unbound",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Jace, Telepath Unbound",
          "counter": "LOYALTY",
          "count": 6
        }
      ]
    },
    {
      "name": "testAnafenzaExileInCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Anafenza, the Foremost",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Skyraker Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Skyraker Giant",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Anafenza, the Foremost",
          "attacker": "Skyraker Giant"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Skyraker Giant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Anafenza, the Foremost",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Anafenza, the Foremost",
          "count": 1
        }
      ]
    }
  ]
});
