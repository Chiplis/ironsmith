#!/usr/bin/env python3
import ast
import json
import os
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT = ROOT / "scripts" / "cards"
OUTPUT_ROOT = ROOT / "scripts" / "ported-mage-tests" / "cards"
RUNNER = ROOT / "scripts" / "mage-port-runner.mjs"

AI_MARKERS = (
    "AITest",
    "AITests",
    "AiTest",
    "ComputerAction",
    "aiPlayStep",
    "aiPlayPriority",
    "MonteCarloAI",
    "MCTS",
)

IGNORED_CALLS = {
    "setStrictChooseMode",
    "skipInitShuffling",
    "setStopAt",
    "execute",
    "assertAllCommandsUsed",
}


CUSTOM_ABILITY_ORACLE_TEXT = {
    'new SimpleActivatedAbility(Zone.ALL, new AddConditionalManaEffect(Mana.RedMana(10), new InstantOrSorcerySpellManaBuilder()), new ManaCostsImpl<>(""))':
        "{0}: Add {R}{R}{R}{R}{R}{R}{R}{R}{R}{R}. Spend this mana only to cast instant or sorcery spells.",
    'new SimpleActivatedAbility( new GainLifeEffect(PartyCount.instance), new ManaCostsImpl<>("{0}") )':
        "{0}: You gain life equal to the number of creatures in your party.",
    'new SimpleActivatedAbility(new GainLifeEffect(PartyCount.instance), new ManaCostsImpl<>("{0}"))':
        "{0}: You gain life equal to the number of creatures in your party.",
}

BOOST_DURATIONS = {
    "EndOfTurn": "end of turn",
    "UntilYourNextTurn": "your next turn",
    "UntilEndOfYourNextTurn": "end of your next turn",
    "UntilEndCombatOfYourNextTurn": "end of combat on your next turn",
    "UntilYourNextEndStep": "your next end step",
    "UntilYourNextUpkeepStep": "your next upkeep",
}


def custom_fixture_text(raw):
    legacy_text = CUSTOM_ABILITY_ORACLE_TEXT.get(re.sub(r"\s+", " ", raw))
    if legacy_text is not None:
        return legacy_text
    compact_raw = re.sub(r'\s+', '', raw)
    fixed = {
        'newAnyColorManaAbility()': '{T}: Add one mana of any color.',
        'newSimpleActivatedAbility(newVentureIntoTheDungeonEffect(),newGenericManaCost(0))':
            '{0}: Venture into the dungeon.',
        'newSimpleActivatedAbility(newTapAllEffect(newFilterCreaturePermanent(SubType.BEAR,"bears")),newManaCostsImpl<>(""))':
            '{0}: Tap all Bears.',
        'newSpellCastControllerTriggeredAbility(newCopyTargetStackObjectEffect(true),StaticFilters.FILTER_SPELL,false,SetTargetPointer.SPELL)':
            'Whenever you cast a spell, copy it. You may choose new targets for the copy.',
        'newSimpleActivatedAbility(newUntapAllControllerEffect(newFilterCreaturePermanent()),newManaCostsImpl<>(""))':
            '{0}: Untap each creature you control.',
        'newSimpleActivatedAbility(newLoseGameSourceControllerEffect(),newManaCostsImpl<>(""))':
            '{0}: You lose the game.',
        'newSimpleActivatedAbility(newWinGameSourceControllerEffect(),newManaCostsImpl<>(""))':
            '{0}: You win the game.',
        'newSimpleStaticAbility(newAbilitiesCostReductionControllerEffect(EquipAbility.class,"equip"))':
            'Equip abilities you activate cost {1} less to activate.',
    }
    if compact_raw in fixed:
        return fixed[compact_raw]
    crime = re.fullmatch(r'newCommittedCrimeTriggeredAbility\(newGainLifeEffect\((\d+)\),false\)', compact_raw)
    if crime:
        return f'Whenever you commit a crime, you gain {crime[1]} life.'
    combat = re.fullmatch(r'newBeginningOfCombatTriggeredAbility\(newGainLifeEffect\((\d+)\)\)', compact_raw)
    if combat:
        return f'At the beginning of combat on your turn, you gain {combat[1]} life.'
    life_cost = re.fullmatch(r'newSimpleActivatedAbility\(newGainLifeEffect\((\d+)\),newPayVariableLifeCost\(\)\)', compact_raw)
    if life_cost:
        return f'Pay X life: You gain {life_cost[1]} life.'
    equip_cost = re.fullmatch(
        r'newSimpleStaticAbility\(newConditionalCostModificationEffect\('
        r'newAbilitiesCostReductionControllerEffect\(EquipAbility\.class,"equip"\),'
        r'(MyTurn|NotMyTurn)Condition\.instance,""\)\)', compact_raw,
    )
    if equip_cost:
        during = "During your turn" if equip_cost[1] == "MyTurn" else "During turns other than yours"
        return f"{during}, equip abilities you activate cost {{1}} less to activate."
    if compact_raw == (
        'newSimpleStaticAbility(newConditionalCostModificationEffect('
        'newAbilitiesCostReductionControllerEffect(EquipAbility.class,"equip"),'
        'NotMyTurnCondition.instance,newSpellsCostIncreasingAllEffect(1,newFilterCard(),TargetController.ANY),""))'
    ):
        return ("During turns other than yours, equip abilities you activate cost {1} less to activate.\n"
                "During your turn, spells cost {1} more to cast.")
    prevent_damage = (
        'newPreventAllDamageToAllEffect(Duration.WhileOnBattlefield,StaticFilters.FILTER_PERMANENT)'
    )
    if compact_raw == f"newSimpleStaticAbility({prevent_damage})":
        return "Prevent all damage that would be dealt to all permanents."
    for condition, during in [("MyTurn", "During your turn"), ("NotMyTurn", "During turns other than yours")]:
        if compact_raw == f'newSimpleStaticAbility(newConditionalPreventionEffect({prevent_damage},{condition}Condition.instance,""))':
            return f"{during}, prevent all damage that would be dealt to all permanents."
    for java, oracle in [
        ("DrawCardSourceControllerEffect", "Draw {amount} cards."),
        ("LoseLifeAllPlayersEffect", "Each player loses {amount} life."),
    ]:
        effect = re.fullmatch(
            rf'newSimpleActivatedAbility\(new{java}\((\d+)\),newManaCostsImpl<>\(""\)\)',
            compact_raw,
        )
        if effect:
            amount = effect[1]
            if java == "DrawCardSourceControllerEffect" and amount == "1":
                return "{0}: Draw a card."
            return "{0}: " + oracle.format(amount=amount)
    boost = re.fullmatch(
        r'newSimpleActivatedAbility\(newBoostSourceEffect\((-?\d+),(-?\d+),Duration\.(\w+)\),newManaCostsImpl<>\("\{0\}"\)\)',
        compact_raw,
    )
    if boost and boost[3] in BOOST_DURATIONS:
        size = f'{int(boost[1]):+d}/{int(boost[2]):+d}'
        if boost[3] == "UntilYourNextUpkeepStep":
            return f'{{0}}: Until your next upkeep, this creature gets {size}.'
        return f'{{0}}: This creature gets {size} until {BOOST_DURATIONS[boost[3]]}.'
    return None


