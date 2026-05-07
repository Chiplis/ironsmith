import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/flip/SasayaOrochiAscendantTest.java",
  "tests": [
    {
      "name": "test_SasayasEssence_SimpleManaCalculation",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sasaya, Orochi Ascendant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fountain of Renewal",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sasaya's Essence",
          "count": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Reveal your hand: If you have seven or more land cards in your hand, flip"
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
          "name": "Sasaya's Essence",
          "count": 1
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
          "source": "assertManaOptions(\"{G}{G}{G}\" + \"{G}{G}\" + \"{G}{G}\" + \"{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "testSasayasEssence",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sasaya, Orochi Ascendant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Reveal your hand: If you have seven or more land cards in your hand, flip"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Upwelling"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sasaya's Essence",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.GREEN, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}{G}{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "testSasayasEssence2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brushland",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sasaya, Orochi Ascendant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Reveal your hand: If you have seven or more land cards in your hand, flip"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Upwelling"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sasaya's Essence",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.GREEN, 2)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}{W}{W}{W}\", manaOptions)"
        }
      ]
    },
    {
      "name": "testSasayasEssence3",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mossfire Valley",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sasaya, Orochi Ascendant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Reveal your hand: If you have seven or more land cards in your hand, flip"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Upwelling"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sasaya's Essence",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertManaPool(playerA, ManaType.GREEN, 2)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}{R}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{G}{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        }
      ]
    }
  ]
});
