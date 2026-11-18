import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/GainAbilityDependenciesTest.java",
  "tests": [
    {
      "name": "test_GenerationByFilters",
      "operations": [
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Ability Grant Fixture",
          "count": 1,
          "fixtureSource": true,
          "typeLine": "Enchantment",
          "oracleText": "Creatures have haste.\nHumans have flying.\nHumans and Orcs have vigilance.\nHuman or Orc creatures named Grizzly Bears have trample.\nCreatures that aren't Humans or Orcs have reach."
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Unnatural Selection",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {"zone": "battlefield", "player": 0, "name": "Grizzly Bears"},
          "count": 1,
          "checks": [
            {"property": "ability", "kind": "static", "value": "haste", "expected": true},
            {"property": "ability", "kind": "static", "value": "flying", "expected": false},
            {"property": "ability", "kind": "static", "value": "vigilance", "expected": false},
            {"property": "ability", "kind": "static", "value": "trample", "expected": false},
            {"property": "ability", "kind": "static", "value": "reach", "expected": true}
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Human"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Unnatural Selection",
          "ability": "Choose a creature type other than Wall",
          "target": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {"zone": "battlefield", "player": 0, "name": "Grizzly Bears"},
          "count": 1,
          "checks": [
            {"property": "subtype", "value": "Human", "expected": true},
            {"property": "ability", "kind": "static", "value": "haste", "expected": true},
            {"property": "ability", "kind": "static", "value": "flying", "expected": true},
            {"property": "ability", "kind": "static", "value": "vigilance", "expected": true},
            {"property": "ability", "kind": "static", "value": "trample", "expected": true},
            {"property": "ability", "kind": "static", "value": "reach", "expected": false}
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Unnatural Selection",
          "ability": "Choose a creature type other than Wall",
          "target": "Runeclaw Bear"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {"zone": "battlefield", "player": 0, "name": "Runeclaw Bear"},
          "count": 1,
          "checks": [
            {"property": "subtype", "value": "Orc", "expected": true},
            {"property": "ability", "kind": "static", "value": "haste", "expected": true},
            {"property": "ability", "kind": "static", "value": "flying", "expected": false},
            {"property": "ability", "kind": "static", "value": "vigilance", "expected": true},
            {"property": "ability", "kind": "static", "value": "trample", "expected": false},
            {"property": "ability", "kind": "static", "value": "reach", "expected": false}
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Unnatural Selection",
          "ability": "Choose a creature type other than Wall",
          "target": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {"zone": "battlefield", "player": 0, "name": "Grizzly Bears"},
          "count": 1,
          "checks": [
            {"property": "subtype", "value": "Orc", "expected": true},
            {"property": "ability", "kind": "static", "value": "haste", "expected": true},
            {"property": "ability", "kind": "static", "value": "flying", "expected": false},
            {"property": "ability", "kind": "static", "value": "vigilance", "expected": true},
            {"property": "ability", "kind": "static", "value": "trample", "expected": true},
            {"property": "ability", "kind": "static", "value": "reach", "expected": false}
          ]
        },
        {
          "op": "assertAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "reach",
          "expected": true
        }
      ]
    },
    {
      "name": "test_SliverGain",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Amorphous Axe",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elephant Ambush",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tempered Sliver",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elephant Ambush"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elephant Token",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {3}",
          "target": "Elephant Token"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Elephant Token",
          "defender": 1
        },
        {
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Elephant Token",
          "counter": "P1P1",
          "count": 1
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
        }
      ]
    }
  ]
});