def main():
    if OUTPUT_ROOT.exists():
        shutil.rmtree(OUTPUT_ROOT)
    OUTPUT_ROOT.mkdir(parents=True, exist_ok=True)

    generated = 0
    skipped_ai = 0
    skipped_empty = 0
    total_tests = 0
    unsupported_statements = 0

    for source in sorted(SOURCE_ROOT.rglob("*.java")):
        text = source.read_text(encoding="utf-8", errors="ignore")
        if any(marker in source.name or marker in text for marker in AI_MARKERS):
            skipped_ai += 1
            continue
        tests = extract_tests(text)
        if not tests:
            skipped_empty += 1
            continue
        stripped_text = strip_comments(text)
        file_constants = extract_constants(stripped_text, {})
        helper_methods = extract_helper_methods(stripped_text)
        multiplayer_setup = source_multiplayer_setup(stripped_text)

        rel = source.relative_to(SOURCE_ROOT)
        out = OUTPUT_ROOT / rel.with_suffix(".test.mjs")
        out.parent.mkdir(parents=True, exist_ok=True)
        test_specs = []
        for name, body, ignore_reason in tests:
            operations, unsupported = translate_body(body, file_constants, helper_methods)
            unsupported_statements += unsupported
            spec = {"name": name, "operations": operations, **multiplayer_setup}
            if ignore_reason is not None:
                spec["skip"] = ignore_reason
            test_specs.append(spec)
            total_tests += 1

        import_path = os.path.relpath(RUNNER, out.parent).replace(os.sep, "/")
        source_path = f"scripts/cards/{rel.as_posix()}"
        contents = (
            f"import {{ registerPortedMageTests }} from \"{import_path}\";\n\n"
            f"registerPortedMageTests({json.dumps({'sourcePath': source_path, 'tests': test_specs}, indent=2)});\n"
        )
        out.write_text(contents, encoding="utf-8")
        generated += 1

    manifest = {
        "generatedFiles": generated,
        "generatedTests": total_tests,
        "skippedAiFiles": skipped_ai,
        "skippedEmptyFiles": skipped_empty,
        "unsupportedStatements": unsupported_statements,
        "outputRoot": str(OUTPUT_ROOT),
    }
    (OUTPUT_ROOT.parent / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))


def source_multiplayer_setup(text):
    base = re.search(r"\bextends (CardTestMultiPlayerBase(?:WithRangeAll)?|CardTestCommander4Players)\b", text)
    if not base:
        return {}
    setup = {"turnOrder": [0, 3, 2, 1]}
    override = re.search(
        r"new FreeForAll\(MultiplayerAttackOption\.\w+,\s*RangeOfInfluence\.(\w+),[^;]*,\s*(\d+),\s*\d+\)",
        text,
    )
    if override:
        setup["startingLife"] = int(override[2])
    if (override and override[1] == "ONE") or (not override and base[1] == "CardTestMultiPlayerBase"):
        setup["rangeOfInfluence"] = 1
    return setup


def extract_tests(text):
    tests = []
    for match in re.finditer(r"@Test\b", text):
        method = re.search(r"public\s+void\s+(\w+)\s*\([^)]*\)\s*\{", text[match.end():])
        if not method:
            continue
        name = method.group(1)
        open_brace = match.end() + method.end() - 1
        close_brace = matching_brace(text, open_brace)
        if close_brace is None:
            continue
        previous_lines = "\n".join(text[: match.start()].splitlines()[-4:])
        annotation_text = previous_lines + "\n" + text[match.start() : match.end() + method.start()]
        tests.append((name, text[open_brace + 1 : close_brace], extract_ignore_reason(annotation_text)))
    return tests


def extract_ignore_reason(annotation_text):
    annotation_text = strip_comments(annotation_text)
    if "@Ignore" not in annotation_text:
        return None
    match = re.search(r'@Ignore\s*(?:\(\s*"((?:\\.|[^"])*)"\s*\))?', annotation_text)
    if not match:
        return "upstream @Ignore"
    if not match.group(1):
        return "upstream @Ignore"
    try:
        return "upstream @Ignore: " + json.loads(f'"{match.group(1)}"')
    except json.JSONDecodeError:
        return "upstream @Ignore: " + match.group(1)


