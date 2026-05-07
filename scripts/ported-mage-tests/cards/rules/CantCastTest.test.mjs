import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/CantCastTest.java",
  "tests": [
    {
      "name": "testVoidWinnowerEvenSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Void Winnower",
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
          "zone": "HAND",
          "player": 0,
          "name": "Jayemdae Tome",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Jayemdae Tome"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Jayemdae Tome\")) { Assert.fail(\"must not have throw error about bad targets, but got:\\n\" + e.getMessage()); } }"
        }
      ]
    },
    {
      "name": "testVoidWinnowerEvenSpellWithX",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blaze",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blaze",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Blaze$targetPlayer=PlayerA\")) { Assert.fail(\"must not have throw error about bad targets, but got:\\n\" + e.getMessage()); } } assertHandCount(playerA, \"Blaze\", 1)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testVoidWinnowerUnevenSpellWithX",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blaze",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blaze",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=4"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Blaze",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blaze",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testVoidWinnowerWithMorph",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pine Walker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pine Walker using Morph"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Pine Walker\")) { Assert.fail(\"must not have throw error about bad targets, but got:\\n\" + e.getMessage()); } } assertPermanentCount(playerA, EmptyNames.FACE_DOWN_CREATURE.getTestCommand(), 0)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Pine Walker",
          "count": 1
        }
      ]
    },
    {
      "name": "testVoidWinnowerZero",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mox Opal",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mox Opal"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Mox Opal\")) { Assert.fail(\"must not have throw error about bad targets, but got:\\n\" + e.getMessage()); } } assertHandCount(playerA, \"Mox Opal\", 1)"
        }
      ]
    },
    {
      "name": "testPanic",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Panic",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroan Conscriptor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Panic",
          "expected": false
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Akroan Conscriptor",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Panic",
          "target": "Silvercoat Lion"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "label": "Cast Panic",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Panic",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Panic",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Panic",
          "count": 1
        }
      ]
    },
    {
      "name": "testEtherswornCanonist",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ethersworn Canonist",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Aether Vial",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Abrupt Decay",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Llanowar Elves"
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Ethersworn Canonist"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Abrupt Decay",
          "target": "Ethersworn Canonist"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Abrupt Decay$target=Ethersworn Canonist\")) { Assert.fail(\"must not have throw error about bad targets, but got:\\n\" + e.getMessage()); } } assertCounterCount(playerA, \"Aether Vial\", CounterType.CHARGE, 2)"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ethersworn Canonist",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Abrupt Decay",
          "count": 1
        }
      ]
    },
    {
      "name": "testAlhammarret",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alhammarret, High Arbiter",
          "count": 4
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
          "player": 1,
          "name": "Damnation",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alhammarret, High Arbiter"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Damnation"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "label": "Cast Damnation",
          "expected": false
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
          "player": 0,
          "name": "Alhammarret, High Arbiter",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Damnation",
          "count": 1
        }
      ]
    }
  ]
});
