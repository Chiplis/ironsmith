#!/usr/bin/env python3
"""Apply the shared cache setup to the main repo and its sibling worktrees."""
import argparse
import datetime
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tomllib


def configure(text):
    """Preserve unrelated settings; refuse to replace another compiler wrapper."""
    parsed = tomllib.loads(text)
    wrapper = parsed.get("build", {}).get("rustc-wrapper")
    if wrapper not in (None, "", "scripts/rustc-cache.sh"):
        raise ValueError(f"existing rustc-wrapper {wrapper!r}; reconcile it manually")
    if not re.search(r"(?m)^\[build\]\s*$", text):
        text += "\n[build]\n"
    match = re.search(r"(?m)^\[build\]\s*$", text)
    start = match.end()
    following = re.search(r"(?m)^\[", text[start:])
    end = start + following.start() if following else len(text)
    section = text[start:end]
    for key, value in (("incremental", "false"), ("rustc-wrapper", '"scripts/rustc-cache.sh"')):
        pattern = rf"(?m)^{re.escape(key)}\s*=.*$"
        setting = f"{key} = {value}"
        if re.search(pattern, section):
            section = re.sub(pattern, setting, section)
        else:
            section = "\n" + setting + section
    result = text[:start] + section.rstrip() + "\n\n" + text[end:]
    updated = tomllib.loads(result)["build"]
    assert updated["incremental"] is False
    assert updated["rustc-wrapper"] == "scripts/rustc-cache.sh"
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="write changes (default: preview)")
    parser.add_argument("--clean-incremental", action="store_true",
                        help="remove old target/**/incremental directories; requires --apply and idle builds")
    args = parser.parse_args()
    if args.clean_incremental and not args.apply:
        parser.error("--clean-incremental requires --apply; stop all Rust builds first")
    if args.clean_incremental:
        processes = subprocess.check_output(["ps", "-axo", "comm="], text=True)
        if any(Path(line.strip()).name in ("cargo", "rustc", "rustdoc") for line in processes.splitlines()):
            raise SystemExit("Rust builds are running; stop them before cleaning incremental data.")
    root = Path(__file__).resolve().parent.parent
    common = Path(subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "--path-format=absolute", "--git-common-dir"], text=True).strip())
    primary = common.parent
    records = subprocess.check_output(
        ["git", "-C", str(root), "worktree", "list", "--porcelain", "-z"]).decode().split("\0")
    roots = [Path(line.removeprefix("worktree ")) for line in records if line.startswith("worktree ")]
    roots = [p for p in roots if p == primary or
             (p.parent == primary.parent and p.name.startswith(primary.name + "-"))]
    wrapper = (root / "scripts/rustc-cache.sh").read_bytes()
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    backup = primary / "target/cache-setup-backups" / stamp
    # Validate every destination before writing any of them.
    pending = []
    for path in roots:
        config = path / ".cargo/config.toml"
        if (path / ".cargo/config").exists():
            raise SystemExit(f"Refusing {path}: legacy .cargo/config takes precedence")
        if not path.is_dir():
            continue
        original = config.read_text() if config.exists() else ""
        try:
            updated = configure(original)
        except ValueError as error:
            raise SystemExit(f"{config}: {error}") from error
        pending.append((path, config, original, updated))
    for path, config, original, updated in pending:
        print(f"{'Configuring' if args.apply else 'Would configure'} {path}")
        if not args.apply:
            continue
        script = path / "scripts/rustc-cache.sh"
        if original != updated or not script.exists() or script.read_bytes() != wrapper:
            saved = backup / hashlib.sha256(str(path).encode()).hexdigest()[:16]
            saved.mkdir(parents=True)
            (saved / "worktree.txt").write_text(str(path) + "\n")
            if config.exists():
                shutil.copy2(config, saved / "config.toml")
            if script.exists():
                shutil.copy2(script, saved / "rustc-cache.sh")
            config.parent.mkdir(parents=True, exist_ok=True)
            config.write_text(updated)
            script.parent.mkdir(parents=True, exist_ok=True)
            script.write_bytes(wrapper)
            script.chmod(0o755)
    if args.apply:
        print(f"Configured {len(pending)} worktrees. Backups (if changed): {backup}")
    if args.clean_incremental:
        # Do not start builds during cleanup. Never follow target symlinks.
        for path, *_ in pending:
            target = path / "target"
            if target.is_symlink():
                continue
            for directory, dirs, _ in os.walk(target, followlinks=False):
                if "incremental" in dirs:
                    cache = Path(directory) / "incremental"
                    dirs.remove("incremental")
                    if not cache.is_symlink():
                        print(f"Removing {cache}")
                        shutil.rmtree(cache)


if __name__ == "__main__":
    main()