def matching_brace(text, start):
    depth = 0
    in_string = False
    escaped = False
    for index in range(start, len(text)):
        char = text[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
    return None


def extract_constants(text, base_constants):
    constants = dict(base_constants)
    declaration = re.compile(
        r'(?:(?:public|private|protected)\s+)?(?:(?:static|final)\s+)*(?:String|int|boolean|CounterType)\s+(\w+)\s*=\s*([^;]+);',
        re.S,
    )
    for assignment in declaration.finditer(text):
        constants[assignment.group(1)] = parse_value(assignment.group(2), constants)
    return constants


def extract_helper_methods(text):
    """Map of private void helper methods (e.g. initLibrary) to their overloads.

    MAGE tests frequently factor shared setup into class-level helpers; inline
    them so the ported operations include the setup instead of dropping it.
    Each entry maps name -> list of (param_names, is_varargs, body).
    """
    helpers = {}
    for match in re.finditer(
        r"(?:private|protected|public)?\s*(?:static\s+)?void\s+(\w+)\s*\(([^)]*)\)\s*\{",
        text,
    ):
        name = match.group(1)
        preceding = "\n".join(text[: match.start()].splitlines()[-4:])
        if "@Test" in preceding:
            continue
        raw_params = match.group(2).strip()
        param_names = []
        is_varargs = False
        valid = True
        if raw_params:
            for param in raw_params.split(","):
                parts = param.strip().split()
                if len(parts) < 2:
                    valid = False
                    break
                if "..." in parts[-2]:
                    is_varargs = True
                param_names.append(parts[-1])
        if not valid:
            continue
        open_brace = text.index("{", match.end() - 1)
        close_brace = matching_brace(text, open_brace)
        if close_brace is None:
            continue
        helpers.setdefault(name, []).append(
            (param_names, is_varargs, text[open_brace + 1 : close_brace])
        )
    return helpers


def bind_helper_body(overloads, args):
    """Pick the best-fitting overload and substitute parameters.

    Java overload resolution is type-driven; approximate it by preferring the
    overload with the most named parameters and rejecting bindings where a
    player-like parameter is bound to a non-player argument.
    """

    def looks_like_player(value):
        return bool(re.search(r"\bplayer[A-D]?\b", value, re.IGNORECASE))

    ranked = sorted(overloads, key=lambda entry: len(entry[0]), reverse=True)
    for param_names, is_varargs, body in ranked:
        if is_varargs:
            if len(args) < len(param_names) - 1:
                continue
            bindings = dict(zip(param_names[:-1], args))
            bindings[param_names[-1]] = ", ".join(args[len(param_names) - 1 :])
        else:
            if len(args) != len(param_names):
                continue
            bindings = dict(zip(param_names, args))
        mismatched = any(
            "player" in name.lower() and value and not looks_like_player(value)
            for name, value in bindings.items()
        )
        if mismatched:
            continue
        if not bindings:
            return body
        pattern = re.compile(
            r'"(?:\\.|[^"\\])*"|\b(' + "|".join(re.escape(name) for name in bindings) + r")\b"
        )
        return pattern.sub(lambda m: bindings[m[1]] if m[1] is not None else m[0], body)
    return None


def translate_body(body, file_constants=None, helpers=None, depth=0):
    constants = dict(file_constants or {})
    helpers = helpers or {}
    ability_vars = {}
    operations = []
    unsupported = 0
    for statement in split_statements(strip_comments(body)):
        statement = statement.strip()
        if not statement:
            continue
        selected = constant_if_branch(statement, constants)
        if selected is not None and depth < 5:
            conditional_ops, conditional_unsupported = translate_body(selected, constants, helpers, depth + 1)
            operations.append({"op": "sequence", "operations": conditional_ops})
            unsupported += conditional_unsupported
            continue
        loop = translate_loop(statement, constants) if statement.startswith("for") else None
        if loop:
            loop_ops, rest = loop
            operations.extend(loop_ops)
            tail_ops, tail_unsupported = translate_body(rest, constants, helpers, depth)
            operations.extend(tail_ops)
            unsupported += tail_unsupported
            continue
        exception_block = split_try_statement(statement) if statement.startswith("try") else None
        if exception_block:
            block, rest = exception_block
            operations.append({"op": "unsupported", "source": compact(block)})
            tail_ops, tail_unsupported = translate_body(rest, constants, helpers, depth)
            operations.extend(tail_ops)
            unsupported += 1 + tail_unsupported
            continue
        if re.match(r"(?:if|for|try|catch|while)\b", statement):
            operations.append({"op": "unsupported", "source": compact(statement)})
            unsupported += 1
            continue
        ability_decl = re.match(
            r"(?:Abilities<[^>]+>|List<[^>]+>)\s+(\w+)\s*=\s*"
            r"(?:new\s+(?:AbilitiesImpl|ArrayList)(?:<[^>]*>)?\(\)|Arrays\.asList\((.*)\))$",
            statement,
            re.S,
        )
        if ability_decl:
            ability_vars[ability_decl.group(1)] = []
            if ability_decl.group(2):
                ability_vars[ability_decl.group(1)] = [
                    parse_value(arg, constants) for arg in split_args(ability_decl.group(2))
                ]
            continue
        ability_add = re.match(r"(\w+)\.add\((.+)\)$", statement, re.S)
        if ability_add and ability_add.group(1) in ability_vars:
            ability_vars[ability_add.group(1)].append(parse_value(ability_add.group(2), constants))
            continue
        assignment = re.match(r'(?:(?:final|static)\s+)*(?:String|int|boolean|CounterType)\s+(\w+)\s*=\s*(.+)$', statement)
        if assignment:
            constants[assignment.group(1)] = parse_value(assignment.group(2), constants)
            continue
        adjustment = re.fullmatch(r"(\w+)\s*(\+\+|--|\+=\s*(.+)|-=\s*(.+))", statement)
        if adjustment and type(constants.get(adjustment[1])) is int:
            variable, operator, increase, decrease = adjustment.groups()
            operand = increase if increase is not None else decrease
            amount = 1 if operand is None else parse_value(operand, constants)
            if type(amount) is int:
                constants[variable] += amount if operator.startswith("+") else -amount
                continue
        if statement.startswith(("Assert.", "Assume.", "assert ")):
            operations.append({"op": "unsupported", "source": compact(statement)})
            unsupported += 1
            continue
        if statement.startswith(("logger.", "System.")):
            continue
        clear_library = re.fullmatch(r"(player[A-D])\.getLibrary\(\)\.clear\(\)", statement)
        if clear_library:
            operations.append({"op": "clearZone", "player": parse_value(clear_library[1], constants), "zone": "library"})
            continue
        helper_call = re.match(r"(?:this\.)?(\w+)\((.*)\)$", statement, re.S)
        if helper_call and helper_call.group(1) in helpers and depth < 5:
            # Prefer a direct framework translation when one exists (e.g.
            # assertLibrary) over inlining the class helper's body.
            direct = parse_call(statement)
            direct_op = None
            if direct:
                direct_op = translate_call(
                    direct[0], direct[1], constants, statement, ability_vars
                )
            if direct_op is not None and direct_op.get("op") != "unsupported":
                operations.append(direct_op)
                continue
            call_args = split_args(helper_call.group(2)) if helper_call.group(2).strip() else []
            bound_body = bind_helper_body(helpers[helper_call.group(1)], call_args)
            if bound_body is not None:
                inlined_ops, inlined_unsupported = translate_body(
                    bound_body, constants, helpers, depth + 1
                )
                operations.extend(inlined_ops)
                unsupported += inlined_unsupported
                continue
        call = parse_call(statement)
        if not call:
            continue
        name, args = call
        op = translate_call(name, args, constants, statement, ability_vars)
        if op is None:
            continue
        if op.get("op") == "unsupported":
            unsupported += 1
        operations.append(op)
    return operations, unsupported


def split_try_statement(statement):
    header = re.match(r"try\s*\{", statement)
    if not header:
        return None
    end = matching_brace(statement, header.end() - 1)
    while end is not None:
        clause = re.match(r"\s*(?:catch\s*\([^)]*\)|finally)\s*\{", statement[end + 1:])
        if not clause:
            return statement[:end + 1], statement[end + 1:].strip()
        end = matching_brace(statement, end + clause.end())
    return None


def translate_loop(statement, constants):
    counted = re.match(r"for\s*\(\s*int (\w+)\s*=\s*(\d+)\s*;\s*\1\s*(<=?)\s*([^;]+);\s*\1\+\+\s*\)\s*\{", statement)
    objects = re.match(r"for\s*\(\s*(?:final\s+)?(?:Card|Permanent)\s+(\w+)\s*:\s*(.*?)\)\s*\{", statement, re.S)
    header = counted or objects
    if not header:
        return None
    end = matching_brace(statement, header.end() - 1)
    if end is None:
        return None
    body = statement[header.end():end].strip()
    rest = statement[end + 1:].strip()
    if counted:
        variable, start, comparison, bound = counted.groups()
        start, bound = int(start), parse_value(bound, constants)
        # Only expand direct framework calls. A loop body can otherwise mutate
        # Java locals or transfer control in ways the replay would miss.
        calls = [parse_call(part) for part in split_statements(body)]
        if type(bound) is not int or not calls or any(call is None for call in calls):
            return None
        stop = bound + (comparison == "<=")
        if stop - start > 1000:
            return None
        phases = []
        for index in range(start, stop):
            for name, args in calls:
                op = translate_call(name, args, {**constants, variable: index}, body)
                if not op or op["op"] == "unsupported":
                    return None
                phases.append(op)
        return phases, rest
    result = translate_object_loop(objects[1], objects[2], body, constants)
    return ([result], rest) if result else None


def translate_object_loop(variable, collection, body, constants):
    collection = re.sub(r"\s+", "", collection)
    player_collection = re.fullmatch(r"(player[A-D])\.get(Library|Hand|Graveyard)\(\)\.getCards\(currentGame\)", collection)
    land_collection = re.fullmatch(
        r"currentGame\.getBattlefield\(\)\.getActivePermanents\(StaticFilters\.FILTER_CONTROLLED_PERMANENT_LAND,(player[A-D])\.getId\(\),currentGame\)",
        collection,
    )
    selector = {}
    if player_collection:
        selector = {"zone": player_collection[2].lower(), "player": parse_value(player_collection[1], constants)}
    elif land_collection:
        selector = {"zone": "battlefield", "player": parse_value(land_collection[1], constants), "cardType": "Land"}
    elif collection == "currentGame.getExile().getAllCards(currentGame)":
        selector = {"zone": "exile"}
    elif collection == "currentGame.getBattlefield().getAllActivePermanents()":
        selector = {"zone": "battlefield"}
    else:
        return None

    prefix = re.escape(variable) + r"\."
    named = re.fullmatch(rf"if\s*\(\s*{prefix}getName\(\)\.equals\(([^()]+)\)\s*\)\s*\{{(.*)\}}", body, re.S)
    creature = re.fullmatch(rf"if\s*\(\s*{prefix}isCreature\(currentGame\)\s*\)\s*\{{(.*)\}}", body, re.S)
    if named:
        name = parse_value(named[1], constants)
        if not isinstance(name, str) or (name == named[1].strip() and name not in constants):
            return None
        selector["name"] = name
        body = named[2]
    if creature:
        selector["cardType"] = "Creature"
        body = creature[1]

    checks = []
    for assertion in split_statements(body):
        match = re.fullmatch(r"(?:Assert\.)?assert(True|False|Equals)\((.*)\)", assertion.strip(), re.S)
        if not match:
            return None
        args = split_args(match[2])
        if match[1] == "Equals":
            if len(args) not in (2, 3):
                return None
            expected = parse_value(args[-2], constants)
        else:
            if len(args) not in (1, 2):
                return None
            expected = match[1] == "True"
        predicate = args[-1].strip()
        if predicate.startswith("!") and type(expected) is bool:
            expected, predicate = not expected, predicate[1:].strip()
        if type(expected) is not bool:
            return None
        if re.fullmatch(prefix + r"isFaceDown\(currentGame\)", predicate):
            checks.append({"property": "faceDown", "expected": expected})
        elif re.fullmatch(prefix + r"isBasic\(currentGame\)", predicate):
            checks.append({"property": "supertype", "value": "Basic", "expected": expected})
        elif color := re.fullmatch(prefix + r"getColor\(currentGame\)\.is(White|Blue|Black|Red|Green)\(\)", predicate):
            checks.append({"property": "color", "value": color[1].lower(), "expected": expected})
        elif subtype := re.fullmatch(prefix + r"hasSubtype\(SubType\.(\w+),\s*currentGame\)", predicate):
            checks.append({"property": "subtype", "value": subtype[1], "expected": expected})
        else:
            return None
    return {"op": "assertEachObject", "selector": selector, "checks": checks} if checks else None


def constant_if_branch(statement, constants):
    start = re.match(r"if\s*\((.*)\)\s*\{", statement.split("{", 1)[0] + "{", re.S)
    if not start:
        return None
    condition = constant_boolean(start[1], constants)
    if condition is None:
        return None
    end = matching_brace(statement, start.end() - 1)
    if end is None:
        return None
    yes = statement[start.end():end]
    tail = statement[end + 1:].strip()
    no = ""
    otherwise = re.match(r"else\s*\{", tail)
    if otherwise:
        end = matching_brace(tail, otherwise.end() - 1)
        if end is None:
            return None
        no, tail = tail[otherwise.end():end], tail[end + 1:].strip()
    elif tail.startswith("else"):
        return None
    return (yes if condition else no) + "\n" + tail


def constant_boolean(raw, constants):
    raw = raw.strip()
    for operator in ("||", "&&"):
        segments = split_top_level_operators(raw, operator)
        if len(segments) > 1:
            parts = [constant_boolean(part, constants) for part in segments]
            if None not in parts:
                return any(parts) if operator == "||" else all(parts)
    if raw.startswith("!"):
        value = constant_boolean(raw[1:], constants)
        return None if value is None else not value
    objects_equal = re.fullmatch(r"Objects\.equals\((.+)\)", raw)
    equals = re.fullmatch(r'(\w+|"(?:\\.|[^"\\])*")\.equals\((.+)\)', raw) if not objects_equal else None
    if equals or objects_equal:
        arguments = list(equals.groups()) if equals else split_args(objects_equal[1])
        if len(arguments) != 2:
            return None
        values = [parse_value(argument, constants) for argument in arguments]
        if any(isinstance(value, str) and argument.strip() == value and argument.strip() not in constants
               for argument, value in zip(arguments, values)):
            return None
        return values[0] == values[1]
    compare = re.fullmatch(r"(.+?)\s*(==|!=)\s*(.+)", raw)
    if compare:
        left, right = (parse_value(compare[index], constants) for index in (1, 3))
        if type(left) not in (int, bool, type(None)) or type(right) not in (int, bool, type(None)):
            return None
        return left == right if compare[2] == "==" else left != right
    value = parse_value(raw, constants)
    return value if type(value) is bool else None


def split_top_level_operators(raw, operator):
    level = 0
    start = 0
    segments = []
    tokens = re.finditer(r'"(?:\\.|[^"\\])*"|[()]|' + re.escape(operator), raw)
    for token in tokens:
        if token[0] == "(":
            level += 1
        elif token[0] == ")":
            level -= 1
        elif token[0] == operator and level == 0:
            segments.append(raw[start:token.start()])
            start = token.end()
    return [*segments, raw[start:]]


def strip_comments(text):
    """Strip Java comments while preserving string literals.

    Card names like "Bottomless Pool // Locker Room" contain "//" inside
    strings; a naive regex strip truncates them and corrupts the constants.
    """
    out = []
    i = 0
    n = len(text)
    in_string = False
    escaped = False
    while i < n:
        char = text[i]
        if in_string:
            out.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            i += 1
            continue
        if char == '"':
            in_string = True
            out.append(char)
            i += 1
            continue
        if char == "/" and i + 1 < n and text[i + 1] == "/":
            while i < n and text[i] != "\n":
                i += 1
            continue
        if char == "/" and i + 1 < n and text[i + 1] == "*":
            end = text.find("*/", i + 2)
            i = n if end == -1 else end + 2
            continue
        out.append(char)
        i += 1
    return "".join(out)


def split_statements(text):
    statements = []
    current = []
    depth = 0
    in_string = False
    escaped = False
    for char in text:
        current.append(char)
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth = max(0, depth - 1)
        elif char == ";" and depth == 0:
            statements.append("".join(current[:-1]))
            current = []
    tail = "".join(current).strip()
    if tail:
        statements.append(tail)
    return statements


def parse_call(statement):
    match = re.match(r"(?:this\.)?(\w+)\s*\((.*)\)$", statement.strip(), flags=re.S)
    if not match:
        return None
    return match.group(1), split_args(match.group(2))


def split_args(raw):
    args = []
    current = []
    depth = 0
    in_string = False
    escaped = False
    for char in raw:
        if in_string:
            current.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
            current.append(char)
        elif char in "([{":
            depth += 1
            current.append(char)
        elif char in ")]}":
            depth -= 1
            current.append(char)
        elif char == "," and depth == 0:
            args.append("".join(current).strip())
            current = []
        else:
            current.append(char)
    tail = "".join(current).strip()
    if tail:
        args.append(tail)
    return args


def translate_call(name, args, constants, source, ability_vars=None):
    ability_vars = ability_vars or {}
    values = [parse_value(arg, constants) for arg in args]
    try:
        if name == "addCard":
            op = {
                "op": "addCard",
                "zone": values[0],
                "player": values[1],
                "name": values[2],
                "count": values[3] if len(values) > 3 else 1,
            }
            if len(values) == 5:
                if not isinstance(values[4], bool):
                    return {"op": "unsupported", "source": compact(source)}
                if values[4]:
                    op["tapped"] = True
            if len(values) > 5:
                return {"op": "unsupported", "source": compact(source)}
            return op
        if name == "addCustomEffect_TargetDestroy" and len(values) in (1, 2):
            count = values[1] if len(values) == 2 else 1
            if type(count) is int and 1 <= count <= 3:
                targets = {1: "target permanent", 2: "two target permanents", 3: "three target permanents"}
                return {"op": "addCard", "zone": "BATTLEFIELD", "player": values[0],
                        "name": "target destroy", "count": 1, "custom": True, "fixtureSource": True,
                        "typeLine": "", "oracleText": f"{{0}}: Destroy {targets[count]}."}
        if name == "addCustomEffect_AllDestroy" and len(values) == 1:
            return {"op": "addCard", "zone": "BATTLEFIELD", "player": values[0],
                    "name": "all destroy", "custom": True, "fixtureSource": True,
                    "typeLine": "", "oracleText": "{0}: Destroy all permanents."}
        if name == "addCustomEffect_TargetDamage" and len(values) == 2 and type(values[1]) is int and values[1] > 0:
            player, amount = values
            fixture = f"target damage {amount} for {args[0]}"
            return {"op": "addCard", "zone": "BATTLEFIELD", "player": player,
                    "name": fixture, "custom": True, "fixtureSource": True, "typeLine": "",
                    "oracleText": f"{{0}}: {fixture} deals {amount} damage to any target."}
        if name == "assertSuspected":
            return {
                "op": "assertSuspected",
                "name": values[0],
                "suspected": values[1],
            }
        if name == "assertLibrary":
            return {
                "op": "assertLibraryOrder",
                "player": values[0],
                "cards": values[1:],
            }
        if name == "setLife":
            return {"op": "setLife", "player": values[0], "life": values[1]}
        if name == "setStrictChooseMode":
            return {"op": "setStrictChooseMode", "value": values[0] if values else True}
        if name == "skipInitShuffling":
            return {"op": "skipInitShuffling"}
        if name == "setChoice":
            return {"op": "setChoice", "player": values[0], "value": values[1] if len(values) > 1 else True}
        if name == "setModeChoice":
            return {"op": "setModeChoice", "player": values[0], "value": values[1] if len(values) > 1 else None}
        if name == "addTarget":
            return {"op": "addTarget", "player": values[0], "target": values[1] if len(values) > 1 else None}
        if name == "addTargetAmount":
            if len(values) == 2 and values[1] == "TestPlayer.TARGET_SKIP":
                return {"op": "addTarget", "player": values[0], "target": values[1]}
            if len(values) == 3 and type(values[2]) is int and values[2] > 0:
                return {"op": "addTargetAmount", "player": values[0], "target": values[1], "amount": values[2]}
        if name == "castSpell":
            op = {"op": "castSpell", "turn": values[0], "phase": values[1], "player": values[2], "name": values[3]}
            target = first_target_arg(values[4:])
            if target is not None:
                op["target"] = target
            if len(values) in (5, 6) and values[-1] is True:
                op["resolve"] = True
            if has_stack_clause(values[4:], "StackClause.WHILE_NOT_ON_STACK"):
                op["waitForStack"] = "WHILE_NOT_ON_STACK"
            return op
        if name == "activateAbility":
            op = {"op": "activateAbility", "turn": values[0], "phase": values[1], "player": values[2], "ability": values[3]}
            target = first_target_arg(values[4:])
            if target is not None:
                op["target"] = target
            if has_stack_clause(values[4:], "StackClause.WHILE_NOT_ON_STACK"):
                op["waitForStack"] = "WHILE_NOT_ON_STACK"
            return op
        if name == "activateManaAbility":
            return {
                "op": "activateManaAbility",
                "turn": values[0],
                "phase": values[1],
                "player": values[2],
                "ability": values[3],
                "count": values[4] if len(values) > 4 else 1,
            }
        if name == "playLand":
            return {"op": "playLand", "turn": values[0], "phase": values[1], "player": values[2], "name": values[3]}
        if name == "attack":
            defender = values[3] if len(values) > 3 else 0 if values[1] == 1 else 1
            return {
                "op": "attack",
                "turn": values[0],
                "player": values[1],
                "attacker": values[2],
                "defender": defender,
            }
        if name == "block":
            return {"op": "block", "turn": values[0], "player": values[1], "blocker": values[2], "attacker": values[3]}
        if name == "setStopAt":
            return {"op": "setStopAt", "turn": values[0], "phase": values[1]}
        if name == "concede" and len(values) == 3 and args[1].startswith("PhaseStep."):
            return {"op": "concede", "turn": values[0], "phase": values[1], "player": values[2]}
        if name == "execute":
            return {"op": "execute"}
        if name == "waitStackResolved":
            op = {"op": "waitStackResolved", "turn": values[0], "phase": values[1], "player": None}
            if len(values) > 2:
                if type(values[2]) is bool:
                    op["once"] = values[2]
                elif args[2].strip() in {"playerA", "playerB", "playerC", "playerD"}:
                    op["player"] = values[2]
                    if len(values) > 3 and type(values[3]) is bool:
                        op["once"] = values[3]
                elif type(values[2]) is int:
                    op["resolutions"] = values[2]
                else:
                    return {"op": "unsupported", "source": compact(source)}
            return op
        if name == "removeAllCardsFromLibrary":
            return {"op": "clearZone", "player": values[0], "zone": "library"}
        if name == "removeAllCardsFromHand":
            return {"op": "clearZone", "player": values[0], "zone": "hand"}
        if name == "checkPlayableAbility":
            return {
                "op": "assertPlayableAbility",
                "turn": values[1],
                "phase": values[2],
                "player": values[3],
                "label": values[4],
                "expected": values[5],
            }
        if name == "checkMonarch" and len(values) == 5 and type(values[1]) is int:
            return {"op": "assertMonarch", "turn": values[1], "phase": values[2],
                    "player": values[3], "monarch": values[4]}
        if name == "checkStackSize":
            if len(values) == 5 and args[2].startswith("PhaseStep."):
                values = values[1:]
            return {
                "op": "assertStackSize",
                "turn": values[0],
                "phase": values[1],
                "player": values[2],
                "count": values[3],
            }
        if name == "assertLife":
            values = drop_leading_message(values, 2)
            return {"op": "assertLife", "player": values[0], "life": values[1]}
        if name == "checkLife" and len(values) == 5 and args[2].startswith("PhaseStep."):
            return {"op": "assertLife", "turn": values[1], "phase": values[2],
                    "player": values[3], "life": values[4], "seatIndependent": True}
        if name == "assertDamageReceived" and len(values) == 3:
            return {"op": "assertDamageReceived", "player": values[0], "name": values[1], "damage": values[2]}
        if name == "checkDamage" and len(values) == 6:
            turn, phase, player, card, damage = values[1:]
            return {"op": "assertDamageReceived", "turn": turn, "phase": phase, "player": player,
                    "name": card, "damage": damage}
        if name == "assertResult" and len(values) == 2 and values[1] in {"GameResult.WON", "GameResult.LOST", "GameResult.DRAW"}:
            return {"op": "assertResult", "player": values[0], "result": values[1].split(".")[1]}
        player_state = {
            "assertWonTheGame": ("hasWon", True), "assertHasNotWonTheGame": ("hasWon", False),
            "assertLostTheGame": ("hasLost", True), "assertHasNotLostTheGame": ("hasLost", False),
        }
        if name in player_state and len(values) == 1:
            property_name, expected = player_state[name]
            return {"op": "assertPlayerState", "player": values[0], "property": property_name, "expected": expected}
        if name == "checkPlayerInGame" and len(values) == 6 and isinstance(values[5], bool):
            _, turn, phase, player, target, in_game = values
            return {"op": "assertPlayerState", "turn": turn, "phase": phase, "player": player,
                    "targetPlayer": target, "property": "hasLeftGame", "expected": not in_game}
        if name == "checkType" and len(values) == 7 and args[5].startswith("CardType.") and isinstance(values[6], bool):
            return {"op": "assertType", "turn": values[1], "phase": values[2], "player": values[3],
                    "name": values[4], "cardType": args[5].split(".")[1], "extra": str(values[6]).lower()}
        if name == "checkStackObject" and len(values) == 6:
            return {"op": "assertStackObject", "turn": values[1], "phase": values[2], "player": values[3],
                    "text": values[4], "count": values[5]}
        if name in {"assertPermanentCount", "checkPermanentCount"}:
            if name == "checkPermanentCount" and len(values) == 7 and args[2].startswith("PhaseStep."):
                _, turn, phase, player, target, card, count = values
                return {"op": "assertPermanentCount", "turn": turn, "phase": phase, "player": player,
                        "targetPlayer": target, "name": card, "count": count}
            return count_assert("assertPermanentCount", values)
        if name in {"assertHandCount", "checkHandCount", "checkHandCardCount"}:
            return count_assert("assertHandCount", values)
        if name in {"assertGraveyardCount", "checkGraveyardCount"}:
            return count_assert("assertGraveyardCount", values)
        if name in {"assertExileCount", "checkExileCount"}:
            return count_assert("assertExileCount", values)
        if name in {"assertLibraryCount", "checkLibraryCount"}:
            return count_assert("assertLibraryCount", values)
        if name == "assertTopCardRevealed" and len(values) == 2 and isinstance(values[1], bool):
            return {"op": "assertTopCardRevealed", "player": values[0], "expected": values[1]}
        if name in {"assertPowerToughness", "checkPT", "assertBasePowerToughness"}:
            op = power_toughness_assert(values)
            if name == "assertBasePowerToughness":
                op["op"] = name
            return op
        if name == "assertTappedCount":
            values = drop_leading_message(values, 3)
            return {"op": "assertTappedCount", "name": values[0], "tapped": values[1], "count": values[2]}
        if name == "assertTapped" and len(values) == 2 and isinstance(values[1], bool):
            return {"op": "assertTapped", "name": values[0], "tapped": values[1]}
        if name == "checkPermanentTapped" and len(values) in {7, 8} and isinstance(values[-2], bool):
            turn, phase, player = values[1:4]
            card, tapped, count = values[-3:]
            operation = {"op": "assertTappedCount", "turn": turn, "phase": phase, "player": player,
                         "name": card, "tapped": tapped, "count": count}
            if len(values) == 8:
                operation["targetPlayer"] = values[4]
            else:
                operation["seatIndependent"] = True
            return operation
        if name == "assertTrue" and len(args) == 1:
            tapped = re.fullmatch(r"getPermanent\((.+),\s*(player[A-D])\)\.isTapped\(\)", args[0])
            if tapped:
                return {"op": "assertTapped", "player": parse_value(tapped[2], constants),
                        "name": parse_value(tapped[1], constants), "tapped": True}
        if name == "assertCounterCount":
            values = drop_leading_message(values, 3)
            if len(values) == 3:
                return {"op": "assertCounterCount", "name": values[0], "counter": values[1], "count": values[2]}
            return {"op": "assertCounterCount", "player": values[0], "name": values[1], "counter": values[2], "count": values[3]}
        if name == "checkPermanentCounters" and len(values) == 7 and args[2].startswith("PhaseStep."):
            _, turn, phase, player, card, counter, count = values
            return {"op": "assertCounterCount", "turn": turn, "phase": phase, "player": player,
                    "name": card, "counter": counter, "count": count, "seatIndependent": True}
        if name == "addCounters" and len(values) == 6:
            turn, phase, player, card, counter, count = values
            if type(turn) is int and type(count) is int and re.fullmatch(r"[A-Z][A-Z0-9_]*", str(counter)):
                return {"op": "addObjectCounters", "turn": turn, "phase": phase, "player": player,
                        "name": card, "counter": counter, "count": count}
        if name == "checkAbility":
            if len(values) == 7:
                values = values[1:]
            if len(values) != 6 or not isinstance(values[-1], bool):
                return {"op": "unsupported", "source": compact(source)}
            turn, phase, player, card, ability, expected = values
            return {
                "op": "assertAbility", "turn": turn, "phase": phase, "player": player,
                "name": card, "ability": ability, "expected": expected,
            }
        if name == "assertAbility":
            values = drop_leading_message(values, 4)
            return {
                "op": "assertAbility",
                "player": values[0],
                "name": values[1],
                "ability": values[2],
                "expected": values[3] if len(values) > 3 else True,
            }
        if name == "assertAbilityCount" and len(values) == 4:
            player, card, ability_class, count = values
            ability_patterns = {
                "SimpleActivatedAbility.class": ("activated", None),
                "ActivatedAbility.class": ("activated", None),
                "ColorlessManaAbility.class": ("activated", r"^\{T\}:\s*Add\s+\{C\}\.?$"),
                "RedManaAbility.class": ("activated", r"^\{T\}:\s*Add\s+\{R\}\.?$"),
                "SagaAbility.class": ("triggered", r"^[IVXLCDM]+(?:\s*,\s*[IVXLCDM]+)*\s*[—–-]"),
                "BeginningOfUpkeepTriggeredAbility.class": ("triggered", r"^(?:∞\s*[—–-]\s*)?At the beginning of [^,]+upkeep\b"),
                "WerewolfFrontTriggeredAbility.class": ("triggered", r"if no spells were cast last turn,\s*transform\b"),
            }
            if ability_class in ability_patterns and type(count) is int:
                kind, pattern = ability_patterns[ability_class]
                operation = {"op": "assertAbilityKindCount", "player": player, "name": card, "kind": kind, "count": count}
                if pattern:
                    operation["textPattern"] = pattern
                return operation
        if name == "assertMultipleInstancesOfAbility" and len(values) == 4:
            player, card, amount, count = values
            if re.fullmatch(r"new\s+PoisonousAbility\(\s*\d+\s*\)", args[2]) and type(count) is int:
                return {"op": "assertAbilityKindCount", "player": player, "name": card, "kind": "triggered",
                        "count": count, "textPattern": rf"^Poisonous\s+{amount}$"}
        if name == "assertAbilities":
            abilities = ability_vars.get(str(values[2]), values[2] if len(values) > 2 else [])
            if isinstance(abilities, str):
                abilities = [abilities]
            return {
                "op": "assertAbilities",
                "player": values[0],
                "name": values[1],
                "abilities": abilities,
            }
        if name == "addCustomCardWithAbility":
            # MAGE signatures:
            #   (name, player, ability)
            #   (name, player, extraAbility, removeDefault, cardType, cost, zone, subTypes...)
            ability_raw = compact(args[2]).strip() if len(args) > 2 else "null"
            oracle_text = custom_fixture_text(ability_raw)
            if len(values) >= 7:
                if values[3] is not None:
                    return {"op": "unsupported", "source": compact(source)}
                card_type = str(values[4]).split(".")[-1].title()
                subtypes = [str(v).split(".")[-1].title() for v in values[7:]]
                type_line = card_type + (" - " + " ".join(subtypes) if subtypes else "")
                if ability_raw not in ("null", "") and oracle_text is None:
                    return {"op": "unsupported", "source": compact(source)}
                op = {
                    "op": "addCard",
                    "zone": str(values[6]).split(".")[-1],
                    "player": values[1],
                    "name": values[0],
                    "custom": True,
                    "typeLine": type_line,
                    "manaCost": values[5],
                    "oracleText": oracle_text or "",
                }
                if ability_raw not in CUSTOM_ABILITY_ORACLE_TEXT:
                    op["fixtureSource"] = True
                op["power"] = op["toughness"] = "1" if card_type == "Creature" else None
                return op
            if oracle_text is not None:
                return {
                    "op": "addCard",
                    "zone": "BATTLEFIELD",
                    "player": values[1],
                    "name": values[0],
                    "custom": True,
                    "fixtureSource": True,
                    "typeLine": "",
                    "power": None,
                    "toughness": None,
                    "oracleText": oracle_text,
                }
            return {"op": "unsupported", "source": compact(source)}
        if name in IGNORED_CALLS:
            return None
    except Exception:
        return {"op": "unsupported", "source": compact(source)}

    if name.startswith(("assert", "check", "cast", "activate", "add", "set", "play", "remove", "wait", "rollback", "concede")):
        return {"op": "unsupported", "source": compact(source)}
    return None


def count_assert(op, values):
    if (
        len(values) == 5
        and isinstance(values[0], str)
        and type(values[1]) is int
        and isinstance(values[2], str)
        and type(values[4]) is int
    ):
        return {"op": op, "turn": values[1], "phase": values[2], "player": values[3], "count": values[4]}
    if (
        len(values) >= 6
        and isinstance(values[0], str)
        and isinstance(values[1], int)
        and isinstance(values[2], str)
    ):
        return {"op": op, "turn": values[1], "phase": values[2], "player": values[3], "name": values[4], "count": values[5]}
    if (
        len(values) >= 5
        and isinstance(values[0], int)
        and isinstance(values[1], str)
    ):
        return {"op": op, "turn": values[0], "phase": values[1], "player": values[2], "name": values[3], "count": values[4]}
    values = drop_leading_message(values, 2)
    if len(values) == 1:
        return {"op": op, "player": 0, "count": values[0]}
    if len(values) == 2:
        if isinstance(values[0], str) and re.fullmatch(r"player[A-D]", values[0]):
            return {"op": op, "player": values[0], "count": values[1]}
        if isinstance(values[0], int) and isinstance(values[1], str):
            return {"op": op, "player": 0, "count": values[0], "name": values[1]}
        if isinstance(values[0], str) and isinstance(values[1], int):
            return {"op": op, "name": values[0], "count": values[1]}
        return {"op": op, "player": values[0], "count": values[1]}
    return {"op": op, "player": values[0], "name": values[1], "count": values[2]}


def power_toughness_assert(values):
    if (
        len(values) >= 7
        and isinstance(values[0], str)
        and isinstance(values[1], int)
        and isinstance(values[2], str)
    ):
        return {
            "op": "assertPowerToughness",
            "turn": values[1],
            "phase": values[2],
            "player": values[3],
            "name": values[4],
            "power": values[5],
            "toughness": values[6],
        }
    if (
        len(values) >= 6
        and isinstance(values[0], int)
        and isinstance(values[1], str)
    ):
        return {
            "op": "assertPowerToughness",
            "turn": values[0],
            "phase": values[1],
            "player": values[2],
            "name": values[3],
            "power": values[4],
            "toughness": values[5],
        }
    values = drop_leading_message(values, 4)
    return {
        "op": "assertPowerToughness",
        "player": values[0],
        "name": values[1],
        "power": values[2],
        "toughness": values[3],
    }


def drop_leading_message(values, minimum_after_drop):
    if (
        len(values) > minimum_after_drop
        and isinstance(values[0], str)
        and not re.fullmatch(r"player[A-D]", values[0])
        and (len(values) < 2 or values[1] in (0, 1) or not isinstance(values[1], int))
    ):
        if len(values) - 1 >= minimum_after_drop:
            return values[1:]
    return values


def first_target_arg(values):
    for value in values:
        if isinstance(value, bool) or value is None:
            continue
        if isinstance(value, str) and (value.startswith("StackClause.") or value.startswith("TargetController.")):
            continue
        return value
    return None


def has_stack_clause(values, clause):
    return any(isinstance(value, str) and value == clause for value in values)


def parse_value(raw, constants):
    raw = raw.strip()
    if raw in constants:
        return constants[raw]
    arithmetic = parse_integer_arithmetic(raw, constants)
    if arithmetic is not None:
        return arithmetic
    concat_parts = split_java_concat(raw)
    if concat_parts and len(concat_parts) > 1:
        parsed = [parse_value(part, constants) for part in concat_parts]
        if all(isinstance(value, str) for value in parsed):
            return "".join(parsed)
    player = re.fullmatch(r"player([A-D])", raw)
    if player:
        return ord(player[1]) - ord("A")
    if raw == "true":
        return True
    if raw == "false":
        return False
    if raw == "null":
        return None
    ternary = re.fullmatch(r"(\w+)\s*\?\s*(-?\d+)\s*:\s*(-?\d+)", raw)
    if ternary:
        predicate = constants.get(ternary[1], {"true": True, "false": False}.get(ternary[1]))
        if isinstance(predicate, bool):
            return int(ternary[2] if predicate else ternary[3])
    string = re.match(r'^"((?:\\.|[^"\\])*)"$', raw, flags=re.S)
    if string:
        return bytes(string.group(1), "utf-8").decode("unicode_escape")
    integer = re.match(r"^-?\d+$", raw)
    if integer:
        return int(raw)
    enum = re.match(r"^(?:Zone|PhaseStep|CounterType|AbilityKey|TargetController)\.(\w+)$", raw)
    if enum:
        return enum.group(1)
    constructor = re.match(r"new\s+\w+\((.*)\)$", raw)
    if constructor:
        args = split_args(constructor.group(1))
        if args:
            return parse_value(args[0], constants)
    method_call = re.match(r"(\w+)\.getInstance\(\)$", raw)
    if method_call:
        return method_call.group(1).replace("Ability", "")
    return raw


def parse_integer_arithmetic(raw, constants):
    if not re.fullmatch(r"[\w\s()+*-]+", raw) or not re.search(r"[+*()-]", raw):
        return None

    def evaluate(node):
        if isinstance(node, ast.Constant) and type(node.value) is int:
            return node.value
        if isinstance(node, ast.Name) and type(constants.get(node.id)) is int:
            return constants[node.id]
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.UAdd, ast.USub)):
            value = evaluate(node.operand)
            return value if isinstance(node.op, ast.UAdd) else -value
        if isinstance(node, ast.BinOp):
            left, right = evaluate(node.left), evaluate(node.right)
            if isinstance(node.op, ast.Add):
                return left + right
            if isinstance(node.op, ast.Sub):
                return left - right
            if isinstance(node.op, ast.Mult):
                return left * right
        raise ValueError

    try:
        return evaluate(ast.parse(raw, mode="eval").body)
    except (SyntaxError, ValueError):
        return None


def split_java_concat(raw):
    parts = []
    current = []
    depth = 0
    in_string = False
    escaped = False
    for char in raw:
        if in_string:
            current.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
            current.append(char)
        elif char in "([{":
            depth += 1
            current.append(char)
        elif char in ")]}":
            depth -= 1
            current.append(char)
        elif char == "+" and depth == 0:
            parts.append("".join(current).strip())
            current = []
        else:
            current.append(char)
    if parts:
        parts.append("".join(current).strip())
    return parts


def compact(statement):
    return re.sub(r"\s+", " ", statement).strip()


if __name__ == "__main__":
    main()
