import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/EnchantedPermanentLKITest.java",
  "tests": [
    {
      "name": "testAuraRemoved",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skyrider Trainee",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Winter's Rest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Disperse",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Winter's Rest",
          "target": "Skyrider Trainee"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Skyrider Trainee",
          "ability": "FlyingAbility.class",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Disperse",
          "target": "Winter's Rest"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Disperse",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(skyrider, true)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Winter's Rest",
          "count": 1
        }
      ]
    },
    {
      "name": "testAuraRemovedDamaging",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skyrider Trainee",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bind the Monster",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Disperse",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bind the Monster",
          "target": "Skyrider Trainee"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Skyrider Trainee",
          "ability": "FlyingAbility.class",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Disperse",
          "target": "Bind the Monster"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Disperse",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(skyrider, true)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Bind the Monster",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "testAuraFlickered",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skyrider Trainee",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dreadful Apathy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Flicker of Fate",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dreadful Apathy",
          "target": "Skyrider Trainee"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Skyrider Trainee",
          "ability": "FlyingAbility.class",
          "expected": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "{2}{W}: Exile enchanted creature."
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Flicker of Fate",
          "target": "Dreadful Apathy"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Thran Golem"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flicker of Fate",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Skyrider Trainee",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Thran Golem",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "testAuraMoved",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skyrider Trainee",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dreadful Apathy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aura Finesse",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dreadful Apathy",
          "target": "Skyrider Trainee"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Dreadful Apathy",
          "objectTarget": "Skyrider Trainee"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "attachment": "Dreadful Apathy",
          "target": "Skyrider Trainee",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Skyrider Trainee",
          "ability": "Flying",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "source": "Dreadful Apathy",
          "ability": "Exile enchanted creature"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "count": 1,
          "source": "Dreadful Apathy",
          "abilityKind": "Activated"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Aura Finesse",
          "target": "Dreadful Apathy^Thran Golem"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "count": 1,
          "source": "Aura Finesse",
          "objectTarget": "Thran Golem",
          "onTop": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "count": 1,
          "source": "Aura Finesse",
          "objectTarget": "Dreadful Apathy"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "count": 1,
          "source": "Dreadful Apathy",
          "abilityKind": "Activated",
          "onTop": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "seatIndependent": true,
          "name": "Aura Finesse",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "seatIndependent": true,
          "count": 1
        },
        {
          "op": "assertAttachedTo",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "attachment": "Dreadful Apathy",
          "target": "Thran Golem",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Skyrider Trainee",
          "ability": "Flying",
          "expected": false,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Thran Golem",
          "ability": "Trample",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Thran Golem",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Skyrider Trainee",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Dreadful Apathy",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Aura Finesse",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Skyrider Trainee",
          "ability": "Flying",
          "expected": false,
          "seatIndependent": true
        }
      ]
    }
  ]
});
