#!/usr/bin/env python3
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

        rel = source.relative_to(SOURCE_ROOT)
        out = OUTPUT_ROOT / rel.with_suffix(".test.mjs")
        out.parent.mkdir(parents=True, exist_ok=True)
        test_specs = []
        for name, body, ignore_reason in tests:
            operations, unsupported = translate_body(body, file_constants, helper_methods)
            unsupported_statements += unsupported
            spec = {"name": name, "operations": operations}
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
        r'(?:(?:public|private|protected)\s+)?(?:(?:static|final)\s+)*(?:String|int|boolean)\s+(\w+)\s*=\s*([^;]+);',
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
            r"\b(" + "|".join(re.escape(name) for name in bindings) + r")\b"
        )
        return pattern.sub(lambda m: bindings[m.group(1)], body)
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
        ability_decl = re.match(r"(?:Abilities<[^>]+>|List<[^>]+>)\s+(\w+)\s*=\s*(?:new\s+AbilitiesImpl<[^>]*>\(\)|new\s+AbilitiesImpl\(\)|Arrays\.asList\((.*)\))$", statement, re.S)
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
        assignment = re.match(r'(?:(?:final|static)\s+)*(?:String|int|boolean)\s+(\w+)\s*=\s*(.+)$', statement)
        if assignment:
            constants[assignment.group(1)] = parse_value(assignment.group(2), constants)
            continue
        if statement.startswith(("logger.", "System.", "Assert.", "Assume.")):
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
            if statement.startswith(("if ", "if(", "for ", "for(", "try", "catch", "while ")):
                operations.append({"op": "unsupported", "source": compact(statement)})
                unsupported += 1
            continue
        name, args = call
        op = translate_call(name, args, constants, statement, ability_vars)
        if op is None:
            continue
        if op.get("op") == "unsupported":
            unsupported += 1
        operations.append(op)
    return operations, unsupported


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
            return {
                "op": "addCard",
                "zone": values[0],
                "player": values[1],
                "name": values[2],
                "count": values[3] if len(values) > 3 else 1,
            }
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
        if name == "castSpell":
            op = {"op": "castSpell", "turn": values[0], "phase": values[1], "player": values[2], "name": values[3]}
            target = first_target_arg(values[4:])
            if target is not None:
                op["target"] = target
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
            return {
                "op": "attack",
                "turn": values[0],
                "player": values[1],
                "attacker": values[2],
                "defender": values[3] if len(values) > 3 else 1,
            }
        if name == "block":
            return {"op": "block", "turn": values[0], "player": values[1], "blocker": values[2], "attacker": values[3]}
        if name == "setStopAt":
            return {"op": "setStopAt", "turn": values[0], "phase": values[1]}
        if name == "execute":
            return {"op": "execute"}
        if name == "waitStackResolved":
            player = values[2] if len(values) > 2 and not isinstance(values[2], bool) else None
            op = {"op": "waitStackResolved", "turn": values[0], "phase": values[1], "player": player}
            if len(values) > 2 and isinstance(values[2], bool):
                op["once"] = values[2]
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
        if name == "checkStackSize":
            values = drop_leading_message(values, 4)
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
        if name in {"assertPermanentCount", "checkPermanentCount"}:
            return count_assert("assertPermanentCount", values)
        if name in {"assertHandCount", "checkHandCount", "checkHandCardCount"}:
            return count_assert("assertHandCount", values)
        if name in {"assertGraveyardCount", "checkGraveyardCount"}:
            return count_assert("assertGraveyardCount", values)
        if name in {"assertExileCount", "checkExileCount"}:
            return count_assert("assertExileCount", values)
        if name == "assertLibraryCount":
            return count_assert("assertLibraryCount", values)
        if name in {"assertPowerToughness", "checkPT"}:
            return power_toughness_assert(values)
        if name == "assertTappedCount":
            values = drop_leading_message(values, 3)
            return {"op": "assertTappedCount", "name": values[0], "tapped": values[1], "count": values[2]}
        if name == "assertCounterCount":
            values = drop_leading_message(values, 3)
            if len(values) == 3:
                return {"op": "assertCounterCount", "player": 0, "name": values[0], "counter": values[1], "count": values[2]}
            return {"op": "assertCounterCount", "player": values[0], "name": values[1], "counter": values[2], "count": values[3]}
        if name in {"assertAbility", "checkAbility"}:
            values = drop_leading_message(values, 4)
            return {
                "op": "assertAbility",
                "player": values[0],
                "name": values[1],
                "ability": values[2],
                "expected": values[3] if len(values) > 3 else True,
            }
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
            oracle_text = CUSTOM_ABILITY_ORACLE_TEXT.get(re.sub(r"\s+", " ", ability_raw))
            if len(values) >= 7:
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
                if card_type != "Creature":
                    op["power"] = None
                    op["toughness"] = None
                return op
            if oracle_text is not None:
                return {
                    "op": "addCard",
                    "zone": "BATTLEFIELD",
                    "player": values[1],
                    "name": values[0],
                    "custom": True,
                    "typeLine": "Enchantment",
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
    concat_parts = split_java_concat(raw)
    if concat_parts and len(concat_parts) > 1:
        parsed = [parse_value(part, constants) for part in concat_parts]
        if all(isinstance(value, str) for value in parsed):
            return "".join(parsed)
    if raw == "playerA":
        return 0
    if raw == "playerB":
        return 1
    if raw == "true":
        return True
    if raw == "false":
        return False
    if raw == "null":
        return None
    arithmetic = parse_integer_arithmetic(raw, constants)
    if arithmetic is not None:
        return arithmetic
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
    parts = re.findall(r"[+-]?[^+-]+", raw)
    if len(parts) <= 1:
        return None
    total = 0
    for part in parts:
        part = part.strip()
        sign = -1 if part.startswith("-") else 1
        part = part.lstrip("+-").strip()
        value = constants.get(part, part)
        if isinstance(value, bool):
            return None
        if isinstance(value, int):
            total += sign * value
            continue
        if isinstance(value, str) and re.match(r"^\d+$", value):
            total += sign * int(value)
            continue
        return None
    return total


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
