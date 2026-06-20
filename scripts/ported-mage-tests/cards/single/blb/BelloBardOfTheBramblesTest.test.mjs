import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/blb/BelloBardOfTheBramblesTest.java",
  "tests": [
    {
      "name": "testBello",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "City on Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Dynamo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bello, Bard of the Brambles",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bello, Bard of the Brambles"
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
          "source": "assertType(cityOnFire, CardType.CREATURE, SubType.ELEMENTAL)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "City on Fire",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "City on Fire",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "City on Fire",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "City on Fire",
          "ability": 1,
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "assertType(thranDynamo, CardType.CREATURE, SubType.ELEMENTAL)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Thran Dynamo",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Thran Dynamo",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Thran Dynamo",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Thran Dynamo",
          "ability": 1,
          "expected": true
        }
      ]
    },
    {
      "name": "testBelloTypeAddition",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "City on Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Dynamo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bello, Bard of the Brambles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bello, Bard of the Brambles"
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
          "source": "assertType(ashaya, CardType.CREATURE, SubType.ELEMENTAL)"
        },
        {
          "op": "unsupported",
          "source": "assertType(ashaya, CardType.LAND, SubType.FOREST)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "new GreenManaAbility()",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "Indestructible",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": 1,
          "expected": false
        },
        {
          "op": "unsupported",
          "source": "assertType(cityOnFire, CardType.LAND, SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertType(cityOnFire, CardType.CREATURE, SubType.ELEMENTAL)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "City on Fire",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "City on Fire",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "City on Fire",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "City on Fire",
          "ability": 1,
          "expected": true
        }
      ]
    },
    {
      "name": "testBelloEquipment",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "City on Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Dynamo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bello, Bard of the Brambles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tangleweave Armor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bello, Bard of the Brambles"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Tangleweave Armor",
          "ability": "Indestructible",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Tangleweave Armor",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Tangleweave Armor",
          "ability": 1,
          "expected": false
        }
      ]
    },
    {
      "name": "testBelloAura",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "City on Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Dynamo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bello, Bard of the Brambles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bear Umbra",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bear Umbra",
          "target": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Bello, Bard of the Brambles"
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
          "source": "assertAttachedTo(playerA, bearUmbra, bear, true)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Bear Umbra",
          "ability": "Indestructible",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Bear Umbra",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Bear Umbra",
          "ability": 1,
          "expected": false
        }
      ]
    },
    {
      "name": "testBelloLessThanFourCmcEnchantment",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "City on Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Dynamo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bello, Bard of the Brambles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Aggravated Assault",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bello, Bard of the Brambles"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Aggravated Assault",
          "ability": "Indestructible",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Aggravated Assault",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Aggravated Assault",
          "ability": 1,
          "expected": false
        }
      ]
    },
    {
      "name": "testBelloLessThanFourCmcArtifact",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "City on Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thran Dynamo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bello, Bard of the Brambles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abandoned Sarcophagus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bello, Bard of the Brambles"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Abandoned Sarcophagus",
          "ability": "Indestructible",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Abandoned Sarcophagus",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Abandoned Sarcophagus",
          "ability": 1,
          "expected": false
        }
      ]
    }
  ]
});
