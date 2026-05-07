import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/AddManaOfAnyTypeProducedTest.java",
  "tests": [
    {
      "name": "testRiverOfTears",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vorinclex, Voice of Hunger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "River of Tears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vedalken Mastermind",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {U}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vedalken Mastermind"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Vedalken Mastermind",
          "count": 1
        }
      ]
    },
    {
      "name": "testVorinclexVoiceofHungerRiverOfTearsManaMultiplier",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "River of Tears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vorinclex, Voice of Hunger",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "River of Tears"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {U}",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.BLUE, 2)"
        }
      ]
    },
    {
      "name": "testGemstoneCavern",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vorinclex, Voice of Hunger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Gemstone Caverns",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Swamp"
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "White"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gemstone Caverns",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Gemstone Caverns",
          "counter": "LUCK",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Gemstone Caverns\", true)"
        }
      ]
    },
    {
      "name": "testCastWithGemstoneCavern",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Vorinclex, Voice of Hunger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Gemstone Caverns",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Swamp"
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {C}",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "White"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Vorinclex, Voice of Hunger"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gemstone Caverns",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Gemstone Caverns",
          "counter": "LUCK",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Vorinclex, Voice of Hunger",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Gemstone Caverns\", true)"
        }
      ]
    },
    {
      "name": "testChooseColorRed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kinnan, Bonder Prodigy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gruul Signet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Massive Raid",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}, {T}: Add {R}{G}",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Massive Raid",
          "target": 1
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
          "op": "unsupported",
          "source": "assertTapped(signet, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Wastes\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, kinnan, 0)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        }
      ]
    },
    {
      "name": "testChooseColorGreen",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kinnan, Bonder Prodigy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gruul Signet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hail Storm",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}, {T}: Add {R}{G}",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hail Storm"
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
          "op": "unsupported",
          "source": "assertTapped(signet, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Wastes\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, kinnan, 1)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 19
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
